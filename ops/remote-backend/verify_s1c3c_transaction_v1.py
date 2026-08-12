#!/usr/bin/env python3
"""Independent S1C-3C authority envelope over the pinned S1C-3B mechanism."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

import s1c3c_schema_preflight_v1 as schema_gate
import verify_s1c3b_transaction_v1 as mechanism_verifier


SCHEMA = "nando.s1c3c-authority-envelope.v1"
STATE_SCHEMA = "nando.s1c3c-state.v1"
FREEZE_SCHEMA = "nando.s1c3c-implementation-freeze.v1"
PAPER_COMMIT = "2a1505055ce98b3f6bed5cb440a0faa345fb78cb"
PAPER_TREE = "68a0dff858e5b49445997f09d17cc52d22e12511"
PAPER_PREREGISTRATION_SHA256 = "d56289d4d67600786fe08c5e8d5478448b75bb1b9aeba9c0291da20d4a192492"
PAPER_CRITIQUE_SHA256 = "2e34b55fccb0dadceec1e97bc9a4880d282308243bf9abb4faf418c6e81b2ff6"
PAPER_VERIFICATION_SHA256 = "cfa0e6cdb4176fb3f191d1f32afa28d4469505db76bffad0d6ad95d4f46b1ff2"
PAPER_MANIFEST_SHA256 = "913eefbb6a021fcedb53b5a788bc5369394c204fec6c5c5ab0077a1d04f08bfe"
MECHANISM_EXECUTOR_SHA256 = "74fde9997bb14f4064aa01303cc67cd79e2dea826f39bfc50850e49394b70523"
MECHANISM_VERIFIER_SHA256 = "72e29c6f52e3e29648a7f1bf13cc66b02ad5f0fe68db07cd9dbfa54ff86561dd"
LEGACY_EXECUTOR_SHA256 = "d0a490d93cc5dbd488119d7cc721de0cf9609ab5d97c87efb8a1de69916ab971"
LEGACY_VERIFIER_SHA256 = "8e383844e7d945cd94829dfd772a68fffcb89457556a30edb41bb42b615162bc"
ATTEMPT_RE = re.compile(r"^\d{8}T\d{6}Z-2a1505055ce9-s1c3c-v1$")
IMPLEMENTATION_FILES = (
    "run_s1c3c_transaction_v1.sh",
    "s1c3c_schema_preflight_v1.py",
    "s1c3c_transaction_v1.py",
    "verify_s1c3c_transaction_v1.py",
    "test_s1c3c_transaction_v1.py",
)
FROZEN_SOURCE_FILES = (
    "ops/remote-backend/run_s1c3c_transaction_v1.sh",
    "ops/remote-backend/s1c3c_schema_preflight_v1.py",
    "ops/remote-backend/s1c3c_transaction_v1.py",
    "ops/remote-backend/verify_s1c3c_transaction_v1.py",
    "ops/remote-backend/test_s1c3c_transaction_v1.py",
    "ops/remote-backend/s1c3b_remote_transaction_v1.py",
    "ops/remote-backend/verify_s1c3b_transaction_v1.py",
    "ops/remote-backend/s1c3_remote_transaction_v7.py",
    "ops/remote-backend/verify_s1c3_transaction_v7.py",
    "plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md",
    "plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_CRITIQUE_V1.md",
    "plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PAPER_VERIFICATION_2026-08-12.md",
    "plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_PAPER_V1/SHA256SUMS",
    "plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_IMPLEMENTATION_V1/schema-owner.worksheet.md",
    "plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_IMPLEMENTATION_V1/schema-owner.result.json",
    "plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_IMPLEMENTATION_V1/freeze-owner.worksheet.md",
    "plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_IMPLEMENTATION_V1/freeze-owner.result.json",
    "plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_IMPLEMENTATION_V1/transaction-owner.worksheet.md",
    "plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_IMPLEMENTATION_V1/transaction-owner.result.json",
    "plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_IMPLEMENTATION_V1/authority-owner.worksheet.md",
    "plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_IMPLEMENTATION_V1/authority-owner.result.json",
    "plans/effect-law-unification-v1/evidence/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V7/transition-serving.env.candidate",
)


class InvalidReceipt(ValueError):
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
        raise InvalidReceipt(error)


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise InvalidReceipt(f"json_invalid:{path.name}") from error
    require(isinstance(value, dict), f"json_not_object:{path.name}")
    return value


def atomic_write(path: Path, value: dict[str, Any], mode: int = 0o400) -> None:
    payload = canonical_bytes(value)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    try:
        os.write(descriptor, payload)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    os.replace(temporary, path)
    os.chmod(path, mode)
    directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def module_path(module: Any) -> Path:
    value = getattr(module, "__file__", None)
    require(isinstance(value, str), "module_path_missing")
    return Path(value).resolve()


def verify_dependencies() -> dict[str, str]:
    root = module_path(mechanism_verifier).parent
    expected = {
        "s1c3b_remote_transaction_v1.py": MECHANISM_EXECUTOR_SHA256,
        "verify_s1c3b_transaction_v1.py": MECHANISM_VERIFIER_SHA256,
        "s1c3_remote_transaction_v7.py": LEGACY_EXECUTOR_SHA256,
        "verify_s1c3_transaction_v7.py": LEGACY_VERIFIER_SHA256,
    }
    for name, expected_sha in expected.items():
        path = root / name
        require(path.is_file(), f"dependency_missing:{name}")
        require(file_digest(path) == expected_sha, f"dependency_drift:{name}")
    return expected


def bundle_identity(bundle: Path, source_commit: str) -> tuple[str, dict[str, str]]:
    require(bundle.is_file(), "source_bundle_missing")
    with tempfile.TemporaryDirectory(prefix="nando-s1c3c-bundle-") as directory:
        repository = Path(directory) / "repository.git"
        completed = subprocess.run(
            ["git", "clone", "--quiet", "--bare", str(bundle), str(repository)],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        require(completed.returncode == 0, "source_bundle_clone_failed")
        tree = subprocess.run(
            ["git", "-C", str(repository), "rev-parse", f"{source_commit}^{{tree}}"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        require(tree.returncode == 0, "source_commit_missing_from_bundle")
        source_tree = tree.stdout.strip()
        require(re.fullmatch(r"[0-9a-f]{40}", source_tree) is not None, "source_tree_invalid")
        files: dict[str, str] = {}
        for name in FROZEN_SOURCE_FILES:
            content = subprocess.run(
                ["git", "-C", str(repository), "show", f"{source_commit}:{name}"],
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            require(content.returncode == 0, f"frozen_source_file_missing:{name}")
            files[name] = digest(content.stdout)
    return source_tree, files


def create_implementation_freeze(
    source_commit: str,
    source_tree: str,
    bundle: Path,
    implementation_directory: Path,
) -> dict[str, Any]:
    require(re.fullmatch(r"[0-9a-f]{40}", source_commit) is not None, "source_commit")
    require(re.fullmatch(r"[0-9a-f]{40}", source_tree) is not None, "source_tree")
    bundle_tree, source_files = bundle_identity(bundle, source_commit)
    require(bundle_tree == source_tree, "source_bundle_tree_mismatch")
    files = {}
    for name in IMPLEMENTATION_FILES:
        path = implementation_directory / name
        require(path.is_file(), f"implementation_file_missing:{name}")
        files[name] = file_digest(path)
        require(
            source_files[f"ops/remote-backend/{name}"] == files[name],
            f"implementation_file_not_bound_to_bundle:{name}",
        )
    receipt = {
        "schema": FREEZE_SCHEMA,
        "paper_commit": PAPER_COMMIT,
        "source_commit": source_commit,
        "source_tree": source_tree,
        "source_bundle_sha256": file_digest(bundle),
        "source_files": source_files,
        "implementation_files": files,
    }
    receipt["implementation_freeze_root_sha256"] = digest(canonical_bytes(receipt))
    return receipt


def verify_implementation_freeze(
    path: Path,
    implementation_directory: Path,
    bundle: Path | None = None,
) -> dict[str, Any]:
    receipt = load_json(path)
    require(receipt.get("schema") == FREEZE_SCHEMA, "implementation_freeze_schema")
    root = receipt.get("implementation_freeze_root_sha256")
    require(
        isinstance(root, str)
        and root == digest(canonical_bytes(receipt, "implementation_freeze_root_sha256")),
        "implementation_freeze_root",
    )
    require(receipt.get("paper_commit") == PAPER_COMMIT, "implementation_freeze_paper")
    require(
        re.fullmatch(r"[0-9a-f]{40}", str(receipt.get("source_commit"))) is not None,
        "implementation_freeze_commit",
    )
    require(
        re.fullmatch(r"[0-9a-f]{40}", str(receipt.get("source_tree"))) is not None,
        "implementation_freeze_tree",
    )
    source_files = receipt.get("source_files")
    require(
        isinstance(source_files, dict) and set(source_files) == set(FROZEN_SOURCE_FILES),
        "implementation_freeze_source_file_set",
    )
    require(
        all(re.fullmatch(r"[0-9a-f]{64}", str(value)) for value in source_files.values()),
        "implementation_freeze_source_file_hash",
    )
    files = receipt.get("implementation_files")
    require(
        isinstance(files, dict) and set(files) == set(IMPLEMENTATION_FILES),
        "implementation_freeze_file_set",
    )
    for name in IMPLEMENTATION_FILES:
        candidate = implementation_directory / name
        require(candidate.is_file(), f"implementation_file_missing:{name}")
        require(file_digest(candidate) == files[name], f"implementation_file_drift:{name}")
    if bundle is not None:
        require(bundle.is_file(), "implementation_bundle_missing")
        require(
            file_digest(bundle) == receipt.get("source_bundle_sha256"),
            "implementation_bundle_drift",
        )
        bundle_tree, bundle_files = bundle_identity(bundle, receipt["source_commit"])
        require(bundle_tree == receipt.get("source_tree"), "implementation_bundle_tree_drift")
        require(bundle_files == source_files, "implementation_bundle_source_files_drift")
    return receipt


def verify_schema_receipt(path: Path) -> dict[str, Any]:
    recorded = load_json(path)
    expected = schema_gate.run_preflight()
    require(recorded == expected, "schema_preflight_receipt_mismatch")
    require(recorded.get("valid") is True, "schema_preflight_invalid")
    require(recorded.get("authority") is False, "schema_preflight_authority")
    require(recorded.get("side_effects") is False, "schema_preflight_side_effects")
    require(
        recorded.get("remote_attempt_created") is False,
        "schema_preflight_remote_attempt",
    )
    root = recorded.get("schema_preflight_root_sha256")
    require(
        isinstance(root, str) and root == digest(canonical_bytes(recorded, "schema_preflight_root_sha256")),
        "schema_preflight_root",
    )
    return recorded


def transaction_id(directory: Path) -> str:
    for name in ("preparation.json", "transaction-state.json"):
        path = directory / name
        if path.is_file():
            value = load_json(path).get("transaction_id")
            if isinstance(value, str):
                require(ATTEMPT_RE.fullmatch(value) is not None, "attempt_id_invalid")
                return value
    raise InvalidReceipt("attempt_id_missing")


def mechanism_result(
    directory: Path, *, predeployment: bool, allow_terminal: bool
) -> dict[str, Any]:
    state = load_json(directory / "transaction-state.json")
    if state.get("state") == "RESOURCE_VETO":
        require(not predeployment, "resource_veto_is_not_predeployment")
        return mechanism_verifier.verify_resource_veto(directory)
    if predeployment:
        return mechanism_verifier.verify_preparation(directory)
    require(allow_terminal, "terminal_verification_not_allowed")
    return mechanism_verifier.verify_final(directory)


VERDICT_MAP = {
    "S1C3B_PREPARATION_PASS": "S1C3C_PREPARATION_PASS",
    "S1C3B_RESOURCE_VETO": "S1C3C_RESOURCE_VETO",
    "S1C3B_DEPLOYMENT_PASS": "S1C3C_DEPLOYMENT_PASS",
    "S1C3B_ROLLBACK_PASS": "S1C3C_ROLLBACK_PASS",
    "S1C3B_VETO": "S1C3C_VETO",
}


def build_envelope(
    directory: Path,
    schema_receipt_path: Path,
    *,
    predeployment: bool = False,
    allow_terminal: bool = False,
    recorded_mechanism_path: Path | None = None,
) -> dict[str, Any]:
    dependencies = verify_dependencies()
    schema_receipt = verify_schema_receipt(schema_receipt_path)
    implementation_freeze = verify_implementation_freeze(
        directory / "implementation-freeze.json", module_path(mechanism_verifier).parent
    )
    mechanism = mechanism_result(
        directory, predeployment=predeployment, allow_terminal=allow_terminal
    )
    if recorded_mechanism_path is not None:
        require(
            load_json(recorded_mechanism_path) == mechanism,
            "recorded_mechanism_verification_mismatch",
        )
    mechanism_verdict = mechanism.get("verdict")
    require(mechanism_verdict in VERDICT_MAP, "mechanism_verdict_invalid")
    verdict = VERDICT_MAP[mechanism_verdict]
    is_deployment = verdict == "S1C3C_DEPLOYMENT_PASS"
    is_preparation = verdict == "S1C3C_PREPARATION_PASS"
    is_resource = verdict == "S1C3C_RESOURCE_VETO"
    is_rollback = verdict in {"S1C3C_ROLLBACK_PASS", "S1C3C_VETO"}
    envelope = {
        "schema": SCHEMA,
        "valid": True,
        "authority": is_preparation or is_deployment or is_rollback,
        "scientific_authority": False,
        "verdict": verdict,
        "transaction_id": transaction_id(directory),
        "attempt_consumed": True,
        "protocol": {
            "commit": PAPER_COMMIT,
            "tree": PAPER_TREE,
            "preregistration_sha256": PAPER_PREREGISTRATION_SHA256,
            "critique_sha256": PAPER_CRITIQUE_SHA256,
            "verification_sha256": PAPER_VERIFICATION_SHA256,
            "manifest_sha256": PAPER_MANIFEST_SHA256,
        },
        "schema_preflight_root_sha256": schema_receipt[
            "schema_preflight_root_sha256"
        ],
        "implementation_freeze_root_sha256": implementation_freeze[
            "implementation_freeze_root_sha256"
        ],
        "implementation_source": {
            "commit": implementation_freeze["source_commit"],
            "tree": implementation_freeze["source_tree"],
            "bundle_sha256": implementation_freeze["source_bundle_sha256"],
        },
        "mechanism": {
            "protocol": "S1C-3B pinned mechanism only",
            "verdict": mechanism_verdict,
            "verification_sha256": digest(canonical_bytes(mechanism)),
            "dependencies": dependencies,
        },
        "production_mutation": is_deployment or is_rollback,
        "production_restored": is_rollback,
        "capture_installed": is_deployment,
        "resource_verdict": (
            "PASS" if is_preparation or is_deployment else "VETO" if is_resource else None
        ),
        "deployment_verdict": (
            "PASS" if is_deployment else "ROLLBACK" if is_rollback else None
        ),
        "s1c4_state": "CLOSED",
        "s2_state": "BLOCKED",
        "model_training_allowed": False,
        "phase_mutation_allowed": False,
    }
    envelope["authority_envelope_root_sha256"] = digest(canonical_bytes(envelope))
    return envelope


def verify_envelope(
    directory: Path,
    schema_receipt: Path,
    envelope_path: Path,
    *,
    allow_terminal: bool,
    recorded_mechanism_path: Path | None = None,
) -> dict[str, Any]:
    recorded = load_json(envelope_path)
    expected = build_envelope(
        directory,
        schema_receipt,
        allow_terminal=allow_terminal,
        recorded_mechanism_path=recorded_mechanism_path,
    )
    require(recorded == expected, "authority_envelope_mismatch")
    return recorded


def terminal_state(
    directory: Path,
    schema_receipt: Path,
    envelope_path: Path,
    *,
    mechanism_state: str,
    recorded_mechanism_path: Path | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    mechanism_state_receipt = load_json(directory / "transaction-state.json")
    observed_state = mechanism_state_receipt.get("state")
    if mechanism_state == "FINAL_VERIFICATION_PENDING":
        require(observed_state == "FINAL_VERIFICATION_PENDING", "mechanism_not_pending_seal")
    else:
        require(observed_state == mechanism_state, "mechanism_state_mismatch")
    require(mechanism_state in {"FINAL_VERIFICATION_PENDING", "COMPLETE", "RESOURCE_VETO"}, "mechanism_not_terminal")
    envelope = verify_envelope(
        directory,
        schema_receipt,
        envelope_path,
        allow_terminal=True,
        recorded_mechanism_path=(
            recorded_mechanism_path
            if recorded_mechanism_path is not None
            else directory / "final-verification.json"
            if mechanism_state == "COMPLETE"
            else None
        ),
    )
    if mechanism_state == "RESOURCE_VETO":
        require(envelope.get("verdict") == "S1C3C_RESOURCE_VETO", "resource_veto_envelope")
        require(envelope.get("production_mutation") is False, "resource_veto_mutation")
    state = {
        "schema": STATE_SCHEMA,
        "state": "COMPLETE",
        "transaction_id": envelope["transaction_id"],
        "verdict": envelope["verdict"],
        "authority_envelope_root_sha256": envelope[
            "authority_envelope_root_sha256"
        ],
        "scientific_authority": False,
    }
    state["state_root_sha256"] = digest(canonical_bytes(state))
    return envelope, state


def seal(directory: Path, schema_receipt: Path, envelope_path: Path) -> dict[str, Any]:
    require(os.geteuid() == 0, "root_required")
    mechanism_state = load_json(directory / "transaction-state.json").get("state")
    envelope, state = terminal_state(
        directory,
        schema_receipt,
        envelope_path,
        mechanism_state=mechanism_state,
        recorded_mechanism_path=(
            directory / "final-verification.json"
            if mechanism_state == "COMPLETE"
            else None
        ),
    )
    atomic_write(directory / "s1c3c-authority-envelope.json", envelope)
    atomic_write(directory / "s1c3c-state.json", state)
    return state


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("transaction_directory", type=Path)
    verify_parser.add_argument("--schema-preflight", type=Path, required=True)
    verify_parser.add_argument("--mechanism-verification", type=Path)
    verify_parser.add_argument("--pre-deployment", action="store_true")
    verify_parser.add_argument("--allow-terminal", action="store_true")
    seal_parser = subparsers.add_parser("seal")
    seal_parser.add_argument("transaction_directory", type=Path)
    seal_parser.add_argument("--schema-preflight", type=Path, required=True)
    seal_parser.add_argument("--authority-envelope", type=Path, required=True)
    freeze_parser = subparsers.add_parser("freeze")
    freeze_parser.add_argument("--source-commit", required=True)
    freeze_parser.add_argument("--source-tree", required=True)
    freeze_parser.add_argument("--bundle", type=Path, required=True)
    freeze_parser.add_argument("--implementation-directory", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "freeze":
            result = create_implementation_freeze(
                args.source_commit,
                args.source_tree,
                args.bundle,
                args.implementation_directory,
            )
        elif args.command == "seal":
            result = seal(
                args.transaction_directory,
                args.schema_preflight,
                args.authority_envelope,
            )
        else:
            result = build_envelope(
                args.transaction_directory,
                args.schema_preflight,
                predeployment=args.pre_deployment,
                allow_terminal=args.allow_terminal,
                recorded_mechanism_path=args.mechanism_verification,
            )
    except (InvalidReceipt, mechanism_verifier.InvalidReceipt, OSError, ValueError) as error:
        print(
            json.dumps(
                {"schema": SCHEMA, "valid": False, "authority": False, "error": str(error)},
                sort_keys=True,
            )
        )
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
