#!/usr/bin/env python3
"""Independent verifier for the S1C-3G stable-health installation attempt."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any

import verify_s1c3f_transaction_v1 as parent


PAPER_COMMIT = "cb273f4c56f3f150730c725d0971adfe850c7eb6"
PAPER_TREE = "ea0d8a01d855242509b27e0978149c645f304bef"
PAPER_PREREGISTRATION_SHA256 = "52891bdcf641ab31e19985dc02f9b60fe6bb12e8a0661a9b732855f641e8f462"
PAPER_CRITIQUE_SHA256 = "14c852e57fff62cba88bac835cef1c4691a6f525a53c4fcbec2a9188ddf01527"
PAPER_MANIFEST_ROOT = "1b80da666bb4d6f39b78b5395653efac8a34b9607aa257270af5a5f094ec50ce"
PREFLIGHT_COMMIT = "8e9dabcd5d5cb1b7e915bfa9fcfd057dd4e6a902"
PREFLIGHT_MANIFEST_SHA256 = "4a93944951629649602aa8889867babacea8eda2ac624f7567d3028cdbab26fd"
PREFLIGHT_RECEIPT_SHA256 = "52dfc968663d1768d663cdedd39b6e51a7fc85f095bd59aa8248af00a48d0650"

PARENT_TRANSACTION_ID = "20260812T163201Z-55376ab7f5fa-s1c3f-v1"
PARENT_STATE_ROOT = "e98b72cac96a14049dc64c728fccba1609a0f2a2f2bec1c744036c19f8afd403"
PARENT_RECEIPT_ROOT = "b19c831e563f715063c2ae026a589e7f1651ab93c05b90390215529eb297a8cf"
PARENT_FINAL_ROOT = "6c64cbf4399fd7f12dfcf808ed2248a8674e1540a80cf28bbab78fca7337e7bf"
PARENT_JOURNAL_ROOT = parent.PARENT_JOURNAL_ROOT
PARENT_FILES = {
    "s1c3f-state.json": "177323da60f283d21cd82dc04bac4086d811677468054a29d14e44b0f678849e",
    "deployment-receipt.json": "f01e20fd927ab93c97119ac7de5ece3822b76f6050ebd60ab4f4b1713f5fed14",
    "final-verification.json": "1a3def2fac50a57c8fc317bdee33f0e86ce5afb8a05954330dac783b02222683",
}

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

ATTEMPT_RE = re.compile(r"^\d{8}T\d{6}Z-[0-9a-f]{12}-s1c3g-v1$")
IMPLEMENTATION_FILES = (
    "s1c3g_remote_transaction_v1.py",
    "verify_s1c3g_transaction_v1.py",
    "test_s1c3g_transaction_v1.py",
    "run_s1c3g_transaction_v1.sh",
    "s1c3f_remote_transaction_v1.py",
    "verify_s1c3f_transaction_v1.py",
    "s1c3e_remote_transaction_v1.py",
    "s1c3b_remote_transaction_v1.py",
    "s1c3_remote_transaction_v7.py",
)

InvalidReceipt = parent.InvalidReceipt
canonical_bytes = parent.canonical_bytes
digest = parent.digest
file_digest = parent.file_digest
load_json = parent.load_json
require = parent.require
exact = parent.exact
verify_root = parent.verify_root


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
    value["projection_root_sha256"] = digest(canonical_bytes(value))
    return value


PROJECTION_ROOT = projection_contract()["projection_root_sha256"]


def stable_health_projection(snapshot: Any, label: str) -> dict[str, Any]:
    require(isinstance(snapshot, dict), f"{label}_object")
    exact(set(snapshot), set(ENDPOINT_CONTRACT), f"{label}_endpoint_set")
    projected: dict[str, Any] = {}
    for endpoint, contract in sorted(ENDPOINT_CONTRACT.items()):
        row = snapshot.get(endpoint)
        require(isinstance(row, dict), f"{label}_{endpoint}_row")
        exact(row.get("url"), contract["url"], f"{label}_{endpoint}_url")
        semantic = row.get("semantic")
        require(isinstance(semantic, dict), f"{label}_{endpoint}_semantic")
        for field in contract["stable_fields"]:
            require(field in semantic, f"{label}_{endpoint}_{field}_missing")
        exact(semantic.get("ok"), True, f"{label}_{endpoint}_ok")
        projected[endpoint] = {
            "url": contract["url"],
            "stable": {field: semantic[field] for field in contract["stable_fields"]},
        }
    exact(projected["hot"]["stable"], projected["cpu"]["stable"], f"{label}_hot_cpu")
    return projected


def verify_stable_receipt(value: Any, label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label}_object")
    exact(set(value), set(ENDPOINT_CONTRACT), f"{label}_endpoint_set")
    for endpoint, contract in sorted(ENDPOINT_CONTRACT.items()):
        row = value.get(endpoint)
        require(isinstance(row, dict), f"{label}_{endpoint}_row")
        exact(set(row), {"url", "stable"}, f"{label}_{endpoint}_fields")
        exact(row.get("url"), contract["url"], f"{label}_{endpoint}_url")
        stable = row.get("stable")
        require(isinstance(stable, dict), f"{label}_{endpoint}_stable")
        exact(set(stable), set(contract["stable_fields"]), f"{label}_{endpoint}_stable_fields")
        exact(stable.get("ok"), True, f"{label}_{endpoint}_ok")
    exact(value["hot"]["stable"], value["cpu"]["stable"], f"{label}_hot_cpu")
    return value


def verify_authority_renewal(value: Any) -> dict[str, Any]:
    require(isinstance(value, dict), "authority_renewal_object")
    exact(
        set(value),
        {
            "schema",
            "before",
            "after",
            "advanced_seconds",
            "observation_seconds",
            "stable_health_projection_root_sha256",
            "stable_health_preserved",
        },
        "authority_renewal_fields",
    )
    exact(value.get("schema"), "nando.s1c3g-authority-renewal-receipt.v1", "authority_renewal_schema")
    exact(value.get("stable_health_projection_root_sha256"), PROJECTION_ROOT, "authority_renewal_projection")
    exact(value.get("stable_health_preserved"), True, "authority_renewal_health")
    observations = []
    for label in ("before", "after"):
        observation = value.get(label)
        require(isinstance(observation, dict), f"authority_{label}_object")
        exact(
            set(observation),
            {
                "endpoint",
                "expires_at_unix",
                "admission_verdict",
                "response_executor_cache_ready",
                "response_active_profiles",
            },
            f"authority_{label}_fields",
        )
        exact(observation.get("endpoint"), ENDPOINT_CONTRACT["hot"]["url"], f"authority_{label}_endpoint")
        exact(observation.get("admission_verdict"), "PASS", f"authority_{label}_admission")
        exact(observation.get("response_executor_cache_ready"), True, f"authority_{label}_cache")
        exact(observation.get("response_active_profiles"), 2, f"authority_{label}_profiles")
        expires_at = observation.get("expires_at_unix")
        require(type(expires_at) is int and expires_at > 0, f"authority_{label}_expiry")
        observations.append(expires_at)
    advanced = value.get("advanced_seconds")
    require(type(advanced) is int and advanced > 0, "authority_advanced")
    exact(advanced, observations[1] - observations[0], "authority_advanced_exact")
    duration = value.get("observation_seconds")
    require(type(duration) in {int, float} and duration >= 0, "authority_duration")
    return value


def create_freeze(source_commit: str, source_tree: str, directory: Path) -> dict[str, Any]:
    require(re.fullmatch(r"[0-9a-f]{40}", source_commit) is not None, "source_commit")
    require(re.fullmatch(r"[0-9a-f]{40}", source_tree) is not None, "source_tree")
    files = []
    for name in IMPLEMENTATION_FILES:
        path = directory / name
        require(path.is_file(), f"implementation_missing:{name}")
        files.append({"path": name, "sha256": file_digest(path), "size_bytes": path.stat().st_size})
    value = {
        "schema": "nando.s1c3g-implementation-freeze.v1",
        "source_commit": source_commit,
        "source_tree": source_tree,
        "paper_commit": PAPER_COMMIT,
        "preflight_commit": PREFLIGHT_COMMIT,
        "stable_health_projection_root_sha256": PROJECTION_ROOT,
        "files": files,
    }
    value["implementation_freeze_root_sha256"] = digest(canonical_bytes(value))
    return value


def verify_freeze(path: Path, directory: Path) -> dict[str, Any]:
    value = load_json(path)
    exact(value.get("schema"), "nando.s1c3g-implementation-freeze.v1", "freeze_schema")
    verify_root(value, "implementation_freeze_root_sha256", "freeze_root")
    exact(value.get("paper_commit"), PAPER_COMMIT, "freeze_paper")
    exact(value.get("preflight_commit"), PREFLIGHT_COMMIT, "freeze_preflight")
    exact(value.get("stable_health_projection_root_sha256"), PROJECTION_ROOT, "freeze_projection")
    expected = []
    for name in IMPLEMENTATION_FILES:
        local = directory / name
        require(local.is_file(), f"freeze_local:{name}")
        expected.append({"path": name, "sha256": file_digest(local), "size_bytes": local.stat().st_size})
    exact(value.get("files"), expected, "freeze_files")
    return value


def verify_parent(directory: Path) -> None:
    root = directory / "s1c3g-parent-evidence"
    for name, expected in PARENT_FILES.items():
        exact(file_digest(root / name), expected, f"parent_file_{name}")
    state = load_json(root / "s1c3f-state.json")
    receipt = load_json(root / "deployment-receipt.json")
    final = load_json(root / "final-verification.json")
    exact(verify_root(state, "state_root_sha256", "parent_state"), PARENT_STATE_ROOT, "parent_state_exact")
    exact(verify_root(receipt, "receipt_root_sha256", "parent_receipt"), PARENT_RECEIPT_ROOT, "parent_receipt_exact")
    exact(verify_root(final, "final_verification_root_sha256", "parent_final"), PARENT_FINAL_ROOT, "parent_final_exact")
    exact(state.get("verdict"), "S1C3F_ROLLBACK_PASS", "parent_verdict")
    exact(receipt.get("journal_after", {}).get("manifest_root_sha256"), PARENT_JOURNAL_ROOT, "parent_journal")


def expected_parent() -> dict[str, Any]:
    return {
        "transaction_id": PARENT_TRANSACTION_ID,
        "state_root_sha256": PARENT_STATE_ROOT,
        "receipt_root_sha256": PARENT_RECEIPT_ROOT,
        "final_verification_root_sha256": PARENT_FINAL_ROOT,
        "journal_root_sha256": PARENT_JOURNAL_ROOT,
        "s1c3d_resource_root_sha256": parent.S1C3D_RESOURCE_ROOT,
        "s1c3d_parity_root_sha256": parent.S1C3D_PARITY_ROOT,
        "optimization_status": "OPTIMIZATION_WATCH",
    }


def verify_preparation(directory: Path, freeze: dict[str, Any]) -> dict[str, Any]:
    value = load_json(directory / "preparation.json")
    verify_root(value, "preparation_root_sha256", "preparation")
    exact(value.get("schema"), "nando.s1c3g-preparation.v1", "preparation_schema")
    transaction = value.get("transaction_id")
    require(isinstance(transaction, str) and ATTEMPT_RE.fullmatch(transaction) is not None, "transaction_id")
    exact(
        value.get("paper"),
        {
            "commit": PAPER_COMMIT,
            "tree": PAPER_TREE,
            "preregistration_sha256": PAPER_PREREGISTRATION_SHA256,
            "critique_sha256": PAPER_CRITIQUE_SHA256,
            "manifest_root_sha256": PAPER_MANIFEST_ROOT,
        },
        "paper",
    )
    exact(value.get("implementation_freeze_root_sha256"), freeze["implementation_freeze_root_sha256"], "freeze_binding")
    exact(
        value.get("implementation_preflight"),
        {
            "commit": PREFLIGHT_COMMIT,
            "manifest_sha256": PREFLIGHT_MANIFEST_SHA256,
            "receipt_sha256": PREFLIGHT_RECEIPT_SHA256,
            "verdict": "READY_TO_IMPLEMENT",
            "blockers": 0,
        },
        "preflight",
    )
    exact(value.get("stable_health_projection"), projection_contract(), "projection_contract")
    exact(value.get("parent"), expected_parent(), "parent")
    exact(file_digest(directory / "candidate-binary"), parent.CANDIDATE_BINARY_SHA256, "candidate_binary")
    exact(file_digest(directory / "candidate-config"), parent.CANDIDATE_CONFIG_SHA256, "candidate_config")
    exact(value.get("economics_before"), {"false_accepts": 0, "runtime_parity_mismatches": 0}, "economics_before")
    parent.verify_opening_journal(value.get("journal_before"), "journal_before")
    before_projection = stable_health_projection(value.get("health_before"), "health_before")
    exact(verify_stable_receipt(value.get("route_probe_before"), "route_before"), before_projection, "route_before_projection")
    verify_parent(directory)
    return value


def verify_final(directory: Path, preparation: dict[str, Any]) -> dict[str, Any]:
    receipt = load_json(directory / "deployment-receipt.json")
    verify_root(receipt, "receipt_root_sha256", "receipt")
    exact(receipt.get("schema"), "nando.s1c3g-deployment-receipt.v1", "receipt_schema")
    verdict = receipt.get("verdict")
    require(
        verdict in {
            "S1C3G_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH",
            "S1C3G_ROLLBACK_PASS",
            "S1C3G_VETO",
        },
        "verdict",
    )
    exact(receipt.get("preparation_root_sha256"), preparation["preparation_root_sha256"], "receipt_preparation")
    exact(receipt.get("parent"), preparation["parent"], "receipt_parent")
    exact(receipt.get("candidate"), preparation["candidate"], "receipt_candidate")
    exact(receipt.get("stable_health_projection_root_sha256"), PROJECTION_ROOT, "receipt_projection")
    exact(receipt.get("scientific_authority"), False, "science")
    exact(receipt.get("model_training"), False, "training")
    exact(receipt.get("phase_mutation"), False, "phase")
    exact(receipt.get("optimization_status"), "OPTIMIZATION_WATCH", "optimization")
    exact(receipt.get("false_accepts_after"), 0, "false_accepts")
    exact(receipt.get("runtime_parity_failures_after"), 0, "parity")
    exact(receipt.get("health_semantics_preserved"), True, "health")
    exact(receipt.get("route_probe_equivalent"), True, "route_equivalent")
    parent.verify_connector(receipt.get("connector_before"), receipt.get("connector_after"))
    deployed = verdict == "S1C3G_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH"
    parent.verify_services(receipt, deployed)
    before_projection = stable_health_projection(receipt.get("health_before"), "health_before")
    after_projection = stable_health_projection(receipt.get("health_after"), "health_after")
    survival_projection = stable_health_projection(receipt.get("health_survival"), "health_survival")
    exact(after_projection, before_projection, "health_after_projection")
    exact(survival_projection, before_projection, "health_survival_projection")
    exact(verify_stable_receipt(receipt.get("route_probe_before"), "route_before"), before_projection, "route_before")
    exact(verify_stable_receipt(receipt.get("route_probe_after"), "route_after"), before_projection, "route_after")
    exact(verify_stable_receipt(receipt.get("route_probe_survival"), "route_survival"), before_projection, "route_survival")
    if deployed:
        exact(receipt.get("capture_available"), True, "capture")
        exact(receipt.get("startup_log_clean"), True, "startup")
        exact(receipt.get("active_packages_preserved"), True, "packages")
        exact(receipt.get("installed_binary_sha256"), parent.CANDIDATE_BINARY_SHA256, "installed_binary")
        exact(receipt.get("installed_config_sha256"), parent.CANDIDATE_CONFIG_SHA256, "installed_config")
        parent.verify_opening_journal(receipt.get("journal_after"), "journal_opening")
        parent.verify_runtime_journal(receipt.get("journal_survival"), receipt["journal_after"], "journal_survival")
        verify_authority_renewal(receipt.get("authority_renewal"))
    else:
        exact("authority_renewal" in receipt, False, "rollback_authority_renewal")
        exact(receipt.get("capture_available"), False, "rollback_capture")
        exact(receipt.get("installed_binary_sha256"), parent.BASELINE_BINARY_SHA256, "rollback_binary")
        exact(receipt.get("installed_config_sha256"), parent.BASELINE_CONFIG_SHA256, "rollback_config")
        forward = receipt.get("journal_forward")
        parent.verify_runtime_journal(forward, preparation["journal_before"], "rollback_forward")
        parent.verify_runtime_journal(receipt.get("journal_after"), forward, "rollback_journal")
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
        "schema": "nando.s1c3g-final-verification.v1",
        "valid": True,
        "authority": True,
        "verdict": verdict,
        "receipt_root_sha256": receipt["receipt_root_sha256"],
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "stable_health_projection_root_sha256": PROJECTION_ROOT,
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
            "schema": "nando.s1c3g-predeployment-verification.v1",
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
        print(json.dumps({"valid": False, "error": str(error)}, sort_keys=True), file=__import__("sys").stderr)
        raise SystemExit(2)
