#!/usr/bin/env python3

from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("verify_s1c1_resource_v3.py")
SPEC = importlib.util.spec_from_file_location("verify_s1c1_resource_v3", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
VERIFY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFY)


def run(
    ordinal: int,
    *,
    test: str,
    binary: str,
    matched: int,
    no_match_field: str,
    no_match: int,
    hard_max: int,
) -> dict[str, object]:
    passed = matched <= 1_000_000 and no_match <= 250_000 and hard_max <= 2_000_000
    return {
        "ordinal": ordinal,
        "test": test,
        "binary_sha256": binary,
        "samples": 4_096,
        "matched_p99_ns": matched,
        no_match_field: no_match,
        "hard_max_ns": hard_max,
        "exit_code": 0 if passed else 101,
    }


def passing_document() -> dict[str, object]:
    targeted = [
        run(
            ordinal,
            test=VERIFY.TARGETED_TEST,
            binary=VERIFY.TARGETED_BINARY_SHA256,
            matched=25_000,
            no_match_field="no_goal_p99_ns",
            no_match=2_000,
            hard_max=40_000,
        )
        for ordinal in range(1, 4)
    ]
    pairs = []
    for ordinal, order in enumerate(VERIFY.PAIR_ORDERS, start=1):
        baseline = run(
            ordinal,
            test=VERIFY.INHERITED_TEST,
            binary=VERIFY.BASELINE_BINARY_SHA256,
            matched=1_050_000,
            no_match_field="no_match_p99_ns",
            no_match=260_000,
            hard_max=1_500_000,
        )
        candidate = run(
            ordinal,
            test=VERIFY.INHERITED_TEST,
            binary=VERIFY.CANDIDATE_BINARY_SHA256,
            matched=1_040_000,
            no_match_field="no_match_p99_ns",
            no_match=255_000,
            hard_max=1_490_000,
        )
        pairs.append(
            {
                "ordinal": ordinal,
                "order": order,
                "baseline": baseline,
                "candidate": candidate,
            }
        )
    return {
        "schema": VERIFY.SCHEMA,
        "protocol_epoch_root": VERIFY.PROTOCOL_EPOCH_ROOT,
        "protocol_parent_commit": VERIFY.PROTOCOL_PARENT_COMMIT,
        "protocol_commit": "1" * 40,
        "source_manifest_root": VERIFY.SOURCE_MANIFEST_ROOT,
        "evidence_directory_name": f"s1c1-v3-{'1' * 8}",
        "evidence_manifest_sha256": "2" * 64,
        "targeted_runs": targeted,
        "inherited_pairs": pairs,
        "safety": {
            "production_service_survival": True,
            "connector_survival": True,
            "false_accepts": 0,
            "runtime_parity_failures": 0,
            "serving_parity_equal": True,
            "structural_verdict": "PASS",
        },
    }


def snapshot_text(label: str, observed_at: datetime) -> str:
    lines = [f"label={label}", observed_at.isoformat(), "loadavg=1.00 1.00 1.00 1/100 1"]
    for ordinal, unit in enumerate(VERIFY.SERVICE_UNITS, start=1):
        lines.extend(
            [
                f"unit={unit}",
                "ActiveState=active",
                f"MainPID={1000 + ordinal}",
                "NRestarts=0",
            ]
        )
    lines.append(
        json.dumps(
            {
                "ok": True,
                "mode": "CPU",
                "admission_verdict": "PASS",
                "transition_false_accepts": 0,
                "response_runtime_parity_mismatches": 0,
                "response_active_profiles": 2,
            },
            separators=(",", ":"),
        )
    )
    return "\n".join(lines) + "\n"


def connector_snapshot(
    label: str, observed_at: datetime, document: dict[str, object]
) -> str:
    return json.dumps(
        {
            "schema": "nando.s1c1-resource-v3.connector-snapshot.v1",
            "label": label,
            "observed_at": observed_at.isoformat(),
            "protocol_commit": document["protocol_commit"],
            "protocol_epoch_root": document["protocol_epoch_root"],
            "active_state": "active",
            "main_pid": 2919,
            "nrestarts": 0,
            "route_receipt_failures": 0,
        },
        sort_keys=True,
    ) + "\n"


