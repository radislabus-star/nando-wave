#!/usr/bin/env python3
"""Authority-free verifier for the consumed S1C-3C terminal attempt."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


SCHEMA = "nando.s1c3c-postmortem-verification.v1"
STATUS_SCHEMA = "nando.s1c3c-terminal-status.v1"
TRANSACTION_ID = "20260812T113705Z-2a1505055ce9-s1c3c-v1"
SOURCE_COMMIT = "739ba005557c578c12bcf421219f4c20bbb1fc17"
PAPER_COMMIT = "2a1505055ce98b3f6bed5cb440a0faa345fb78cb"
LOCAL_MANIFEST_ROOT = "54c223887103d3f781e23df124f158c794a86533823559557377b75c1ea54bee"
REMOTE_MANIFEST_ROOT = "eaaf3977b4d5545d87a017e4c79b6b2e3eaba4aac45082583281e5ffa03588dd"
RESOURCE_ROOT = "174bc9ac3f7e7a6d53561bca3059c721bb59e2e6886baf6af29bb201016d23d0"
PARITY_ROOT = "6390d4ac7b5acebbbf7b9d56b692662a64c7ddfa7fc39a8482a1797f9f62af6e"
MONITOR_ROOT = "6270d152046ac614ddfbb1a9bfe1d6a3a254f067092e4138c2d9d58d0abc9b5b"
FREEZE_ROOT = "8df006e693d5ab582a189a185c84722d3017f483028242302e582490d72b4b9d"
SCHEMA_ROOT = "228f602627b7ca32437924e3776b13a44f9439307bd7bf4bc2d179386fe9daf7"
OWNERSHIP_ROOT = "a93e4d839af3d02b76c560ef0a113977659aaf97b6b56e56fa0a97448b585430"
LOCAL_FILES = 46
LOCAL_BYTES = 74_701_441
REMOTE_FILES = 39
REMOTE_BYTES = 74_696_658
SETTLEMENT_P99_LIMIT_NS = 5_000_000
RESOURCE_FAILURES = [
    "parity:byte_identity",
    "three-sync-2:settlement_p99_ns",
    "three-sync-3:settlement_p99_ns",
]
INSTRUMENT_FAILURES = [
    "three-sync-2:test_assertion_failed",
    "three-sync-3:test_assertion_failed",
]
REQUIRED_FILE_SHA256 = {
    "connector-after.json": "86f5a000d229fe8088763f44b205df2a6c9f7730bf0ea85deeee71f3123b8866",
    "connector-before.json": "64e2270dd31c7da733bd751d4d7d240cc40b28b8d4644cca9cb3c926ae7b9fc6",
    "evidence/parity-baseline.log": "b02f80968d061c869dbe5f052ff345dcfd8593d94116fdf4e5d88ed04b688238",
    "evidence/parity-candidate.log": "2e7df73c19853871b1ec30d7d8063655a85b18d524c6dce78743e0846a01144d",
    "implementation-freeze.json": "2541cea82f4cbbb6c66e8e8c9414dae727de9031870c8151a90571821c7a8886",
    "measurement-monitor-receipt.json": "35b315c1acf50b7d6b80d6ff120c0b5c77a9159e0558780782512b9487aa7ed0",
    "oracle-ownership-receipt.json": "572d4997d5c94623d66e9067fd33b825138bde42b7f2d5957b6ed74afb423832",
    "parity-receipt.json": "a5bbafa032d7865a1e22a0742d0ec7750192efbb1aa96095cd306ee466bfe03e",
    "resource-receipt.json": "9b22229dfe524c99b2920bbf6a259ec3ad710f90a3653d8801756d4aa500fa09",
    "s1c3c-authority-envelope.candidate.json": "4fd1bfe65143318c6bf1bfd18807bf692dbe2f00e6ce61ceae041458c3f5d40c",
    "schema-preflight.json": "af8ff240d8a2b1dc7bd3866e6091995c1e11182e81c7a77e1a119cd04c4f1cbf",
    "transaction-state.json": "0b60459e1cfe43559612393ff6e0088c18450de7ca74d026f532c446cd83d838",
}


class InvalidEvidence(ValueError):
    pass


def canonical_bytes(value: Any, omit: str | None = None) -> bytes:
    if omit is not None:
        value = dict(value)
        value.pop(omit, None)
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def file_digest(path: Path) -> str:
    return digest(path.read_bytes())


def require(condition: bool, error: str) -> None:
    if not condition:
        raise InvalidEvidence(error)


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise InvalidEvidence(f"json_invalid:{path.name}") from error
    require(isinstance(value, dict), f"json_not_object:{path.name}")
    return value


def verify_root(value: dict[str, Any], field: str, expected: str, label: str) -> None:
    require(value.get(field) == expected, f"{label}_root_unexpected")
    require(digest(canonical_bytes(value, field)) == expected, f"{label}_root_invalid")


def manifest(directory: Path) -> tuple[str, int, int]:
    rows = []
    total = 0
    for path in sorted(candidate for candidate in directory.rglob("*") if candidate.is_file()):
        relative = path.relative_to(directory).as_posix()
        size = path.stat().st_size
        rows.append(f"{file_digest(path)} {size} {relative}\n")
        total += size
    return digest("".join(rows).encode()), len(rows), total


def verify_normalized_manifest(path: Path) -> tuple[str, int, int]:
    rows = path.read_text(encoding="utf-8").splitlines()
    total = 0
    normalized = []
    for index, row in enumerate(rows, start=1):
        match = re.fullmatch(r"([0-9a-f]{64}) ([0-9]+) ([^\x00]+)", row)
        require(match is not None, f"remote_manifest_row_invalid:{index}")
        total += int(match.group(2))
        normalized.append(f"{row}\n")
    return digest("".join(normalized).encode()), len(rows), total


def verify_attempt(directory: Path, remote_manifest: Path) -> dict[str, Any]:
    require(directory.name == TRANSACTION_ID, "transaction_directory")
    for relative, expected in REQUIRED_FILE_SHA256.items():
        path = directory / relative
        require(path.is_file(), f"required_file_missing:{relative}")
        require(file_digest(path) == expected, f"required_file_drift:{relative}")

    local_root, local_files, local_bytes = manifest(directory)
    require((local_root, local_files, local_bytes) == (LOCAL_MANIFEST_ROOT, LOCAL_FILES, LOCAL_BYTES), "local_manifest")
    remote_root, remote_files, remote_bytes = verify_normalized_manifest(remote_manifest)
    require((remote_root, remote_files, remote_bytes) == (REMOTE_MANIFEST_ROOT, REMOTE_FILES, REMOTE_BYTES), "remote_manifest")

    state = load_json(directory / "transaction-state.json")
    require(state == {
        "production_mutation": False,
        "resource_root_sha256": RESOURCE_ROOT,
        "schema": "nando.s1c3b-state.v1",
        "state": "RESOURCE_VETO",
        "transaction_id": TRANSACTION_ID,
        "verdict": "S1C3B_RESOURCE_VETO",
    }, "transaction_state")

    freeze = load_json(directory / "implementation-freeze.json")
    verify_root(freeze, "implementation_freeze_root_sha256", FREEZE_ROOT, "freeze")
    require(freeze.get("source_commit") == SOURCE_COMMIT, "freeze_source_commit")
    require(freeze.get("paper_commit") == PAPER_COMMIT, "freeze_paper_commit")

    schema = load_json(directory / "schema-preflight.json")
    verify_root(schema, "schema_preflight_root_sha256", SCHEMA_ROOT, "schema")
    require(schema.get("valid") is True and schema.get("authority") is False, "schema_boundary")
    require(schema.get("side_effects") is False and schema.get("remote_attempt_created") is False, "schema_side_effects")

    ownership = load_json(directory / "oracle-ownership-receipt.json")
    verify_root(ownership, "ownership_root_sha256", OWNERSHIP_ROOT, "ownership")
    require(ownership.get("transaction_id") == TRANSACTION_ID, "ownership_transaction")

    monitor = load_json(directory / "measurement-monitor-receipt.json")
    verify_root(monitor, "monitor_root_sha256", MONITOR_ROOT, "monitor")
    require(monitor.get("transaction_id") == TRANSACTION_ID, "monitor_transaction")
    require(monitor.get("instrument_pass") is True and monitor.get("monitor_errors") == [], "monitor_instrument")

    parity = load_json(directory / "parity-receipt.json")
    verify_root(parity, "parity_root_sha256", PARITY_ROOT, "parity")
    require(parity.get("byte_identical") is False and parity.get("row_count") == 4, "parity_verdict")
    rows = parity.get("rows")
    require(isinstance(rows, list) and len(rows) == 2, "parity_rows")
    for row, label in zip(rows, ("baseline", "candidate"), strict=True):
        require(row.get("label") == label, f"parity_label:{label}")
        require(row.get("command", {}).get("returncode") == 101, f"parity_returncode:{label}")
        log = (directory / "evidence" / f"parity-{label}.log").read_text(encoding="utf-8")
        require("PermissionDenied" in log and 'message: "Permission denied"' in log, f"parity_permission:{label}")

    resource = load_json(directory / "resource-receipt.json")
    verify_root(resource, "resource_root_sha256", RESOURCE_ROOT, "resource")
    require(resource.get("monitor_root_sha256") == MONITOR_ROOT, "resource_monitor")
    require(resource.get("parity_root_sha256") == PARITY_ROOT, "resource_parity")
    require(resource.get("oracle_ownership_root_sha256") == OWNERSHIP_ROOT, "resource_ownership")
    require(resource.get("all_pass") is False and resource.get("round_count") == 3, "resource_verdict")
    require(resource.get("resource_failures") == RESOURCE_FAILURES, "resource_failures")
    require(resource.get("instrument_failures") == INSTRUMENT_FAILURES, "instrument_failures")
    rounds = resource.get("metrics", {}).get("three_ledger_sync")
    require(isinstance(rounds, list) and len(rounds) == 3, "three_sync_rows")
    settlement = {row.get("label"): row.get("metrics", {}).get("settlement_p99_ns") for row in rounds}
    require(settlement == {
        "three-sync-1": 4_431_531,
        "three-sync-2": 5_097_076,
        "three-sync-3": 6_104_611,
    }, "settlement_p99")
    require(settlement["three-sync-1"] <= SETTLEMENT_P99_LIMIT_NS, "settlement_round_1")
    require(settlement["three-sync-2"] > SETTLEMENT_P99_LIMIT_NS, "settlement_round_2")
    require(settlement["three-sync-3"] > SETTLEMENT_P99_LIMIT_NS, "settlement_round_3")

    envelope = load_json(directory / "s1c3c-authority-envelope.candidate.json")
    require(envelope == {
        "authority": False,
        "error": "parity_output_mismatch",
        "schema": "nando.s1c3c-authority-envelope.v1",
        "valid": False,
    }, "authority_envelope")

    before = load_json(directory / "connector-before.json")
    after = load_json(directory / "connector-after.json")
    for snapshot in (before, after):
        require(snapshot.get("active_state") == "active", "connector_active")
        require(snapshot.get("main_pid") == 2919, "connector_pid")
        require(snapshot.get("nrestarts") == 0, "connector_restarts")
        require(snapshot.get("route_receipt_failures") == 0, "connector_receipts")

    report = {
        "schema": SCHEMA,
        "valid": True,
        "authority": False,
        "scientific_authority": False,
        "transaction_id": TRANSACTION_ID,
        "operational_verdict": "RESOURCE_VETO",
        "production_mutation": False,
        "capture_installed": False,
        "attempt_consumed": True,
        "authority_envelope": "UNSEALED",
        "authority_envelope_error": "parity_output_mismatch",
        "blocker": "parity_output_mismatch_in_frozen_terminal_verifier",
        "resource_failures": RESOURCE_FAILURES,
        "settlement_p99_limit_ns": SETTLEMENT_P99_LIMIT_NS,
        "settlement_p99_ns": settlement,
        "parity": {
            "byte_identical": False,
            "baseline_returncode": 101,
            "candidate_returncode": 101,
            "failure": "registry_permission_denied",
        },
        "roots": {
            "resource": RESOURCE_ROOT,
            "parity": PARITY_ROOT,
            "monitor": MONITOR_ROOT,
            "freeze": FREEZE_ROOT,
            "schema": SCHEMA_ROOT,
            "local_manifest": LOCAL_MANIFEST_ROOT,
            "remote_manifest": REMOTE_MANIFEST_ROOT,
        },
        "s1c4_started": False,
        "s2_started": False,
        "model_training_allowed": False,
        "phase_mutation_allowed": False,
        "rerun_allowed": False,
    }
    report["postmortem_root_sha256"] = digest(canonical_bytes(report))
    return report


def terminal_status(report: dict[str, Any]) -> dict[str, Any]:
    status = {
        "schema": STATUS_SCHEMA,
        "transaction_id": TRANSACTION_ID,
        "verdict": "RESOURCE_VETO",
        "resource_verdict": "S1C3B_RESOURCE_VETO",
        "deployment_verdict": None,
        "blocker": "parity_output_mismatch_in_frozen_terminal_verifier",
        "capture_installed": False,
        "production_mutation": False,
        "attempt_consumed": True,
        "authority_envelope": "UNSEALED",
        "authority_ready": False,
        "scientific_authority": False,
        "s1c4_started": False,
        "s2_started": False,
        "model_training_allowed": False,
        "phase_mutation_allowed": False,
        "rerun_allowed": False,
        "resource_root_sha256": RESOURCE_ROOT,
        "parity_root_sha256": PARITY_ROOT,
        "postmortem_root_sha256": report["postmortem_root_sha256"],
    }
    status["terminal_status_root_sha256"] = digest(canonical_bytes(status))
    return status


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--attempt", type=Path, required=True)
    parser.add_argument("--remote-manifest", type=Path, required=True)
    parser.add_argument("--status", action="store_true")
    arguments = parser.parse_args()
    try:
        report = verify_attempt(arguments.attempt.resolve(), arguments.remote_manifest.resolve())
        value = terminal_status(report) if arguments.status else report
    except (InvalidEvidence, OSError) as error:
        print(json.dumps({"schema": SCHEMA, "valid": False, "authority": False, "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(value, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
