#!/usr/bin/env python3
"""Fault-injection tests for the S1C-3 receipt verifier."""

from __future__ import annotations

import copy
import importlib.util
import json
import os
import pwd
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("verify_s1c3_transaction_v3.py")
SPEC = importlib.util.spec_from_file_location("s1c3_verifier", MODULE_PATH)
assert SPEC and SPEC.loader
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)

EXECUTOR_PATH = Path(__file__).with_name("s1c3_remote_transaction_v3.py")
EXECUTOR_SPEC = importlib.util.spec_from_file_location("s1c3_executor", EXECUTOR_PATH)
assert EXECUTOR_SPEC and EXECUTOR_SPEC.loader
executor = importlib.util.module_from_spec(EXECUTOR_SPEC)
EXECUTOR_SPEC.loader.exec_module(executor)


def rooted(value: dict, field: str) -> dict:
    value = copy.deepcopy(value)
    value.pop(field, None)
    value[field] = verifier.digest(verifier.canonical_bytes(value, omit_field=field))
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


def executable_identities() -> dict:
    result = {}
    for index, name in enumerate(sorted(verifier.EXECUTABLE_KEYS)):
        source = verifier.BASELINE_COMMIT if name == "parity-baseline" else verifier.CANDIDATE_COMMIT
        result[name] = {
            "path": f"/tmp/{name}",
            "sha256": f"{index + 1:x}" * 64,
            "size_bytes": 100 + index,
            "mode_octal": "0755",
            "source_identity": source,
        }
    return result


def ownership_receipt() -> dict:
    rows = {}
    base = "/home/e/.cache/nando-s1c3-fixture"
    probe = {
        "writer_uid": verifier.ORACLE_UID,
        "writer_gid": verifier.ORACLE_GID,
        "probe_uid": verifier.ORACLE_UID,
        "probe_gid": verifier.ORACLE_GID,
        "probe_mode_octal": "0600",
        "create_fsync_unlink_pass": True,
        "directory_fsync_pass": True,
    }
    for label in ("baseline", "candidate"):
        workspace = f"{base}/oracle-{label}"
        rows[label] = rooted({
            "schema": verifier.OWNERSHIP_ROW_SCHEMA,
            "label": label,
            "workspace": {"path": workspace, "uid": verifier.ORACLE_UID,
                          "gid": verifier.ORACLE_GID, "mode_octal": "0750"},
            "src": {"path": f"{workspace}/src", "uid": verifier.ORACLE_UID,
                    "gid": verifier.ORACLE_GID, "mode_octal": "0750"},
            "cargo_toml": {"path": f"{workspace}/Cargo.toml", "uid": verifier.ORACLE_UID,
                           "gid": verifier.ORACLE_GID, "mode_octal": "0640"},
            "main_rs": {"path": f"{workspace}/src/main.rs", "uid": verifier.ORACLE_UID,
                        "gid": verifier.ORACLE_GID, "mode_octal": "0640"},
            "probe": copy.deepcopy(probe),
            "probe_retained": False,
        }, "ownership_row_root_sha256")
    return rooted({
        "schema": verifier.OWNERSHIP_SCHEMA,
        "transaction_id": "fixture",
        "build_user": {"name": "e", "uid": verifier.ORACLE_UID,
                       "gid": verifier.ORACLE_GID},
        "rows": rows,
        "rows_root_sha256": verifier.digest(verifier.canonical_bytes(rows)),
    }, "ownership_root_sha256")


def quiescence_receipt(ownership: dict) -> dict:
    executables = executable_identities()
    samples = []
    start = 1_000_000_000
    for index in range(30):
        end = start + 1_000_000_000
        samples.append({
            "started_at": f"2026-08-12T00:00:{index:02d}Z",
            "ended_at": f"2026-08-12T00:00:{index + 1:02d}Z",
            "start_monotonic_ns": start,
            "end_monotonic_ns": end,
            "interval_seconds": 1.0,
            "cpu4_busy_percent": 1.0,
            "io_some_avg10": 0.0,
            "io_full_avg10": 0.0,
            "build_processes_start": [],
            "build_processes_end": [],
            "process_races": 0,
            "loadavg_end": "1.0 1.0 1.0 1/1 1",
            "eligible_base": True,
        })
        start = end
    return rooted({
        "schema": verifier.QUIESCENCE_SCHEMA,
        "transaction_id": "fixture",
        "candidate_commit": verifier.CANDIDATE_COMMIT,
        "candidate_tree": verifier.CANDIDATE_TREE,
        "detector_schema": "proc-comm-exe-basename-v1",
        "forbidden_build_names": list(verifier.FORBIDDEN_BUILD_NAMES),
        "maximum_wait_seconds": 1800,
        "required_intervals": 30,
        "thresholds": {
            "interval_min_seconds": 0.90,
            "interval_max_seconds": 1.50,
            "cpu4_max_percent": 20.0,
            "cpu4_mean_max_percent": 5.0,
            "io_some_avg10_max": 0.20,
            "io_full_avg10_max": 0.05,
        },
        "eligibility_started_at": "2026-08-12T00:00:00Z",
        "eligibility_reached_at": "2026-08-12T00:00:30Z",
        "attempted_samples": copy.deepcopy(samples),
        "eligible_window": copy.deepcopy(samples),
        "eligible_cpu4_mean_percent": 1.0,
        "eligible_window_root_sha256": verifier.digest(verifier.canonical_bytes(samples)),
        "executables": executables,
        "oracle_ownership_root_sha256": ownership["ownership_root_sha256"],
    }, "quiescence_root_sha256")


