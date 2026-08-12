#!/usr/bin/env python3
"""Independent receipt verifier for the S1C-3H compatibility installation."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any

import verify_s1c3g_transaction_v1 as parent


PAPER_COMMIT = "1c50fea7119a123379bb7dca5a0eccbda63a9a7b"
PAPER_TREE = "4cd54fd00b349d4fa70236cda6a90c1d1cf80a18"
PAPER_PREREGISTRATION_SHA256 = "5fbbd860c832e9787a0f1a28ef56acd6292080c972d7804380e61e99156e1496"
PAPER_CRITIQUE_SHA256 = "50f51ca3dcef403a82facaee0276f8a3281d93bcb5180dc1d6b694f661502bd4"
PREFLIGHT_COMMIT = "be77371dff05b5eade9841e6612a59937648c2c8"
PREFLIGHT_TREE = "84a00fa4fc11630cc524d2f5e33761f3e8e086d0"
PREFLIGHT_MANIFEST_SHA256 = "fe7dadd86dbd1e52af48b3ae010feb43c5df1ba01de288d3186a3c56a55397bf"
PREFLIGHT_RECEIPT_SHA256 = "8f110a0ad2f09e7928a64f438498bc07e3c22053dda65091725b404e835cd63d"
CANDIDATE_SOURCE_COMMIT = "03e3dd00c90206e2f705371318c50dd50537d6d8"
CANDIDATE_SOURCE_TREE = "06a9df51797dffc127fec41672bddae29c38bb92"
BASELINE_TRANSITION_SHA256 = "6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58"
BASELINE_AUTHORITY_SHA256 = "634f4aaeadeb5815ea1bf67a5ea76d63aae782dae63f8505d40f810e21ad2c3a"
BASELINE_CONFIG_SHA256 = "cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5"
BASELINE_RUNTIME_CONTRACT = "f8d955826f258388f7225086dc57e6557a0fa75bb94809ae7da2cb0e428bd55a"

ATTEMPT_RE = re.compile(r"^\d{8}T\d{6}Z-[0-9a-f]{12}-s1c3h-v1$")
IMPLEMENTATION_FILES = (
    "s1c3h_remote_transaction_v1.py",
    "verify_s1c3h_transaction_v1.py",
    "test_s1c3h_transaction_v1.py",
    "run_s1c3h_transaction_v1.sh",
    "s1c3g_remote_transaction_v1.py",
    "verify_s1c3g_transaction_v1.py",
    "s1c3f_remote_transaction_v1.py",
    "verify_s1c3f_transaction_v1.py",
    "s1c3e_remote_transaction_v1.py",
    "s1c3b_remote_transaction_v1.py",
    "s1c3_remote_transaction_v7.py",
)
COMPATIBILITY_FILES = (
    "admission.json",
    "response-admission-controller-report.json",
    "response-admission-controller.json",
    "response-admission-controller.marker.json",
    "response-authority-candidate.json",
    "response-authority-sidecar-current-v2.json",
    "response-registry.json",
)
TRIGGER_UNITS = (
    "nando-response-admission.path",
    "nando-response-admission.timer",
    "nando-live-transition-gate.path",
    "nando-live-transition-gate.timer",
)
ONESHOT_UNITS = (
    "nando-response-admission.service",
    "nando-live-transition-gate.service",
)
RECOVERY_RECEIPT_SCHEMA = "nando.s1c3h-interrupted-recovery-receipt.v1"


class InvalidReceipt(ValueError):
    pass


def require(condition: bool, label: str) -> None:
    if not condition:
        raise InvalidReceipt(label)


def exact(actual: Any, expected: Any, label: str) -> None:
    require(actual == expected, f"{label}:{actual!r}:{expected!r}")


def canonical_bytes(value: dict[str, Any], omit: str | None = None) -> bytes:
    payload = {key: item for key, item in value.items() if key != omit}
    return (
        json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode()


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def file_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, dict), f"json_object:{path.name}")
    return value


def verify_root(value: dict[str, Any], field: str, label: str) -> str:
    root = value.get(field)
    require(isinstance(root, str) and re.fullmatch(r"[0-9a-f]{64}", root) is not None, f"{label}_hash")
    exact(root, digest_bytes(canonical_bytes(value, field)), f"{label}_root")
    return root


def runtime_contract(binary: Path) -> str:
    value = subprocess.run(
        [str(binary), "--print-runtime-contract-sha256"],
        check=True,
        capture_output=True,
        timeout=10,
    ).stdout.decode("ascii", "strict").strip()
    require(re.fullmatch(r"[0-9a-f]{64}", value) is not None, "runtime_contract")
    return value


def tree_manifest(root: Path) -> list[dict[str, Any]]:
    require(root.is_dir() and not root.is_symlink(), f"tree:{root}")
    rows = []
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink(), f"symlink:{path}")
        if path.is_file():
            rows.append(
                {
                    "path": str(path.relative_to(root)),
                    "size_bytes": path.stat().st_size,
                    "sha256": file_digest(path),
                    "mode_octal": f"{path.stat().st_mode & 0o7777:04o}",
                }
            )
    return rows


def compatibility_manifest(root: Path) -> dict[str, dict[str, Any]]:
    rows = {}
    for name in COMPATIBILITY_FILES:
        path = root / name
        require(path.is_file() and not path.is_symlink(), f"compatibility:{name}")
        rows[name] = {
            "size_bytes": path.stat().st_size,
            "sha256": file_digest(path),
            "mode_octal": f"{path.stat().st_mode & 0o7777:04o}",
        }
    return rows


def transport_projection(rows: Any) -> Any:
    if isinstance(rows, list):
        return [transport_projection(row) for row in rows]
    if isinstance(rows, dict):
        return {
            key: transport_projection(value)
            for key, value in rows.items()
            if key not in {"uid", "gid"}
        }
    return rows


def verify_recorded_owners(rows: Any) -> None:
    if isinstance(rows, list):
        for row in rows:
            verify_recorded_owners(row)
    elif isinstance(rows, dict):
        if "uid" in rows or "gid" in rows:
            require(type(rows.get("uid")) is int and rows["uid"] >= 0, "recorded_uid")
            require(type(rows.get("gid")) is int and rows["gid"] >= 0, "recorded_gid")
        for value in rows.values():
            verify_recorded_owners(value)


def verify_compatibility_snapshot(root: Path) -> dict[str, Any]:
    value = load_json(root / "snapshot.json")
    verify_root(value, "snapshot_root_sha256", "compatibility_snapshot")
    exact(value.get("schema"), "nando.s1c3h-compatibility-snapshot.v1", "snapshot_schema")
    verify_recorded_owners(value.get("compatibility_files"))
    verify_recorded_owners(value.get("generation_manifest"))
    exact(
        transport_projection(value.get("compatibility_files")),
        compatibility_manifest(root),
        "snapshot_files",
    )
    exact(
        transport_projection(value.get("generation_manifest")),
        tree_manifest(root / "generation"),
        "snapshot_generation",
    )
    pointer = load_json(root / "response-authority-sidecar-current-v2.json")
    exact(pointer.get("generation_root_sha256"), value.get("generation_root_sha256"), "snapshot_pointer")
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
        "schema": "nando.s1c3h-implementation-freeze.v1",
        "source_commit": source_commit,
        "source_tree": source_tree,
        "paper": {
            "commit": PAPER_COMMIT,
            "tree": PAPER_TREE,
            "preregistration_sha256": PAPER_PREREGISTRATION_SHA256,
            "critique_sha256": PAPER_CRITIQUE_SHA256,
        },
        "preflight": {
            "commit": PREFLIGHT_COMMIT,
            "tree": PREFLIGHT_TREE,
            "manifest_sha256": PREFLIGHT_MANIFEST_SHA256,
            "receipt_sha256": PREFLIGHT_RECEIPT_SHA256,
        },
        "files": files,
    }
    value["implementation_freeze_root_sha256"] = digest_bytes(canonical_bytes(value))
    return value


def verify_freeze(path: Path, directory: Path) -> dict[str, Any]:
    value = load_json(path)
    verify_root(value, "implementation_freeze_root_sha256", "freeze")
    exact(value.get("schema"), "nando.s1c3h-implementation-freeze.v1", "freeze_schema")
    expected = create_freeze(value.get("source_commit"), value.get("source_tree"), directory)
    exact(value, expected, "freeze_content")
    return value


def create_build_receipt(transition: Path, authority: Path, config: Path) -> dict[str, Any]:
    transition_contract = runtime_contract(transition)
    authority_contract = runtime_contract(authority)
    value = {
        "schema": "nando.s1c3h-candidate-build-receipt.v1",
        "source": {"commit": CANDIDATE_SOURCE_COMMIT, "tree": CANDIDATE_SOURCE_TREE},
        "pair": {
            "transition_sha256": file_digest(transition),
            "authority_sha256": file_digest(authority),
            "transition_runtime_contract_sha256": transition_contract,
            "authority_runtime_contract_sha256": authority_contract,
            "pair_contract_equal": transition_contract == authority_contract,
        },
        "config_sha256": file_digest(config),
    }
    require(value["pair"]["pair_contract_equal"], "candidate_pair_contract")
    value["build_receipt_root_sha256"] = digest_bytes(canonical_bytes(value))
    return value


def verify_build_receipt(directory: Path) -> dict[str, Any]:
    value = load_json(directory / "candidate-build-receipt.json")
    verify_root(value, "build_receipt_root_sha256", "build_receipt")
    exact(value.get("source"), {"commit": CANDIDATE_SOURCE_COMMIT, "tree": CANDIDATE_SOURCE_TREE}, "build_source")
    pair = value.get("pair")
    require(isinstance(pair, dict) and pair.get("pair_contract_equal") is True, "build_pair")
    exact(pair.get("transition_runtime_contract_sha256"), pair.get("authority_runtime_contract_sha256"), "build_contract")
    exact(file_digest(directory / "candidate-nando-transition-serving"), pair.get("transition_sha256"), "candidate_transition")
    exact(file_digest(directory / "candidate-nando-response-admission"), pair.get("authority_sha256"), "candidate_authority")
    exact(file_digest(directory / "candidate-transition-serving.env"), value.get("config_sha256"), "candidate_config")
    return value


def verify_preparation(directory: Path, freeze: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    value = load_json(directory / "preparation.json")
    verify_root(value, "preparation_root_sha256", "preparation")
    exact(value.get("schema"), "nando.s1c3h-preparation.v1", "preparation_schema")
    require(isinstance(value.get("transaction_id"), str) and ATTEMPT_RE.fullmatch(value["transaction_id"]) is not None, "transaction_id")
    exact(value.get("paper_commit"), PAPER_COMMIT, "paper_commit")
    exact(value.get("preflight_commit"), PREFLIGHT_COMMIT, "preflight_commit")
    exact(value.get("implementation_freeze_root_sha256"), freeze["implementation_freeze_root_sha256"], "freeze_binding")
    build = verify_build_receipt(directory)
    exact(value.get("candidate_build_receipt_root_sha256"), build["build_receipt_root_sha256"], "build_binding")
    exact(value.get("candidate_pair"), build["pair"], "candidate_pair")
    exact(value.get("candidate_config_sha256"), build["config_sha256"], "candidate_config")
    exact(value.get("economics_before"), {"false_accepts": 0, "runtime_parity_mismatches": 0}, "economics_before")
    parent.parent.verify_opening_journal(value.get("baseline", {}).get("journal"), "journal_before")
    return value, build


def verify_triggers(before: Any, after: Any) -> None:
    require(isinstance(before, dict) and isinstance(after, dict), "triggers")
    exact(set(after), set(before), "trigger_set")
    exact(set(before), set((*TRIGGER_UNITS, *ONESHOT_UNITS)), "trigger_units")
    for unit in TRIGGER_UNITS:
        exact(
            after[unit].get("active_state"),
            before[unit].get("active_state"),
            f"trigger:{unit}",
        )
    for unit in ONESHOT_UNITS:
        row = after[unit]
        exact(row.get("active_state"), "inactive", f"oneshot_state:{unit}")
        exact(row.get("result"), "success", f"oneshot_result:{unit}")
        exact(row.get("exec_main_status"), 0, f"oneshot_status:{unit}")


def verify_services(preparation: dict[str, Any], receipt: dict[str, Any], deployed: bool) -> None:
    before = preparation.get("services_before")
    survival = receipt.get("services_survival")
    require(isinstance(before, dict) and isinstance(survival, dict), "services")
    transition = "nando-transition-serving.service"
    for unit, opening in before.items():
        final = survival.get(unit)
        require(isinstance(final, dict), f"service:{unit}")
        exact(final.get("active_state"), "active", f"service_active:{unit}")
        exact(final.get("nrestarts"), opening.get("nrestarts"), f"service_restarts:{unit}")
        exact(final.get("fragment_sha256"), opening.get("fragment_sha256"), f"service_fragment:{unit}")
        if unit == transition:
            if deployed:
                require(final.get("main_pid") != opening.get("main_pid"), "transition_not_restarted")
        else:
            exact(final, opening, f"service_unchanged:{unit}")


def verify_final(directory: Path, preparation: dict[str, Any], build: dict[str, Any]) -> dict[str, Any]:
    receipt = load_json(directory / "deployment-receipt.json")
    verify_root(receipt, "receipt_root_sha256", "receipt")
    exact(receipt.get("schema"), "nando.s1c3h-deployment-receipt.v1", "receipt_schema")
    verdict = receipt.get("verdict")
    require(verdict in {"S1C3H_DEPLOYMENT_PASS", "S1C3H_ROLLBACK_PASS"}, "verdict")
    deployed = verdict == "S1C3H_DEPLOYMENT_PASS"
    exact(receipt.get("preparation_root_sha256"), preparation["preparation_root_sha256"], "receipt_preparation")
    pending = load_json(directory / "pending-receipt.json")
    verify_root(pending, "pending_root_sha256", "pending")
    exact(receipt.get("pending_root_sha256"), pending["pending_root_sha256"], "pending_binding")
    diagnostic = load_json(directory / "candidate-diagnostic.json")
    verify_root(diagnostic, "diagnostic_root_sha256", "diagnostic")
    exact(receipt.get("diagnostic_root_sha256"), diagnostic["diagnostic_root_sha256"], "diagnostic_binding")
    exact(receipt.get("scientific_authority"), False, "scientific_authority")
    exact(receipt.get("economics"), {"false_accepts": 0, "runtime_parity_mismatches": 0}, "economics")
    exact(receipt.get("nginx_pid_after"), preparation.get("nginx_pid_before"), "nginx_pid")
    parent.parent.verify_connector(receipt.get("connector_before"), receipt.get("connector_after"))
    verify_triggers(preparation.get("triggers_before"), receipt.get("triggers_after"))
    verify_services(preparation, receipt, deployed)
    parent.parent.verify_runtime_journal(receipt.get("journal_after"), preparation["baseline"]["journal"], "journal_after")
    pair = receipt.get("installed_pair")
    require(isinstance(pair, dict) and pair.get("pair_contract_equal") is True, "installed_pair")
    if deployed:
        exact(pair, build["pair"], "deployed_pair")
        exact(receipt.get("installed_config_sha256"), build["config_sha256"], "deployed_config")
        installed_unit = directory / "evidence" / "installed-unit"
        exact(
            file_digest(installed_unit / "nando-transition-serving"),
            build["pair"]["transition_sha256"],
            "installed_transition_bytes",
        )
        exact(
            file_digest(installed_unit / "nando-response-admission"),
            build["pair"]["authority_sha256"],
            "installed_authority_bytes",
        )
        exact(
            file_digest(installed_unit / "transition-serving.env"),
            build["config_sha256"],
            "installed_config_bytes",
        )
        recorded_unit = receipt.get("installed_unit")
        require(isinstance(recorded_unit, dict), "installed_unit_manifest")
        for name, path in (
            ("nando-transition-serving", installed_unit / "nando-transition-serving"),
            ("nando-response-admission", installed_unit / "nando-response-admission"),
            ("transition-serving.env", installed_unit / "transition-serving.env"),
        ):
            exact(recorded_unit[name].get("sha256"), file_digest(path), f"installed_unit:{name}")
            exact(recorded_unit[name].get("size_bytes"), path.stat().st_size, f"installed_unit_size:{name}")
        exact(receipt.get("capture_installed"), True, "capture_installed")
        exact(receipt.get("capture_environment"), {
            "NANDO_GROUNDED_DECISION_JOURNAL": "/var/lib/nando-wave/transition/grounded-meaning-v1/decision-contract-precommits-v1",
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
        }, "capture_environment")
        renewal = receipt.get("authority_renewal")
        require(isinstance(renewal, dict), "renewal")
        require(renewal.get("renewed_expires_at_unix", 0) > renewal.get("opening_expires_at_unix", 0), "renewal_expiry")
        exact(renewal.get("runtime_contract_sha256"), build["pair"]["transition_runtime_contract_sha256"], "renewal_contract")
        snapshot = verify_compatibility_snapshot(directory / "evidence" / "installed-compatibility")
        exact(receipt.get("installed_compatibility_snapshot_root_sha256"), snapshot["snapshot_root_sha256"], "snapshot_binding")
        admission = load_json(directory / "evidence" / "installed-compatibility" / "admission.json")
        authority = admission.get("response_authority")
        exact(admission.get("verdict"), "PASS", "admission_verdict")
        require(isinstance(authority, dict), "admission_authority")
        exact(authority.get("runtime_build_sha256"), build["pair"]["transition_runtime_contract_sha256"], "admission_contract")
        exact(len(authority.get("packages", [])), 2, "admission_packages")
    else:
        exact(pair.get("transition_sha256"), BASELINE_TRANSITION_SHA256, "rollback_transition")
        exact(pair.get("authority_sha256"), BASELINE_AUTHORITY_SHA256, "rollback_authority")
        exact(pair.get("transition_runtime_contract_sha256"), BASELINE_RUNTIME_CONTRACT, "rollback_contract")
        exact(receipt.get("installed_config_sha256"), BASELINE_CONFIG_SHA256, "rollback_config")
        exact(receipt.get("capture_installed"), False, "rollback_capture")
    return receipt


def verify(directory: Path, freeze_path: Path, predeployment: bool) -> dict[str, Any]:
    freeze = verify_freeze(freeze_path, Path(__file__).resolve().parent)
    preparation, build = verify_preparation(directory, freeze)
    if predeployment:
        value = {
            "schema": "nando.s1c3h-predeployment-verification.v1",
            "valid": True,
            "authority": True,
            "verdict": "S1C3H_PREPARATION_PASS",
            "preparation_root_sha256": preparation["preparation_root_sha256"],
            "implementation_freeze_root_sha256": freeze["implementation_freeze_root_sha256"],
            "candidate_build_receipt_root_sha256": build["build_receipt_root_sha256"],
        }
        value["predeployment_verification_root_sha256"] = digest_bytes(canonical_bytes(value))
        return value
    receipt = verify_final(directory, preparation, build)
    value = {
        "schema": "nando.s1c3h-final-verification.v1",
        "valid": True,
        "authority": True,
        "verdict": receipt["verdict"],
        "receipt_root_sha256": receipt["receipt_root_sha256"],
        "capture_installed": receipt["capture_installed"],
        "scientific_authority": False,
    }
    value["final_verification_root_sha256"] = digest_bytes(canonical_bytes(value))
    return value


def verify_interrupted_recovery(
    directory: Path, expected_source_commit: str, expected_source_tree: str
) -> dict[str, Any]:
    receipt = load_json(directory / "interrupted-recovery-receipt.json")
    verify_root(receipt, "recovery_receipt_root_sha256", "recovery_receipt")
    exact(receipt.get("schema"), RECOVERY_RECEIPT_SCHEMA, "recovery_schema")
    exact(receipt.get("verdict"), "S1C3H_INTERRUPTED_RECOVERY_PASS", "recovery_verdict")
    exact(
        receipt.get("repair_source"),
        {"commit": expected_source_commit, "tree": expected_source_tree},
        "recovery_source",
    )
    preparation = load_json(directory / "preparation.json")
    verify_root(preparation, "preparation_root_sha256", "preparation")
    exact(
        receipt.get("preparation_root_sha256"),
        preparation["preparation_root_sha256"],
        "recovery_preparation",
    )
    diagnostic = load_json(directory / "candidate-diagnostic.json")
    verify_root(diagnostic, "diagnostic_root_sha256", "diagnostic")
    exact(
        receipt.get("diagnostic_root_sha256"),
        diagnostic["diagnostic_root_sha256"],
        "recovery_diagnostic",
    )
    pair = receipt.get("production_pair")
    require(isinstance(pair, dict), "recovery_pair")
    exact(pair.get("transition_sha256"), BASELINE_TRANSITION_SHA256, "recovery_transition")
    exact(pair.get("authority_sha256"), BASELINE_AUTHORITY_SHA256, "recovery_authority")
    exact(
        pair.get("transition_runtime_contract_sha256"),
        BASELINE_RUNTIME_CONTRACT,
        "recovery_contract",
    )
    exact(pair.get("authority_runtime_contract_sha256"), BASELINE_RUNTIME_CONTRACT, "recovery_authority_contract")
    exact(pair.get("pair_contract_equal"), True, "recovery_pair_equal")
    exact(receipt.get("production_config_sha256"), BASELINE_CONFIG_SHA256, "recovery_config")
    services = receipt.get("services")
    require(isinstance(services, dict), "recovery_services")
    for unit, opening in preparation.get("services_before", {}).items():
        current = services.get(unit)
        require(isinstance(current, dict), f"recovery_service:{unit}")
        exact(current.get("active_state"), "active", f"recovery_service_active:{unit}")
        exact(current.get("nrestarts"), opening.get("nrestarts"), f"recovery_service_restarts:{unit}")
    verify_triggers(preparation.get("triggers_before"), receipt.get("triggers"))
    recovery_journal = receipt.get("journal")
    opening_journal = preparation["baseline"]["journal"]
    if recovery_journal != opening_journal:
        parent.parent.verify_runtime_journal(
            recovery_journal, opening_journal, "recovery_journal"
        )
    exact(receipt.get("economics"), {"false_accepts": 0, "runtime_parity_mismatches": 0}, "recovery_economics")
    exact(receipt.get("nginx_pid_before"), preparation.get("nginx_pid_before"), "recovery_nginx_before")
    exact(receipt.get("nginx_pid_after"), preparation.get("nginx_pid_before"), "recovery_nginx_after")
    exact(receipt.get("connector_survival"), "UNKNOWN_MISSING_ORIGINAL_BEFORE_ARTIFACT", "recovery_connector_scope")
    require(
        receipt.get("diagnostic_scope")
        in {
            "PRIMARY_FAILURE_PRESERVED",
            "RECOVERY_DIAGNOSTIC_ONLY_PRIMARY_FAILURE_WAS_OVERWRITTEN",
        },
        "recovery_diagnostic_scope",
    )
    exact(receipt.get("capture_installed"), False, "recovery_capture")
    exact(receipt.get("scientific_authority"), False, "recovery_science")
    health = receipt.get("health", {})
    require(isinstance(health, dict), "recovery_health")
    for label in ("hot", "cpu"):
        stable = health.get(label, {}).get("stable", {})
        exact(stable.get("ok"), True, f"recovery_health_ok:{label}")
        exact(stable.get("mode"), "CPU", f"recovery_health_mode:{label}")
        exact(stable.get("admission_verdict"), "PASS", f"recovery_health_admission:{label}")
        exact(stable.get("response_active_profiles"), 2, f"recovery_health_profiles:{label}")
        exact(stable.get("response_executor_cache_ready"), True, f"recovery_health_cache:{label}")
    terminal_path = directory / "s1c3h-state.json"
    if terminal_path.exists():
        terminal = load_json(terminal_path)
        verify_root(terminal, "state_root_sha256", "recovery_terminal")
        exact(terminal.get("state"), "COMPLETE", "recovery_terminal_state")
        exact(terminal.get("verdict"), receipt["verdict"], "recovery_terminal_verdict")
        exact(
            terminal.get("recovery_receipt_root_sha256"),
            receipt["recovery_receipt_root_sha256"],
            "recovery_terminal_receipt",
        )
        exact(
            terminal.get("connector_survival"),
            receipt["connector_survival"],
            "recovery_terminal_connector",
        )
        exact(terminal.get("capture_installed"), False, "recovery_terminal_capture")
        exact(
            terminal.get("scientific_authority"),
            False,
            "recovery_terminal_science",
        )
    value = {
        "schema": "nando.s1c3h-interrupted-recovery-verification.v1",
        "valid": True,
        "authority": True,
        "verdict": receipt["verdict"],
        "recovery_receipt_root_sha256": receipt["recovery_receipt_root_sha256"],
        "capture_installed": False,
        "scientific_authority": False,
    }
    value["recovery_verification_root_sha256"] = digest_bytes(canonical_bytes(value))
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    freeze = commands.add_parser("create-freeze")
    freeze.add_argument("--source-commit", required=True)
    freeze.add_argument("--source-tree", required=True)
    build = commands.add_parser("create-build-receipt")
    build.add_argument("--transition", type=Path, required=True)
    build.add_argument("--authority", type=Path, required=True)
    build.add_argument("--config", type=Path, required=True)
    check = commands.add_parser("verify")
    check.add_argument("directory", type=Path)
    check.add_argument("--implementation-freeze", type=Path, required=True)
    check.add_argument("--predeployment", action="store_true")
    recovery = commands.add_parser("verify-interrupted-recovery")
    recovery.add_argument("directory", type=Path)
    recovery.add_argument("--expected-source-commit", required=True)
    recovery.add_argument("--expected-source-tree", required=True)
    args = parser.parse_args()
    if args.command == "create-freeze":
        value = create_freeze(args.source_commit, args.source_tree, Path(__file__).resolve().parent)
    elif args.command == "create-build-receipt":
        value = create_build_receipt(args.transition, args.authority, args.config)
    elif args.command == "verify":
        value = verify(args.directory, args.implementation_freeze, args.predeployment)
    else:
        value = verify_interrupted_recovery(
            args.directory, args.expected_source_commit, args.expected_source_tree
        )
    print(json.dumps(value, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(json.dumps({"schema": "nando.s1c3h-verifier-error.v1", "error": str(error)}, sort_keys=True), file=__import__("sys").stderr)
        raise SystemExit(1)
