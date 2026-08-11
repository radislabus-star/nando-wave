#!/usr/bin/env python3
"""Fault-injection tests for the S1C-3 receipt verifier."""

from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("verify_s1c3_transaction_v1.py")
SPEC = importlib.util.spec_from_file_location("s1c3_verifier", MODULE_PATH)
assert SPEC and SPEC.loader
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)


def rooted(value: dict, field: str) -> dict:
    value = copy.deepcopy(value)
    value.pop(field, None)
    value[field] = verifier.digest(verifier.canonical_bytes(value))
    return value


def service_snapshot(transition_pid: int, untouched_pid_offset: int = 0) -> dict:
    result = {}
    for index, unit in enumerate(verifier.ALL_UNITS):
        result[unit] = {
            "active_state": "active",
            "sub_state": "running",
            "main_pid": transition_pid if unit == verifier.TRANSITION_UNIT else 2000 + index + untouched_pid_offset,
            "nrestarts": 0,
            "fragment_sha256": verifier.digest(unit.encode()),
        }
    return result


def connector() -> dict:
    return {
        "schema": "nando.s1c3-connector-snapshot.v1",
        "label": "before",
        "observed_at": "2026-08-12T00:00:00Z",
        "active_state": "active",
        "main_pid": 2919,
        "nrestarts": 0,
        "route_receipt_failures": 0,
        "command_sha256": "a" * 64,
    }


def resource_receipt() -> dict:
    hot = {"p99_ns": 10_000, "no_goal_p99_ns": 1_000, "hard_max_ns": 20_000, "samples": 4096}
    single = {"p99_ns": 1_000_000, "hard_max_ns": 2_000_000, "samples": 1024, "segments": 2}
    three = {"p99_ns": 2_000_000, "hard_max_ns": 3_000_000, "samples": 256}
    return rooted({
        "schema": verifier.RESOURCE_SCHEMA,
        "candidate_commit": verifier.CANDIDATE_COMMIT,
        "observed_at": "2026-08-12T00:00:00Z",
        "metrics": {
            "hot_latency": [copy.deepcopy(hot) for _ in range(3)],
            "single_ledger_sync": [copy.deepcopy(single) for _ in range(3)],
            "three_ledger_sync": [copy.deepcopy(three) for _ in range(3)],
            "idle_cpu": {"elapsed_ticks": 0, "ticks_per_second": 100, "percent_of_one_core": 0.0},
            "rss": {"capture_off_bytes": 10, "capture_on_bytes": 20, "delta_bytes": 10},
        },
        "frozen_bounds": {
            "max_precommit_bytes": 32 * 1024,
            "max_typed_goal_bytes": 4 * 1024,
            "max_k1_actions": 256,
            "segment_bytes": 64 * 1024 * 1024,
            "journal_quota_bytes": 2 * 1024 * 1024 * 1024,
            "persisted_raw_payload_bytes": 0,
        },
        "all_pass": True,
    }, "resource_root_sha256")


def parity_receipt() -> dict:
    return rooted({
        "schema": verifier.PARITY_SCHEMA,
        "baseline_output_sha256": "b" * 64,
        "candidate_output_sha256": "b" * 64,
        "byte_identical": True,
        "rows": 16,
    }, "parity_root_sha256")


def preparation(resource: dict, parity: dict) -> dict:
    rollback_entries = [
        {"path": "nando-transition-serving", "sha256": verifier.BASELINE_BINARY_SHA256,
         "size_bytes": 90},
        {"path": "transition-serving.env", "sha256": verifier.BASELINE_CONFIG_SHA256,
         "size_bytes": 10},
        {"path": "nando-transition-serving.service", "sha256": verifier.UNIT_SHA256,
         "size_bytes": 10},
        {"path": "previous-deployment-receipt.json", "sha256": "f" * 64,
         "size_bytes": 10},
    ]
    rollback_manifest = "".join(
        f'{entry["sha256"]} {entry["size_bytes"]} {entry["path"]}\n'
        for entry in sorted(rollback_entries, key=lambda item: item["path"])
    ).encode("ascii")
    return rooted({
        "schema": verifier.PREPARATION_SCHEMA,
        "transaction_id": "fixture",
        "state": "PREPARED",
        "created_at": "2026-08-12T00:00:00Z",
        "paper": {"commit": verifier.PAPER_COMMIT,
                  "manifest_root_sha256": verifier.PAPER_MANIFEST_ROOT,
                  "verification_sha256": verifier.PAPER_VERIFICATION_SHA256},
        "candidate": {"source_commit": verifier.CANDIDATE_COMMIT,
                      "source_tree": verifier.CANDIDATE_TREE,
                      "cargo_lock_sha256": verifier.CARGO_LOCK_SHA256,
                      "binary_sha256": "d" * 64,
                      "binary_size_bytes": 100,
                      "config_sha256": verifier.CANDIDATE_CONFIG_SHA256},
        "baseline": {"source_commit": verifier.BASELINE_COMMIT,
                     "source_tree": verifier.BASELINE_TREE,
                     "deployment_receipt_root_sha256": verifier.BASELINE_RECEIPT_ROOT,
                     "binary_sha256": verifier.BASELINE_BINARY_SHA256,
                     "binary_size_bytes": 90,
                     "config_sha256": verifier.BASELINE_CONFIG_SHA256},
        "toolchain": {},
        "immutable": {"unit_sha256": verifier.UNIT_SHA256,
                      "phase_config_sha256": verifier.PHASE_CONFIG_SHA256,
                      "authority_config_sha256": verifier.AUTHORITY_CONFIG_SHA256},
        "services_before": service_snapshot(1000),
        "health_before": {},
        "economics_before": {"false_accepts": 0, "runtime_parity_mismatches": 0},
        "route_probe_before": {},
        "connector_before": connector(),
        "journal_before": {},
        "resource_root_sha256": resource["resource_root_sha256"],
        "parity_root_sha256": parity["parity_root_sha256"],
        "rollback": {
            "manifest_root_sha256": verifier.digest(rollback_manifest),
            "entries": rollback_entries,
        },
        "intent": [],
    }, "preparation_root_sha256")


