#!/usr/bin/env python3
"""Record-aware S1C-3F transaction over the S1C-3E mechanism."""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import hashlib
import io
import json
import os
import shutil
import struct
from pathlib import Path
from typing import Any

import s1c3e_remote_transaction_v1 as base


PAPER_COMMIT = "e6c733a243fb8c95920d971c17edf1c3cda65def"
PAPER_TREE = "0e914870e08e132c93d9608c1e4873c0636ed5d4"
PAPER_PREREGISTRATION_SHA256 = "3df587badc3f99e714f6bccae03524d812e22fe85ff37d59da5a34b0958d0034"
PAPER_CRITIQUE_SHA256 = "c61f3d22ed74e7926b57c3b0a8cf25e26ce11a2e1ba5dee43cb67f564a3df6e0"
PAPER_MANIFEST_ROOT = "489db0f03647fadd7354eea5d19d54bba3c4ebb03aa5017c272110f88bbca362"

PARENT_TRANSACTION_ID = "20260812T153838Z-25c0f1168fa4-s1c3e-v1"
PARENT_DIRECTORY = Path("/var/lib/nando-wave/deployments") / PARENT_TRANSACTION_ID
PARENT_STATE_ROOT = "442aefee66e7c04c561143b00d0a3fd6bcb01f65f5149571bd0f3d35f6b2a77c"
PARENT_RECEIPT_ROOT = "baedc988ac8664cde09946d2f8b780ce9221086dd8d6d68bb02280582214ecb0"
PARENT_FINAL_ROOT = "637d90ffece05f628d6ed27041dbe22706a3df4ef04240185096980b484fdcf3"
PARENT_JOURNAL_ROOT = "6ab9cd4823f1d737ec731e9c96049c0492d1966d91b3408dd524ea23b1c8666c"
PARENT_FILE_SHA256 = {
    "s1c3e-state.json": "e9b5588110aa238060d02a985157688a3f472f7ce0d8eca5cd9036abda9e9b2d",
    "deployment-receipt.json": "21fb0fe55b74ff748d6a68d181710f87f643260921cc0e7c98e4e1f0b50a8eef",
    "final-verification.json": "1f38df7dbfd05649e8f99e58d687341daf171f8bcc7c4ab2f1d168b7c2cf24de",
}

SEGMENT_MAGIC = b"NTF1"
MAX_FRAME_PAYLOAD_BYTES = 16 * 1024 * 1024
MAGIC_ONLY_SHA256 = "4fc61a14f994e28249509ec2504e89df30497a2aa76b1d9c5f6c38e2acee6072"

PREPARATION_SCHEMA = "nando.s1c3f-preparation.v1"
PREDEPLOYMENT_SCHEMA = "nando.s1c3f-predeployment-verification.v1"
PENDING_SCHEMA = "nando.s1c3f-pending-receipt.v1"
RECEIPT_SCHEMA = "nando.s1c3f-deployment-receipt.v1"
STATE_SCHEMA = "nando.s1c3f-state.v1"

ORIGINAL_JOURNAL_SNAPSHOT = base.journal_snapshot
ORIGINAL_VERIFY_CURRENT_PRODUCTION = base.verify_current_production
ORIGINAL_VERIFY_PARENT = base.verify_parent
ORIGINAL_PROVISION = base.provision_empty_directory
ORIGINAL_REQUIRE_OPEN = base.require_exact_empty_runtime_journal
ORIGINAL_REQUIRE_SURVIVAL = base.require_runtime_journal_shape
ORIGINAL_CLEANUP = base.cleanup_operational_empty_journal
ORIGINAL_VERIFY_PREDEPLOYMENT = base.verify_predeployment
ORIGINAL_ROLLBACK = base.rollback


