#!/usr/bin/env python3
"""Fail-closed verifier for the frozen S1C-3 V7 transaction receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path
from typing import Any


SCHEMA = "nando.s1c3-transaction-receipt.v7"
PREPARATION_SCHEMA = "nando.s1c3-transaction-preparation.v7"
RESOURCE_SCHEMA = "nando.s1c3-resource-receipt.v7"
PARITY_SCHEMA = "nando.s1c3-parity-receipt.v7"
QUIESCENCE_SCHEMA = "nando.s1c3-quiescence-receipt.v7"
CONTAMINATION_SCHEMA = "nando.s1c3-measurement-contamination-receipt.v7"
OWNERSHIP_SCHEMA = "nando.s1c3-oracle-ownership-receipt.v7"
OWNERSHIP_ROW_SCHEMA = "nando.s1c3-oracle-ownership-row.v7"
PROCESS_SNAPSHOT_SCHEMA = "nando.s1c3-process-snapshot.v7"
PROCESS_DETECTOR_SCHEMA = "proc-stat-starttime-three-status-v1"

OBSERVABLE_USER_PROCESS = "OBSERVABLE_USER_PROCESS"
PROVEN_NON_EXECUTING = "PROVEN_NON_EXECUTING"
UNRESOLVED_PROCESS_OBSERVATION = "UNRESOLVED_PROCESS_OBSERVATION"

PAPER_COMMIT = "75223baaa4958b3bd3e950c87cfdcc0a375ccc85"
PAPER_MANIFEST_ROOT = "4c259dd1f42d7d77a9839956c138744cfff42dc0ee91330c361065e005d72581"
PAPER_VERIFICATION_SHA256 = "293146e532d7a4d0bb5c2fa031d8a29543aa311a69d4dee1d6264362a4290b4b"
CANDIDATE_COMMIT = "03e3dd00c90206e2f705371318c50dd50537d6d8"
CANDIDATE_TREE = "06a9df51797dffc127fec41672bddae29c38bb92"
PRODUCTION_PROJECTION_PATH = (
    "crates/nando-transition-serving/src/grounded_decision_capture.rs"
)
PRODUCTION_PROJECTION_SCHEMA = "prefix-through-first-cfg-test-line-v1"
PRODUCTION_PROJECTION_SHA256 = (
    "10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316"
)
CARGO_LOCK_SHA256 = "0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1"
CANDIDATE_CONFIG_SHA256 = "1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6"
ORACLE_SOURCE_SHA256 = "bc5a2255de62a05b44be677ba67331cfbf47b884557f8d8a0d3ac06e46c64b26"
ORACLE_LOCK_SHA256 = "498855d2a867ba80dba55ad1609bf937852aa61e9de97203d26f067a619da32b"
ORACLE_PACKAGE_NAME = "s1c3-parity-oracle"
ORACLE_PACKAGE_VERSION = "0.1.0"
ORACLE_EDITION = "2024"
ORACLE_CARGO_COMMAND_PREFIX = [
    "/home/e/.cargo/bin/cargo", "build", "--release", "--quiet", "--offline",
    "--locked", "--manifest-path",
]
ORACLE_CARGO_ENVIRONMENT = {
    "CARGO_NET_OFFLINE": "true",
    "RUSTFLAGS": "-Awarnings",
}
ORACLE_BUILD_CONTRACT_SCHEMA = "nando.s1c3-oracle-build-contract.v1"
BASELINE_COMMIT = "663959064a37caf7eb917fc99dfedb6386355fa6"
BASELINE_TREE = "05460ccbc9c44ac8b7174318903c0211de709e2e"
BASELINE_RECEIPT_ROOT = "785450d76037410d96baade19c2b6bb7f0fb24c6be034e2166be5533c7dd985b"
BASELINE_BINARY_SHA256 = "6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58"
BASELINE_CONFIG_SHA256 = "cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5"
UNIT_SHA256 = "6e9d2fe41b1db95f94768d1ab41dffce1f15be92e2f774832c7fe392bb77b135"
PHASE_CONFIG_SHA256 = "5c019cebbde083f963c03619ff1d938786f5b4ec58730dddd5b34adeb33cce31"
AUTHORITY_CONFIG_SHA256 = "d40b7262ff6d744a393b0fc03a5d06610d01728aa2f4603199ca8567189ec88f"
ORACLE_UID = 1000
ORACLE_GID = 1000

TRANSITION_UNIT = "nando-transition-serving.service"
UNTOUCHED_UNITS = (
    "nando-transport-gateway.service",
    "nando-response-learning.service",
    "nando-gateway-control.service",
    "nando-operator-certification-authority.service",
)
ALL_UNITS = (TRANSITION_UNIT, *UNTOUCHED_UNITS)

FORBIDDEN_BUILD_NAMES = (
    "cargo", "rustc", "sccache", "cc", "cc1", "cc1plus", "gcc", "g++",
    "clang", "clang++", "ld", "ld.lld", "lld", "mold", "ninja", "make",
    "cmake", "meson",
)
MEASUREMENT_CPU_POOL = [4, 6]
PHYSICAL_CORE_SIBLINGS = {4: [4, 5], 6: [6, 7]}
EXPECTED_CORE_IDS = {4: 8, 6: 12}
EXPECTED_MAX_FREQUENCY_KHZ = 5_400_000
LOGICAL_CPU_POOL = [4, 5, 6, 7]
EXECUTABLE_KEYS = {
    "candidate-binary", "test-response-actor", "test-transition-serving",
    "parity-baseline", "parity-candidate",
}
MEASUREMENT_LABELS = [
    "hot-1", "hot-2", "hot-3",
    "single-sync-1", "single-sync-2", "single-sync-3",
    "three-sync-1", "three-sync-2", "three-sync-3",
    "idle", "rss-capture_off", "rss-capture_on",
    "parity-baseline", "parity-candidate",
]


class InvalidReceipt(ValueError):
    pass


def canonical_bytes(value: Any, *, omit_field: str | None = None) -> bytes:
    if omit_field is not None:
        value = dict(value)
        value.pop(omit_field, None)
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


def require_number(value: Any, label: str) -> float:
    require(type(value) in (int, float) and value >= 0, f"{label}_invalid")
    return float(value)


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise InvalidReceipt(f"json_invalid:{path.name}:{error}") from error
    require(isinstance(value, dict), f"json_not_object:{path.name}")
    return value


def verify_embedded_root(value: dict[str, Any], field: str, label: str) -> str:
    expected = require_hash(value.get(field), field)
    actual = digest(canonical_bytes(value, omit_field=field))
    require_exact(actual, expected, f"{label}_root")
    return actual


def require_field_set(value: Any, fields: set[str], label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label}_not_object")
    require_exact(set(value), fields, f"{label}_fields")
    return value


def verify_proc_identity(value: Any, label: str) -> dict[str, Any]:
    row = require_field_set(value, {"pid", "comm", "state", "starttime"}, label)
    require(require_nonnegative_int(row["pid"], f"{label}_pid") > 0, f"{label}_pid_zero")
    require(isinstance(row["comm"], str), f"{label}_comm_invalid")
    require(isinstance(row["state"], str) and len(row["state"]) == 1,
            f"{label}_state_invalid")
    require_nonnegative_int(row["starttime"], f"{label}_starttime")
    return row


def verify_stat_observation(value: Any, label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label}_not_object")
    status = value.get("status")
    if status == "VALUE":
        require_field_set(value, {"status", "parsed", "raw"}, label)
        require(isinstance(value["raw"], str), f"{label}_raw_invalid")
        try:
            parsed = independently_parse_proc_stat(value["raw"])
        except ValueError as error:
            raise InvalidReceipt(f"{label}_raw_malformed") from error
        require_exact(value["parsed"], parsed, f"{label}_parsed")
        verify_proc_identity(parsed, f"{label}_parsed")
    elif status == "MALFORMED":
        require_field_set(value, {"status", "error", "raw"}, label)
        require(isinstance(value["error"], str), f"{label}_error_invalid")
        require(isinstance(value["raw"], str), f"{label}_raw_invalid")
        try:
            independently_parse_proc_stat(value["raw"])
        except ValueError:
            pass
        else:
            raise InvalidReceipt(f"{label}_malformed_claim_false")
    elif status in {"ENOENT", "PERMISSION_DENIED"}:
        require_field_set(value, {"status"}, label)
    elif status == "OS_ERROR":
        require_field_set(value, {"status", "error"}, label)
        require(isinstance(value["error"], str), f"{label}_error_invalid")
    else:
        raise InvalidReceipt(f"{label}_status_invalid")
    return value


def independently_parse_proc_stat(raw: str) -> dict[str, Any]:
    opening = raw.find("(")
    closing = raw.rfind(")")
    if opening <= 0 or closing <= opening or closing + 2 > len(raw):
        raise ValueError("stat_shape_invalid")
    try:
        pid = int(raw[:opening].strip())
    except ValueError as error:
        raise ValueError("stat_pid_invalid") from error
    tail = raw[closing + 2:].split()
    if len(tail) <= 19 or len(tail[0]) != 1:
        raise ValueError("stat_tail_invalid")
    try:
        starttime = int(tail[19])
    except ValueError as error:
        raise ValueError("stat_starttime_invalid") from error
    if pid <= 0 or starttime < 0:
        raise ValueError("stat_identity_invalid")
    return {
        "pid": pid,
        "comm": raw[opening + 1:closing],
        "state": tail[0],
        "starttime": starttime,
    }


def verify_status_observation(value: Any, label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label}_not_object")
    status = value.get("status")
    if status == "VALUE":
        require_field_set(value, {"status", "kthread_line"}, label)
        require(isinstance(value["kthread_line"], str), f"{label}_line_invalid")
    elif status in {"ENOENT", "PERMISSION_DENIED"}:
        require_field_set(value, {"status"}, label)
    elif status in {"MALFORMED", "OS_ERROR"}:
        require_field_set(value, {"status", "error"}, label)
        require(isinstance(value["error"], str), f"{label}_error_invalid")
    else:
        raise InvalidReceipt(f"{label}_status_invalid")
    return value


def verify_cmdline_observation(value: Any, label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label}_not_object")
    status = value.get("status")
    if status == "VALUE":
        require_field_set(value, {"status", "payload_hex"}, label)
        require(isinstance(value["payload_hex"], str), f"{label}_payload_invalid")
        try:
            bytes.fromhex(value["payload_hex"])
        except ValueError as error:
            raise InvalidReceipt(f"{label}_payload_invalid") from error
    elif status in {"ENOENT", "PERMISSION_DENIED"}:
        require_field_set(value, {"status"}, label)
    elif status == "OS_ERROR":
        require_field_set(value, {"status", "error"}, label)
        require(isinstance(value["error"], str), f"{label}_error_invalid")
    else:
        raise InvalidReceipt(f"{label}_status_invalid")
    return value


def verify_exe_observation(value: Any, label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label}_not_object")
    status = value.get("status")
    if status == "VALUE":
        require_field_set(value, {"status", "target", "basename"}, label)
        require(isinstance(value["target"], str) and isinstance(value["basename"], str),
                f"{label}_value_invalid")
        require_exact(value["basename"], os.path.basename(value["target"]),
                      f"{label}_basename")
    elif status in {"ENOENT", "PERMISSION_DENIED"}:
        require_field_set(value, {"status"}, label)
    elif status in {"MALFORMED", "OS_ERROR"}:
        require_field_set(value, {"status", "error"}, label)
        require(isinstance(value["error"], str), f"{label}_error_invalid")
    else:
        raise InvalidReceipt(f"{label}_status_invalid")
    return value


def parse_kthread_line(line: str) -> bool:
    match = re.fullmatch(r"Kthread:\s*([01])", line)
    if match is None:
        raise ValueError("kthread_line_invalid")
    return match.group(1) == "1"


def independently_classify_process(row: dict[str, Any], label: str) -> dict[str, Any]:
    require_field_set(row, {
        "pid", "opening_stat", "status", "cmdline", "exe", "closing_stat",
        "classification", "reason", "identity", "executable_basename",
        "forbidden_names",
    }, label)
    pid = require_nonnegative_int(row["pid"], f"{label}_pid")
    require(pid > 0, f"{label}_pid_zero")
    opening_observation = verify_stat_observation(row["opening_stat"], f"{label}_opening")
    closing_observation = verify_stat_observation(row["closing_stat"], f"{label}_closing")
    status_observation = verify_status_observation(row["status"], f"{label}_status")
    cmdline_observation = verify_cmdline_observation(row["cmdline"], f"{label}_cmdline")
    exe_observation = verify_exe_observation(row["exe"], f"{label}_exe")
    result: dict[str, Any] = {
        "classification": UNRESOLVED_PROCESS_OBSERVATION,
        "reason": "unclassified",
        "identity": None,
        "executable_basename": None,
        "forbidden_names": [],
    }
    if opening_observation["status"] != "VALUE":
        result["reason"] = f"opening_stat_{opening_observation['status'].lower()}"
    else:
        opening = opening_observation["parsed"]
        if opening["pid"] != pid:
            result["reason"] = "opening_stat_pid_mismatch"
        else:
            if closing_observation["status"] == "ENOENT":
                result.update({
                    "classification": PROVEN_NON_EXECUTING,
                    "reason": "PID_VANISHED",
                    "identity": {
                        "pid": pid,
                        "starttime": opening["starttime"],
                        "opening_comm": opening["comm"],
                        "closing_comm": None,
                        "opening_state": opening["state"],
                        "closing_state": None,
                    },
                })
            elif closing_observation["status"] != "VALUE":
                result["reason"] = f"closing_stat_{closing_observation['status'].lower()}"
            else:
                closing = closing_observation["parsed"]
                if closing["pid"] != pid:
                    result["reason"] = "closing_stat_pid_mismatch"
                elif opening["starttime"] != closing["starttime"]:
                    result["reason"] = "PID_REUSE"
                else:
                    identity = {
                        "pid": pid,
                        "starttime": opening["starttime"],
                        "opening_comm": opening["comm"],
                        "closing_comm": closing["comm"],
                        "opening_state": opening["state"],
                        "closing_state": closing["state"],
                    }
                    result["identity"] = identity
                    if opening["state"] == closing["state"] == "Z":
                        result.update({
                            "classification": PROVEN_NON_EXECUTING,
                            "reason": "STABLE_ZOMBIE",
                        })
                    else:
                        auxiliary = (
                            status_observation, cmdline_observation, exe_observation
                        )
                        bad = next((item for item in auxiliary
                                    if item["status"] not in {"VALUE", "ENOENT"}), None)
                        if bad is not None:
                            result["reason"] = f"auxiliary_{bad['status'].lower()}"
                        elif status_observation["status"] != "VALUE":
                            result["reason"] = f"status_{status_observation['status'].lower()}"
                        elif cmdline_observation["status"] != "VALUE":
                            result["reason"] = f"cmdline_{cmdline_observation['status'].lower()}"
                        else:
                            try:
                                kthread = parse_kthread_line(status_observation["kthread_line"])
                            except ValueError:
                                result["reason"] = "auxiliary_malformed"
                            else:
                                cmdline = bytes.fromhex(cmdline_observation["payload_hex"])
                                if exe_observation["status"] == "VALUE":
                                    basename = exe_observation["basename"]
                                    forbidden = sorted(
                                        {opening["comm"], closing["comm"], basename}
                                        .intersection(FORBIDDEN_BUILD_NAMES)
                                    )
                                    result.update({
                                        "classification": OBSERVABLE_USER_PROCESS,
                                        "reason": "STABLE_EXECUTABLE",
                                        "executable_basename": basename,
                                        "forbidden_names": forbidden,
                                    })
                                elif exe_observation["status"] != "ENOENT":
                                    result["reason"] = f"exe_{exe_observation['status'].lower()}"
                                elif kthread and not cmdline:
                                    result.update({
                                        "classification": PROVEN_NON_EXECUTING,
                                        "reason": "STABLE_KERNEL_THREAD",
                                    })
                                else:
                                    result["reason"] = "missing_exe_without_kernel_thread_proof"
    require_exact(row["classification"], result["classification"],
                  f"{label}_classification")
    require_exact(row["reason"], result["reason"], f"{label}_reason")
    require_exact(row["identity"], result["identity"], f"{label}_identity")
    require_exact(row["executable_basename"], result["executable_basename"],
                  f"{label}_executable")
    require_exact(row["forbidden_names"], result["forbidden_names"],
                  f"{label}_forbidden")
    return result


def process_row_projection(pid: int, result: dict[str, Any]) -> dict[str, Any]:
    identity = result.get("identity") or {}
    return {
        "pid": pid,
        "reason": result["reason"],
        "starttime": identity.get("starttime"),
        "opening_comm": identity.get("opening_comm"),
        "closing_comm": identity.get("closing_comm"),
        "executable_basename": result.get("executable_basename"),
    }


def verify_process_snapshot(value: Any, label: str) -> dict[str, Any]:
    snapshot = require_field_set(value, {
        "schema", "detector_schema", "rows", "summary", "unresolved_rows",
        "forbidden_process_matches", "process_snapshot_root_sha256",
    }, label)
    require_exact(snapshot["schema"], PROCESS_SNAPSHOT_SCHEMA, f"{label}_schema")
    require_exact(snapshot["detector_schema"], PROCESS_DETECTOR_SCHEMA,
                  f"{label}_detector")
    verify_embedded_root(snapshot, "process_snapshot_root_sha256", label)
    rows = snapshot["rows"]
    require(isinstance(rows, list), f"{label}_rows_invalid")
    pids: list[int] = []
    results: list[tuple[int, dict[str, Any]]] = []
    for index, row in enumerate(rows):
        require(isinstance(row, dict), f"{label}_row_{index}_not_object")
        pid = require_nonnegative_int(row.get("pid"), f"{label}_row_{index}_pid")
        pids.append(pid)
        results.append((pid, independently_classify_process(row, f"{label}_row_{index}")))
    require_exact(pids, sorted(set(pids)), f"{label}_pid_order")
    reasons = sorted({result["reason"] for _, result in results})
    summary = {
        "total_rows": len(results),
        "observable_user_process_count": sum(
            result["classification"] == OBSERVABLE_USER_PROCESS for _, result in results
        ),
        "proven_vanished_count": sum(
            result["reason"] == "PID_VANISHED" for _, result in results
        ),
        "proven_zombie_count": sum(
            result["reason"] == "STABLE_ZOMBIE" for _, result in results
        ),
        "proven_kernel_thread_count": sum(
            result["reason"] == "STABLE_KERNEL_THREAD" for _, result in results
        ),
        "unresolved_process_count": sum(
            result["classification"] == UNRESOLVED_PROCESS_OBSERVATION
            for _, result in results
        ),
        "reason_counts": {
            reason: sum(result["reason"] == reason for _, result in results)
            for reason in reasons
        },
    }
    unresolved = [
        process_row_projection(pid, result) for pid, result in results
        if result["classification"] == UNRESOLVED_PROCESS_OBSERVATION
    ]
    forbidden = [
        {**process_row_projection(pid, result), "matched_names": result["forbidden_names"]}
        for pid, result in results if result["forbidden_names"]
    ]
    require_exact(snapshot["summary"], summary, f"{label}_summary")
    require_exact(snapshot["unresolved_rows"], unresolved, f"{label}_unresolved")
    require_exact(snapshot["forbidden_process_matches"], forbidden, f"{label}_forbidden")
    return snapshot


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
    require_exact(candidate.get("production_projection_path"), PRODUCTION_PROJECTION_PATH,
                  "candidate_production_projection_path")
    require_exact(candidate.get("production_projection_schema"), PRODUCTION_PROJECTION_SCHEMA,
                  "candidate_production_projection_schema")
    require_exact(candidate.get("production_projection_sha256"),
                  PRODUCTION_PROJECTION_SHA256,
                  "candidate_production_projection")
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
    require_hash(preparation.get("quiescence_root_sha256"), "preparation_quiescence_root")
    require(preparation.get("measurement_cpu") in MEASUREMENT_CPU_POOL,
            "preparation_measurement_cpu_invalid")
    require_hash(
        preparation.get("measurement_contamination_root_sha256"),
        "preparation_contamination_root",
    )
    require_hash(preparation.get("executable_set_root_sha256"), "preparation_executable_root")
    require_hash(
        preparation.get("oracle_ownership_root_sha256"),
        "preparation_oracle_ownership_root",
    )
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


def oracle_manifest(source: str) -> bytes:
    return (
        f'''[package]\nname = "{ORACLE_PACKAGE_NAME}"\n'''
        f'''version = "{ORACLE_PACKAGE_VERSION}"\n'''
        f'''edition = "{ORACLE_EDITION}"\n\n'''
        f'''[dependencies]\n'''
        f'''nando-response-actor = {{ path = "{source}/crates/nando-response-actor" }}\n'''
        f'''serde_json = "1"\n'''
    ).encode("utf-8")


def expected_oracle_paths(transaction_id: str, label: str) -> dict[str, str]:
    base = f"/home/e/.cache/nando-s1c3-{transaction_id}"
    workspace = f"{base}/oracle-{label}"
    source = f"{base}/baseline" if label == "baseline" else f"{base}/candidate"
    target = f"{base}/oracle-target-{label}"
    return {
        "base": base,
        "workspace": workspace,
        "source": source,
        "target": target,
        "manifest": f"{workspace}/Cargo.toml",
        "lock": f"{workspace}/Cargo.lock",
        "main": f"{workspace}/src/main.rs",
        "executable": f"{target}/release/{ORACLE_PACKAGE_NAME}",
    }


def verify_executables(
    executables: Any,
    transaction_id: str,
) -> tuple[dict[str, Any], str]:
    require(isinstance(executables, dict), "executables_not_object")
    require_exact(set(executables), EXECUTABLE_KEYS, "executable_key_set")
    expected_sources = {
        "candidate-binary": CANDIDATE_COMMIT,
        "test-response-actor": CANDIDATE_COMMIT,
        "test-transition-serving": CANDIDATE_COMMIT,
        "parity-baseline": BASELINE_COMMIT,
        "parity-candidate": CANDIDATE_COMMIT,
    }
    for name, identity in executables.items():
        require(isinstance(identity, dict), f"executable_{name}_not_object")
        require(isinstance(identity.get("path"), str) and identity["path"].startswith("/"),
                f"executable_{name}_path_invalid")
        require_hash(identity.get("sha256"), f"executable_{name}_sha256")
        require(require_nonnegative_int(identity.get("size_bytes"), f"executable_{name}_size") > 0,
                f"executable_{name}_size_zero")
        mode = identity.get("mode_octal")
        require(isinstance(mode, str) and len(mode) == 4 and all(char in "01234567" for char in mode),
                f"executable_{name}_mode_invalid")
        require(int(mode, 8) & 0o111 != 0, f"executable_{name}_not_executable")
        require_exact(identity.get("source_identity"), expected_sources[name],
                      f"executable_{name}_source")
    for label in ("baseline", "candidate"):
        require_exact(
            executables[f"parity-{label}"].get("path"),
            expected_oracle_paths(transaction_id, label)["executable"],
            f"executable_parity-{label}_fresh_path",
        )
    return executables, digest(canonical_bytes(executables))


def verify_oracle_file_identity(
    identity: Any,
    expected_path: str,
    expected_mode: str,
    label: str,
) -> None:
    require(isinstance(identity, dict), f"ownership_{label}_not_object")
    require_exact(
        set(identity),
        {"path", "uid", "gid", "mode_octal"},
        f"ownership_{label}_field_set",
    )
    require_exact(identity.get("path"), expected_path, f"ownership_{label}_path")
    require_exact(identity.get("uid"), ORACLE_UID, f"ownership_{label}_uid")
    require_exact(identity.get("gid"), ORACLE_GID, f"ownership_{label}_gid")
    require_exact(identity.get("mode_octal"), expected_mode, f"ownership_{label}_mode")


def verify_ownership(path: Path) -> tuple[dict[str, Any], str]:
    ownership = load_json(path)
    require_exact(ownership.get("schema"), OWNERSHIP_SCHEMA, "ownership_schema")
    ownership_root = verify_embedded_root(
        ownership, "ownership_root_sha256", "ownership"
    )
    require((path.stat().st_mode & 0o7777) == 0o400, "ownership_file_mode")
    transaction_id = ownership.get("transaction_id")
    require(isinstance(transaction_id, str) and transaction_id, "ownership_transaction_id")
    require_exact(
        ownership.get("build_user"),
        {"name": "e", "uid": ORACLE_UID, "gid": ORACLE_GID},
        "ownership_build_user",
    )
    build_contract = ownership.get("oracle_build_contract")
    require(isinstance(build_contract, dict), "ownership_build_contract_not_object")
    expected_manifests = {
        label: digest(oracle_manifest(expected_oracle_paths(transaction_id, label)["source"]))
        for label in ("baseline", "candidate")
    }
    require_exact(
        build_contract,
        {
            "schema": ORACLE_BUILD_CONTRACT_SCHEMA,
            "package": {
                "name": ORACLE_PACKAGE_NAME,
                "version": ORACLE_PACKAGE_VERSION,
                "edition": ORACLE_EDITION,
            },
            "oracle_source_sha256": ORACLE_SOURCE_SHA256,
            "cargo_lock_sha256": ORACLE_LOCK_SHA256,
            "offline": True,
            "locked": True,
            "cargo_net_offline": "true",
            "manifest_sha256": expected_manifests,
        },
        "ownership_build_contract",
    )
    rows = ownership.get("rows")
    require(isinstance(rows, dict), "ownership_rows_not_object")
    require_exact(set(rows), {"baseline", "candidate"}, "ownership_row_labels")
    require_exact(
        ownership.get("rows_root_sha256"),
        digest(canonical_bytes(rows)),
        "ownership_rows_root",
    )
    expected_probe = {
        "writer_uid": ORACLE_UID,
        "writer_gid": ORACLE_GID,
        "probe_uid": ORACLE_UID,
        "probe_gid": ORACLE_GID,
        "probe_mode_octal": "0600",
        "create_fsync_unlink_pass": True,
        "directory_fsync_pass": True,
    }
    expected_fields = {
        "schema", "label", "workspace", "src", "cargo_toml", "main_rs",
        "main_rs_sha256", "cargo_lock", "manifest_sha256", "build_command", "build_environment",
        "target_directory", "executable_path", "probe", "probe_retained",
        "ownership_row_root_sha256",
    }
    for row_label in ("baseline", "candidate"):
        row = rows[row_label]
        require(isinstance(row, dict), f"ownership_row_{row_label}_not_object")
        require_exact(set(row), expected_fields, f"ownership_row_{row_label}_field_set")
        require_exact(row.get("schema"), OWNERSHIP_ROW_SCHEMA,
                      f"ownership_row_{row_label}_schema")
        verify_embedded_root(
            row, "ownership_row_root_sha256", f"ownership_row_{row_label}"
        )
        require_exact(row.get("label"), row_label, f"ownership_row_{row_label}_label")
        paths = expected_oracle_paths(transaction_id, row_label)
        workspace = paths["workspace"]
        verify_oracle_file_identity(
            row.get("workspace"), workspace, "0750", f"{row_label}_workspace"
        )
        verify_oracle_file_identity(
            row.get("src"), f"{workspace}/src", "0750", f"{row_label}_src"
        )
        verify_oracle_file_identity(
            row.get("cargo_toml"), f"{workspace}/Cargo.toml", "0640",
            f"{row_label}_cargo_toml",
        )
        verify_oracle_file_identity(
            row.get("main_rs"), f"{workspace}/src/main.rs", "0640",
            f"{row_label}_main_rs",
        )
        require_exact(row.get("main_rs_sha256"), ORACLE_SOURCE_SHA256,
                      f"ownership_{row_label}_main_rs_sha256")
        lock_identity = row.get("cargo_lock")
        require(isinstance(lock_identity, dict), f"ownership_{row_label}_cargo_lock_not_object")
        require_exact(
            set(lock_identity),
            {"path", "uid", "gid", "mode_octal", "sha256_before_build", "sha256_after_build"},
            f"ownership_{row_label}_cargo_lock_field_set",
        )
        require_exact(lock_identity.get("path"), paths["lock"],
                      f"ownership_{row_label}_cargo_lock_path")
        require_exact(lock_identity.get("uid"), ORACLE_UID,
                      f"ownership_{row_label}_cargo_lock_uid")
        require_exact(lock_identity.get("gid"), ORACLE_GID,
                      f"ownership_{row_label}_cargo_lock_gid")
        require_exact(lock_identity.get("mode_octal"), "0640",
                      f"ownership_{row_label}_cargo_lock_mode")
        require_exact(lock_identity.get("sha256_before_build"), ORACLE_LOCK_SHA256,
                      f"ownership_{row_label}_cargo_lock_before")
        require_exact(lock_identity.get("sha256_after_build"), ORACLE_LOCK_SHA256,
                      f"ownership_{row_label}_cargo_lock_after")
        require_exact(row.get("manifest_sha256"), expected_manifests[row_label],
                      f"ownership_{row_label}_manifest_sha256")
        expected_command = [*ORACLE_CARGO_COMMAND_PREFIX, paths["manifest"]]
        require_exact(row.get("build_command"), expected_command,
                      f"ownership_{row_label}_build_command")
        require_exact(
            row.get("build_environment"),
            {**ORACLE_CARGO_ENVIRONMENT, "CARGO_TARGET_DIR": paths["target"]},
            f"ownership_{row_label}_build_environment",
        )
        require_exact(row.get("target_directory"), paths["target"],
                      f"ownership_{row_label}_target_directory")
        require_exact(row.get("executable_path"), paths["executable"],
                      f"ownership_{row_label}_executable_path")
        require_exact(row.get("probe"), expected_probe, f"ownership_row_{row_label}_probe")
        require_exact(row.get("probe_retained"), False,
                      f"ownership_row_{row_label}_probe_retained")
    return ownership, ownership_root


def verify_quiescence(
    path: Path,
    ownership: dict[str, Any],
    ownership_root: str,
) -> tuple[dict[str, Any], dict[str, Any], str]:
    quiescence = load_json(path)
    require_exact(quiescence.get("schema"), QUIESCENCE_SCHEMA, "quiescence_schema")
    verify_embedded_root(quiescence, "quiescence_root_sha256", "quiescence")
    require_exact(quiescence.get("candidate_commit"), CANDIDATE_COMMIT, "quiescence_candidate")
    require_exact(quiescence.get("candidate_tree"), CANDIDATE_TREE, "quiescence_tree")
    require_exact(
        quiescence.get("oracle_ownership_root_sha256"),
        ownership_root,
        "quiescence_ownership_root",
    )
    require_exact(quiescence.get("detector_schema"), PROCESS_DETECTOR_SCHEMA,
                  "quiescence_detector")
    require_exact(
        quiescence.get("snapshot_schema"),
        "adjacent-cpu-and-process-snapshots-v1",
        "quiescence_snapshot_schema",
    )
    require_exact(quiescence.get("forbidden_build_names"), list(FORBIDDEN_BUILD_NAMES),
                  "quiescence_forbidden_names")
    require_exact(quiescence.get("maximum_wait_seconds"), 1800, "quiescence_max_wait")
    require_exact(quiescence.get("required_intervals"), 30, "quiescence_intervals")
    require_exact(
        quiescence.get("thresholds"),
        {
            "interval_min_seconds": 0.90,
            "interval_max_seconds": 1.50,
            "cpu_per_interval_max_percent": 20.0,
            "cpu_window_mean_max_percent": 5.0,
            "io_some_avg10_max": 0.20,
            "io_full_avg10_max": 0.05,
        },
        "quiescence_thresholds",
    )
    require_exact(
        quiescence.get("measurement_cpu_pool"), MEASUREMENT_CPU_POOL,
        "quiescence_cpu_pool",
    )
    topology = quiescence.get("topology")
    expected_topology = [
        {
            "representative_cpu": representative,
            "siblings": PHYSICAL_CORE_SIBLINGS[representative],
            "core_id": EXPECTED_CORE_IDS[representative],
            "max_frequency_khz": EXPECTED_MAX_FREQUENCY_KHZ,
            "logical_cpus": [
                {
                    "cpu": cpu,
                    "online": True,
                    "core_id": EXPECTED_CORE_IDS[representative],
                    "max_frequency_khz": EXPECTED_MAX_FREQUENCY_KHZ,
                }
                for cpu in PHYSICAL_CORE_SIBLINGS[representative]
            ],
        }
        for representative in MEASUREMENT_CPU_POOL
    ]
    require_exact(topology, expected_topology, "quiescence_topology")
    require_exact(
        quiescence.get("topology_root_sha256"), digest(canonical_bytes(topology)),
        "quiescence_topology_root",
    )
    attempted = quiescence.get("attempted_samples")
    require(isinstance(attempted, list) and attempted, "quiescence_attempted_samples")
    require_exact(
        quiescence.get("attempted_interval_count"), len(attempted),
        "quiescence_attempted_count",
    )
    require_exact(
        quiescence.get("attempted_samples_root_sha256"), digest(canonical_bytes(attempted)),
        "quiescence_attempted_root",
    )
    process_snapshots = quiescence.get("process_snapshots")
    require(isinstance(process_snapshots, list), "quiescence_process_snapshots")
    require_exact(len(process_snapshots), len(attempted) + 1,
                  "quiescence_process_snapshot_count_derived")
    require_exact(quiescence.get("process_snapshot_count"), len(process_snapshots),
                  "quiescence_process_snapshot_count")
    require_exact(
        quiescence.get("process_snapshots_root_sha256"),
        digest(canonical_bytes(process_snapshots)),
        "quiescence_process_snapshots_root",
    )
    verified_process_snapshots = [
        verify_process_snapshot(snapshot, f"quiescence_process_snapshot_{index}")
        for index, snapshot in enumerate(process_snapshots)
    ]
    eligibility_start = require_nonnegative_int(
        quiescence.get("eligibility_started_monotonic_ns"),
        "quiescence_eligibility_start",
    )
    eligibility_finish = require_nonnegative_int(
        quiescence.get("eligibility_finished_monotonic_ns"),
        "quiescence_eligibility_finish",
    )
    require(eligibility_finish >= eligibility_start, "quiescence_eligibility_order")
    windows: dict[int, list[dict[str, Any]]] = {
        representative: [] for representative in MEASUREMENT_CPU_POOL
    }
    current_streaks = {representative: 0 for representative in MEASUREMENT_CPU_POOL}
    longest_streaks = {representative: 0 for representative in MEASUREMENT_CPU_POOL}
    minimum_means: dict[int, dict[int, float | None]] = {
        representative: {cpu: None for cpu in PHYSICAL_CORE_SIBLINGS[representative]}
        for representative in MEASUREMENT_CPU_POOL
    }
    first_pass: tuple[int, int, list[dict[str, Any]], dict[str, float]] | None = None
    census_keys = [
        "forbidden_build_process",
        "unresolved_process_observation",
        "interval_duration",
        *(f"cpu_busy_per_interval:{cpu}" for cpu in LOGICAL_CPU_POOL),
        *(f"cpu_busy_window_mean:{cpu}" for cpu in LOGICAL_CPU_POOL),
        "io_some",
        "io_full",
    ]
    census = {key: 0 for key in census_keys}
    previous_end = None
    previous_end_counters = None
    previous_end_root = None
    for index, sample in enumerate(attempted):
        require(isinstance(sample, dict), f"quiescence_sample_{index}_not_object")
        interval = require_number(sample.get("interval_seconds"), f"quiescence_sample_{index}_interval")
        start = require_nonnegative_int(sample.get("start_monotonic_ns"), f"quiescence_sample_{index}_start")
        end = require_nonnegative_int(sample.get("end_monotonic_ns"), f"quiescence_sample_{index}_end")
        require(end > start, f"quiescence_sample_{index}_order")
        require(
            abs(interval - (end - start) / 1_000_000_000) <= 1e-12,
            f"quiescence_sample_{index}_interval_mismatch",
        )
        if previous_end is not None:
            require_exact(start, previous_end, f"quiescence_sample_{index}_continuity")
            require_exact(
                sample.get("start_cpu_counters"), previous_end_counters,
                f"quiescence_sample_{index}_counter_continuity",
            )
            require_exact(
                sample.get("start_cpu_snapshot_root_sha256"), previous_end_root,
                f"quiescence_sample_{index}_snapshot_continuity",
            )
        previous_end = end
        start_counters = sample.get("start_cpu_counters")
        end_counters = sample.get("end_cpu_counters")
        for boundary, counters in (("start", start_counters), ("end", end_counters)):
            require(isinstance(counters, dict), f"quiescence_sample_{index}_{boundary}_counters")
            require_exact(set(counters), {str(cpu) for cpu in LOGICAL_CPU_POOL},
                          f"quiescence_sample_{index}_{boundary}_counter_set")
            for cpu in LOGICAL_CPU_POOL:
                row = counters[str(cpu)]
                require(isinstance(row, dict), f"quiescence_sample_{index}_{boundary}_{cpu}")
                require_exact(set(row), {"total", "idle"},
                              f"quiescence_sample_{index}_{boundary}_{cpu}_fields")
                total = require_nonnegative_int(row.get("total"), f"counter_{index}_{boundary}_{cpu}_total")
                idle = require_nonnegative_int(row.get("idle"), f"counter_{index}_{boundary}_{cpu}_idle")
                require(idle <= total, f"counter_{index}_{boundary}_{cpu}_idle_budget")
        start_root = digest(canonical_bytes(start_counters))
        end_root = digest(canonical_bytes(end_counters))
        require_exact(sample.get("start_cpu_snapshot_root_sha256"), start_root,
                      f"quiescence_sample_{index}_start_snapshot_root")
        require_exact(sample.get("end_cpu_snapshot_root_sha256"), end_root,
                      f"quiescence_sample_{index}_end_snapshot_root")
        previous_end_counters = end_counters
        previous_end_root = end_root
        busy = sample.get("cpu_busy_percent")
        require(isinstance(busy, dict), f"quiescence_sample_{index}_busy")
        require_exact(set(busy), {str(cpu) for cpu in LOGICAL_CPU_POOL},
                      f"quiescence_sample_{index}_busy_set")
        calculated_busy: dict[str, float] = {}
        for cpu in LOGICAL_CPU_POOL:
            key = str(cpu)
            total_delta = end_counters[key]["total"] - start_counters[key]["total"]
            idle_delta = end_counters[key]["idle"] - start_counters[key]["idle"]
            require(total_delta > 0 and 0 <= idle_delta <= total_delta,
                    f"quiescence_sample_{index}_counter_delta_{cpu}")
            calculated = 100.0 * (total_delta - idle_delta) / total_delta
            recorded = require_number(busy.get(key), f"quiescence_sample_{index}_busy_{cpu}")
            require(abs(recorded - calculated) <= 1e-9,
                    f"quiescence_sample_{index}_busy_{cpu}_mismatch")
            calculated_busy[key] = calculated
        process_start = verified_process_snapshots[index]
        process_end = verified_process_snapshots[index + 1]
        require_exact(
            sample.get("start_process_snapshot_root_sha256"),
            process_start["process_snapshot_root_sha256"],
            f"quiescence_sample_{index}_process_start_root",
        )
        require_exact(
            sample.get("end_process_snapshot_root_sha256"),
            process_end["process_snapshot_root_sha256"],
            f"quiescence_sample_{index}_process_end_root",
        )
        build_start = process_start["forbidden_process_matches"]
        build_end = process_end["forbidden_process_matches"]
        require_exact(sample.get("build_processes_start"), build_start,
                      f"quiescence_sample_{index}_build_start")
        require_exact(sample.get("build_processes_end"), build_end,
                      f"quiescence_sample_{index}_build_end")
        unresolved = bool(
            process_start["unresolved_rows"] or process_end["unresolved_rows"]
        )
        some = require_number(sample.get("io_some_avg10"), f"quiescence_sample_{index}_some")
        full = require_number(sample.get("io_full_avg10"), f"quiescence_sample_{index}_full")
        expected_blockers = []
        if build_start or build_end:
            expected_blockers.append("forbidden_build_process")
        if unresolved:
            expected_blockers.append("unresolved_process_observation")
        if not 0.90 <= interval <= 1.50:
            expected_blockers.append("interval_duration")
        for cpu in LOGICAL_CPU_POOL:
            if calculated_busy[str(cpu)] > 20.0:
                expected_blockers.append(f"cpu_busy_per_interval:{cpu}")
        if some > 0.20:
            expected_blockers.append("io_some")
        if full > 0.05:
            expected_blockers.append("io_full")
        require_exact(sample.get("blockers"), expected_blockers,
                      f"quiescence_sample_{index}_blockers")
        for blocker in expected_blockers:
            census[blocker] += 1
        global_blockers = {
            "forbidden_build_process", "unresolved_process_observation",
            "interval_duration", "io_some", "io_full",
        }
        expected_eligible_cores = [
            representative
            for representative in MEASUREMENT_CPU_POOL
            if not global_blockers.intersection(expected_blockers)
            and all(
                f"cpu_busy_per_interval:{cpu}" not in expected_blockers
                for cpu in PHYSICAL_CORE_SIBLINGS[representative]
            )
        ]
        require_exact(sample.get("eligible_physical_cores"), expected_eligible_cores,
                      f"quiescence_sample_{index}_eligible_cores")
        passing_cores = []
        expected_mean_blockers = []
        for representative in MEASUREMENT_CPU_POOL:
            if representative not in expected_eligible_cores:
                windows[representative].clear()
                current_streaks[representative] = 0
                continue
            current_streaks[representative] += 1
            longest_streaks[representative] = max(
                longest_streaks[representative], current_streaks[representative]
            )
            windows[representative].append(sample)
            if len(windows[representative]) > 30:
                windows[representative].pop(0)
            if len(windows[representative]) < 30:
                continue
            means = {
                cpu: sum(row["cpu_busy_percent"][str(cpu)] for row in windows[representative]) / 30
                for cpu in PHYSICAL_CORE_SIBLINGS[representative]
            }
            for cpu, mean in means.items():
                prior = minimum_means[representative][cpu]
                minimum_means[representative][cpu] = mean if prior is None else min(prior, mean)
                if mean > 5.0:
                    expected_mean_blockers.append(cpu)
                    census[f"cpu_busy_window_mean:{cpu}"] += 1
            if all(mean <= 5.0 for mean in means.values()):
                passing_cores.append(representative)
        expected_mean_blockers = sorted(set(expected_mean_blockers))
        require_exact(sample.get("window_mean_blockers"), expected_mean_blockers,
                      f"quiescence_sample_{index}_mean_blockers")
        if passing_cores and first_pass is None:
            selected = min(passing_cores)
            selected_window = list(windows[selected])
            selected_means = {
                str(cpu): sum(row["cpu_busy_percent"][str(cpu)] for row in selected_window) / 30
                for cpu in PHYSICAL_CORE_SIBLINGS[selected]
            }
            first_pass = (index, selected, selected_window, selected_means)
    require_exact(
        quiescence.get("longest_eligible_streaks"),
        {str(cpu): longest_streaks[cpu] for cpu in MEASUREMENT_CPU_POOL},
        "quiescence_longest_streaks",
    )
    recorded_minimum = quiescence.get("minimum_completed_window_mean_percent")
    require(isinstance(recorded_minimum, dict), "quiescence_minimum_means")
    for representative in MEASUREMENT_CPU_POOL:
        recorded_core = recorded_minimum.get(str(representative))
        require(isinstance(recorded_core, dict), f"quiescence_minimum_means_{representative}")
        require_exact(set(recorded_core), {str(cpu) for cpu in PHYSICAL_CORE_SIBLINGS[representative]},
                      f"quiescence_minimum_means_{representative}_set")
        for cpu in PHYSICAL_CORE_SIBLINGS[representative]:
            expected = minimum_means[representative][cpu]
            actual = recorded_core[str(cpu)]
            if expected is None:
                require_exact(actual, None, f"quiescence_minimum_mean_{cpu}")
            else:
                require(abs(require_number(actual, f"quiescence_minimum_mean_{cpu}") - expected) <= 1e-9,
                        f"quiescence_minimum_mean_{cpu}_mismatch")
    require_exact(quiescence.get("blocker_census"), census, "quiescence_blocker_census")
    require(eligibility_start <= attempted[0]["start_monotonic_ns"],
            "quiescence_start_boundary")
    require(eligibility_finish >= attempted[-1]["end_monotonic_ns"],
            "quiescence_finish_boundary")
    verdict = quiescence.get("verdict")
    require(verdict in {"PASS", "TIMEOUT"}, "quiescence_verdict_invalid")
    if verdict == "PASS":
        require(first_pass is not None, "quiescence_pass_without_window")
        index, selected, expected_window, expected_means = first_pass
        require_exact(index, len(attempted) - 1, "quiescence_not_first_pass")
        require_exact(quiescence.get("selected_cpu"), selected, "quiescence_selected_cpu")
        require_exact(quiescence.get("eligible_window"), expected_window, "quiescence_window")
        require_exact(quiescence.get("eligible_window_cpu_mean_percent"), expected_means,
                      "quiescence_window_means")
        require_exact(quiescence.get("eligible_window_root_sha256"),
                      digest(canonical_bytes(expected_window)), "quiescence_window_root")
        require(isinstance(quiescence.get("eligibility_reached_at"), str),
                "quiescence_reached_at")
    else:
        require(first_pass is None, "quiescence_timeout_with_passing_window")
        require_exact(quiescence.get("selected_cpu"), None, "quiescence_timeout_selected_cpu")
        require_exact(quiescence.get("eligible_window"), None, "quiescence_timeout_window")
        require_exact(quiescence.get("eligible_window_cpu_mean_percent"), None,
                      "quiescence_timeout_window_means")
        require_exact(quiescence.get("eligible_window_root_sha256"), None,
                      "quiescence_timeout_window_root")
        require_exact(quiescence.get("eligibility_reached_at"), None,
                      "quiescence_timeout_reached_at")
        elapsed = (eligibility_finish - eligibility_start) / 1_000_000_000
        require(elapsed >= 1800.0, "quiescence_timeout_deadline_not_reached")
        observed_coverage = (
            attempted[-1]["end_monotonic_ns"] - attempted[0]["start_monotonic_ns"]
        ) / 1_000_000_000
        require(observed_coverage >= 1800.0, "quiescence_timeout_coverage_incomplete")
    executables, executable_root = verify_executables(
        quiescence.get("executables"), ownership["transaction_id"]
    )
    require_exact(quiescence.get("executable_set_root_sha256"), executable_root,
                  "quiescence_executable_root")
    require((path.stat().st_mode & 0o7777) == 0o400, "quiescence_file_mode")
    return quiescence, executables, executable_root


def verify_contamination(
    contamination: dict[str, Any],
    quiescence_root: str,
    executable_root: str,
    selected_cpu: int,
) -> None:
    require_exact(contamination.get("schema"), CONTAMINATION_SCHEMA, "contamination_schema")
    verify_embedded_root(
        contamination, "measurement_contamination_root_sha256", "contamination"
    )
    require_exact(contamination.get("quiescence_root_sha256"), quiescence_root,
                  "contamination_quiescence_root")
    require_exact(contamination.get("executable_set_root_sha256"), executable_root,
                  "contamination_executable_root")
    require_exact(contamination.get("measurement_cpu"), selected_cpu,
                  "contamination_measurement_cpu")
    require_exact(contamination.get("monitor_interval_seconds"), 0.5,
                  "contamination_monitor_interval")
    require_exact(contamination.get("maximum_sample_gap_seconds"), 2.0,
                  "contamination_maximum_gap")
    require(require_number(contamination.get("observed_max_sample_gap_seconds"),
                           "contamination_observed_gap") <= 2.0,
            "contamination_observed_gap_budget")
    require_exact(contamination.get("metric_labels"), MEASUREMENT_LABELS,
                  "contamination_metric_labels")
    expected_boundaries = [
        {"label": label, "phase": phase}
        for label in MEASUREMENT_LABELS
        for phase in ("before", "after")
    ]
    boundaries = contamination.get("boundaries")
    require(isinstance(boundaries, list), "contamination_boundaries_invalid")
    require_exact(
        [{"label": row.get("label"), "phase": row.get("phase")} for row in boundaries],
        expected_boundaries,
        "contamination_boundary_order",
    )
    samples = contamination.get("samples")
    require(isinstance(samples, list) and samples, "contamination_samples_empty")
    previous = None
    observed_gaps = []
    expected_matches = []
    expected_unresolved = []
    for index, sample in enumerate(samples):
        require(isinstance(sample, dict), f"contamination_sample_{index}_not_object")
        snapshot = verify_process_snapshot(
            sample.get("process_snapshot"), f"contamination_process_snapshot_{index}"
        )
        require_exact(
            sample.get("process_snapshot_root_sha256"),
            snapshot["process_snapshot_root_sha256"],
            f"contamination_sample_{index}_process_root",
        )
        require_exact(sample.get("build_processes"), snapshot["forbidden_process_matches"],
                      f"contamination_sample_{index}_build")
        require_exact(sample.get("unresolved_processes"), snapshot["unresolved_rows"],
                      f"contamination_sample_{index}_unresolved")
        expected_matches.extend(
            {"sample_index": index, "process": process}
            for process in snapshot["forbidden_process_matches"]
        )
        expected_unresolved.extend(
            {"sample_index": index, "process": process}
            for process in snapshot["unresolved_rows"]
        )
        current = require_nonnegative_int(sample.get("monotonic_ns"), f"contamination_sample_{index}_time")
        if previous is not None:
            require(current >= previous, f"contamination_sample_{index}_order")
            observed_gaps.append((current - previous) / 1_000_000_000)
        previous = current
    actual_max_gap = max(observed_gaps, default=0.0)
    require(actual_max_gap <= 2.0, "contamination_actual_gap_budget")
    require(abs(require_number(contamination.get("observed_max_sample_gap_seconds"),
                               "contamination_recorded_gap") - actual_max_gap) <= 1e-9,
            "contamination_gap_mismatch")
    require_exact(contamination.get("forbidden_process_matches"), expected_matches,
                  "contamination_matches")
    require_exact(contamination.get("unresolved_process_observations"), expected_unresolved,
                  "contamination_unresolved")
    require_exact(expected_matches, [], "contamination_forbidden_processes")
    require_exact(expected_unresolved, [], "contamination_unresolved_processes")
    require_exact(contamination.get("monitor_errors"), [], "contamination_errors")
    require_exact(contamination.get("contaminated"), False, "contamination_verdict")


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


def verify_resource(
    resource: dict[str, Any],
    quiescence_root: str,
    contamination_root: str,
    executable_root: str,
    ownership_root: str,
    selected_cpu: int,
) -> None:
    require_exact(resource.get("schema"), RESOURCE_SCHEMA, "resource_schema")
    verify_embedded_root(resource, "resource_root_sha256", "resource")
    require_exact(resource.get("all_pass"), True, "resource_all_pass")
    require_exact(resource.get("quiescence_root_sha256"), quiescence_root,
                  "resource_quiescence_root")
    require_exact(resource.get("measurement_contamination_root_sha256"), contamination_root,
                  "resource_contamination_root")
    require_exact(resource.get("executable_set_root_sha256"), executable_root,
                  "resource_executable_root")
    require_exact(resource.get("oracle_ownership_root_sha256"), ownership_root,
                  "resource_ownership_root")
    require_exact(resource.get("measurement_cpu"), selected_cpu, "resource_measurement_cpu")
    require_exact(resource.get("direct_exec_only"), True, "resource_direct_exec")
    require_exact(resource.get("compiler_invocations_after_quiescence"), 0,
                  "resource_compiler_invocations")
    metrics = resource.get("metrics", {})
    for name, limit_p99, limit_max, expected_samples in (
        ("hot_latency", 1_000_000, 2_000_000, 4096),
        ("single_ledger_sync", 5_000_000, 20_000_000, 1024),
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
    three_runs = metrics.get("three_ledger_sync")
    require(isinstance(three_runs, list) and len(three_runs) == 3,
            "three_ledger_sync_run_count")
    expected_stage_fields = {
        "precommit_p99_ns",
        "precommit_hard_max_ns",
        "settlement_p99_ns",
        "settlement_hard_max_ns",
        "episode_p99_ns",
        "episode_hard_max_ns",
        "samples",
    }
    for index, run in enumerate(three_runs):
        require(isinstance(run, dict), f"three_ledger_sync_{index}_not_object")
        require_exact(set(run), expected_stage_fields,
                      f"three_ledger_sync_{index}_field_set")
        for field in ("precommit_p99_ns", "settlement_p99_ns"):
            require(require_nonnegative_int(run.get(field),
                                            f"three_ledger_sync_{index}_{field}") <= 5_000_000,
                    f"three_ledger_sync_{index}_{field}_budget")
        for field in (
            "precommit_hard_max_ns",
            "settlement_hard_max_ns",
            "episode_hard_max_ns",
        ):
            require(require_nonnegative_int(run.get(field),
                                            f"three_ledger_sync_{index}_{field}") <= 20_000_000,
                    f"three_ledger_sync_{index}_{field}_budget")
        require_nonnegative_int(run.get("episode_p99_ns"),
                                f"three_ledger_sync_{index}_episode_p99_ns")
        require_exact(run.get("samples"), 256, f"three_ledger_sync_{index}_samples")
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


def verify_parity(parity: dict[str, Any], executables: dict[str, Any]) -> None:
    require_exact(parity.get("schema"), PARITY_SCHEMA, "parity_schema")
    verify_embedded_root(parity, "parity_root_sha256", "parity")
    require_exact(parity.get("byte_identical"), True, "parity_identical")
    require_exact(parity.get("baseline_output_sha256"), parity.get("candidate_output_sha256"),
                  "parity_output")
    require(require_nonnegative_int(parity.get("rows"), "parity_rows") > 0, "parity_rows_zero")
    require_exact(parity.get("direct_exec_only"), True, "parity_direct_exec")
    require_exact(parity.get("baseline_oracle_sha256"),
                  executables["parity-baseline"]["sha256"], "parity_baseline_oracle")
    require_exact(parity.get("candidate_oracle_sha256"),
                  executables["parity-candidate"]["sha256"], "parity_candidate_oracle")


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


def verify_preparation(directory: Path) -> dict[str, Any]:
    ownership, ownership_root = verify_ownership(
        directory / "oracle-ownership-receipt.json"
    )
    quiescence, executables, executable_root = verify_quiescence(
        directory / "quiescence-receipt.json", ownership, ownership_root
    )
    if quiescence["verdict"] == "TIMEOUT":
        state = load_json(directory / "transaction-state.json")
        require_exact(state.get("schema"), "nando.s1c3-state.v7", "timeout_state_schema")
        require_exact(state.get("state"), "QUIESCENCE_TIMEOUT", "timeout_state")
        require_exact(state.get("transaction_id"), quiescence.get("transaction_id"),
                      "timeout_transaction_id")
        require_exact(state.get("quiescence_root_sha256"),
                      quiescence["quiescence_root_sha256"], "timeout_quiescence_root")
        forbidden = {
            "measurement-contamination-receipt.json",
            "measurement-failure.json",
            "preflight-failure.json",
            "resource-receipt.json",
            "parity-receipt.json",
            "preparation.json",
            "predeployment-verification.json",
            "candidate-binary",
            "candidate-config",
            "rollback-manifest.sha256",
            "pending-receipt.json",
            "deployment-receipt.json",
        }
        present = sorted(name for name in forbidden if (directory / name).exists())
        require_exact(present, [], "timeout_post_quiescence_artifacts")
        rollback = directory / "rollback"
        if rollback.exists():
            require_exact(
                sorted(path.name for path in rollback.iterdir()), [],
                "timeout_rollback_artifacts",
            )
        return {
            "schema": "nando.s1c3-verification.v7",
            "valid": True,
            "authority": False,
            "verdict": "INVALID_ENVIRONMENT_QUIESCENCE_TIMEOUT",
            "oracle_ownership_root_sha256": ownership_root,
            "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        }
    require_exact(quiescence.get("verdict"), "PASS", "quiescence_pass_verdict")
    selected_cpu = quiescence.get("selected_cpu")
    require(selected_cpu in MEASUREMENT_CPU_POOL, "selected_cpu_invalid")
    contamination = load_json(directory / "measurement-contamination-receipt.json")
    preparation = load_json(directory / "preparation.json")
    resource = load_json(directory / "resource-receipt.json")
    parity = load_json(directory / "parity-receipt.json")
    verify_contamination(
        contamination, quiescence["quiescence_root_sha256"], executable_root,
        selected_cpu,
    )
    verify_identity(preparation)
    require_exact(ownership.get("transaction_id"), preparation.get("transaction_id"),
                  "ownership_transaction_binding")
    require_exact(quiescence.get("transaction_id"), preparation.get("transaction_id"),
                  "quiescence_transaction_id")
    require_exact(contamination.get("transaction_id"), preparation.get("transaction_id"),
                  "contamination_transaction_id")
    verify_resource(
        resource,
        quiescence["quiescence_root_sha256"],
        contamination["measurement_contamination_root_sha256"],
        executable_root,
        ownership_root,
        selected_cpu,
    )
    verify_parity(parity, executables)
    require_exact(preparation.get("quiescence_root_sha256"),
                  quiescence["quiescence_root_sha256"], "preparation_quiescence_binding")
    require_exact(preparation.get("measurement_cpu"), selected_cpu,
                  "preparation_measurement_cpu")
    require_exact(preparation.get("measurement_contamination_root_sha256"),
                  contamination["measurement_contamination_root_sha256"],
                  "preparation_contamination_binding")
    require_exact(preparation.get("executable_set_root_sha256"), executable_root,
                  "preparation_executable_binding")
    require_exact(preparation.get("oracle_ownership_root_sha256"), ownership_root,
                  "preparation_ownership_binding")
    require_exact(preparation.get("candidate", {}).get("binary_sha256"),
                  executables["candidate-binary"]["sha256"],
                  "preparation_candidate_executable_hash")
    require_exact(preparation.get("candidate", {}).get("binary_size_bytes"),
                  executables["candidate-binary"]["size_bytes"],
                  "preparation_candidate_executable_size")
    require_exact(preparation.get("resource_root_sha256"), resource["resource_root_sha256"],
                  "preparation_resource_binding")
    require_exact(preparation.get("parity_root_sha256"), parity["parity_root_sha256"],
                  "preparation_parity_binding")
    result = {
        "schema": "nando.s1c3-predeployment-verification.v7",
        "valid": True,
        "authority": True,
        "verdict": "S1C3_PREPARATION_PASS",
        "selected_cpu": selected_cpu,
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "oracle_ownership_root_sha256": ownership_root,
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
        "resource_root_sha256": resource["resource_root_sha256"],
        "parity_root_sha256": parity["parity_root_sha256"],
    }
    result["predeployment_verification_root_sha256"] = digest(canonical_bytes(result))
    return result


def verify_predeployment_receipt(path: Path, expected: dict[str, Any]) -> str:
    receipt = load_json(path)
    root = verify_embedded_root(
        receipt,
        "predeployment_verification_root_sha256",
        "predeployment_verification",
    )
    require_exact(receipt, expected, "predeployment_verification_receipt")
    return root


def verify_receipt(directory: Path) -> dict[str, Any]:
    prepared = verify_preparation(directory)
    ownership, ownership_root = verify_ownership(
        directory / "oracle-ownership-receipt.json"
    )
    quiescence, executables, executable_root = verify_quiescence(
        directory / "quiescence-receipt.json", ownership, ownership_root
    )
    if quiescence["verdict"] == "TIMEOUT":
        return prepared
    predeployment_root = verify_predeployment_receipt(
        directory / "predeployment-verification.json", prepared
    )
    selected_cpu = quiescence["selected_cpu"]
    contamination = load_json(directory / "measurement-contamination-receipt.json")
    preparation = load_json(directory / "preparation.json")
    resource = load_json(directory / "resource-receipt.json")
    parity = load_json(directory / "parity-receipt.json")
    receipt = load_json(directory / "deployment-receipt.json")
    require_exact(receipt.get("schema"), SCHEMA, "receipt_schema")
    verify_embedded_root(receipt, "receipt_root_sha256", "receipt")
    require_exact(receipt.get("preparation_root_sha256"), preparation["preparation_root_sha256"],
                  "receipt_preparation_root")
    require_exact(receipt.get("quiescence_root_sha256"),
                  quiescence["quiescence_root_sha256"], "receipt_quiescence_root")
    require_exact(receipt.get("measurement_cpu"), selected_cpu,
                  "receipt_measurement_cpu")
    require_exact(receipt.get("measurement_contamination_root_sha256"),
                  contamination["measurement_contamination_root_sha256"],
                  "receipt_contamination_root")
    require_exact(receipt.get("executable_set_root_sha256"), executable_root,
                  "receipt_executable_root")
    require_exact(receipt.get("oracle_ownership_root_sha256"), ownership_root,
                  "receipt_ownership_root")
    require_exact(receipt.get("transaction_id"), preparation.get("transaction_id"),
                  "receipt_transaction_id")
    require_exact(receipt.get("resource_root_sha256"), resource["resource_root_sha256"],
                  "receipt_resource_root")
    require_exact(receipt.get("parity_root_sha256"), parity["parity_root_sha256"],
                  "receipt_parity_root")
    require_exact(receipt.get("predeployment_verification_root_sha256"),
                  predeployment_root, "receipt_predeployment_verification_root")
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
        "schema": "nando.s1c3-verification.v7",
        "valid": True,
        "authority": True,
        "verdict": verdict,
        "receipt_root_sha256": receipt["receipt_root_sha256"],
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "oracle_ownership_root_sha256": ownership_root,
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("transaction_directory", type=Path)
    parser.add_argument("--allow-rollback", action="store_true")
    parser.add_argument("--pre-deployment", action="store_true")
    args = parser.parse_args()
    try:
        result = (
            verify_preparation(args.transaction_directory)
            if args.pre_deployment
            else verify_receipt(args.transaction_directory)
        )
        if result["verdict"] == "S1C3_ROLLBACK_PASS" and not args.allow_rollback:
            raise InvalidReceipt("rollback_is_not_deployment_pass")
        if args.pre_deployment and result.get("authority") is not True:
            raise InvalidReceipt("predeployment_authority_false")
    except InvalidReceipt as error:
        print(json.dumps({"schema": "nando.s1c3-verification.v7", "valid": False,
                          "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
