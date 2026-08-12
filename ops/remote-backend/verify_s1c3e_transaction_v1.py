#!/usr/bin/env python3
"""Independent verifier for the frozen S1C-3E ownership repair."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
from pathlib import Path
from typing import Any


PAPER_COMMIT = "c635ac27e9b1f49e1977c15bee1d03afe5f6d7e5"
PAPER_TREE = "553ff1803e5532ca93ed13b3eac996e6fdd4ae58"
PAPER_PREREGISTRATION_SHA256 = "b612127470ff38cc6957afa22d66359a97ce467c8ea41e77045b74bc064e2ff5"
PAPER_CRITIQUE_SHA256 = "b2ab39e1282c785fa8836093e1a269d33dc0d6db5cec5aec921084c32917291c"
PAPER_MANIFEST_ROOT = "2d11b32323e3d4b8dd37f48629e18cdc1d3304491ea3fad1b2bdb5359013ec2a"
PARENT_STATE_ROOT = "6ec0baf716a12467b9f7ca6e18bc6e6bf4543f1c95432dc65daf7b3ce5685ffb"
PARENT_RESOURCE_ROOT = "c917e62a85d2776e3a20d3efd72b16230a0689c73975b786d6ab8687c1176038"
PARENT_PARITY_ROOT = "55ae110ce15f198e0741890e856e5822170e1ba479870ea9c03ac4bd34ad3ea9"
PARENT_CLASSIFICATION_ROOT = "0d4f71a40c616a124f6aee3c03e4a868b4823f7aaf5c3bb7c48dee026aea7c01"
CANDIDATE_BINARY_SHA256 = "360498a0908739cad6f1ac21cf4053b7421daaf8b1d9a6502b72132a94a692df"
CANDIDATE_CONFIG_SHA256 = "1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6"
BASELINE_BINARY_SHA256 = "6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58"
BASELINE_CONFIG_SHA256 = "cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5"
EMPTY_SHA256 = hashlib.sha256(b"").hexdigest()
JOURNAL_PATH = "/var/lib/nando-wave/transition/grounded-meaning-v1/decision-contract-precommits-v1"
EXPECTED_SEGMENTS = (
    "decision-precommit-00000000000000000000.cbor",
    "goal-satisfaction-00000000000000000000.cbor",
    "selected-action-binding-00000000000000000000.cbor",
)
ATTEMPT_RE = re.compile(r"^\d{8}T\d{6}Z-[0-9a-f]{12}-s1c3e-v1$")
IMPLEMENTATION_FILES = (
    "s1c3e_remote_transaction_v1.py",
    "verify_s1c3e_transaction_v1.py",
    "test_s1c3e_transaction_v1.py",
    "run_s1c3e_transaction_v1.sh",
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


def require_exact(actual: Any, expected: Any, label: str) -> None:
    require(actual == expected, label)


def verify_root(value: dict[str, Any], field: str, label: str) -> str:
    root = value.get(field)
    require(isinstance(root, str) and len(root) == 64, f"{label}_missing")
    require_exact(root, digest(canonical_bytes(value, field)), f"{label}_mismatch")
    return root


def atomic_write(path: Path, value: dict[str, Any]) -> None:
    payload = canonical_bytes(value) + b"\n"
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    with temporary.open("xb") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())
    os.chmod(temporary, 0o400)
    os.replace(temporary, path)


def create_implementation_freeze(source_commit: str, source_tree: str, directory: Path) -> dict[str, Any]:
    require(re.fullmatch(r"[0-9a-f]{40}", source_commit) is not None, "source_commit")
    require(re.fullmatch(r"[0-9a-f]{40}", source_tree) is not None, "source_tree")
    files = []
    for name in IMPLEMENTATION_FILES:
        path = directory / name
        require(path.is_file(), f"implementation_missing:{name}")
        files.append({"path": name, "sha256": file_digest(path), "size_bytes": path.stat().st_size})
    value = {
        "schema": "nando.s1c3e-implementation-freeze.v1",
        "source_commit": source_commit,
        "source_tree": source_tree,
        "paper_commit": PAPER_COMMIT,
        "files": files,
    }
    value["implementation_freeze_root_sha256"] = digest(canonical_bytes(value))
    return value


def verify_implementation_freeze(path: Path, directory: Path) -> dict[str, Any]:
    value = load_json(path)
    require_exact(value.get("schema"), "nando.s1c3e-implementation-freeze.v1", "freeze_schema")
    verify_root(value, "implementation_freeze_root_sha256", "freeze_root")
    require_exact(value.get("paper_commit"), PAPER_COMMIT, "freeze_paper_commit")
    expected = []
    for name in IMPLEMENTATION_FILES:
        local = directory / name
        require(local.is_file(), f"freeze_local_missing:{name}")
        expected.append({"path": name, "sha256": file_digest(local), "size_bytes": local.stat().st_size})
    require_exact(value.get("files"), expected, "freeze_files")
    return value


def verify_preparation(directory: Path, freeze: dict[str, Any]) -> dict[str, Any]:
    value = load_json(directory / "preparation.json")
    verify_root(value, "preparation_root_sha256", "preparation_root")
    require_exact(value.get("schema"), "nando.s1c3e-preparation.v1", "preparation_schema")
    transaction_id = value.get("transaction_id")
    require(isinstance(transaction_id, str) and ATTEMPT_RE.fullmatch(transaction_id) is not None, "transaction_id")
    require_exact(value.get("paper"), {
        "commit": PAPER_COMMIT,
        "tree": PAPER_TREE,
        "preregistration_sha256": PAPER_PREREGISTRATION_SHA256,
        "critique_sha256": PAPER_CRITIQUE_SHA256,
        "manifest_root_sha256": PAPER_MANIFEST_ROOT,
    }, "paper")
    require_exact(value.get("implementation_freeze_root_sha256"), freeze["implementation_freeze_root_sha256"], "freeze_binding")
    require_exact(value.get("parent"), {
        "transaction_id": "20260812T145640Z-c3eaddc55dfc-s1c3d-v1",
        "state_root_sha256": PARENT_STATE_ROOT,
        "resource_root_sha256": PARENT_RESOURCE_ROOT,
        "parity_root_sha256": PARENT_PARITY_ROOT,
        "classification_root_sha256": PARENT_CLASSIFICATION_ROOT,
        "optimization_status": "OPTIMIZATION_WATCH",
    }, "parent_binding")
    require_exact(value.get("candidate"), {
        "binary_sha256": CANDIDATE_BINARY_SHA256,
        "config_sha256": CANDIDATE_CONFIG_SHA256,
    }, "candidate_binding")
    require_exact(file_digest(directory / "candidate-binary"), CANDIDATE_BINARY_SHA256, "candidate_binary_file")
    require_exact(file_digest(directory / "candidate-config"), CANDIDATE_CONFIG_SHA256, "candidate_config_file")
    parent_directory = directory / "parent-evidence"
    parent_state = load_json(parent_directory / "s1c3d-state.json")
    parent_resource = load_json(parent_directory / "resource-receipt.json")
    parent_parity = load_json(parent_directory / "parity-receipt.json")
    require_exact(verify_root(parent_state, "state_root_sha256", "parent_state"), PARENT_STATE_ROOT, "parent_state_exact")
    require_exact(verify_root(parent_resource, "resource_root_sha256", "parent_resource"), PARENT_RESOURCE_ROOT, "parent_resource_exact")
    require_exact(verify_root(parent_parity, "parity_root_sha256", "parent_parity"), PARENT_PARITY_ROOT, "parent_parity_exact")
    require_exact(parent_state.get("verdict"), "S1C3D_ROLLBACK_PASS", "parent_verdict")
    require_exact(parent_resource.get("classification", {}).get("classification_root_sha256"), PARENT_CLASSIFICATION_ROOT, "parent_classification")
    require_exact(parent_resource.get("classification", {}).get("hard_gate_status"), "PASS", "parent_hard_gate")
    require_exact(parent_resource.get("classification", {}).get("optimization_status"), "OPTIMIZATION_WATCH", "parent_optimization")
    require_exact(value.get("production"), {
        "binary_sha256": BASELINE_BINARY_SHA256,
        "config_sha256": BASELINE_CONFIG_SHA256,
    }, "production_binding")
    require_exact(value.get("journal_before"), {
        "present": False,
        "directory": None,
        "entries": [],
        "total_bytes": 0,
        "manifest_root_sha256": EMPTY_SHA256,
    }, "journal_before")
    require_exact(value.get("economics_before"), {
        "false_accepts": 0,
        "runtime_parity_mismatches": 0,
    }, "economics_before")
    return value


def verify_empty_journal(value: Any, label: str) -> None:
    require(isinstance(value, dict), f"{label}_object")
    require_exact(value.get("present"), True, f"{label}_present")
    directory = value.get("directory")
    require(isinstance(directory, dict), f"{label}_directory")
    require_exact(directory.get("path"), JOURNAL_PATH, f"{label}_directory_path")
    require_exact(directory.get("mode_octal"), "0700", f"{label}_directory_mode")
    require(isinstance(directory.get("uid"), int), f"{label}_directory_uid")
    require(isinstance(directory.get("gid"), int), f"{label}_directory_gid")
    entries = value.get("entries")
    require(isinstance(entries, list), f"{label}_entries")
    require_exact([row.get("path") for row in entries], list(EXPECTED_SEGMENTS), f"{label}_segment_set")
    for row in entries:
        name = row.get("path")
        require_exact(row.get("size_bytes"), 0, f"{label}_{name}_size")
        require_exact(row.get("sha256"), EMPTY_SHA256, f"{label}_{name}_sha")
        require_exact(row.get("uid"), directory.get("uid"), f"{label}_{name}_uid")
        require_exact(row.get("gid"), directory.get("gid"), f"{label}_{name}_gid")
        require_exact(row.get("mode_octal"), "0600", f"{label}_{name}_mode")
    require_exact(value.get("total_bytes"), 0, f"{label}_total")


def verify_runtime_journal(value: Any, opening: dict[str, Any], label: str) -> None:
    require(isinstance(value, dict), f"{label}_object")
    require_exact(value.get("present"), True, f"{label}_present")
    require_exact(value.get("directory"), opening.get("directory"), f"{label}_directory")
    entries = value.get("entries")
    require(isinstance(entries, list), f"{label}_entries")
    require_exact([row.get("path") for row in entries], list(EXPECTED_SEGMENTS), f"{label}_segments")
    opening_by_path = {row["path"]: row for row in opening["entries"]}
    for row in entries:
        old = opening_by_path[row["path"]]
        require_exact(row.get("uid"), old.get("uid"), f"{label}_{row['path']}_uid")
        require_exact(row.get("gid"), old.get("gid"), f"{label}_{row['path']}_gid")
        require_exact(row.get("mode_octal"), "0600", f"{label}_{row['path']}_mode")
        require(isinstance(row.get("size_bytes"), int) and row["size_bytes"] >= 0, f"{label}_{row['path']}_size")


def verify_connector(before: Any, after: Any) -> None:
    require(isinstance(before, dict) and isinstance(after, dict), "connector_object")
    for field in ("main_pid", "nrestarts", "route_receipt_failures", "command_sha256", "active_state"):
        require_exact(after.get(field), before.get(field), f"connector_{field}")
    require_exact(after.get("route_receipt_failures"), 0, "connector_receipt_failures")


def verify_services(receipt: dict[str, Any], deployment: bool) -> None:
    before = receipt.get("services_before")
    after = receipt.get("services_after")
    survival = receipt.get("services_survival")
    require(
        isinstance(before, dict) and isinstance(after, dict) and isinstance(survival, dict),
        "services_object",
    )
    require_exact(set(after), set(before), "services_after_set")
    require_exact(set(survival), set(before), "services_survival_set")
    transition = "nando-transition-serving.service"
    for unit in before:
        for value in (after[unit], survival[unit]):
            require_exact(value.get("active_state"), "active", f"{unit}_active")
            require(isinstance(value.get("main_pid"), int) and value["main_pid"] > 0, f"{unit}_pid")
            require_exact(value.get("nrestarts"), before[unit].get("nrestarts"), f"{unit}_nrestarts")
            require_exact(value.get("fragment_sha256"), before[unit].get("fragment_sha256"), f"{unit}_fragment")
        if unit == transition:
            require_exact(after[unit].get("main_pid"), survival[unit].get("main_pid"), "transition_survival_pid")
            if deployment:
                require(after[unit].get("main_pid") != before[unit].get("main_pid"), "transition_pid_not_replaced")
        else:
            require_exact(after[unit], before[unit], f"{unit}_after_untouched")
            require_exact(survival[unit], before[unit], f"{unit}_survival_untouched")


def verify_health(receipt: dict[str, Any]) -> None:
    before = receipt.get("health_before")
    after = receipt.get("health_after")
    survival = receipt.get("health_survival")
    require(
        isinstance(before, dict) and isinstance(after, dict) and isinstance(survival, dict),
        "health_object",
    )
    for label in before:
        require_exact(after.get(label, {}).get("semantic"), before[label].get("semantic"), f"health_{label}_after")
        require_exact(survival.get(label, {}).get("semantic"), before[label].get("semantic"), f"health_{label}_survival")


def verify_final(directory: Path, preparation: dict[str, Any]) -> dict[str, Any]:
    receipt = load_json(directory / "deployment-receipt.json")
    verify_root(receipt, "receipt_root_sha256", "receipt_root")
    require_exact(receipt.get("schema"), "nando.s1c3e-deployment-receipt.v1", "receipt_schema")
    verdict = receipt.get("verdict")
    require(verdict in {
        "S1C3E_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH",
        "S1C3E_ROLLBACK_PASS",
        "S1C3E_VETO",
    }, "verdict")
    require_exact(receipt.get("preparation_root_sha256"), preparation["preparation_root_sha256"], "receipt_preparation")
    require_exact(receipt.get("parent"), preparation["parent"], "receipt_parent")
    require_exact(receipt.get("candidate"), preparation["candidate"], "receipt_candidate")
    require_exact(receipt.get("scientific_authority"), False, "scientific_authority")
    require_exact(receipt.get("model_training"), False, "model_training")
    require_exact(receipt.get("phase_mutation"), False, "phase_mutation")
    require_exact(receipt.get("optimization_status"), "OPTIMIZATION_WATCH", "optimization_status")
    require_exact(receipt.get("false_accepts_after"), 0, "false_accepts")
    require_exact(receipt.get("runtime_parity_failures_after"), 0, "runtime_parity")
    verify_connector(receipt.get("connector_before"), receipt.get("connector_after"))
    deployment = verdict == "S1C3E_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH"
    verify_services(receipt, deployment)
    verify_health(receipt)
    require_exact(receipt.get("health_semantics_preserved"), True, "health_semantics_preserved")
    if deployment:
        require_exact(receipt.get("capture_available"), True, "capture_available")
        require_exact(receipt.get("startup_log_clean"), True, "startup_log")
        require_exact(receipt.get("active_packages_preserved"), True, "active_packages")
        require_exact(receipt.get("installed_binary_sha256"), CANDIDATE_BINARY_SHA256, "installed_binary")
        require_exact(receipt.get("installed_config_sha256"), CANDIDATE_CONFIG_SHA256, "installed_config")
        require_exact(receipt.get("capture_environment"), {
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
            "NANDO_GROUNDED_DECISION_JOURNAL": JOURNAL_PATH,
        }, "capture_environment")
        verify_empty_journal(receipt.get("journal_after"), "journal_after")
        verify_runtime_journal(receipt.get("journal_survival"), receipt["journal_after"], "journal_survival")
    else:
        require_exact(receipt.get("capture_available"), False, "rollback_capture")
        require_exact(receipt.get("installed_binary_sha256"), BASELINE_BINARY_SHA256, "rollback_binary")
        require_exact(receipt.get("installed_config_sha256"), BASELINE_CONFIG_SHA256, "rollback_config")
        journal = receipt.get("journal_after")
        require(isinstance(journal, dict), "rollback_journal")
        if journal.get("present"):
            require(
                isinstance(journal.get("entries"), list)
                and any(row.get("size_bytes", 0) > 0 for row in journal["entries"]),
                "rollback_empty_journal_not_removed",
            )
    cursor = None
    if deployment:
        journal = receipt["journal_after"]
        cursor = {
            "schema": "nando.s1c4-append-cursor.v1",
            "transaction_id": receipt["transaction_id"],
            "deployment_receipt_root_sha256": receipt["receipt_root_sha256"],
            "journal_manifest_root_sha256": journal["manifest_root_sha256"],
            "journal_entries": journal["entries"],
            "retroactive_rows_allowed": False,
            "scientific_authority": False,
            "state": "COLLECTING",
        }
        cursor["cursor_root_sha256"] = digest(canonical_bytes(cursor))
    result = {
        "schema": "nando.s1c3e-final-verification.v1",
        "valid": True,
        "authority": True,
        "verdict": verdict,
        "receipt_root_sha256": receipt["receipt_root_sha256"],
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "capture_installed": deployment,
        "s1c4_state": "COLLECTING" if deployment else "CLOSED",
        "s1c4_cursor": cursor,
        "scientific_authority": False,
        "model_training": False,
        "phase_mutation": False,
    }
    result["final_verification_root_sha256"] = digest(canonical_bytes(result))
    return result


def verify(directory: Path, freeze_path: Path, predeployment: bool) -> dict[str, Any]:
    freeze = verify_implementation_freeze(freeze_path, Path(__file__).resolve().parent)
    preparation = verify_preparation(directory, freeze)
    if predeployment:
        result = {
            "schema": "nando.s1c3e-predeployment-verification.v1",
            "valid": True,
            "authority": True,
            "verdict": "S1C3E_PREPARATION_PASS_WITH_OPTIMIZATION_WATCH",
            "preparation_root_sha256": preparation["preparation_root_sha256"],
            "parent_state_root_sha256": PARENT_STATE_ROOT,
            "parent_resource_root_sha256": PARENT_RESOURCE_ROOT,
            "parent_parity_root_sha256": PARENT_PARITY_ROOT,
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
        result = create_implementation_freeze(args.source_commit, args.source_tree, args.implementation_directory)
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