def parse_framed_segment(path: Path, prefix: dict[str, Any] | None = None) -> dict[str, Any]:
    payload = path.read_bytes()
    if len(payload) < len(SEGMENT_MAGIC) or payload[:4] != SEGMENT_MAGIC:
        raise base.GateFailure(f"s1c3f_bad_magic:{path.name}")
    offset = 4
    records = 0
    while offset < len(payload):
        if len(payload) - offset < 12:
            raise base.GateFailure(f"s1c3f_partial_frame_header:{path.name}:{offset}")
        payload_bytes = struct.unpack_from("<I", payload, offset)[0]
        expected_digest = struct.unpack_from("<Q", payload, offset + 4)[0]
        if payload_bytes > MAX_FRAME_PAYLOAD_BYTES:
            raise base.GateFailure(f"s1c3f_frame_payload_budget:{path.name}:{payload_bytes}")
        frame_end = offset + 12 + payload_bytes
        if frame_end > len(payload):
            raise base.GateFailure(f"s1c3f_partial_frame_payload:{path.name}:{offset}")
        frame_payload = payload[offset + 12 : frame_end]
        actual_digest = int.from_bytes(hashlib.sha256(frame_payload).digest()[:8], "little")
        if actual_digest != expected_digest:
            raise base.GateFailure(f"s1c3f_frame_digest:{path.name}:{offset}")
        records += 1
        offset = frame_end
    result = {
        "record_count": records,
        "format_bytes": 4,
        "frame_bytes": len(payload) - 4,
        "tail_bytes": len(payload) - offset,
    }
    if prefix is not None:
        prefix_size = prefix.get("size_bytes")
        if type(prefix_size) is not int or prefix_size < 4 or prefix_size > len(payload):
            raise base.GateFailure(f"s1c3f_prefix_size:{path.name}")
        prefix_sha256 = hashlib.sha256(payload[:prefix_size]).hexdigest()
        result.update(
            {
                "prefix_size_bytes": prefix_size,
                "prefix_sha256": prefix_sha256,
                "prefix_preserved": prefix_sha256 == prefix.get("sha256"),
            }
        )
    return result


def journal_snapshot(prefix_reference: dict[str, Any] | None = None) -> dict[str, Any]:
    reference_rows = {
        row["path"]: row for row in (prefix_reference or {}).get("entries", [])
    }
    for _ in range(3):
        value = ORIGINAL_JOURNAL_SNAPSHOT()
        consistent = True
        for row in value.get("entries", []):
            path = base.JOURNAL / row["path"]
            parsed = parse_framed_segment(path, reference_rows.get(row["path"]))
            if path.stat().st_size != row.get("size_bytes") or base.sha256_file(path) != row.get("sha256"):
                consistent = False
                break
            row.update(parsed)
        if consistent:
            value["record_count"] = sum(
                row.get("record_count", 0) for row in value.get("entries", [])
            )
            return value
    raise base.GateFailure("s1c3f_journal_changed_during_snapshot")


def require_prefix_preserved(
    reference: dict[str, Any], current: dict[str, Any]
) -> None:
    reference_rows = {row["path"]: row for row in reference.get("entries", [])}
    current_rows = {row["path"]: row for row in current.get("entries", [])}
    if reference_rows.keys() != current_rows.keys():
        raise base.GateFailure("s1c3f_prefix_segment_set")
    for name, old in reference_rows.items():
        row = current_rows[name]
        if (
            row.get("prefix_size_bytes") != old.get("size_bytes")
            or row.get("prefix_sha256") != old.get("sha256")
            or row.get("prefix_preserved") is not True
        ):
            raise base.GateFailure(f"s1c3f_prefix_changed:{name}")


def require_magic_only_opening(snapshot: dict[str, Any]) -> None:
    ORIGINAL_REQUIRE_SURVIVAL(snapshot)
    if snapshot.get("record_count") != 0:
        raise base.GateFailure("s1c3f_precursor_records_present")
    if snapshot.get("manifest_root_sha256") != PARENT_JOURNAL_ROOT:
        raise base.GateFailure("s1c3f_opening_journal_root")
    for row in snapshot["entries"]:
        if (
            row.get("size_bytes") != 4
            or row.get("sha256") != MAGIC_ONLY_SHA256
            or row.get("format_bytes") != 4
            or row.get("frame_bytes") != 0
            or row.get("record_count") != 0
            or row.get("tail_bytes") != 0
        ):
            raise base.GateFailure(f's1c3f_opening_segment_not_magic_only:{row.get("path")}')