def contamination_receipt(quiescence: dict) -> dict:
    samples = []
    boundaries = []
    now = 10_000_000_000
    for label in verifier.MEASUREMENT_LABELS:
        for phase in ("before", "after"):
            boundaries.append({"label": label, "phase": phase, "observed_at": "2026-08-12T00:01:00Z"})
            samples.append({
                "label": label,
                "observed_at": "2026-08-12T00:01:00Z",
                "monotonic_ns": now,
                "cpu4_total": 100,
                "cpu4_idle": 99,
                "io_pressure": {"some": {"avg10": 0.0, "total": 0}, "full": {"avg10": 0.0, "total": 0}},
                "loadavg": "1.0 1.0 1.0 1/1 1",
                "build_processes": [],
                "process_races": 0,
                "kind": f"boundary-{phase}",
            })
            now += 100_000_000
    executable_root = verifier.digest(verifier.canonical_bytes(quiescence["executables"]))
    return rooted({
        "schema": verifier.CONTAMINATION_SCHEMA,
        "transaction_id": "fixture",
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "executable_set_root_sha256": executable_root,
        "monitor_interval_seconds": 0.5,
        "maximum_sample_gap_seconds": 2.0,
        "observed_max_sample_gap_seconds": 0.1,
        "metric_labels": list(verifier.MEASUREMENT_LABELS),
        "boundaries": boundaries,
        "samples": samples,
        "forbidden_process_matches": [],
        "monitor_errors": [],
        "contaminated": False,
    }, "measurement_contamination_root_sha256")


def resource_receipt(quiescence: dict, contamination: dict) -> dict:
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
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
        "executable_set_root_sha256": verifier.digest(
            verifier.canonical_bytes(quiescence["executables"])
        ),
        "oracle_ownership_root_sha256": quiescence["oracle_ownership_root_sha256"],
        "direct_exec_only": True,
        "compiler_invocations_after_quiescence": 0,
        "all_pass": True,
    }, "resource_root_sha256")


def parity_receipt(quiescence: dict) -> dict:
    return rooted({
        "schema": verifier.PARITY_SCHEMA,
        "baseline_output_sha256": "b" * 64,
        "candidate_output_sha256": "b" * 64,
        "byte_identical": True,
        "rows": 16,
        "direct_exec_only": True,
        "baseline_oracle_sha256": quiescence["executables"]["parity-baseline"]["sha256"],
        "candidate_oracle_sha256": quiescence["executables"]["parity-candidate"]["sha256"],
    }, "parity_root_sha256")


def preparation(quiescence: dict, contamination: dict, resource: dict, parity: dict) -> dict:
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
                      "binary_sha256": quiescence["executables"]["candidate-binary"]["sha256"],
                      "binary_size_bytes": quiescence["executables"]["candidate-binary"]["size_bytes"],
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
        "oracle_ownership_root_sha256": quiescence["oracle_ownership_root_sha256"],
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
        "executable_set_root_sha256": verifier.digest(
            verifier.canonical_bytes(quiescence["executables"])
        ),
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


