#!/usr/bin/env python3
"""Independent record-aware verifier for S1C-3F."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import struct
from pathlib import Path
from typing import Any


PAPER_COMMIT = "e6c733a243fb8c95920d971c17edf1c3cda65def"
PAPER_TREE = "0e914870e08e132c93d9608c1e4873c0636ed5d4"
PAPER_PREREGISTRATION_SHA256 = "3df587badc3f99e714f6bccae03524d812e22fe85ff37d59da5a34b0958d0034"
PAPER_CRITIQUE_SHA256 = "c61f3d22ed74e7926b57c3b0a8cf25e26ce11a2e1ba5dee43cb67f564a3df6e0"
PAPER_MANIFEST_ROOT = "489db0f03647fadd7354eea5d19d54bba3c4ebb03aa5017c272110f88bbca362"
PARENT_STATE_ROOT = "442aefee66e7c04c561143b00d0a3fd6bcb01f65f5149571bd0f3d35f6b2a77c"
PARENT_RECEIPT_ROOT = "baedc988ac8664cde09946d2f8b780ce9221086dd8d6d68bb02280582214ecb0"
PARENT_FINAL_ROOT = "637d90ffece05f628d6ed27041dbe22706a3df4ef04240185096980b484fdcf3"
PARENT_JOURNAL_ROOT = "6ab9cd4823f1d737ec731e9c96049c0492d1966d91b3408dd524ea23b1c8666c"
PARENT_FILES = {
    "s1c3e-state.json": "e9b5588110aa238060d02a985157688a3f472f7ce0d8eca5cd9036abda9e9b2d",
    "deployment-receipt.json": "21fb0fe55b74ff748d6a68d181710f87f643260921cc0e7c98e4e1f0b50a8eef",
    "final-verification.json": "1f38df7dbfd05649e8f99e58d687341daf171f8bcc7c4ab2f1d168b7c2cf24de",
}
S1C3D_RESOURCE_ROOT = "c917e62a85d2776e3a20d3efd72b16230a0689c73975b786d6ab8687c1176038"
S1C3D_PARITY_ROOT = "55ae110ce15f198e0741890e856e5822170e1ba479870ea9c03ac4bd34ad3ea9"
CANDIDATE_BINARY_SHA256 = "360498a0908739cad6f1ac21cf4053b7421daaf8b1d9a6502b72132a94a692df"
CANDIDATE_CONFIG_SHA256 = "1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6"
BASELINE_BINARY_SHA256 = "6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58"
BASELINE_CONFIG_SHA256 = "cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5"
MAGIC = b"NTF1"
MAGIC_SHA256 = "4fc61a14f994e28249509ec2504e89df30497a2aa76b1d9c5f6c38e2acee6072"
MAX_PAYLOAD = 16 * 1024 * 1024
JOURNAL_PATH = "/var/lib/nando-wave/transition/grounded-meaning-v1/decision-contract-precommits-v1"
EXPECTED_SEGMENTS = (
    "decision-precommit-00000000000000000000.cbor",
    "goal-satisfaction-00000000000000000000.cbor",
    "selected-action-binding-00000000000000000000.cbor",
)
ATTEMPT_RE = re.compile(r"^\d{8}T\d{6}Z-[0-9a-f]{12}-s1c3f-v1$")
IMPLEMENTATION_FILES = (
    "s1c3f_remote_transaction_v1.py",
    "verify_s1c3f_transaction_v1.py",
    "test_s1c3f_transaction_v1.py",
    "run_s1c3f_transaction_v1.sh",
    "s1c3e_remote_transaction_v1.py",
    "s1c3b_remote_transaction_v1.py",
    "s1c3_remote_transaction_v7.py",
)


class InvalidReceipt(ValueError):
    pass


def canonical_bytes(value: Any, omit: str | None = None) -> bytes:
    if omit is not None and isinstance(value, dict):
        value = {key: item for key, item in value.items() if key != omit}
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def file_digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise InvalidReceipt(f"object_required:{path.name}")
    return value


def require(condition: bool, label: str) -> None:
    if not condition:
        raise InvalidReceipt(label)


def exact(actual: Any, expected: Any, label: str) -> None:
    require(actual == expected, label)


def verify_root(value: dict[str, Any], field: str, label: str) -> str:
    root = value.get(field)
    require(isinstance(root, str) and len(root) == 64, f"{label}_missing")
    exact(root, digest(canonical_bytes(value, field)), f"{label}_mismatch")
    return root


def parse_segment_bytes(payload: bytes, label: str) -> dict[str, int]:
    require(payload[:4] == MAGIC and len(payload) >= 4, f"{label}_magic")
    offset = 4
    records = 0
    while offset < len(payload):
        require(len(payload) - offset >= 12, f"{label}_partial_header")
        payload_bytes = struct.unpack_from("<I", payload, offset)[0]
        expected_digest = struct.unpack_from("<Q", payload, offset + 4)[0]
        require(payload_bytes <= MAX_PAYLOAD, f"{label}_payload_budget")
        end = offset + 12 + payload_bytes
        require(end <= len(payload), f"{label}_partial_payload")
        frame = payload[offset + 12 : end]
        actual_digest = int.from_bytes(hashlib.sha256(frame).digest()[:8], "little")
        exact(actual_digest, expected_digest, f"{label}_digest")
        records += 1
        offset = end
    return {"format_bytes": 4, "frame_bytes": len(payload) - 4, "record_count": records, "tail_bytes": 0}


def parse_segment_file(path: Path) -> dict[str, int]:
    return parse_segment_bytes(path.read_bytes(), path.name)


def create_freeze(source_commit: str, source_tree: str, directory: Path) -> dict[str, Any]:
    require(re.fullmatch(r"[0-9a-f]{40}", source_commit) is not None, "source_commit")
    require(re.fullmatch(r"[0-9a-f]{40}", source_tree) is not None, "source_tree")
    files = []
    for name in IMPLEMENTATION_FILES:
        path = directory / name
        require(path.is_file(), f"implementation_missing:{name}")
        files.append({"path": name, "sha256": file_digest(path), "size_bytes": path.stat().st_size})
    value = {
        "schema": "nando.s1c3f-implementation-freeze.v1",
        "source_commit": source_commit,
        "source_tree": source_tree,
        "paper_commit": PAPER_COMMIT,
        "files": files,
    }
    value["implementation_freeze_root_sha256"] = digest(canonical_bytes(value))
    return value


def verify_freeze(path: Path, directory: Path) -> dict[str, Any]:
    value = load_json(path)
    exact(value.get("schema"), "nando.s1c3f-implementation-freeze.v1", "freeze_schema")
    verify_root(value, "implementation_freeze_root_sha256", "freeze_root")
    exact(value.get("paper_commit"), PAPER_COMMIT, "freeze_paper")
    expected = []
    for name in IMPLEMENTATION_FILES:
        local = directory / name
        require(local.is_file(), f"freeze_local:{name}")
        expected.append({"path": name, "sha256": file_digest(local), "size_bytes": local.stat().st_size})
    exact(value.get("files"), expected, "freeze_files")
    return value


def verify_parent(directory: Path) -> None:
    root = directory / "s1c3f-parent-evidence"
    for name, expected in PARENT_FILES.items():
        exact(file_digest(root / name), expected, f"parent_file_{name}")
    state = load_json(root / "s1c3e-state.json")
    receipt = load_json(root / "deployment-receipt.json")
    final = load_json(root / "final-verification.json")
    exact(verify_root(state, "state_root_sha256", "parent_state"), PARENT_STATE_ROOT, "parent_state_exact")
    exact(verify_root(receipt, "receipt_root_sha256", "parent_receipt"), PARENT_RECEIPT_ROOT, "parent_receipt_exact")
    exact(verify_root(final, "final_verification_root_sha256", "parent_final"), PARENT_FINAL_ROOT, "parent_final_exact")
    exact(state.get("verdict"), "S1C3E_ROLLBACK_PASS", "parent_verdict")
    exact(receipt.get("journal_after", {}).get("manifest_root_sha256"), PARENT_JOURNAL_ROOT, "parent_journal")


def verify_opening_journal(value: Any, label: str) -> None:
    require(isinstance(value, dict), f"{label}_object")
    exact(value.get("present"), True, f"{label}_present")
    directory = value.get("directory")
    require(isinstance(directory, dict), f"{label}_directory")
    exact(directory.get("path"), JOURNAL_PATH, f"{label}_path")
    exact(directory.get("mode_octal"), "0700", f"{label}_directory_mode")
    entries = value.get("entries")
    require(isinstance(entries, list), f"{label}_entries")
    exact([row.get("path") for row in entries], list(EXPECTED_SEGMENTS), f"{label}_segments")
    for row in entries:
        exact(row.get("size_bytes"), 4, f"{label}_{row['path']}_size")
        exact(row.get("sha256"), MAGIC_SHA256, f"{label}_{row['path']}_sha")
        exact(row.get("uid"), directory.get("uid"), f"{label}_{row['path']}_uid")
        exact(row.get("gid"), directory.get("gid"), f"{label}_{row['path']}_gid")
        exact(row.get("mode_octal"), "0600", f"{label}_{row['path']}_mode")
        exact(row.get("format_bytes"), 4, f"{label}_{row['path']}_format")
        exact(row.get("frame_bytes"), 0, f"{label}_{row['path']}_frame")
        exact(row.get("record_count"), 0, f"{label}_{row['path']}_records")
        exact(row.get("tail_bytes"), 0, f"{label}_{row['path']}_tail")
    exact(value.get("record_count"), 0, f"{label}_record_count")
    exact(value.get("manifest_root_sha256"), PARENT_JOURNAL_ROOT, f"{label}_root")


def verify_runtime_journal(value: Any, opening: dict[str, Any], label: str) -> None:
    require(isinstance(value, dict), f"{label}_object")
    exact(value.get("directory"), opening.get("directory"), f"{label}_directory")
    entries = value.get("entries")
    require(isinstance(entries, list), f"{label}_entries")
    exact([row.get("path") for row in entries], list(EXPECTED_SEGMENTS), f"{label}_segments")
    opening_rows = {row["path"]: row for row in opening["entries"]}
    for row in entries:
        old = opening_rows[row["path"]]
        exact(row.get("uid"), old.get("uid"), f"{label}_{row['path']}_uid")
        exact(row.get("gid"), old.get("gid"), f"{label}_{row['path']}_gid")
        exact(row.get("mode_octal"), "0600", f"{label}_{row['path']}_mode")
        exact(row.get("format_bytes"), 4, f"{label}_{row['path']}_format")
        exact(row.get("tail_bytes"), 0, f"{label}_{row['path']}_tail")
        require(row.get("size_bytes", 0) >= old["size_bytes"], f"{label}_{row['path']}_size")
        exact(row.get("prefix_size_bytes"), old.get("size_bytes"), f"{label}_{row['path']}_prefix_size")
        exact(row.get("prefix_sha256"), old.get("sha256"), f"{label}_{row['path']}_prefix_sha")
        exact(row.get("prefix_preserved"), True, f"{label}_{row['path']}_prefix_preserved")


def verify_preparation(directory: Path, freeze: dict[str, Any]) -> dict[str, Any]:
    value = load_json(directory / "preparation.json")
    verify_root(value, "preparation_root_sha256", "preparation")
    exact(value.get("schema"), "nando.s1c3f-preparation.v1", "preparation_schema")
    transaction = value.get("transaction_id")
    require(isinstance(transaction, str) and ATTEMPT_RE.fullmatch(transaction) is not None, "transaction_id")
    exact(value.get("paper"), {
        "commit": PAPER_COMMIT,
        "tree": PAPER_TREE,
        "preregistration_sha256": PAPER_PREREGISTRATION_SHA256,
        "critique_sha256": PAPER_CRITIQUE_SHA256,
        "manifest_root_sha256": PAPER_MANIFEST_ROOT,
    }, "paper")
    exact(value.get("implementation_freeze_root_sha256"), freeze["implementation_freeze_root_sha256"], "freeze_binding")
    exact(value.get("parent"), {
        "transaction_id": "20260812T153838Z-25c0f1168fa4-s1c3e-v1",
        "state_root_sha256": PARENT_STATE_ROOT,
        "receipt_root_sha256": PARENT_RECEIPT_ROOT,
        "final_verification_root_sha256": PARENT_FINAL_ROOT,
        "journal_root_sha256": PARENT_JOURNAL_ROOT,
        "s1c3d_resource_root_sha256": S1C3D_RESOURCE_ROOT,
        "s1c3d_parity_root_sha256": S1C3D_PARITY_ROOT,
        "optimization_status": "OPTIMIZATION_WATCH",
    }, "parent")
    exact(file_digest(directory / "candidate-binary"), CANDIDATE_BINARY_SHA256, "candidate_binary")
    exact(file_digest(directory / "candidate-config"), CANDIDATE_CONFIG_SHA256, "candidate_config")
    exact(value.get("economics_before"), {"false_accepts": 0, "runtime_parity_mismatches": 0}, "economics_before")
    verify_opening_journal(value.get("journal_before"), "journal_before")
    verify_parent(directory)
    return value


def verify_connector(before: Any, after: Any) -> None:
    require(isinstance(before, dict) and isinstance(after, dict), "connector")
    for field in ("main_pid", "nrestarts", "route_receipt_failures", "command_sha256", "active_state"):
        exact(after.get(field), before.get(field), f"connector_{field}")
    exact(after.get("route_receipt_failures"), 0, "connector_failures")


def verify_services(receipt: dict[str, Any], deployed: bool) -> None:
    before = receipt.get("services_before")
    after = receipt.get("services_after")
    survival = receipt.get("services_survival")
    require(isinstance(before, dict) and isinstance(after, dict) and isinstance(survival, dict), "services")
    transition = "nando-transition-serving.service"
    for unit in before:
        for row in (after[unit], survival[unit]):
            exact(row.get("active_state"), "active", f"{unit}_active")
            exact(row.get("nrestarts"), before[unit].get("nrestarts"), f"{unit}_restarts")
            exact(row.get("fragment_sha256"), before[unit].get("fragment_sha256"), f"{unit}_fragment")
        if unit == transition:
            exact(after[unit].get("main_pid"), survival[unit].get("main_pid"), "transition_survival")
            if deployed:
                require(after[unit].get("main_pid") != before[unit].get("main_pid"), "transition_not_replaced")
        else:
            exact(after[unit], before[unit], f"{unit}_after")
            exact(survival[unit], before[unit], f"{unit}_survival")


def verify_final(directory: Path, preparation: dict[str, Any]) -> dict[str, Any]:
    receipt = load_json(directory / "deployment-receipt.json")
    verify_root(receipt, "receipt_root_sha256", "receipt")
    exact(receipt.get("schema"), "nando.s1c3f-deployment-receipt.v1", "receipt_schema")
    verdict = receipt.get("verdict")
    require(verdict in {"S1C3F_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH", "S1C3F_ROLLBACK_PASS", "S1C3F_VETO"}, "verdict")
    exact(receipt.get("preparation_root_sha256"), preparation["preparation_root_sha256"], "receipt_preparation")
    exact(receipt.get("parent"), preparation["parent"], "receipt_parent")
    exact(receipt.get("candidate"), preparation["candidate"], "receipt_candidate")
    exact(receipt.get("scientific_authority"), False, "science")
    exact(receipt.get("model_training"), False, "training")
    exact(receipt.get("phase_mutation"), False, "phase")
    exact(receipt.get("optimization_status"), "OPTIMIZATION_WATCH", "optimization")
    exact(receipt.get("false_accepts_after"), 0, "false_accepts")
    exact(receipt.get("runtime_parity_failures_after"), 0, "parity")
    exact(receipt.get("health_semantics_preserved"), True, "health")
    verify_connector(receipt.get("connector_before"), receipt.get("connector_after"))
    deployed = verdict == "S1C3F_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH"
    verify_services(receipt, deployed)
    if deployed:
        exact(receipt.get("capture_available"), True, "capture")
        exact(receipt.get("startup_log_clean"), True, "startup")
        exact(receipt.get("active_packages_preserved"), True, "packages")
        exact(receipt.get("installed_binary_sha256"), CANDIDATE_BINARY_SHA256, "installed_binary")
        exact(receipt.get("installed_config_sha256"), CANDIDATE_CONFIG_SHA256, "installed_config")
        verify_opening_journal(receipt.get("journal_after"), "journal_opening")
        verify_runtime_journal(receipt.get("journal_survival"), receipt["journal_after"], "journal_survival")
    else:
        exact(receipt.get("capture_available"), False, "rollback_capture")
        exact(receipt.get("installed_binary_sha256"), BASELINE_BINARY_SHA256, "rollback_binary")
        exact(receipt.get("installed_config_sha256"), BASELINE_CONFIG_SHA256, "rollback_config")
        forward = receipt.get("journal_forward")
        verify_runtime_journal(forward, preparation["journal_before"], "rollback_forward")
        verify_runtime_journal(receipt.get("journal_after"), forward, "rollback_journal")
    cursor = None
    if deployed:
        opening = receipt["journal_after"]
        cursor = {
            "schema": "nando.s1c4-append-cursor.v1",
            "transaction_id": receipt["transaction_id"],
            "deployment_receipt_root_sha256": receipt["receipt_root_sha256"],
            "journal_manifest_root_sha256": opening["manifest_root_sha256"],
            "journal_entries": opening["entries"],
            "record_count_at_open": 0,
            "retroactive_rows_allowed": False,
            "scientific_authority": False,
            "state": "COLLECTING",
        }
        cursor["cursor_root_sha256"] = digest(canonical_bytes(cursor))
    result = {
        "schema": "nando.s1c3f-final-verification.v1",
        "valid": True,
        "authority": True,
        "verdict": verdict,
        "receipt_root_sha256": receipt["receipt_root_sha256"],
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "capture_installed": deployed,
        "s1c4_state": "COLLECTING" if deployed else "CLOSED",
        "s1c4_cursor": cursor,
        "scientific_authority": False,
        "model_training": False,
        "phase_mutation": False,
    }
    result["final_verification_root_sha256"] = digest(canonical_bytes(result))
    return result


def verify(directory: Path, freeze_path: Path, predeployment: bool) -> dict[str, Any]:
    freeze = verify_freeze(freeze_path, Path(__file__).resolve().parent)
    preparation = verify_preparation(directory, freeze)
    if predeployment:
        result = {
            "schema": "nando.s1c3f-predeployment-verification.v1",
            "valid": True,
            "authority": True,
            "verdict": "S1C3F_PREPARATION_PASS_WITH_OPTIMIZATION_WATCH",
            "preparation_root_sha256": preparation["preparation_root_sha256"],
            "parent_state_root_sha256": PARENT_STATE_ROOT,
            "parent_receipt_root_sha256": PARENT_RECEIPT_ROOT,
            "parent_final_root_sha256": PARENT_FINAL_ROOT,
            "opening_journal_root_sha256": PARENT_JOURNAL_ROOT,
            "scientific_authority": False,
        }
        result["predeployment_verification_root_sha256"] = digest(canonical_bytes(result))
        return result
    return verify_final(directory, preparation)


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    freeze_parser = commands.add_parser("create-freeze")
    freeze_parser.add_argument("--source-commit", required=True)
    freeze_parser.add_argument("--source-tree", required=True)
    freeze_parser.add_argument("--implementation-directory", type=Path, default=Path(__file__).resolve().parent)
    verify_parser = commands.add_parser("verify")
    verify_parser.add_argument("directory", type=Path)
    verify_parser.add_argument("--implementation-freeze", type=Path, required=True)
    verify_parser.add_argument("--predeployment", action="store_true")
    args = parser.parse_args()
    if args.command == "create-freeze":
        result = create_freeze(args.source_commit, args.source_tree, args.implementation_directory)
    else:
        result = verify(args.directory, args.implementation_freeze, args.predeployment)
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except InvalidReceipt as error:
        print(json.dumps({"valid": False, "error": str(error)}, sort_keys=True), file=os.sys.stderr)
        raise SystemExit(2)