def require_valid_survival(snapshot: dict[str, Any]) -> None:
    ORIGINAL_REQUIRE_SURVIVAL(snapshot)
    for row in snapshot["entries"]:
        if row.get("format_bytes") != 4 or row.get("tail_bytes") != 0:
            raise base.GateFailure(f's1c3f_survival_segment_invalid:{row.get("path")}')


def verify_parent() -> dict[str, Any]:
    if not PARENT_DIRECTORY.is_dir():
        raise base.GateFailure("s1c3f_parent_missing")
    for name, expected in PARENT_FILE_SHA256.items():
        actual = base.sha256_file(PARENT_DIRECTORY / name)
        if actual != expected:
            raise base.GateFailure(f"s1c3f_parent_file:{name}:{actual}")
    state = base.read_json(PARENT_DIRECTORY / "s1c3e-state.json")
    receipt = base.read_json(PARENT_DIRECTORY / "deployment-receipt.json")
    final = base.read_json(PARENT_DIRECTORY / "final-verification.json")
    checks = (
        (state.get("verdict"), "S1C3E_ROLLBACK_PASS", "verdict"),
        (state.get("state_root_sha256"), PARENT_STATE_ROOT, "state_root"),
        (receipt.get("receipt_root_sha256"), PARENT_RECEIPT_ROOT, "receipt_root"),
        (final.get("final_verification_root_sha256"), PARENT_FINAL_ROOT, "final_root"),
        (receipt.get("journal_after", {}).get("manifest_root_sha256"), PARENT_JOURNAL_ROOT, "journal_root"),
        (receipt.get("installed_binary_sha256"), base.BASELINE_BINARY_SHA256, "baseline_binary"),
        (receipt.get("installed_config_sha256"), base.BASELINE_CONFIG_SHA256, "baseline_config"),
    )
    for actual, expected, label in checks:
        if actual != expected:
            raise base.GateFailure(f"s1c3f_parent_{label}:{actual}")
    return {
        "transaction_id": PARENT_TRANSACTION_ID,
        "state_root_sha256": PARENT_STATE_ROOT,
        "receipt_root_sha256": PARENT_RECEIPT_ROOT,
        "final_verification_root_sha256": PARENT_FINAL_ROOT,
        "journal_root_sha256": PARENT_JOURNAL_ROOT,
        "s1c3d_resource_root_sha256": base.PARENT_RESOURCE_ROOT,
        "s1c3d_parity_root_sha256": base.PARENT_PARITY_ROOT,
        "optimization_status": "OPTIMIZATION_WATCH",
    }


def verify_current_production() -> None:
    if base.sha256_file(base.PRODUCTION_BINARY) != base.BASELINE_BINARY_SHA256:
        raise base.GateFailure("STALE_BEFORE_MUTATION:baseline_binary")
    if base.sha256_file(base.PRODUCTION_CONFIG) != base.BASELINE_CONFIG_SHA256:
        raise base.GateFailure("STALE_BEFORE_MUTATION:baseline_config")
    require_magic_only_opening(journal_snapshot())


def verify_existing_directory() -> dict[str, Any]:
    value = journal_snapshot()
    require_magic_only_opening(value)
    return value


def preserve_journal() -> bool:
    require_valid_survival(journal_snapshot())
    return False


def read_only_route_receipt() -> dict[str, Any]:
    health = base.health_snapshot()
    return {
        label: {"url": row["url"], "semantic": row["semantic"]}
        for label, row in sorted(health.items())
    }


def verify_predeployment(root: Path, path: Path) -> dict[str, Any]:
    value = base.read_json(path)
    preparation = base.read_json(root / "preparation.json")
    if value.get("predeployment_verification_root_sha256") != base.sha256_bytes(
        base.canonical_bytes(value, "predeployment_verification_root_sha256")
    ):
        raise base.GateFailure("s1c3f_predeployment_root")
    expected = {
        "schema": PREDEPLOYMENT_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": "S1C3F_PREPARATION_PASS_WITH_OPTIMIZATION_WATCH",
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "parent_state_root_sha256": PARENT_STATE_ROOT,
        "parent_receipt_root_sha256": PARENT_RECEIPT_ROOT,
        "parent_final_root_sha256": PARENT_FINAL_ROOT,
        "opening_journal_root_sha256": PARENT_JOURNAL_ROOT,
    }
    if {key: value.get(key) for key in expected} != expected:
        raise base.GateFailure("s1c3f_predeployment_mismatch")
    return value


