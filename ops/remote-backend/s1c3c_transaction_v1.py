#!/usr/bin/env python3
"""Authority wrapper for the separately preregistered S1C-3C transaction."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import sys
from pathlib import Path

import s1c3b_remote_transaction_v1 as mechanism
import verify_s1c3c_transaction_v1 as authority


SCHEMA = "nando.s1c3c-executor-error.v1"
ATTEMPT_RE = re.compile(r"^\d{8}T\d{6}Z-2a1505055ce9-s1c3c-v1$")


def require_attempt(value: str) -> None:
    if ATTEMPT_RE.fullmatch(value) is None:
        raise mechanism.GateFailure("s1c3c_attempt_id_invalid")


def verify_predeployment(
    root: Path,
    schema_receipt: Path,
    mechanism_receipt: Path,
    authority_envelope: Path,
) -> None:
    recorded = authority.load_json(authority_envelope)
    expected = authority.build_envelope(
        root,
        schema_receipt,
        predeployment=True,
        recorded_mechanism_path=mechanism_receipt,
    )
    if recorded != expected:
        raise mechanism.GateFailure("s1c3c_predeployment_authority_mismatch")
    if recorded.get("verdict") != "S1C3C_PREPARATION_PASS":
        raise mechanism.GateFailure("s1c3c_predeployment_not_pass")
    if recorded.get("authority") is not True:
        raise mechanism.GateFailure("s1c3c_predeployment_authority_false")


def prepare(args: argparse.Namespace) -> int:
    require_attempt(args.transaction_id)
    authority.verify_dependencies()
    authority.verify_schema_receipt(Path(args.schema_preflight))
    freeze = authority.verify_implementation_freeze(
        Path(args.implementation_freeze),
        Path(__file__).resolve().parent,
        Path(args.bundle),
    )
    try:
        return mechanism.prepare(args)
    finally:
        root = Path(args.transaction_directory)
        if root.is_dir():
            authority.atomic_write(root / "implementation-freeze.json", freeze)


def execute(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    require_attempt(authority.transaction_id(root))
    verify_predeployment(
        root,
        Path(args.schema_preflight),
        Path(args.mechanism_predeployment_verification),
        Path(args.authority_predeployment_envelope),
    )
    mechanism_args = argparse.Namespace(
        transaction_directory=args.transaction_directory,
        predeployment_verification=args.mechanism_predeployment_verification,
    )
    return mechanism.execute(mechanism_args)


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
    authority.verify_schema_receipt(Path(args.schema_preflight))
    authority.verify_implementation_freeze(
        root / "implementation-freeze.json", Path(__file__).resolve().parent
    )
    state = mechanism.read_json(root / "transaction-state.json").get("state")
    if state not in {"PREPARED", "PREFLIGHT_FAILURE"}:
        raise mechanism.GateFailure(f"s1c3c_predeployment_abort_state_invalid:{state}")
    mechanism.verify_current_production()
    if (root / "preparation.json").is_file():
        preparation = mechanism.read_json(root / "preparation.json")
        if mechanism.service_snapshot() != preparation["services_before"]:
            raise mechanism.GateFailure("s1c3c_predeployment_abort_service_drift")
    receipt = {
        "schema": "nando.s1c3c-preflight-failure.v1",
        "transaction_id": transaction,
        "verdict": "S1C3C_PREFLIGHT_FAILURE",
        "reason": args.reason,
        "attempt_consumed": True,
        "production_mutation": False,
        "capture_installed": False,
        "scientific_authority": False,
        "s1c4_state": "CLOSED",
        "s2_state": "BLOCKED",
    }
    receipt["preflight_failure_root_sha256"] = authority.digest(
        authority.canonical_bytes(receipt)
    )
    authority.atomic_write(root / "s1c3c-preflight-failure.json", receipt)
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
        "verdict": "S1C3C_PREFLIGHT_FAILURE",
        "attempt_consumed": True,
        "production_mutation": False,
        "scientific_authority": False,
        "preflight_failure_root_sha256": receipt[
            "preflight_failure_root_sha256"
        ],
    }
    terminal["state_root_sha256"] = authority.digest(authority.canonical_bytes(terminal))
    authority.atomic_write(root / "s1c3c-state.json", terminal)
    print(json.dumps(terminal, sort_keys=True))
    return 0


def seal(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    require_attempt(authority.transaction_id(root))
    state = mechanism.read_json(root / "transaction-state.json").get("state")
    if state == "FINAL_VERIFICATION_PENDING":
        envelope, terminal = authority.terminal_state(
            root,
            Path(args.schema_preflight),
            Path(args.authority_envelope),
            mechanism_state="FINAL_VERIFICATION_PENDING",
            recorded_mechanism_path=Path(args.mechanism_final_verification),
        )
    elif state in {"COMPLETE", "RESOURCE_VETO"}:
        envelope, terminal = authority.terminal_state(
            root,
            Path(args.schema_preflight),
            Path(args.authority_envelope),
            mechanism_state=state,
            recorded_mechanism_path=(
                Path(args.mechanism_final_verification) if state == "COMPLETE" else None
            ),
        )
    else:
        raise mechanism.GateFailure(f"s1c3c_mechanism_seal_state_invalid:{state}")
    if state == "FINAL_VERIFICATION_PENDING":
        mechanism.seal(
            argparse.Namespace(
                transaction_directory=args.transaction_directory,
                final_verification=args.mechanism_final_verification,
            )
        )
    authority.atomic_write(root / "s1c3c-authority-envelope.json", envelope)
    authority.atomic_write(root / "s1c3c-state.json", terminal)
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
    prepare_parser.add_argument("--schema-preflight", required=True)
    prepare_parser.add_argument("--implementation-freeze", required=True)
    execute_parser = subparsers.add_parser("execute")
    execute_parser.add_argument("--transaction-directory", required=True)
    execute_parser.add_argument("--schema-preflight", required=True)
    execute_parser.add_argument("--mechanism-predeployment-verification", required=True)
    execute_parser.add_argument("--authority-predeployment-envelope", required=True)
    rollback_parser = subparsers.add_parser("rollback")
    rollback_parser.add_argument("--transaction-directory", required=True)
    rollback_parser.add_argument("--reason", required=True)
    finalize_parser = subparsers.add_parser("finalize")
    finalize_parser.add_argument("--transaction-directory", required=True)
    finalize_parser.add_argument("--connector-after", required=True)
    abort_parser = subparsers.add_parser("abort-predeployment")
    abort_parser.add_argument("--transaction-directory", required=True)
    abort_parser.add_argument("--schema-preflight", required=True)
    abort_parser.add_argument("--reason", required=True)
    seal_parser = subparsers.add_parser("seal")
    seal_parser.add_argument("--transaction-directory", required=True)
    seal_parser.add_argument("--schema-preflight", required=True)
    seal_parser.add_argument("--mechanism-final-verification", required=True)
    seal_parser.add_argument("--authority-envelope", required=True)
    args = parser.parse_args()
    try:
        if args.command == "prepare":
            return prepare(args)
        return locked(args)
    except (
        mechanism.GateFailure,
        authority.InvalidReceipt,
        authority.mechanism_verifier.InvalidReceipt,
        OSError,
        ValueError,
    ) as error:
        print(json.dumps({"schema": SCHEMA, "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