def deployment_receipt(
    prep: dict,
    quiescence: dict,
    contamination: dict,
    resource: dict,
    parity: dict,
) -> dict:
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
        "oracle_ownership_root_sha256": quiescence["oracle_ownership_root_sha256"],
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
        "executable_set_root_sha256": verifier.digest(
            verifier.canonical_bytes(quiescence["executables"])
        ),
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
        self.ownership = ownership_receipt()
        self.quiescence = quiescence_receipt(self.ownership)
        self.contamination = contamination_receipt(self.quiescence)
        self.resource = resource_receipt(self.quiescence, self.contamination)
        self.parity = parity_receipt(self.quiescence)
        self.prep = preparation(
            self.quiescence, self.contamination, self.resource, self.parity
        )
        self.receipt = deployment_receipt(
            self.prep, self.quiescence, self.contamination, self.resource, self.parity
        )
        self.write_all()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self, name: str, value: dict) -> None:
        path = self.directory / name
        if path.exists():
            path.chmod(0o600)
        path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

    def write_all(self) -> None:
        self.write("oracle-ownership-receipt.json", self.ownership)
        (self.directory / "oracle-ownership-receipt.json").chmod(0o400)
        self.write("quiescence-receipt.json", self.quiescence)
        (self.directory / "quiescence-receipt.json").chmod(0o400)
        self.write("measurement-contamination-receipt.json", self.contamination)
        self.write("resource-receipt.json", self.resource)
        self.write("parity-receipt.json", self.parity)
        self.write("preparation.json", self.prep)
        self.write("deployment-receipt.json", self.receipt)

    def rebind_environment(self) -> None:
        executable_root = verifier.digest(
            verifier.canonical_bytes(self.quiescence["executables"])
        )
        self.contamination["quiescence_root_sha256"] = self.quiescence[
            "quiescence_root_sha256"
        ]
        self.contamination["executable_set_root_sha256"] = executable_root
        self.contamination = rooted(
            self.contamination, "measurement_contamination_root_sha256"
        )
        self.resource["quiescence_root_sha256"] = self.quiescence[
            "quiescence_root_sha256"
        ]
        self.resource["measurement_contamination_root_sha256"] = self.contamination[
            "measurement_contamination_root_sha256"
        ]
        self.resource["executable_set_root_sha256"] = executable_root
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.parity["baseline_oracle_sha256"] = self.quiescence["executables"][
            "parity-baseline"
        ]["sha256"]
        self.parity["candidate_oracle_sha256"] = self.quiescence["executables"][
            "parity-candidate"
        ]["sha256"]
        self.parity = rooted(self.parity, "parity_root_sha256")
        self.prep["quiescence_root_sha256"] = self.quiescence[
            "quiescence_root_sha256"
        ]
        self.prep["measurement_contamination_root_sha256"] = self.contamination[
            "measurement_contamination_root_sha256"
        ]
        self.prep["executable_set_root_sha256"] = executable_root
        self.prep["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.prep["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["quiescence_root_sha256"] = self.quiescence[
            "quiescence_root_sha256"
        ]
        self.receipt["measurement_contamination_root_sha256"] = self.contamination[
            "measurement_contamination_root_sha256"
        ]
        self.receipt["executable_set_root_sha256"] = executable_root
        self.receipt["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.receipt["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()

    def assert_invalid(self, expected: str) -> None:
        with self.assertRaisesRegex(verifier.InvalidReceipt, expected):
            verifier.verify_receipt(self.directory)

    def rehash_ownership(self, label: str) -> None:
        self.ownership["rows"][label] = rooted(
            self.ownership["rows"][label], "ownership_row_root_sha256"
        )
        self.ownership["rows_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.ownership["rows"])
        )
        self.ownership = rooted(self.ownership, "ownership_root_sha256")
        self.write_all()

    def test_valid_deployment_pass(self) -> None:
        result = verifier.verify_receipt(self.directory)
        self.assertEqual(result["verdict"], "S1C3_DEPLOYMENT_PASS")

    def test_oracle_workspace_owner_mismatch_is_rejected(self) -> None:
        self.ownership["rows"]["baseline"]["workspace"]["uid"] = 0
        self.rehash_ownership("baseline")
        self.assert_invalid("ownership_baseline_workspace_uid_mismatch")

    def test_oracle_workspace_unwritable_mode_is_rejected(self) -> None:
        self.ownership["rows"]["candidate"]["workspace"]["mode_octal"] = "0550"
        self.rehash_ownership("candidate")
        self.assert_invalid("ownership_candidate_workspace_mode_mismatch")

    def test_oracle_probe_retained_is_rejected(self) -> None:
        self.ownership["rows"]["baseline"]["probe_retained"] = True
        self.rehash_ownership("baseline")
        self.assert_invalid("ownership_row_baseline_probe_retained_mismatch")

    def test_oracle_ownership_root_mismatch_is_rejected(self) -> None:
        self.ownership["ownership_root_sha256"] = "0" * 64
        self.write_all()
        self.assert_invalid("ownership_root_mismatch")

    def test_oracle_ownership_file_mode_is_rejected(self) -> None:
        (self.directory / "oracle-ownership-receipt.json").chmod(0o600)
        self.assert_invalid("ownership_file_mode")

    def test_quiescence_ownership_cross_binding_is_rejected(self) -> None:
        self.quiescence["oracle_ownership_root_sha256"] = "f" * 64
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.rebind_environment()
        self.assert_invalid("quiescence_ownership_root_mismatch")

    def test_non_root_probe_smoke_uses_production_helper(self) -> None:
        account = pwd.getpwuid(os.geteuid())
        self.assertNotEqual(account.pw_uid, 0)
        workspace = self.directory / "ownership-smoke"
        workspace.mkdir(mode=0o750)
        result = executor.run_oracle_ownership_probe(
            workspace,
            account.pw_name,
            self.directory / "ownership-smoke.log",
        )
        self.assertEqual(result["writer_uid"], account.pw_uid)
        self.assertFalse((workspace / ".s1c3-v3-ownership-probe").exists())

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

    def test_quiescence_build_process_is_rejected(self) -> None:
        process = {"pid": 7, "comm": "rustc", "executable_basename": "rustc"}
        self.quiescence["eligible_window"][0]["build_processes_start"] = [process]
        self.quiescence["attempted_samples"][-30]["build_processes_start"] = [process]
        self.quiescence["eligible_window_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.quiescence["eligible_window"])
        )
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.rebind_environment()
        self.assert_invalid("quiescence_sample_0_build_start_mismatch")

    def test_quiescence_cpu_mean_is_rejected(self) -> None:
        for attempted, window in zip(
            self.quiescence["attempted_samples"][-30:], self.quiescence["eligible_window"]
        ):
            attempted["cpu4_busy_percent"] = 6.0
            window["cpu4_busy_percent"] = 6.0
        self.quiescence["eligible_cpu4_mean_percent"] = 6.0
        self.quiescence["eligible_window_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.quiescence["eligible_window"])
        )
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.rebind_environment()
        self.assert_invalid("quiescence_cpu_mean_budget")

    def test_quiescence_io_pressure_is_rejected(self) -> None:
        self.quiescence["eligible_window"][0]["io_some_avg10"] = 0.21
        self.quiescence["attempted_samples"][-30]["io_some_avg10"] = 0.21
        self.quiescence["eligible_window_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.quiescence["eligible_window"])
        )
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.rebind_environment()
        self.assert_invalid("quiescence_sample_0_some_budget")

    def test_quiescence_file_mode_is_rejected(self) -> None:
        (self.directory / "quiescence-receipt.json").chmod(0o600)
        self.assert_invalid("quiescence_file_mode")

    def test_measurement_build_process_is_rejected(self) -> None:
        process = {"pid": 8, "comm": "cargo", "executable_basename": "cargo"}
        self.contamination["samples"][0]["build_processes"] = [process]
        self.contamination = rooted(
            self.contamination, "measurement_contamination_root_sha256"
        )
        self.rebind_environment()
        self.assert_invalid("contamination_sample_0_build_mismatch")

    def test_measurement_gap_is_rejected(self) -> None:
        self.contamination["samples"][1]["monotonic_ns"] = (
            self.contamination["samples"][0]["monotonic_ns"] + 2_100_000_000
        )
        for index in range(2, len(self.contamination["samples"])):
            self.contamination["samples"][index]["monotonic_ns"] = (
                self.contamination["samples"][index - 1]["monotonic_ns"] + 100_000_000
            )
        self.contamination["observed_max_sample_gap_seconds"] = 2.1
        self.contamination = rooted(
            self.contamination, "measurement_contamination_root_sha256"
        )
        self.rebind_environment()
        self.assert_invalid("contamination_observed_gap_budget")

    def test_resource_direct_exec_false_is_rejected(self) -> None:
        self.resource["direct_exec_only"] = False
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.rebind_environment()
        self.assert_invalid("resource_direct_exec_mismatch")

    def test_compiler_after_quiescence_is_rejected(self) -> None:
        self.resource["compiler_invocations_after_quiescence"] = 1
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.rebind_environment()
        self.assert_invalid("resource_compiler_invocations_mismatch")

    def test_parity_oracle_substitution_is_rejected(self) -> None:
        self.parity["candidate_oracle_sha256"] = "f" * 64
        self.parity = rooted(self.parity, "parity_root_sha256")
        self.prep["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("parity_candidate_oracle_mismatch")

    def test_remote_executor_has_no_build_command_after_quiescence(self) -> None:
        executor = Path(__file__).with_name("s1c3_remote_transaction_v3.py").read_text(
            encoding="utf-8"
        )
        measured_stage = executor.split(
            "quiescence = wait_for_quiescence", 1
        )[1].split("def exact_untouched", 1)[0]
        self.assertNotIn('"/home/e/.cargo/bin/cargo"', measured_stage)
        self.assertNotIn('"/home/e/.cargo/bin/rustc"', measured_stage)

    def test_receipt_tamper_without_rehash_is_rejected(self) -> None:
        self.receipt["survival_seconds"] = 14
        self.write_all()
        self.assert_invalid("receipt_root_mismatch")


if __name__ == "__main__":
    unittest.main()