def patch_mechanism() -> None:
    base.PAPER_COMMIT = PAPER_COMMIT
    base.PAPER_TREE = PAPER_TREE
    base.PAPER_PREREGISTRATION_SHA256 = PAPER_PREREGISTRATION_SHA256
    base.PAPER_CRITIQUE_SHA256 = PAPER_CRITIQUE_SHA256
    base.PAPER_MANIFEST_ROOT = PAPER_MANIFEST_ROOT
    base.PREPARATION_SCHEMA = PREPARATION_SCHEMA
    base.PREDEPLOYMENT_SCHEMA = PREDEPLOYMENT_SCHEMA
    base.PENDING_SCHEMA = PENDING_SCHEMA
    base.RECEIPT_SCHEMA = RECEIPT_SCHEMA
    base.STATE_SCHEMA = STATE_SCHEMA
    base.verify_parent = verify_parent
    base.verify_current_production = verify_current_production
    base.journal_snapshot = journal_snapshot
    base.provision_empty_directory = verify_existing_directory
    base.require_exact_empty_runtime_journal = require_magic_only_opening
    base.require_runtime_journal_shape = require_valid_survival
    base.cleanup_operational_empty_journal = preserve_journal
    base.verify_predeployment = verify_predeployment
    base.route_probe = read_only_route_receipt


