#!/usr/bin/env python3
"""Fail-closed verifier for the frozen S1C-3 transaction receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any


SCHEMA = "nando.s1c3-transaction-receipt.v1"
PREPARATION_SCHEMA = "nando.s1c3-transaction-preparation.v1"
RESOURCE_SCHEMA = "nando.s1c3-resource-receipt.v1"
PARITY_SCHEMA = "nando.s1c3-parity-receipt.v1"

PAPER_COMMIT = "b3ee186d49d848b1917472f427d6afc59459c7cd"
PAPER_MANIFEST_ROOT = "ebb5067060f69722341120ae8105849cbd45f585611a30741e1db7d33ace3ab3"
PAPER_VERIFICATION_SHA256 = "41da0d1cc419690261c701133fca0c123eafdacfc9fd14a28287453af1112deb"
CANDIDATE_COMMIT = "a3ea27a49af397ef79e5c9ec80089ecf53a41d59"
CANDIDATE_TREE = "670d9c4ed170a76f107db13262abcd7cc035578e"
CARGO_LOCK_SHA256 = "0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1"
CANDIDATE_CONFIG_SHA256 = "1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6"
BASELINE_COMMIT = "663959064a37caf7eb917fc99dfedb6386355fa6"
BASELINE_TREE = "05460ccbc9c44ac8b7174318903c0211de709e2e"
BASELINE_RECEIPT_ROOT = "785450d76037410d96baade19c2b6bb7f0fb24c6be034e2166be5533c7dd985b"
BASELINE_BINARY_SHA256 = "6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58"
BASELINE_CONFIG_SHA256 = "cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5"
UNIT_SHA256 = "6e9d2fe41b1db95f94768d1ab41dffce1f15be92e2f774832c7fe392bb77b135"
PHASE_CONFIG_SHA256 = "5c019cebbde083f963c03619ff1d938786f5b4ec58730dddd5b34adeb33cce31"
AUTHORITY_CONFIG_SHA256 = "d40b7262ff6d744a393b0fc03a5d06610d01728aa2f4603199ca8567189ec88f"

TRANSITION_UNIT = "nando-transition-serving.service"
UNTOUCHED_UNITS = (
    "nando-transport-gateway.service",
    "nando-response-learning.service",
    "nando-gateway-control.service",
    "nando-operator-certification-authority.service",
)
ALL_UNITS = (TRANSITION_UNIT, *UNTOUCHED_UNITS)


class InvalidReceipt(ValueError):
    pass


def canonical_bytes(value: Any, *, omit_root: bool = False) -> bytes:
    if omit_root:
        value = dict(value)
        for field in (
            "receipt_root_sha256",
            "preparation_root_sha256",
            "resource_root_sha256",
            "parity_root_sha256",
        ):
            if field in value:
                value.pop(field)
                break
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def require(condition: bool, error: str) -> None:
    if not condition:
        raise InvalidReceipt(error)


def require_exact(value: Any, expected: Any, label: str) -> None:
    require(value == expected, f"{label}_mismatch")


def require_hash(value: Any, label: str) -> str:
    require(
        isinstance(value, str)
        and len(value) == 64
        and all(char in "0123456789abcdef" for char in value),
        f"{label}_invalid",
    )
    return value


def require_nonnegative_int(value: Any, label: str) -> int:
    require(type(value) is int and value >= 0, f"{label}_invalid")
    return value


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise InvalidReceipt(f"json_invalid:{path.name}:{error}") from error
    require(isinstance(value, dict), f"json_not_object:{path.name}")
    return value


def verify_embedded_root(value: dict[str, Any], field: str, label: str) -> str:
    expected = require_hash(value.get(field), field)
    actual = digest(canonical_bytes(value, omit_root=True))
    require_exact(actual, expected, f"{label}_root")
    return actual


def verify_identity(preparation: dict[str, Any]) -> None:
    require_exact(preparation.get("schema"), PREPARATION_SCHEMA, "preparation_schema")
    verify_embedded_root(preparation, "preparation_root_sha256", "preparation")
    paper = preparation.get("paper", {})
    candidate = preparation.get("candidate", {})
    baseline = preparation.get("baseline", {})
    immutable = preparation.get("immutable", {})
    require_exact(paper.get("commit"), PAPER_COMMIT, "paper_commit")
    require_exact(paper.get("manifest_root_sha256"), PAPER_MANIFEST_ROOT, "paper_manifest")
    require_exact(paper.get("verification_sha256"), PAPER_VERIFICATION_SHA256,
                  "paper_verification")
    require_exact(candidate.get("source_commit"), CANDIDATE_COMMIT, "candidate_commit")
    require_exact(candidate.get("source_tree"), CANDIDATE_TREE, "candidate_tree")
    require_exact(candidate.get("cargo_lock_sha256"), CARGO_LOCK_SHA256, "cargo_lock")
    require_exact(candidate.get("config_sha256"), CANDIDATE_CONFIG_SHA256, "candidate_config")
    require_hash(candidate.get("binary_sha256"), "candidate_binary")
    require(require_nonnegative_int(candidate.get("binary_size_bytes"), "candidate_binary_size") > 0,
            "candidate_binary_size_zero")
    require_exact(baseline.get("source_commit"), BASELINE_COMMIT, "baseline_commit")
    require_exact(baseline.get("source_tree"), BASELINE_TREE, "baseline_tree")
    require_exact(baseline.get("deployment_receipt_root_sha256"), BASELINE_RECEIPT_ROOT,
                  "baseline_receipt")
    require_exact(baseline.get("binary_sha256"), BASELINE_BINARY_SHA256, "baseline_binary")
    require_exact(baseline.get("config_sha256"), BASELINE_CONFIG_SHA256, "baseline_config")
    require_exact(immutable.get("unit_sha256"), UNIT_SHA256, "unit")
    require_exact(immutable.get("phase_config_sha256"), PHASE_CONFIG_SHA256, "phase_config")
    require_exact(immutable.get("authority_config_sha256"), AUTHORITY_CONFIG_SHA256,
                  "authority_config")
    rollback = preparation.get("rollback", {})
    require_hash(rollback.get("manifest_root_sha256"), "rollback_manifest")
    entries = rollback.get("entries")
    require(isinstance(entries, list), "rollback_entries_invalid")
    require_exact(
        {entry.get("path") for entry in entries if isinstance(entry, dict)},
        {
            "nando-transition-serving",
            "transition-serving.env",
            "nando-transition-serving.service",
            "previous-deployment-receipt.json",
        },
        "rollback_entry_set",
    )
    entry_by_path = {entry["path"]: entry for entry in entries}
    manifest = "".join(
        f'{entry["sha256"]} {entry["size_bytes"]} {entry["path"]}\n'
        for entry in sorted(entries, key=lambda item: item["path"])
    ).encode("ascii")
    require_exact(digest(manifest), rollback.get("manifest_root_sha256"),
                  "rollback_manifest")
    require_exact(entry_by_path["nando-transition-serving"].get("sha256"),
                  BASELINE_BINARY_SHA256, "rollback_binary")
    require_exact(entry_by_path["transition-serving.env"].get("sha256"),
                  BASELINE_CONFIG_SHA256, "rollback_config")
    require_exact(entry_by_path["nando-transition-serving.service"].get("sha256"),
                  UNIT_SHA256, "rollback_unit")


def verify_service_snapshot(snapshot: Any, label: str) -> dict[str, Any]:
    require(isinstance(snapshot, dict), f"{label}_not_object")
    require_exact(set(snapshot), set(ALL_UNITS), f"{label}_unit_set")
    for unit, state in snapshot.items():
        require(isinstance(state, dict), f"{label}_{unit}_not_object")
        require_exact(state.get("active_state"), "active", f"{label}_{unit}_active")
        require(require_nonnegative_int(state.get("main_pid"), f"{label}_{unit}_pid") > 0,
                f"{label}_{unit}_pid_zero")
        require_nonnegative_int(state.get("nrestarts"), f"{label}_{unit}_nrestarts")
        require_hash(state.get("fragment_sha256"), f"{label}_{unit}_fragment")
    return snapshot


def verify_resource(resource: dict[str, Any]) -> None:
    require_exact(resource.get("schema"), RESOURCE_SCHEMA, "resource_schema")
    verify_embedded_root(resource, "resource_root_sha256", "resource")
    require_exact(resource.get("all_pass"), True, "resource_all_pass")
    metrics = resource.get("metrics", {})
    for name, limit_p99, limit_max, expected_samples in (
        ("hot_latency", 1_000_000, 2_000_000, 4096),
        ("single_ledger_sync", 5_000_000, 20_000_000, 1024),
        ("three_ledger_sync", 5_000_000, 20_000_000, 256),
    ):
        runs = metrics.get(name)
        require(isinstance(runs, list) and len(runs) == 3, f"{name}_run_count")
        for index, run in enumerate(runs):
            require(isinstance(run, dict), f"{name}_{index}_not_object")
            require(require_nonnegative_int(run.get("p99_ns"), f"{name}_{index}_p99") <= limit_p99,
                    f"{name}_{index}_p99_budget")
            require(require_nonnegative_int(run.get("hard_max_ns"), f"{name}_{index}_max") <= limit_max,
                    f"{name}_{index}_max_budget")
            require_exact(run.get("samples"), expected_samples, f"{name}_{index}_samples")
            if name == "hot_latency":
                require(require_nonnegative_int(run.get("no_goal_p99_ns"),
                                                f"{name}_{index}_no_goal") <= 250_000,
                        f"{name}_{index}_no_goal_budget")
    idle = metrics.get("idle_cpu", {})
    require(type(idle.get("percent_of_one_core")) in (int, float), "idle_cpu_percent_invalid")
    require(0 <= idle["percent_of_one_core"] <= 0.25, "idle_cpu_budget")
    rss = metrics.get("rss", {})
    require(require_nonnegative_int(rss.get("delta_bytes"), "rss_delta") <= 16 * 1024 * 1024,
            "rss_delta_budget")
    require_exact(
        resource.get("frozen_bounds"),
        {
            "max_precommit_bytes": 32 * 1024,
            "max_typed_goal_bytes": 4 * 1024,
            "max_k1_actions": 256,
            "segment_bytes": 64 * 1024 * 1024,
            "journal_quota_bytes": 2 * 1024 * 1024 * 1024,
            "persisted_raw_payload_bytes": 0,
        },
        "frozen_bounds",
    )


def verify_parity(parity: dict[str, Any]) -> None:
    require_exact(parity.get("schema"), PARITY_SCHEMA, "parity_schema")
    verify_embedded_root(parity, "parity_root_sha256", "parity")
    require_exact(parity.get("byte_identical"), True, "parity_identical")
    require_exact(parity.get("baseline_output_sha256"), parity.get("candidate_output_sha256"),
                  "parity_output")
    require(require_nonnegative_int(parity.get("rows"), "parity_rows") > 0, "parity_rows_zero")


def verify_connector(before: Any, after: Any) -> None:
    require(isinstance(before, dict) and isinstance(after, dict), "connector_not_object")
    for field in ("main_pid", "nrestarts", "route_receipt_failures", "command_sha256"):
        require_exact(after.get(field), before.get(field), f"connector_{field}")
    require_exact(before.get("active_state"), "active", "connector_before_active")
    require_exact(after.get("active_state"), "active", "connector_after_active")


def verify_journal(before: Any, after: Any, *, rollback: bool) -> None:
    require(isinstance(before, dict) and isinstance(after, dict), "journal_not_object")
    require_nonnegative_int(before.get("total_bytes"), "journal_before_bytes")
    require_nonnegative_int(after.get("total_bytes"), "journal_after_bytes")
    require(after["total_bytes"] <= 2 * 1024 * 1024 * 1024, "journal_quota")
    require_exact(after.get("raw_payload_bytes"), 0, "journal_raw_payload")
    require_hash(before.get("manifest_root_sha256"), "journal_before_root")
    require_hash(after.get("manifest_root_sha256"), "journal_after_root")
    if rollback:
        require_exact(after.get("preserved_prefixes"), True, "rollback_journal_prefixes")


def verify_receipt(directory: Path) -> dict[str, Any]:
    preparation = load_json(directory / "preparation.json")
    resource = load_json(directory / "resource-receipt.json")
    parity = load_json(directory / "parity-receipt.json")
    receipt = load_json(directory / "deployment-receipt.json")
    verify_identity(preparation)
    verify_resource(resource)
    verify_parity(parity)
    require_exact(receipt.get("schema"), SCHEMA, "receipt_schema")
    verify_embedded_root(receipt, "receipt_root_sha256", "receipt")
    require_exact(receipt.get("preparation_root_sha256"), preparation["preparation_root_sha256"],
                  "receipt_preparation_root")
    require_exact(receipt.get("transaction_id"), preparation.get("transaction_id"),
                  "receipt_transaction_id")
    require_exact(receipt.get("resource_root_sha256"), resource["resource_root_sha256"],
                  "receipt_resource_root")
    require_exact(receipt.get("parity_root_sha256"), parity["parity_root_sha256"],
                  "receipt_parity_root")
    verdict = receipt.get("verdict")
    require(verdict in {"S1C3_DEPLOYMENT_PASS", "S1C3_ROLLBACK_PASS", "S1C3_VETO"},
            "verdict_invalid")

    before = verify_service_snapshot(receipt.get("services_before"), "services_before")
    after = verify_service_snapshot(receipt.get("services_after"), "services_after")
    survival = verify_service_snapshot(receipt.get("services_survival"), "services_survival")
    rollback = verdict == "S1C3_ROLLBACK_PASS"
    expected_binary = BASELINE_BINARY_SHA256 if rollback else preparation["candidate"]["binary_sha256"]
    expected_config = BASELINE_CONFIG_SHA256 if rollback else CANDIDATE_CONFIG_SHA256
    require_exact(receipt.get("installed_binary_sha256"), expected_binary, "installed_binary")
    require_exact(receipt.get("installed_config_sha256"), expected_config, "installed_config")

    old_pid = before[TRANSITION_UNIT]["main_pid"]
    new_pid = after[TRANSITION_UNIT]["main_pid"]
    require(new_pid != old_pid, "transition_pid_did_not_change")
    require_exact(survival[TRANSITION_UNIT]["main_pid"], new_pid, "transition_survival_pid")
    require_exact(after[TRANSITION_UNIT]["nrestarts"], before[TRANSITION_UNIT]["nrestarts"],
                  "transition_nrestarts")
    require_exact(survival[TRANSITION_UNIT]["nrestarts"], before[TRANSITION_UNIT]["nrestarts"],
                  "transition_survival_nrestarts")
    for unit in UNTOUCHED_UNITS:
        require_exact(after[unit], before[unit], f"untouched_after_{unit}")
        require_exact(survival[unit], before[unit], f"untouched_survival_{unit}")

    verify_connector(receipt.get("connector_before"), receipt.get("connector_after"))
    verify_journal(receipt.get("journal_before"), receipt.get("journal_after"), rollback=rollback)
    require_exact(receipt.get("survival_seconds"), 15, "survival_seconds")
    require_exact(receipt.get("startup_log_clean"), True, "startup_log")
    require_exact(receipt.get("health_semantics_preserved"), True, "health_semantics")
    require_exact(receipt.get("route_probe_equivalent"), True, "route_probe")
    require_exact(receipt.get("active_packages_preserved"), True, "active_packages")
    require_exact(receipt.get("false_accepts_after"), 0, "false_accepts")
    require_exact(receipt.get("runtime_parity_failures_after"), 0, "runtime_parity_failures")
    immutable_after = receipt.get("immutable_after", {})
    require_exact(immutable_after.get("unit_sha256"), UNIT_SHA256, "final_unit")
    require_exact(immutable_after.get("phase_config_sha256"), PHASE_CONFIG_SHA256, "final_phase")
    require_exact(immutable_after.get("authority_config_sha256"), AUTHORITY_CONFIG_SHA256,
                  "final_authority")
    if verdict == "S1C3_DEPLOYMENT_PASS":
        environment = receipt.get("capture_environment", {})
        require_exact(environment.get("NANDO_GROUNDED_DECISION_SHADOW_ENABLED"), "1",
                      "capture_enabled")
        require_exact(environment.get("NANDO_GROUNDED_DECISION_JOURNAL"),
                      "/var/lib/nando-wave/transition/grounded-meaning-v1/decision-contract-precommits-v1",
                      "capture_journal")
        require_exact(receipt.get("capture_available"), True, "capture_available")
    require(verdict != "S1C3_VETO", "terminal_veto")
    return {
        "schema": "nando.s1c3-verification.v1",
        "valid": True,
        "verdict": verdict,
        "receipt_root_sha256": receipt["receipt_root_sha256"],
        "preparation_root_sha256": preparation["preparation_root_sha256"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("transaction_directory", type=Path)
    parser.add_argument("--allow-rollback", action="store_true")
    args = parser.parse_args()
    try:
        result = verify_receipt(args.transaction_directory)
        if result["verdict"] == "S1C3_ROLLBACK_PASS" and not args.allow_rollback:
            raise InvalidReceipt("rollback_is_not_deployment_pass")
    except InvalidReceipt as error:
        print(json.dumps({"schema": "nando.s1c3-verification.v1", "valid": False,
                          "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