def journal() -> dict:
    return {"present": True, "entries": [], "total_bytes": 0,
            "manifest_root_sha256": verifier.digest(b""), "raw_payload_bytes": 0,
            "preserved_prefixes": True}


def deployment_receipt(prep: dict, resource: dict, parity: dict) -> dict:
    before = service_snapshot(1000)
    after = service_snapshot(1001)
    survival = copy.deepcopy(after)
    before_connector = connector()
    after_connector = copy.deepcopy(before_connector)
    after_connector["label"] = "after"
    return rooted({
        "schema": verifier.SCHEMA,
        "transaction_id": "fixture",
        "verdict": "S1C3_DEPLOYMENT_PASS",
        "finalized_at": "2026-08-12T00:00:16Z",
        "preparation_root_sha256": prep["preparation_root_sha256"],
        "resource_root_sha256": resource["resource_root_sha256"],
        "parity_root_sha256": parity["parity_root_sha256"],
        "services_before": before,
        "services_after": after,
        "services_survival": survival,
        "connector_before": before_connector,
        "connector_after": after_connector,
        "installed_binary_sha256": prep["candidate"]["binary_sha256"],
        "installed_config_sha256": verifier.CANDIDATE_CONFIG_SHA256,
        "immutable_after": {"unit_sha256": verifier.UNIT_SHA256,
                            "phase_config_sha256": verifier.PHASE_CONFIG_SHA256,
                            "authority_config_sha256": verifier.AUTHORITY_CONFIG_SHA256},
        "capture_environment": {
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
            "NANDO_GROUNDED_DECISION_JOURNAL": "/var/lib/nando-wave/transition/grounded-meaning-v1/decision-contract-precommits-v1",
        },
        "capture_available": True,
        "startup_log_clean": True,
        "health_semantics_preserved": True,
        "route_probe_equivalent": True,
        "active_packages_preserved": True,
        "false_accepts_after": 0,
        "runtime_parity_failures_after": 0,
        "journal_before": journal(),
        "journal_after": journal(),
        "survival_seconds": 15,
    }, "receipt_root_sha256")


class TransactionVerifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.directory = Path(self.temp.name)
        self.resource = resource_receipt()
        self.parity = parity_receipt()
        self.prep = preparation(self.resource, self.parity)
        self.receipt = deployment_receipt(self.prep, self.resource, self.parity)
        self.write_all()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self, name: str, value: dict) -> None:
        (self.directory / name).write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

    def write_all(self) -> None:
        self.write("resource-receipt.json", self.resource)
        self.write("parity-receipt.json", self.parity)
        self.write("preparation.json", self.prep)
        self.write("deployment-receipt.json", self.receipt)

    def assert_invalid(self, expected: str) -> None:
        with self.assertRaisesRegex(verifier.InvalidReceipt, expected):
            verifier.verify_receipt(self.directory)

    def test_valid_deployment_pass(self) -> None:
        result = verifier.verify_receipt(self.directory)
        self.assertEqual(result["verdict"], "S1C3_DEPLOYMENT_PASS")

    def test_candidate_config_drift_is_rejected(self) -> None:
        self.prep["candidate"]["config_sha256"] = "0" * 64
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("candidate_config_mismatch")

    def test_untouched_pid_change_is_rejected(self) -> None:
        unit = verifier.UNTOUCHED_UNITS[0]
        self.receipt["services_after"][unit]["main_pid"] += 1
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("untouched_after")

    def test_extra_restart_is_rejected(self) -> None:
        self.receipt["services_survival"][verifier.TRANSITION_UNIT]["nrestarts"] = 1
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("transition_survival_nrestarts")

    def test_connector_failure_change_is_rejected(self) -> None:
        self.receipt["connector_after"]["route_receipt_failures"] = 1
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("connector_route_receipt_failures")

    def test_raw_payload_is_rejected(self) -> None:
        self.receipt["journal_after"]["raw_payload_bytes"] = 1
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("journal_raw_payload_mismatch")

    def test_parity_divergence_is_rejected(self) -> None:
        self.parity["byte_identical"] = False
        self.parity = rooted(self.parity, "parity_root_sha256")
        self.prep["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("parity_identical_mismatch")

    def test_resource_budget_breach_is_rejected(self) -> None:
        self.resource["metrics"]["hot_latency"][0]["p99_ns"] = 1_000_001
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.prep["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("hot_latency_0_p99_budget")

    def test_capture_unavailable_is_rejected(self) -> None:
        self.receipt["capture_available"] = False
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("capture_available_mismatch")

    def test_receipt_tamper_without_rehash_is_rejected(self) -> None:
        self.receipt["survival_seconds"] = 14
        self.write_all()
        self.assert_invalid("receipt_root_mismatch")


if __name__ == "__main__":
    unittest.main()