def prepare(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    output = io.StringIO()
    try:
        with contextlib.redirect_stdout(output):
            result = base.prepare(args)
        parent_evidence = root / "s1c3f-parent-evidence"
        parent_evidence.mkdir(mode=0o700)
        for name in PARENT_FILE_SHA256:
            destination = parent_evidence / name
            shutil.copy2(PARENT_DIRECTORY / name, destination)
            os.chmod(destination, 0o400)
            descriptor = os.open(destination, os.O_RDONLY)
            try:
                os.fsync(descriptor)
            finally:
                os.close(descriptor)
        base.fsync_directory(parent_evidence)
        base.fsync_directory(root)
    except Exception as error:
        if root.is_dir():
            base.write_json(
                root / "preflight-failure.json",
                {"schema": "nando.s1c3f-preflight-failure.v1", "error": str(error)},
                0o400,
            )
            base.write_json(
                root / "transaction-state.json",
                {
                    "schema": STATE_SCHEMA,
                    "state": "PREFLIGHT_FAILURE",
                    "transaction_id": args.transaction_id,
                },
                0o600,
            )
            base.fsync_directory(root)
        raise
    print(output.getvalue().strip())
    return result


def _rewrite_pending(root: Path, *, rollback: bool) -> None:
    pending = base.read_json(root / "pending-receipt.json")
    pending["schema"] = PENDING_SCHEMA
    if rollback:
        pending["verdict"] = "S1C3F_ROLLBACK_PASS"
    else:
        pending["verdict"] = "S1C3F_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH"
    pending["journal_opening"] = pending.get("journal_after")
    base.write_json(root / "pending-receipt.json", pending, 0o600)


def execute(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    try:
        result = base.execute(args)
    except Exception as error:
        if (root / "pending-receipt.json").is_file():
            _rewrite_pending(root, rollback=True)
        raise base.GateFailure(str(error).replace("S1C3E_", "S1C3F_")) from error
    pending = base.read_json(root / "pending-receipt.json")
    survival = journal_snapshot(pending["journal_after"])
    require_prefix_preserved(pending["journal_after"], survival)
    pending["journal_survival"] = survival
    base.write_json(root / "pending-receipt.json", pending, 0o600)
    _rewrite_pending(root, rollback=False)
    return result


def rollback(root: Path, reason: str) -> None:
    preparation = base.read_json(root / "preparation.json")
    journal_forward = journal_snapshot(preparation["journal_before"])
    ORIGINAL_ROLLBACK(root, reason)
    journal_after = journal_snapshot(journal_forward)
    require_prefix_preserved(journal_forward, journal_after)
    pending = base.read_json(root / "pending-receipt.json")
    pending["journal_forward"] = journal_forward
    pending["journal_after"] = journal_after
    base.write_json(root / "pending-receipt.json", pending, 0o600)
    _rewrite_pending(root, rollback=True)


patch_mechanism()
base.rollback = rollback


def finalize(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    result = base.finalize(args)
    receipt = base.read_json(root / "deployment-receipt.json")
    pending = base.read_json(root / "pending-receipt.json")
    receipt["schema"] = RECEIPT_SCHEMA
    receipt["verdict"] = str(receipt["verdict"]).replace("S1C3E_", "S1C3F_")
    if "journal_forward" in pending:
        receipt["journal_forward"] = pending["journal_forward"]
    receipt["receipt_root_sha256"] = base.sha256_bytes(
        base.canonical_bytes(receipt, "receipt_root_sha256")
    )
    base.write_json(root / "deployment-receipt.json", receipt, 0o400)
    return result


def _promote_state_namespace(root: Path, verdict: str | None = None) -> None:
    old = root / "s1c3e-state.json"
    state = base.read_json(old)
    state["schema"] = STATE_SCHEMA
    state["verdict"] = verdict or str(state["verdict"]).replace("S1C3E_", "S1C3F_")
    state["state_root_sha256"] = base.sha256_bytes(
        base.canonical_bytes(state, "state_root_sha256")
    )
    base.write_json(root / "s1c3f-state.json", state, 0o400)
    old.unlink()
    base.fsync_directory(root)


def seal(args: argparse.Namespace) -> int:
    result = base.seal(args)
    root = Path(args.transaction_directory)
    _promote_state_namespace(root)
    return result


def abort_predeployment(args: argparse.Namespace) -> int:
    result = base.abort_predeployment(args)
    root = Path(args.transaction_directory)
    _promote_state_namespace(root, "S1C3F_PREFLIGHT_FAILURE")
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    prepare_parser = commands.add_parser("prepare")
    prepare_parser.add_argument("--transaction-id", required=True)
    prepare_parser.add_argument("--transaction-directory", required=True)
    prepare_parser.add_argument("--connector-before", required=True)
    prepare_parser.add_argument("--implementation-freeze", required=True)
    execute_parser = commands.add_parser("execute")
    execute_parser.add_argument("--transaction-directory", required=True)
    execute_parser.add_argument("--predeployment-verification", required=True)
    rollback_parser = commands.add_parser("rollback")
    rollback_parser.add_argument("--transaction-directory", required=True)
    rollback_parser.add_argument("--reason", required=True)
    finalize_parser = commands.add_parser("finalize")
    finalize_parser.add_argument("--transaction-directory", required=True)
    finalize_parser.add_argument("--connector-after", required=True)
    seal_parser = commands.add_parser("seal")
    seal_parser.add_argument("--transaction-directory", required=True)
    seal_parser.add_argument("--final-verification", required=True)
    abort_parser = commands.add_parser("abort-predeployment")
    abort_parser.add_argument("--transaction-directory", required=True)
    abort_parser.add_argument("--reason", required=True)
    args = parser.parse_args()
    if args.command == "prepare":
        return prepare(args)
    root = Path(args.transaction_directory)
    descriptor = os.open(root / ".mutation.lock", os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX)
        if args.command == "execute":
            return execute(args)
        if args.command == "rollback":
            rollback(root, args.reason)
            return 0
        if args.command == "finalize":
            return finalize(args)
        if args.command == "seal":
            return seal(args)
        return abort_predeployment(args)
    finally:
        os.close(descriptor)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(json.dumps({"schema": "nando.s1c3f-error.v1", "error": str(error)}, sort_keys=True), file=os.sys.stderr)
        raise SystemExit(1)
