#!/usr/bin/env python3
"""Focused executor and wrapper tests for S1C-3D."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import s1c3d_remote_transaction_v1 as executor
import s1c3d_transaction_v1 as wrapper


VALID_ATTEMPT = "20260812T170000Z-c3eaddc55dfc-s1c3d-v1"


def command(returncode: int = 0) -> dict[str, object]:
    return {
        "observed_affinity": [executor.base.MEASUREMENT_CPU],
        "returncode": returncode,
        "executable_sha256": executor.FROZEN_TRANSITION_TEST_EXECUTABLE_SHA256,
    }


def metric_row(label: str, kind: str, *, p99: int, hard_max: int, returncode: int = 0) -> dict[str, object]:
    if kind == "single":
        metrics = {"p99_ns": p99, "hard_max_ns": hard_max, "samples": 1024, "segments": 5}
        test = executor.base.SINGLE_SYNC_TEST
    else:
        metrics = {
            "precommit_p99_ns": 3_000_000,
            "precommit_hard_max_ns": 4_000_000,
            "settlement_p99_ns": p99,
            "settlement_hard_max_ns": hard_max,
            "episode_p99_ns": max(p99, 6_000_000),
            "episode_hard_max_ns": max(hard_max, 8_000_000),
            "samples": 256,
        }
        test = executor.base.THREE_SYNC_TEST
    return {
        "label": label,
        "test": test,
        "returncode": returncode,
        "test_assertion_pass": returncode == 0,
        "metrics": metrics,
        "command": command(returncode),
    }


def resource(three: dict[str, object]) -> dict[str, object]:
    hot = {
        "label": "hot-1",
        "returncode": 0,
        "test_assertion_pass": True,
        "metrics": {"p99_ns": 10_000, "no_goal_p99_ns": 500, "hard_max_ns": 20_000, "samples": 4096},
        "command": command(),
    }
    single = metric_row("single-sync-1", "single", p99=3_000_000, hard_max=4_000_000)
    idle = {
        "label": "idle",
        "returncode": 0,
        "test_assertion_pass": True,
        "metrics": {"elapsed_ticks": 0, "ticks_per_second": 100, "percent_of_one_core": 0.0},
        "command": command(),
    }
    return {
        "floor_probes": [
            {"label": f"floor-{position}-{index}", "records": executor.base.FLOOR_RECORDS, "returncode": 0, "error": None}
            for index in range(1, 4)
            for position in ("before", "after")
        ],
        "metrics": {
            "hot_latency": [dict(hot, label=f"hot-{index}") for index in range(1, 4)],
            "single_ledger_sync": [dict(single, label=f"single-sync-{index}") for index in range(1, 4)],
            "three_ledger_sync": [
                dict(metric_row(f"three-sync-{index}", "three", p99=4_000_000, hard_max=7_000_000))
                for index in (1, 3)
            ] + [three],
            "idle_cpu": idle,
            "rss": {
                "delta_bytes": 0,
                "rows": [
                    {"label": "capture_off", "rss_bytes": 10, "sample_count": 20, "error": None},
                    {"label": "capture_on", "rss_bytes": 10, "sample_count": 20, "error": None},
                ],
            },
        },
        "_s1c3d_parity": {"byte_identical": True, "row_count": 16},
    }


class ClassificationTests(unittest.TestCase):
    def classify(self, row: dict[str, object], log: str) -> dict[str, object]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            (source / executor.GROUNDED_CAPTURE_SOURCE.parent).mkdir(parents=True)
            (source / executor.GROUNDED_CAPTURE_SOURCE).write_text("frozen")
            evidence = root / "evidence"
            evidence.mkdir()
            (evidence / f'{row["label"]}.log').write_text(log)
            with mock.patch.object(executor.base, "sha256_file", return_value=executor.FROZEN_GROUNDED_CAPTURE_SOURCE_SHA256):
                return executor.classify_resource(resource(row), source, evidence)

    def test_small_target_deviation_is_watch_not_veto(self) -> None:
        row = metric_row("three-sync-2", "three", p99=5_010_709, hard_max=7_000_000, returncode=101)
        result = self.classify(
            row,
            "panicked at source:1:\nsettlement sync p99 exceeded 5 ms\n"
            "test result: FAILED. 0 passed; 1 failed; 0 ignored\n",
        )
        self.assertEqual(result["hard_gate_status"], "PASS")
        self.assertEqual(result["optimization_status"], "OPTIMIZATION_WATCH")
        self.assertEqual(result["legacy_target_assertions"], ["three-sync-2"])
        self.assertEqual(result["optimization_watches"][0]["observed_ns"], 5_010_709)

    def test_hard_max_above_twenty_ms_is_safety_veto(self) -> None:
        row = metric_row("three-sync-2", "three", p99=5_010_709, hard_max=20_000_001, returncode=101)
        result = self.classify(row, "panicked at source:1:\nsettlement sync p99 exceeded 5 ms\n")
        self.assertEqual(result["hard_gate_status"], "VETO")
        self.assertIn("three-sync-2:settlement_hard_max_ns", result["operational_safety_failures"])

    def test_unrelated_panic_is_correctness_veto(self) -> None:
        row = metric_row("three-sync-2", "three", p99=5_010_709, hard_max=7_000_000, returncode=101)
        result = self.classify(row, "panicked at source:1:\nunrelated invariant failed\n")
        self.assertEqual(result["hard_gate_status"], "VETO")
        self.assertIn("three-sync-2:test_assertion_failed", result["correctness_failures"])

    def test_snapshot_probe_contract_covers_files_and_directory(self) -> None:
        source = executor.SNAPSHOT_PERMISSION_PROBE
        for marker in ("chmod", "write", "unlink", "rename"):
            self.assertIn(f'"{marker}"', source)
        self.assertIn("SNAPSHOT_PARENT", Path(executor.__file__).read_text())


class WrapperTests(unittest.TestCase):
    def test_attempt_namespace_is_new(self) -> None:
        wrapper.require_attempt(VALID_ATTEMPT)
        with self.assertRaisesRegex(executor.GateFailure, "attempt_id"):
            wrapper.require_attempt("20260812T113705Z-2a1505055ce9-s1c3c-v1")

    def test_predeployment_is_verified_before_projection_and_mutation(self) -> None:
        source = Path(wrapper.__file__).read_text()
        execute = source.split("def execute(", 1)[1].split("def rollback(", 1)[0]
        self.assertLess(execute.index("verify_predeployment("), execute.index("_legacy_predeployment_projection("))
        self.assertLess(execute.index("_legacy_predeployment_projection("), execute.index("mechanism.execute("))

    def test_preflight_abort_is_fail_closed(self) -> None:
        source = Path(wrapper.__file__).read_text()
        abort = source.split("def abort_predeployment(", 1)[1].split("def seal(", 1)[0]
        for marker in (
            '"production_mutation": False',
            '"capture_installed": False',
            '"scientific_authority": False',
            '"s1c4_state": "CLOSED"',
        ):
            self.assertIn(marker, abort)

    def test_resource_veto_is_sealed_without_mutation(self) -> None:
        source = Path(wrapper.__file__).read_text()
        seal = source.split("def seal_resource_veto(", 1)[1].split("def locked(", 1)[0]
        self.assertIn('"S1C3D_CORRECTNESS_VETO"', seal)
        self.assertIn('"S1C3D_SAFETY_VETO"', seal)
        self.assertIn('"production_mutation": False', seal)
        self.assertIn('"s1c4_state": "CLOSED"', seal)

    def test_launcher_gates_precede_attempt_side_effects(self) -> None:
        source = (Path(wrapper.__file__).resolve().parent / "run_s1c3d_transaction_v1.sh").read_text()
        test_gate = source.index("python3 -m unittest")
        compile_gate = source.index("python3 -m py_compile")
        push_gate = source.index("git ls-remote origin")
        for marker in (
            "timestamp=$(date",
            "install -d -m 0700 \"$local_dir\"",
            "prior_attempts=$(ssh",
            "connector_snapshot before",
            "git bundle create",
        ):
            side_effect = source.index(marker)
            self.assertLess(test_gate, side_effect, marker)
            self.assertLess(compile_gate, side_effect, marker)
            self.assertLess(push_gate, side_effect, marker)

    def test_launcher_shell_syntax(self) -> None:
        completed = subprocess.run(
            ["bash", "-n", str(Path(wrapper.__file__).resolve().parent / "run_s1c3d_transaction_v1.sh")],
            check=False,
        )
        self.assertEqual(completed.returncode, 0)


if __name__ == "__main__":
    unittest.main()