def write_evidence(root: Path, document: dict[str, object]) -> None:
    root.mkdir()
    (root / "environment.txt").write_text(
        "\n".join(
            [
                f"protocol_commit={document['protocol_commit']}",
                f"protocol_parent_commit={VERIFY.PROTOCOL_PARENT_COMMIT}",
                f"protocol_epoch_root={VERIFY.PROTOCOL_EPOCH_ROOT}",
                f"source_manifest_root={VERIFY.SOURCE_MANIFEST_ROOT}",
                "boot_id=5e35be95-c789-4ce9-b74e-3d42c3b81a3a",
                f"{VERIFY.BASELINE_BINARY_SHA256}  /home/e/.cache/nando-wave-s1c1-baseline-target/release/deps/f7_generation_shadow_v3-257d2fa93e7c240e",
                f"{VERIFY.CANDIDATE_BINARY_SHA256}  /home/e/.cache/nando-wave-s1c1-target/release/deps/f7_generation_shadow_v3-257d2fa93e7c240e",
                f"{VERIFY.TARGETED_BINARY_SHA256}  /home/e/.cache/nando-wave-s1c1-target/release/deps/nando_response_actor-94c534b357a046f6",
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    cursor = datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc)
    for label in VERIFY.RUN_LABELS:
        run_value, no_match_field, test = VERIFY.document_run(document, label)
        run = run_value
        metric_name = "S1C_HOT_LATENCY" if label.startswith("T") else "F7E_LATENCY"
        metric = (
            f"{metric_name} matched_p99_ns={run['matched_p99_ns']} "
            f"{no_match_field}={run[no_match_field]} hard_max_ns={run['hard_max_ns']} "
            f"samples={run['samples']}"
        )
        (root / f"{label}.log").write_text(
            f"running 1 test\ntest {test} ... {metric}\n",
            encoding="utf-8",
        )
        (root / f"{label}.exit").write_text(f"{run['exit_code']}\n", encoding="ascii")
        before = cursor
        after = before + timedelta(milliseconds=100)
        (root / f"{label}.before.snapshot").write_text(
            snapshot_text(f"{label}.before", before), encoding="utf-8"
        )
        (root / f"{label}.after.snapshot").write_text(
            snapshot_text(f"{label}.after", after), encoding="utf-8"
        )
        cursor = after + timedelta(seconds=2)

    final_at = cursor
    (root / "final.snapshot").write_text(
        snapshot_text("final", final_at), encoding="utf-8"
    )
    first_before = datetime(2026, 8, 11, 11, 59, 59, tzinfo=timezone.utc)
    (root / "local_connector.before").write_text(
        connector_snapshot("before", first_before, document), encoding="utf-8"
    )
    (root / "local_connector.after").write_text(
        connector_snapshot("after", final_at + timedelta(seconds=1), document),
        encoding="utf-8",
    )
    manifest_root, _ = VERIFY.canonical_evidence_manifest(root)
    document["evidence_manifest_sha256"] = manifest_root


class ResourceVerifierTests(unittest.TestCase):
    def test_accepts_relative_non_regression_when_both_absolute_sides_fail(self) -> None:
        result, code = VERIFY.verify_measurement_math(passing_document())
        self.assertEqual(code, 0)
        self.assertEqual(result["verdict"], "PASS")
        self.assertEqual(result["inherited_absolute_environment"], "FAIL")
        self.assertFalse(result["deployment_allowed"])

    def test_targeted_absolute_failure_is_veto(self) -> None:
        document = passing_document()
        document["targeted_runs"][1]["matched_p99_ns"] = 1_000_001
        document["targeted_runs"][1]["exit_code"] = 101
        result, code = VERIFY.verify_measurement_math(document)
        self.assertEqual(code, 1)
        self.assertEqual(result["targeted_gate"], "VETO")

    def test_more_than_ten_percent_median_regression_is_veto(self) -> None:
        document = passing_document()
        for pair in document["inherited_pairs"]:
            pair["candidate"]["matched_p99_ns"] = 1_200_000
            pair["candidate"]["exit_code"] = 101
        result, code = VERIFY.verify_measurement_math(document)
        self.assertEqual(code, 1)
        self.assertEqual(result["inherited_regression_gate"], "VETO")

    def test_candidate_cannot_lose_an_absolute_pass(self) -> None:
        document = passing_document()
        baseline = document["inherited_pairs"][0]["baseline"]
        candidate = document["inherited_pairs"][0]["candidate"]
        baseline.update(
            matched_p99_ns=900_000,
            no_match_p99_ns=200_000,
            hard_max_ns=1_000_000,
            exit_code=0,
        )
        candidate.update(
            matched_p99_ns=1_000_001,
            no_match_p99_ns=220_000,
            hard_max_ns=1_100_000,
            exit_code=101,
        )
        result, code = VERIFY.verify_measurement_math(document)
        self.assertEqual(code, 1)
        self.assertEqual(result["inherited_regression_gate"], "VETO")

    def test_safety_watch_is_veto(self) -> None:
        document = passing_document()
        document["safety"]["structural_verdict"] = "WATCH"
        result, code = VERIFY.verify_measurement_math(document)
        self.assertEqual(code, 1)
        self.assertEqual(result["safety_gate"], "VETO")

    def test_identity_drift_is_invalid(self) -> None:
        document = passing_document()
        document["targeted_runs"][0]["binary_sha256"] = "f" * 64
        with self.assertRaisesRegex(VERIFY.InvalidMeasurements, "binary_mismatch"):
            VERIFY.verify_measurement_math(document)

    def test_unknown_field_is_invalid(self) -> None:
        document = copy.deepcopy(passing_document())
        document["unexpected"] = True
        with self.assertRaisesRegex(VERIFY.InvalidMeasurements, "extra"):
            VERIFY.verify_measurement_math(document)

    def test_accepts_git_sha1_and_rejects_sha256_in_commit_field(self) -> None:
        document = passing_document()
        VERIFY.verify_measurement_math(document)
        document["protocol_commit"] = "1" * 64
        with self.assertRaisesRegex(VERIFY.InvalidMeasurements, "protocol_commit_invalid"):
            VERIFY.verify_measurement_math(document)

    def test_protocol_epoch_root_is_exact(self) -> None:
        document = passing_document()
        document["protocol_epoch_root"] = "f" * 64
        with self.assertRaisesRegex(VERIFY.InvalidMeasurements, "protocol_epoch_root_mismatch"):
            VERIFY.verify_measurement_math(document)

    def test_raw_evidence_is_bound_to_measurements_and_services(self) -> None:
        document = passing_document()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / document["evidence_directory_name"]
            write_evidence(root, document)
            result, code = VERIFY.verify_measurements(document, root)
        self.assertEqual(code, 0)
        self.assertEqual(result["evidence_gate"], "PASS")

    def test_transcribed_metric_cannot_disagree_with_raw_log(self) -> None:
        document = passing_document()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / document["evidence_directory_name"]
            write_evidence(root, document)
            document["targeted_runs"][0]["matched_p99_ns"] += 1
            with self.assertRaisesRegex(VERIFY.InvalidMeasurements, "metrics_mismatch"):
                VERIFY.verify_measurements(document, root)

    def test_connector_snapshot_is_bound_to_protocol_commit(self) -> None:
        document = passing_document()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / document["evidence_directory_name"]
            write_evidence(root, document)
            path = root / "local_connector.after"
            value = json.loads(path.read_text(encoding="utf-8"))
            value["protocol_commit"] = "f" * 40
            path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")
            document["evidence_manifest_sha256"] = VERIFY.canonical_evidence_manifest(root)[0]
            with self.assertRaisesRegex(VERIFY.InvalidMeasurements, "protocol_commit_mismatch"):
                VERIFY.verify_measurements(document, root)

    def test_extra_directory_is_rejected_from_evidence_set(self) -> None:
        document = passing_document()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / document["evidence_directory_name"]
            write_evidence(root, document)
            (root / "extra").mkdir()
            with self.assertRaisesRegex(VERIFY.InvalidMeasurements, "entries_unexpected"):
                VERIFY.verify_measurements(document, root)


if __name__ == "__main__":
    unittest.main()
