#!/usr/bin/env python3
"""Authority wrapper for the separately rooted S1C-3D transaction."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import sys
from pathlib import Path

import s1c3d_remote_transaction_v1 as mechanism
import verify_s1c3d_transaction_v1 as authority


SCHEMA = "nando.s1c3d-executor-error.v1"
ATTEMPT_RE = re.compile(r"^\d{8}T\d{6}Z-c3eaddc55dfc-s1c3d-v1$")


def require_attempt(value: str) -> None:
    if ATTEMPT_RE.fullmatch(value) is None:
        raise mechanism.GateFailure("s1c3d_attempt_id_invalid")


def verify_predeployment(
    root: Path,
    freeze_path: Path,
    verification_path: Path,
    envelope_path: Path,
) -> dict[str, object]:
    recorded = authority.load_json(envelope_path)
    expected = authority.build_envelope(
        root,
        freeze_path,
        predeployment=True,
        recorded_verification=verification_path,
    )
    if recorded != expected:
        raise mechanism.GateFailure("s1c3d_predeployment_authority_mismatch")
    if recorded.get("authority") is not True:
        raise mechanism.GateFailure("s1c3d_predeployment_authority_false")
    if recorded.get("verdict") not in {
        "S1C3D_PREPARATION_PASS",
        "S1C3D_PREPARATION_PASS_WITH_OPTIMIZATION_WATCH",
    }:
        raise mechanism.GateFailure("s1c3d_predeployment_not_pass")
    return recorded


def cleanup_parity_snapshot(root: Path) -> None:
    parity_path = root / "parity-receipt.json"
    if not parity_path.is_file():
        return
    parity = mechanism.read_json(parity_path)
    snapshot = parity.get("snapshot", {})
    directory = snapshot.get("directory", {}).get("path")
    if not isinstance(directory, str):
        raise mechanism.GateFailure("s1c3d_snapshot_cleanup_path_missing")
    fixture = Path(directory)
    try:
        fixture.relative_to(mechanism.SNAPSHOT_PARENT)
    except ValueError as error:
        raise mechanism.GateFailure("s1c3d_snapshot_cleanup_path_outside_parent") from error
    mechanism._remove_snapshot(fixture)
    if fixture.exists():
        raise mechanism.GateFailure("s1c3d_snapshot_cleanup_failed")


def prepare(args: argparse.Namespace) -> int:
    require_attempt(args.transaction_id)
    authority.verify_dependencies()
    freeze = authority.verify_implementation_freeze(
        Path(args.implementation_freeze),
        Path(__file__).resolve().parent,
        Path(args.bundle),
    )
    try:
        result = mechanism.prepare(args)
    except Exception:
        work = Path(f"/home/e/.cache/nando-s1c3-{args.transaction_id}")
        mechanism._remove_snapshot(mechanism.SNAPSHOT_PARENT / work.name)
        raise
    finally:
        root = Path(args.transaction_directory)
        if root.is_dir():
            authority.atomic_write(root / "implementation-freeze.json", freeze)
    return result


def execute(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    require_attempt(authority.transaction_id(root))
    envelope = verify_predeployment(
        root,
        root / "implementation-freeze.json",
        Path(args.predeployment_verification),
        Path(args.authority_envelope),
    )
    projection = authority._legacy_predeployment_projection(
        root, envelope["verification"]
    )
    projection_path = root / "mechanism-predeployment-projection.json"
    authority.atomic_write(projection_path, projection)
    return mechanism.execute(
        argparse.Namespace(
            transaction_directory=args.transaction_directory,
            predeployment_verification=str(projection_path),
        )
    )


def rollback(args: argparse.Namespace) -> int:
    require_attempt(authority.transaction_id(Path(args.transaction_directory)))
    mechanism.rollback(Path(args.transaction_directory), args.reason)
    return 0


def finalize(args: argparse.Namespace) -> int:
    require_attempt(authority.transaction_id(Path(args.transaction_directory)))
    return mechanism.finalize(args)


def abort_predeployment(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    transaction = authority.transaction_id(root)
    require_attempt(transaction)
    authority.verify_dependencies()
    authority.verify_implementation_freeze(
        root / "implementation-freeze.json", Path(__file__).resolve().parent
    )
    state = mechanism.read_json(root / "transaction-state.json").get("state")
    if state not in {"PREPARED", "PREFLIGHT_FAILURE"}:
        raise mechanism.GateFailure(f"s1c3d_predeployment_abort_state_invalid:{state}")
    mechanism.verify_current_production()
    if (root / "preparation.json").is_file():
        preparation = mechanism.read_json(root / "preparation.json")
        if mechanism.service_snapshot() != preparation["services_before"]:
            raise mechanism.GateFailure("s1c3d_predeployment_abort_service_drift")
    receipt = {
        "schema": "nando.s1c3d-preflight-failure.v1",
        "transaction_id": transaction,
        "verdict": "S1C3D_PREFLIGHT_FAILURE",
        "reason": args.reason,
        "identity_consumed": True,
        "production_mutation": False,
        "capture_installed": False,
        "scientific_authority": False,
        "s1c4_state": "CLOSED",
        "s2_state": "BLOCKED",
        "model_training": False,
        "phase_mutation": False,
    }
    receipt["preflight_failure_root_sha256"] = authority.digest(
        authority.canonical_bytes(receipt)
    )
    cleanup_parity_snapshot(root)
    authority.atomic_write(root / "s1c3d-preflight-failure.json", receipt)
    mechanism.write_json(
        root / "transaction-state.json",
        {
            "schema": mechanism.STATE_SCHEMA,
            "state": "PREFLIGHT_FAILURE",
            "transaction_id": transaction,
            "production_mutation": False,
        },
        0o600,
    )
    terminal = {
        "schema": authority.STATE_SCHEMA,
        "state": "PREFLIGHT_FAILURE",
        "transaction_id": transaction,
        "verdict": receipt["verdict"],
        "identity_consumed": True,
        "production_mutation": False,
        "scientific_authority": False,
        "preflight_failure_root_sha256": receipt[
            "preflight_failure_root_sha256"
        ],
    }
    terminal["state_root_sha256"] = authority.digest(
        authority.canonical_bytes(terminal)
    )
    authority.atomic_write(root / "s1c3d-state.json", terminal)
    print(json.dumps(terminal, sort_keys=True))
    return 0


def seal(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    require_attempt(authority.transaction_id(root))
    if mechanism.read_json(root / "transaction-state.json").get("state") != "FINAL_VERIFICATION_PENDING":
        raise mechanism.GateFailure("s1c3d_final_seal_state_invalid")
    recorded = authority.load_json(Path(args.authority_envelope))
    expected = authority.build_envelope(
        root,
        root / "implementation-freeze.json",
        predeployment=False,
        recorded_verification=Path(args.final_verification),
    )
    if recorded != expected:
        raise mechanism.GateFailure("s1c3d_final_authority_mismatch")
    with authority.patched_legacy_verifier():
        legacy = authority.legacy_verifier.verify_final(root)
    cleanup_parity_snapshot(root)
    legacy_path = root / "mechanism-final-verification-projection.json"
    authority.atomic_write(legacy_path, legacy)
    mechanism.seal(
        argparse.Namespace(
            transaction_directory=args.transaction_directory,
            final_verification=str(legacy_path),
        )
    )
    verification = recorded["verification"]
    if verification.get("s1c4_cursor") is not None:
        authority.atomic_write(root / "s1c4-append-cursor.json", verification["s1c4_cursor"])
    authority.atomic_write(root / "s1c3d-authority-envelope.json", recorded)
    terminal = {
        "schema": authority.STATE_SCHEMA,
        "state": "COMPLETE",
        "transaction_id": recorded["transaction_id"],
        "verdict": recorded["verdict"],
        "authority_envelope_root_sha256": recorded[
            "authority_envelope_root_sha256"
        ],
        "capture_installed": recorded["capture_installed"],
        "production_mutation": recorded["production_mutation"],
        "production_restored": recorded["production_restored"],
        "s1c4_state": recorded["s1c4_state"],
        "scientific_authority": False,
        "model_training": False,
        "phase_mutation": False,
    }
    terminal["state_root_sha256"] = authority.digest(
        authority.canonical_bytes(terminal)
    )
    authority.atomic_write(root / "s1c3d-state.json", terminal)
    print(json.dumps(terminal, sort_keys=True))
    return 0


def seal_resource_veto(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    require_attempt(authority.transaction_id(root))
    if mechanism.read_json(root / "transaction-state.json").get("state") != "RESOURCE_VETO":
        raise mechanism.GateFailure("s1c3d_resource_veto_state_invalid")
    recorded = authority.load_json(Path(args.authority_envelope))
    expected = authority.build_envelope(
        root,
        root / "implementation-freeze.json",
        predeployment=False,
        recorded_verification=Path(args.terminal_verification),
    )
    if recorded != expected:
        raise mechanism.GateFailure("s1c3d_resource_veto_authority_mismatch")
    if recorded["verdict"] not in {
        "S1C3D_CORRECTNESS_VETO",
        "S1C3D_SAFETY_VETO",
    }:
        raise mechanism.GateFailure("s1c3d_resource_veto_verdict_invalid")
    cleanup_parity_snapshot(root)
    authority.atomic_write(root / "s1c3d-authority-envelope.json", recorded)
    terminal = {
        "schema": authority.STATE_SCHEMA,
        "state": "COMPLETE",
        "transaction_id": recorded["transaction_id"],
        "verdict": recorded["verdict"],
        "authority_envelope_root_sha256": recorded[
            "authority_envelope_root_sha256"
        ],
        "identity_consumed": True,
        "production_mutation": False,
        "capture_installed": False,
        "s1c4_state": "CLOSED",
        "scientific_authority": False,
        "model_training": False,
        "phase_mutation": False,
    }
    terminal["state_root_sha256"] = authority.digest(
        authority.canonical_bytes(terminal)
    )
    authority.atomic_write(root / "s1c3d-state.json", terminal)
    print(json.dumps(terminal, sort_keys=True))
    return 0


def locked(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    descriptor = os.open(root / ".mutation.lock", os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX)
        if args.command == "execute":
            return execute(args)
        if args.command == "rollback":
            return rollback(args)
        if args.command == "finalize":
            return finalize(args)
        if args.command == "abort-predeployment":
            return abort_predeployment(args)
        if args.command == "seal-resource-veto":
            return seal_resource_veto(args)
        return seal(args)
    finally:
        os.close(descriptor)


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    prepare_parser = subparsers.add_parser("prepare")
    prepare_parser.add_argument("--transaction-id", required=True)
    prepare_parser.add_argument("--transaction-directory", required=True)
    prepare_parser.add_argument("--bundle", required=True)
    prepare_parser.add_argument("--candidate-config", required=True)
    prepare_parser.add_argument("--parity-source", required=True)
    prepare_parser.add_argument("--oracle-lock", required=True)
    prepare_parser.add_argument("--connector-before", required=True)
    prepare_parser.add_argument("--implementation-freeze", required=True)
    execute_parser = subparsers.add_parser("execute")
    execute_parser.add_argument("--transaction-directory", required=True)
    execute_parser.add_argument("--predeployment-verification", required=True)
    execute_parser.add_argument("--authority-envelope", required=True)
    rollback_parser = subparsers.add_parser("rollback")
    rollback_parser.add_argument("--transaction-directory", required=True)
    rollback_parser.add_argument("--reason", required=True)
    finalize_parser = subparsers.add_parser("finalize")
    finalize_parser.add_argument("--transaction-directory", required=True)
    finalize_parser.add_argument("--connector-after", required=True)
    abort_parser = subparsers.add_parser("abort-predeployment")
    abort_parser.add_argument("--transaction-directory", required=True)
    abort_parser.add_argument("--reason", required=True)
    seal_parser = subparsers.add_parser("seal")
    seal_parser.add_argument("--transaction-directory", required=True)
    seal_parser.add_argument("--final-verification", required=True)
    seal_parser.add_argument("--authority-envelope", required=True)
    veto_parser = subparsers.add_parser("seal-resource-veto")
    veto_parser.add_argument("--transaction-directory", required=True)
    veto_parser.add_argument("--terminal-verification", required=True)
    veto_parser.add_argument("--authority-envelope", required=True)
    args = parser.parse_args()
    try:
        if args.command == "prepare":
            return prepare(args)
        return locked(args)
    except (
        mechanism.GateFailure,
        authority.InvalidReceipt,
        authority.legacy_verifier.InvalidReceipt,
        OSError,
        ValueError,
    ) as error:
        print(json.dumps({"schema": SCHEMA, "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
