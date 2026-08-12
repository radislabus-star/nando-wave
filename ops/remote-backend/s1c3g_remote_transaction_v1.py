#!/usr/bin/env python3
"""Endpoint-owned stable-health repair over the frozen S1C-3F mechanism."""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import io
import json
import os
import shutil
import time
from pathlib import Path
from typing import Any

import s1c3f_remote_transaction_v1 as parent


base = parent.base

PAPER_COMMIT = "cb273f4c56f3f150730c725d0971adfe850c7eb6"
PAPER_TREE = "ea0d8a01d855242509b27e0978149c645f304bef"
PAPER_PREREGISTRATION_SHA256 = "52891bdcf641ab31e19985dc02f9b60fe6bb12e8a0661a9b732855f641e8f462"
PAPER_CRITIQUE_SHA256 = "14c852e57fff62cba88bac835cef1c4691a6f525a53c4fcbec2a9188ddf01527"
PAPER_MANIFEST_ROOT = "1b80da666bb4d6f39b78b5395653efac8a34b9607aa257270af5a5f094ec50ce"
PREFLIGHT_COMMIT = "8e9dabcd5d5cb1b7e915bfa9fcfd057dd4e6a902"
PREFLIGHT_MANIFEST_SHA256 = "4a93944951629649602aa8889867babacea8eda2ac624f7567d3028cdbab26fd"
PREFLIGHT_RECEIPT_SHA256 = "52dfc968663d1768d663cdedd39b6e51a7fc85f095bd59aa8248af00a48d0650"

PARENT_TRANSACTION_ID = "20260812T163201Z-55376ab7f5fa-s1c3f-v1"
PARENT_DIRECTORY = Path("/var/lib/nando-wave/deployments") / PARENT_TRANSACTION_ID
PARENT_STATE_ROOT = "e98b72cac96a14049dc64c728fccba1609a0f2a2f2bec1c744036c19f8afd403"
PARENT_RECEIPT_ROOT = "b19c831e563f715063c2ae026a589e7f1651ab93c05b90390215529eb297a8cf"
PARENT_FINAL_ROOT = "6c64cbf4399fd7f12dfcf808ed2248a8674e1540a80cf28bbab78fca7337e7bf"
PARENT_JOURNAL_ROOT = parent.PARENT_JOURNAL_ROOT
PARENT_FILE_SHA256 = {
    "s1c3f-state.json": "177323da60f283d21cd82dc04bac4086d811677468054a29d14e44b0f678849e",
    "deployment-receipt.json": "f01e20fd927ab93c97119ac7de5ece3822b76f6050ebd60ab4f4b1713f5fed14",
    "final-verification.json": "1a3def2fac50a57c8fc317bdee33f0e86ce5afb8a05954330dac783b02222683",
}

PREPARATION_SCHEMA = "nando.s1c3g-preparation.v1"
PREDEPLOYMENT_SCHEMA = "nando.s1c3g-predeployment-verification.v1"
PENDING_SCHEMA = "nando.s1c3g-pending-receipt.v1"
RECEIPT_SCHEMA = "nando.s1c3g-deployment-receipt.v1"
STATE_SCHEMA = "nando.s1c3g-state.v1"
PROJECTION_SCHEMA = "nando.s1c3g-stable-health-projection.v1"

ENDPOINT_CONTRACT = {
    "control": {
        "url": "http://127.0.0.1:18788/health",
        "stable_fields": ("ok", "service", "mode"),
    },
    "cpu": {
        "url": "http://192.168.3.94:8787/cpu-health",
        "stable_fields": (
            "ok",
            "service",
            "mode",
            "admission_verdict",
            "response_executor_cache_ready",
            "response_active_profiles",
        ),
    },
    "gateway": {
        "url": "http://192.168.3.94:8787/health",
        "stable_fields": ("ok", "service"),
    },
    "hot": {
        "url": "http://127.0.0.1:18789/health",
        "stable_fields": (
            "ok",
            "service",
            "mode",
            "admission_verdict",
            "response_executor_cache_ready",
            "response_active_profiles",
        ),
    },
}

PARENT_ROLLBACK = parent.rollback
HEALTH_HTTP_JSON = base.base.legacy.http_json


def projection_contract() -> dict[str, Any]:
    value = {
        "schema": PROJECTION_SCHEMA,
        "endpoints": {
            label: {
                "url": contract["url"],
                "stable_fields": list(contract["stable_fields"]),
            }
            for label, contract in sorted(ENDPOINT_CONTRACT.items())
        },
        "cross_endpoint_equalities": [{"left": "hot", "right": "cpu"}],
        "whole_object_equality": False,
    }
    value["projection_root_sha256"] = base.sha256_bytes(base.canonical_bytes(value))
    return value


PROJECTION_ROOT = projection_contract()["projection_root_sha256"]
AUTHORITY_RENEWAL_TIMEOUT_SECONDS = 50.0
AUTHORITY_RENEWAL_POLL_SECONDS = 0.5


def stable_health_projection(snapshot: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(snapshot, dict) or set(snapshot) != set(ENDPOINT_CONTRACT):
        raise base.GateFailure("s1c3g_endpoint_set")
    projected: dict[str, Any] = {}
    for label, contract in sorted(ENDPOINT_CONTRACT.items()):
        row = snapshot.get(label)
        if not isinstance(row, dict) or row.get("url") != contract["url"]:
            raise base.GateFailure(f"s1c3g_endpoint_url:{label}")
        semantic = row.get("semantic")
        if not isinstance(semantic, dict):
            raise base.GateFailure(f"s1c3g_endpoint_semantic:{label}")
        missing = [field for field in contract["stable_fields"] if field not in semantic]
        if missing:
            raise base.GateFailure(f"s1c3g_stable_field_missing:{label}:{','.join(missing)}")
        if semantic.get("ok") is not True:
            raise base.GateFailure(f"s1c3g_endpoint_not_ok:{label}")
        projected[label] = {
            "url": contract["url"],
            "stable": {field: semantic[field] for field in contract["stable_fields"]},
        }
    if projected["hot"]["stable"] != projected["cpu"]["stable"]:
        raise base.GateFailure("s1c3g_hot_cpu_projection_mismatch")
    return projected


def semantic_health_equal(before: dict[str, Any], after: dict[str, Any]) -> bool:
    return stable_health_projection(before) == stable_health_projection(after)


def read_only_route_receipt() -> dict[str, Any]:
    return stable_health_projection(base.health_snapshot())


def authority_lease_observation() -> dict[str, Any]:
    value = HEALTH_HTTP_JSON(ENDPOINT_CONTRACT["hot"]["url"])
    expires_at = value.get("response_admission_expires_at_unix")
    if type(expires_at) is not int or expires_at <= 0:
        raise base.GateFailure("s1c3g_authority_expiry_invalid")
    expected = {
        "admission_verdict": "PASS",
        "response_executor_cache_ready": True,
        "response_active_profiles": 2,
    }
    actual = {field: value.get(field) for field in expected}
    if actual != expected:
        raise base.GateFailure(f"s1c3g_authority_not_ready:{actual}")
    return {
        "endpoint": ENDPOINT_CONTRACT["hot"]["url"],
        "expires_at_unix": expires_at,
        **actual,
    }


def wait_for_authority_renewal(
    expected_projection: dict[str, Any],
    *,
    timeout: float = AUTHORITY_RENEWAL_TIMEOUT_SECONDS,
) -> dict[str, Any]:
    before = authority_lease_observation()
    started = time.monotonic()
    deadline = started + timeout
    while time.monotonic() < deadline:
        time.sleep(AUTHORITY_RENEWAL_POLL_SECONDS)
        after = authority_lease_observation()
        if after["expires_at_unix"] <= before["expires_at_unix"]:
            continue
        observed_projection = stable_health_projection(base.health_snapshot())
        if observed_projection != expected_projection:
            raise base.GateFailure("s1c3g_authority_renewal_health_changed")
        return {
            "schema": "nando.s1c3g-authority-renewal-receipt.v1",
            "before": before,
            "after": after,
            "advanced_seconds": after["expires_at_unix"] - before["expires_at_unix"],
            "observation_seconds": round(time.monotonic() - started, 6),
            "stable_health_projection_root_sha256": PROJECTION_ROOT,
            "stable_health_preserved": True,
        }
    raise base.GateFailure("s1c3g_authority_renewal_timeout")


def verify_parent() -> dict[str, Any]:
    if not PARENT_DIRECTORY.is_dir():
        raise base.GateFailure("s1c3g_parent_missing")
    for name, expected in PARENT_FILE_SHA256.items():
        actual = base.sha256_file(PARENT_DIRECTORY / name)
        if actual != expected:
            raise base.GateFailure(f"s1c3g_parent_file:{name}:{actual}")
    state = base.read_json(PARENT_DIRECTORY / "s1c3f-state.json")
    receipt = base.read_json(PARENT_DIRECTORY / "deployment-receipt.json")
    final = base.read_json(PARENT_DIRECTORY / "final-verification.json")
    checks = (
        (state.get("verdict"), "S1C3F_ROLLBACK_PASS", "verdict"),
        (state.get("state_root_sha256"), PARENT_STATE_ROOT, "state_root"),
        (receipt.get("receipt_root_sha256"), PARENT_RECEIPT_ROOT, "receipt_root"),
        (final.get("final_verification_root_sha256"), PARENT_FINAL_ROOT, "final_root"),
        (receipt.get("journal_after", {}).get("manifest_root_sha256"), PARENT_JOURNAL_ROOT, "journal_root"),
        (receipt.get("installed_binary_sha256"), base.BASELINE_BINARY_SHA256, "baseline_binary"),
        (receipt.get("installed_config_sha256"), base.BASELINE_CONFIG_SHA256, "baseline_config"),
    )
    for actual, expected, label in checks:
        if actual != expected:
            raise base.GateFailure(f"s1c3g_parent_{label}:{actual}")
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


def verify_predeployment(root: Path, path: Path) -> dict[str, Any]:
    value = base.read_json(path)
    preparation = base.read_json(root / "preparation.json")
    if value.get("predeployment_verification_root_sha256") != base.sha256_bytes(
        base.canonical_bytes(value, "predeployment_verification_root_sha256")
    ):
        raise base.GateFailure("s1c3g_predeployment_root")
    expected = {
        "schema": PREDEPLOYMENT_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": "S1C3G_PREPARATION_PASS_WITH_OPTIMIZATION_WATCH",
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "parent_state_root_sha256": PARENT_STATE_ROOT,
        "parent_receipt_root_sha256": PARENT_RECEIPT_ROOT,
        "parent_final_root_sha256": PARENT_FINAL_ROOT,
        "opening_journal_root_sha256": PARENT_JOURNAL_ROOT,
        "stable_health_projection_root_sha256": PROJECTION_ROOT,
    }
    if {key: value.get(key) for key in expected} != expected:
        raise base.GateFailure("s1c3g_predeployment_mismatch")
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
    base.verify_predeployment = verify_predeployment
    base.route_probe = read_only_route_receipt
    base.semantic_health_equal = semantic_health_equal


def _terminal_preflight_failure(root: Path, transaction_id: str, error: Exception) -> None:
    base.write_json(
        root / "preflight-failure.json",
        {"schema": "nando.s1c3g-preflight-failure.v1", "error": str(error)},
        0o400,
    )
    base.write_json(
        root / "transaction-state.json",
        {"schema": STATE_SCHEMA, "state": "PREFLIGHT_FAILURE", "transaction_id": transaction_id},
        0o600,
    )
    base.fsync_directory(root)


def prepare(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    output = io.StringIO()
    try:
        with contextlib.redirect_stdout(output):
            base.prepare(args)
        parent_evidence = root / "s1c3g-parent-evidence"
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
        preparation = base.read_json(root / "preparation.json")
        preparation["implementation_preflight"] = {
            "commit": PREFLIGHT_COMMIT,
            "manifest_sha256": PREFLIGHT_MANIFEST_SHA256,
            "receipt_sha256": PREFLIGHT_RECEIPT_SHA256,
            "verdict": "READY_TO_IMPLEMENT",
            "blockers": 0,
        }
        preparation["stable_health_projection"] = projection_contract()
        preparation["preparation_root_sha256"] = base.sha256_bytes(
            base.canonical_bytes(preparation, "preparation_root_sha256")
        )
        base.write_json(root / "preparation.json", preparation, 0o400)
        base.fsync_directory(root)
    except Exception as error:
        if root.is_dir():
            _terminal_preflight_failure(root, args.transaction_id, error)
        raise
    print(
        json.dumps(
            {
                "state": "PREPARED",
                "transaction_directory": str(root),
                "preparation_root_sha256": preparation["preparation_root_sha256"],
            },
            sort_keys=True,
        )
    )
    return 0


def _rewrite_pending(root: Path, *, rollback: bool) -> None:
    pending = base.read_json(root / "pending-receipt.json")
    pending["schema"] = PENDING_SCHEMA
    pending["verdict"] = (
        "S1C3G_ROLLBACK_PASS" if rollback else "S1C3G_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH"
    )
    pending["stable_health_projection_root_sha256"] = PROJECTION_ROOT
    pending["route_probe_equivalent"] = (
        pending.get("route_probe_after") == base.read_json(root / "preparation.json").get("route_probe_before")
        and pending.get("route_probe_survival") == base.read_json(root / "preparation.json").get("route_probe_before")
    )
    base.write_json(root / "pending-receipt.json", pending, 0o600)


def execute(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    try:
        result = base.execute(args)
        pending = base.read_json(root / "pending-receipt.json")
        preparation = base.read_json(root / "preparation.json")
        expected_projection = stable_health_projection(preparation["health_before"])
        pending["authority_renewal"] = wait_for_authority_renewal(expected_projection)
        survival = parent.journal_snapshot(pending["journal_after"])
        parent.require_prefix_preserved(pending["journal_after"], survival)
        pending["journal_survival"] = survival
        base.write_json(root / "pending-receipt.json", pending, 0o600)
        _rewrite_pending(root, rollback=False)
        return result
    except Exception as error:
        state_path = root / "transaction-state.json"
        if state_path.is_file() and base.read_json(state_path).get("state") == "FINALIZE_PENDING":
            rollback(root, f"authority_renewal:{error}")
        if (root / "pending-receipt.json").is_file():
            _rewrite_pending(root, rollback=True)
        message = str(error).replace("S1C3E_", "S1C3G_").replace("S1C3F_", "S1C3G_")
        raise base.GateFailure(message) from error


def rollback(root: Path, reason: str) -> None:
    PARENT_ROLLBACK(root, reason)
    _rewrite_pending(root, rollback=True)


def finalize(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    result = base.finalize(args)
    receipt = base.read_json(root / "deployment-receipt.json")
    pending = base.read_json(root / "pending-receipt.json")
    receipt["schema"] = RECEIPT_SCHEMA
    receipt["verdict"] = str(receipt["verdict"]).replace("S1C3E_", "S1C3G_").replace("S1C3F_", "S1C3G_")
    receipt["stable_health_projection_root_sha256"] = PROJECTION_ROOT
    receipt["route_probe_equivalent"] = pending["route_probe_equivalent"]
    if "authority_renewal" in pending:
        receipt["authority_renewal"] = pending["authority_renewal"]
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
    state["verdict"] = verdict or str(state["verdict"]).replace("S1C3E_", "S1C3G_").replace("S1C3F_", "S1C3G_")
    state["state_root_sha256"] = base.sha256_bytes(
        base.canonical_bytes(state, "state_root_sha256")
    )
    base.write_json(root / "s1c3g-state.json", state, 0o400)
    old.unlink()
    base.fsync_directory(root)


def seal(args: argparse.Namespace) -> int:
    result = base.seal(args)
    _promote_state_namespace(Path(args.transaction_directory))
    return result


def abort_predeployment(args: argparse.Namespace) -> int:
    result = base.abort_predeployment(args)
    _promote_state_namespace(Path(args.transaction_directory), "S1C3G_PREFLIGHT_FAILURE")
    return result


patch_mechanism()
base.rollback = rollback


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
        print(json.dumps({"schema": "nando.s1c3g-error.v1", "error": str(error)}, sort_keys=True), file=os.sys.stderr)
        raise SystemExit(1)
