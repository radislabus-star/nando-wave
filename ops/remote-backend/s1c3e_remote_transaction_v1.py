#!/usr/bin/env python3
"""Root-only executor for the frozen S1C-3E ownership repair."""

from __future__ import annotations

import argparse
import fcntl
import grp
import json
import os
import pwd
import shutil
import stat
import time
from pathlib import Path
from typing import Any

import s1c3b_remote_transaction_v1 as base


PAPER_COMMIT = "c635ac27e9b1f49e1977c15bee1d03afe5f6d7e5"
PAPER_TREE = "553ff1803e5532ca93ed13b3eac996e6fdd4ae58"
PAPER_PREREGISTRATION_SHA256 = "b612127470ff38cc6957afa22d66359a97ce467c8ea41e77045b74bc064e2ff5"
PAPER_CRITIQUE_SHA256 = "b2ab39e1282c785fa8836093e1a269d33dc0d6db5cec5aec921084c32917291c"
PAPER_MANIFEST_ROOT = "2d11b32323e3d4b8dd37f48629e18cdc1d3304491ea3fad1b2bdb5359013ec2a"

PARENT_TRANSACTION_ID = "20260812T145640Z-c3eaddc55dfc-s1c3d-v1"
PARENT_DIRECTORY = Path("/var/lib/nando-wave/deployments") / PARENT_TRANSACTION_ID
PARENT_STATE_ROOT = "6ec0baf716a12467b9f7ca6e18bc6e6bf4543f1c95432dc65daf7b3ce5685ffb"
PARENT_RESOURCE_ROOT = "c917e62a85d2776e3a20d3efd72b16230a0689c73975b786d6ab8687c1176038"
PARENT_PARITY_ROOT = "55ae110ce15f198e0741890e856e5822170e1ba479870ea9c03ac4bd34ad3ea9"
PARENT_CLASSIFICATION_ROOT = "0d4f71a40c616a124f6aee3c03e4a868b4823f7aaf5c3bb7c48dee026aea7c01"
CANDIDATE_BINARY_SHA256 = "360498a0908739cad6f1ac21cf4053b7421daaf8b1d9a6502b72132a94a692df"
CANDIDATE_CONFIG_SHA256 = "1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6"
BASELINE_BINARY_SHA256 = base.BASELINE_BINARY_SHA256
BASELINE_CONFIG_SHA256 = base.BASELINE_CONFIG_SHA256

PRODUCTION_BINARY = base.PRODUCTION_BINARY
PRODUCTION_CONFIG = base.PRODUCTION_CONFIG
TRANSITION_UNIT = base.TRANSITION_UNIT
JOURNAL = base.JOURNAL
EXPECTED_SEGMENTS = (
    "decision-precommit-00000000000000000000.cbor",
    "goal-satisfaction-00000000000000000000.cbor",
    "selected-action-binding-00000000000000000000.cbor",
)

PREPARATION_SCHEMA = "nando.s1c3e-preparation.v1"
PREDEPLOYMENT_SCHEMA = "nando.s1c3e-predeployment-verification.v1"
PENDING_SCHEMA = "nando.s1c3e-pending-receipt.v1"
RECEIPT_SCHEMA = "nando.s1c3e-deployment-receipt.v1"
STATE_SCHEMA = "nando.s1c3e-state.v1"

GateFailure = base.GateFailure
canonical_bytes = base.canonical_bytes
sha256_bytes = base.sha256_bytes
sha256_file = base.sha256_file
read_json = base.read_json
write_json = base.write_json
atomic_write = base.atomic_write
fsync_directory = base.fsync_directory
service_snapshot = base.service_snapshot
require_active = base.require_active
health_snapshot = base.health_snapshot
economics_snapshot = base.economics_snapshot
route_probe = base.route_probe
process_environment = base.process_environment
process_rss = base.process_rss
install_pair = base.install_pair
wait_for_service = base.wait_for_service
systemctl = base.systemctl
run = base.run
exact_untouched = base.exact_untouched
semantic_health_equal = base.semantic_health_equal


def rooted(value: dict[str, Any], field: str) -> dict[str, Any]:
    value[field] = sha256_bytes(canonical_bytes(value, field))
    return value


def _mode(path: Path) -> str:
    return f"{stat.S_IMODE(path.stat().st_mode):04o}"


def journal_snapshot() -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    if JOURNAL.exists():
        if JOURNAL.is_symlink() or not JOURNAL.is_dir():
            raise GateFailure("s1c3e_journal_directory_invalid")
        for path in sorted(JOURNAL.iterdir()):
            if path.is_symlink() or not path.is_file():
                raise GateFailure(f"s1c3e_journal_entry_invalid:{path.name}")
            value = path.stat()
            entries.append(
                {
                    "path": path.name,
                    "size_bytes": value.st_size,
                    "sha256": sha256_file(path),
                    "uid": value.st_uid,
                    "gid": value.st_gid,
                    "mode_octal": f"{stat.S_IMODE(value.st_mode):04o}",
                }
            )
    directory = None
    if JOURNAL.exists():
        value = JOURNAL.stat()
        directory = {
            "path": str(JOURNAL),
            "uid": value.st_uid,
            "gid": value.st_gid,
            "mode_octal": f"{stat.S_IMODE(value.st_mode):04o}",
        }
    manifest = "".join(
        f'{row["sha256"]} {row["size_bytes"]} {row["uid"]}:{row["gid"]} '
        f'{row["mode_octal"]} {row["path"]}\n'
        for row in entries
    ).encode()
    return {
        "present": JOURNAL.exists(),
        "directory": directory,
        "entries": entries,
        "total_bytes": sum(row["size_bytes"] for row in entries),
        "manifest_root_sha256": sha256_bytes(manifest),
    }


def require_exact_empty_runtime_journal(snapshot: dict[str, Any]) -> None:
    user = pwd.getpwnam("e")
    group = grp.getgrnam("e")
    expected_directory = {
        "path": str(JOURNAL),
        "uid": user.pw_uid,
        "gid": group.gr_gid,
        "mode_octal": "0700",
    }
    if snapshot.get("directory") != expected_directory:
        raise GateFailure("s1c3e_journal_directory_identity_mismatch")
    entries = snapshot.get("entries")
    if not isinstance(entries, list) or [row.get("path") for row in entries] != list(EXPECTED_SEGMENTS):
        raise GateFailure("s1c3e_journal_segment_set_mismatch")
    for row in entries:
        if row.get("size_bytes") != 0 or row.get("sha256") != sha256_bytes(b""):
            raise GateFailure(f's1c3e_journal_segment_nonempty:{row.get("path")}')
        if row.get("uid") != user.pw_uid or row.get("gid") != group.gr_gid:
            raise GateFailure(f's1c3e_journal_segment_owner:{row.get("path")}')
        if row.get("mode_octal") != "0600":
            raise GateFailure(f's1c3e_journal_segment_mode:{row.get("path")}')
    if snapshot.get("total_bytes") != 0:
        raise GateFailure("s1c3e_journal_not_empty")


def require_runtime_journal_shape(snapshot: dict[str, Any]) -> None:
    user = pwd.getpwnam("e")
    group = grp.getgrnam("e")
    if snapshot.get("directory") != {
        "path": str(JOURNAL),
        "uid": user.pw_uid,
        "gid": group.gr_gid,
        "mode_octal": "0700",
    }:
        raise GateFailure("s1c3e_journal_survival_directory")
    entries = snapshot.get("entries")
    if not isinstance(entries, list) or [row.get("path") for row in entries] != list(EXPECTED_SEGMENTS):
        raise GateFailure("s1c3e_journal_survival_segments")
    for row in entries:
        if row.get("uid") != user.pw_uid or row.get("gid") != group.gr_gid:
            raise GateFailure(f's1c3e_journal_survival_owner:{row.get("path")}')
        if row.get("mode_octal") != "0600" or not isinstance(row.get("size_bytes"), int):
            raise GateFailure(f's1c3e_journal_survival_file:{row.get("path")}')


def provision_empty_directory() -> dict[str, Any]:
    if JOURNAL.exists():
        raise GateFailure("s1c3e_journal_preexists")
    JOURNAL.mkdir(mode=0o700)
    user = pwd.getpwnam("e")
    group = grp.getgrnam("e")
    os.chown(JOURNAL, user.pw_uid, group.gr_gid)
    os.chmod(JOURNAL, 0o700)
    fsync_directory(JOURNAL.parent)
    if any(JOURNAL.iterdir()):
        raise GateFailure("s1c3e_provisioned_directory_not_empty")
    snapshot = journal_snapshot()
    if snapshot["directory"] != {
        "path": str(JOURNAL),
        "uid": user.pw_uid,
        "gid": group.gr_gid,
        "mode_octal": "0700",
    }:
        raise GateFailure("s1c3e_provisioned_directory_identity")
    return snapshot


def cleanup_operational_empty_journal() -> bool:
    if not JOURNAL.exists():
        return True
    snapshot = journal_snapshot()
    if not snapshot.get("entries"):
        directory = snapshot.get("directory")
        user = pwd.getpwnam("e")
        group = grp.getgrnam("e")
        if directory == {
            "path": str(JOURNAL),
            "uid": user.pw_uid,
            "gid": group.gr_gid,
            "mode_octal": "0700",
        }:
            JOURNAL.rmdir()
            fsync_directory(JOURNAL.parent)
            return True
        return False
    try:
        require_exact_empty_runtime_journal(snapshot)
    except GateFailure:
        return False
    for path in JOURNAL.iterdir():
        path.unlink()
    JOURNAL.rmdir()
    fsync_directory(JOURNAL.parent)
    return True


def verify_parent() -> dict[str, Any]:
    if not PARENT_DIRECTORY.is_dir():
        raise GateFailure("s1c3e_parent_missing")
    state = read_json(PARENT_DIRECTORY / "s1c3d-state.json")
    resource = read_json(PARENT_DIRECTORY / "resource-receipt.json")
    parity = read_json(PARENT_DIRECTORY / "parity-receipt.json")
    checks = (
        (state.get("verdict"), "S1C3D_ROLLBACK_PASS", "parent_verdict"),
        (state.get("state_root_sha256"), PARENT_STATE_ROOT, "parent_state_root"),
        (resource.get("resource_root_sha256"), PARENT_RESOURCE_ROOT, "parent_resource_root"),
        (parity.get("parity_root_sha256"), PARENT_PARITY_ROOT, "parent_parity_root"),
        (
            resource.get("classification", {}).get("classification_root_sha256"),
            PARENT_CLASSIFICATION_ROOT,
            "parent_classification_root",
        ),
        (resource.get("classification", {}).get("hard_gate_status"), "PASS", "parent_hard_gate"),
        (
            resource.get("classification", {}).get("optimization_status"),
            "OPTIMIZATION_WATCH",
            "parent_optimization",
        ),
        (
            sha256_file(PARENT_DIRECTORY / "candidate-binary"),
            CANDIDATE_BINARY_SHA256,
            "parent_candidate_binary",
        ),
        (
            sha256_file(PARENT_DIRECTORY / "candidate-config"),
            CANDIDATE_CONFIG_SHA256,
            "parent_candidate_config",
        ),
    )
    for actual, expected, label in checks:
        if actual != expected:
            raise GateFailure(f"s1c3e_{label}_mismatch:{actual}")
    return {
        "transaction_id": PARENT_TRANSACTION_ID,
        "state_root_sha256": PARENT_STATE_ROOT,
        "resource_root_sha256": PARENT_RESOURCE_ROOT,
        "parity_root_sha256": PARENT_PARITY_ROOT,
        "classification_root_sha256": PARENT_CLASSIFICATION_ROOT,
        "optimization_status": "OPTIMIZATION_WATCH",
    }


def verify_current_production() -> None:
    if sha256_file(PRODUCTION_BINARY) != BASELINE_BINARY_SHA256:
        raise GateFailure("STALE_BEFORE_MUTATION:baseline_binary")
    if sha256_file(PRODUCTION_CONFIG) != BASELINE_CONFIG_SHA256:
        raise GateFailure("STALE_BEFORE_MUTATION:baseline_config")
    if JOURNAL.exists():
        raise GateFailure("STALE_BEFORE_MUTATION:journal_present")


def prepare(args: argparse.Namespace) -> int:
    if os.geteuid() != 0:
        raise GateFailure("root_required")
    root = Path(args.transaction_directory)
    if root.exists():
        raise GateFailure("s1c3e_transaction_exists")
    root.mkdir(parents=True, mode=0o700)
    (root / "rollback").mkdir(mode=0o700)
    (root / "evidence").mkdir(mode=0o700)
    try:
        implementation_freeze = read_json(Path(args.implementation_freeze))
        implementation_freeze_root = implementation_freeze.get(
            "implementation_freeze_root_sha256"
        )
        if implementation_freeze_root != sha256_bytes(
            canonical_bytes(implementation_freeze, "implementation_freeze_root_sha256")
        ):
            raise GateFailure("s1c3e_implementation_freeze_root")
        parent = verify_parent()
        verify_current_production()
        services = service_snapshot()
        require_active(services)
        health = health_snapshot()
        economics = economics_snapshot()
        if economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
            raise GateFailure("s1c3e_baseline_economics_unsafe")
        route = route_probe()
        connector = read_json(Path(args.connector_before))
        transition_rss = process_rss(services[TRANSITION_UNIT]["main_pid"])
        rollback_files = {
            "nando-transition-serving": PRODUCTION_BINARY,
            "transition-serving.env": PRODUCTION_CONFIG,
        }
        for name, source in rollback_files.items():
            destination = root / "rollback" / name
            shutil.copy2(source, destination)
            os.chmod(destination, 0o500 if name == "nando-transition-serving" else 0o400)
        shutil.copy2(PARENT_DIRECTORY / "candidate-binary", root / "candidate-binary")
        shutil.copy2(PARENT_DIRECTORY / "candidate-config", root / "candidate-config")
        os.chmod(root / "candidate-binary", 0o500)
        os.chmod(root / "candidate-config", 0o400)
        parent_evidence = root / "parent-evidence"
        parent_evidence.mkdir(mode=0o700)
        for name in ("s1c3d-state.json", "resource-receipt.json", "parity-receipt.json"):
            shutil.copy2(PARENT_DIRECTORY / name, parent_evidence / name)
            os.chmod(parent_evidence / name, 0o400)
        preparation = rooted(
            {
                "schema": PREPARATION_SCHEMA,
                "transaction_id": args.transaction_id,
                "paper": {
                    "commit": PAPER_COMMIT,
                    "tree": PAPER_TREE,
                    "preregistration_sha256": PAPER_PREREGISTRATION_SHA256,
                    "critique_sha256": PAPER_CRITIQUE_SHA256,
                    "manifest_root_sha256": PAPER_MANIFEST_ROOT,
                },
                "implementation_freeze_root_sha256": implementation_freeze_root,
                "parent": parent,
                "candidate": {
                    "binary_sha256": CANDIDATE_BINARY_SHA256,
                    "config_sha256": CANDIDATE_CONFIG_SHA256,
                },
                "production": {
                    "binary_sha256": BASELINE_BINARY_SHA256,
                    "config_sha256": BASELINE_CONFIG_SHA256,
                },
                "services_before": services,
                "health_before": health,
                "economics_before": economics,
                "route_probe_before": route,
                "connector_before": connector,
                "journal_before": journal_snapshot(),
                "transition_rss_before": transition_rss,
            },
            "preparation_root_sha256",
        )
        write_json(root / "preparation.json", preparation, 0o400)
        write_json(root / "implementation-freeze.json", implementation_freeze, 0o400)
        write_json(
            root / "transaction-state.json",
            {"schema": STATE_SCHEMA, "state": "PREPARED", "transaction_id": args.transaction_id},
            0o600,
        )
        fsync_directory(root)
        print(json.dumps({"state": "PREPARED", "preparation_root_sha256": preparation["preparation_root_sha256"]}, sort_keys=True))
        return 0
    except Exception as error:
        write_json(root / "preflight-failure.json", {"error": str(error)}, 0o400)
        write_json(
            root / "transaction-state.json",
            {"schema": STATE_SCHEMA, "state": "PREFLIGHT_FAILURE", "transaction_id": args.transaction_id},
            0o600,
        )
        raise


def verify_predeployment(root: Path, path: Path) -> dict[str, Any]:
    value = read_json(path)
    preparation = read_json(root / "preparation.json")
    if value.get("predeployment_verification_root_sha256") != sha256_bytes(
        canonical_bytes(value, "predeployment_verification_root_sha256")
    ):
        raise GateFailure("s1c3e_predeployment_root")
    expected = {
        "schema": PREDEPLOYMENT_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": "S1C3E_PREPARATION_PASS_WITH_OPTIMIZATION_WATCH",
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "parent_state_root_sha256": PARENT_STATE_ROOT,
        "parent_resource_root_sha256": PARENT_RESOURCE_ROOT,
        "parent_parity_root_sha256": PARENT_PARITY_ROOT,
    }
    if {key: value.get(key) for key in expected} != expected:
        raise GateFailure("s1c3e_predeployment_mismatch")
    return value


def _startup_log(root: Path, since: float) -> bool:
    output = run(
        ["journalctl", "-u", TRANSITION_UNIT, "--since", f"@{since:.6f}", "--no-pager", "-o", "cat"],
        timeout=10,
    ).stdout
    atomic_write(root / "evidence" / "startup.log", output, 0o400)
    return b"nando-grounded-decision shadow unavailable" not in output


def rollback(root: Path, reason: str) -> None:
    state = read_json(root / "transaction-state.json").get("state")
    if state not in {"ROLLBACK_ARMED", "FINALIZE_PENDING", "FINAL_VERIFICATION_PENDING"}:
        raise GateFailure(f"s1c3e_rollback_state:{state}")
    preparation = read_json(root / "preparation.json")
    journal_forward = journal_snapshot()
    systemctl("stop", TRANSITION_UNIT, check=False)
    install_pair(root / "rollback" / "nando-transition-serving", root / "rollback" / "transition-serving.env")
    systemctl("start", TRANSITION_UNIT)
    services_after, health_after = wait_for_service()
    time.sleep(15)
    services_survival, health_survival = wait_for_service()
    exact_untouched(preparation["services_before"], services_after)
    exact_untouched(preparation["services_before"], services_survival)
    removed = cleanup_operational_empty_journal()
    journal_after = journal_snapshot()
    if sha256_file(PRODUCTION_BINARY) != BASELINE_BINARY_SHA256 or sha256_file(PRODUCTION_CONFIG) != BASELINE_CONFIG_SHA256:
        raise GateFailure("s1c3e_rollback_pair_mismatch")
    pending = {
        "schema": PENDING_SCHEMA,
        "verdict": "S1C3E_ROLLBACK_PASS",
        "rollback_reason": reason,
        "services_after": services_after,
        "services_survival": services_survival,
        "health_after": health_after,
        "health_survival": health_survival,
        "route_probe_after": route_probe(),
        "route_probe_survival": route_probe(),
        "journal_forward": journal_forward,
        "journal_after": journal_after,
        "empty_operational_journal_removed": removed,
        "capture_environment": {},
        "capture_available": False,
        "startup_log_clean": True,
        "active_packages_preserved": True,
        "health_semantics_preserved": semantic_health_equal(
            preparation["health_before"], health_after
        )
        and semantic_health_equal(preparation["health_before"], health_survival),
        "economics": economics_snapshot(),
        "installed_binary_sha256": sha256_file(PRODUCTION_BINARY),
        "installed_config_sha256": sha256_file(PRODUCTION_CONFIG),
        "transition_rss_after": process_rss(services_survival[TRANSITION_UNIT]["main_pid"]),
    }
    write_json(root / "pending-receipt.json", pending, 0o600)
    write_json(
        root / "transaction-state.json",
        {"schema": STATE_SCHEMA, "state": "ROLLBACK_PENDING", "transaction_id": preparation["transaction_id"]},
        0o600,
    )


def execute(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    preparation = read_json(root / "preparation.json")
    verification = verify_predeployment(root, Path(args.predeployment_verification))
    state = read_json(root / "transaction-state.json")
    if state.get("state") != "PREPARED":
        raise GateFailure("s1c3e_not_prepared")
    verify_current_production()
    if service_snapshot() != preparation["services_before"]:
        raise GateFailure("STALE_BEFORE_MUTATION:services")
    if economics_snapshot() != preparation["economics_before"]:
        raise GateFailure("STALE_BEFORE_MUTATION:economics")
    if route_probe() != preparation["route_probe_before"]:
        raise GateFailure("STALE_BEFORE_MUTATION:route")
    write_json(root / "predeployment-verification.json", verification, 0o400)
    write_json(
        root / "transaction-state.json",
        {"schema": STATE_SCHEMA, "state": "ROLLBACK_ARMED", "transaction_id": preparation["transaction_id"]},
        0o600,
    )
    fsync_directory(root)
    old_pid = preparation["services_before"][TRANSITION_UNIT]["main_pid"]
    stopped = False
    try:
        provisioned = provision_empty_directory()
        write_json(root / "journal-provisioning.json", provisioned, 0o400)
        started_at = time.time()
        systemctl("stop", TRANSITION_UNIT)
        stopped = True
        deadline = time.monotonic() + 20
        while Path(f"/proc/{old_pid}").exists() and time.monotonic() < deadline:
            time.sleep(0.1)
        if Path(f"/proc/{old_pid}").exists():
            raise GateFailure("s1c3e_old_pid_alive")
        install_pair(root / "candidate-binary", root / "candidate-config")
        if sha256_file(PRODUCTION_BINARY) != CANDIDATE_BINARY_SHA256 or sha256_file(PRODUCTION_CONFIG) != CANDIDATE_CONFIG_SHA256:
            raise GateFailure("s1c3e_candidate_install_mismatch")
        systemctl("start", TRANSITION_UNIT)
        services_after, health_after = wait_for_service()
        exact_untouched(preparation["services_before"], services_after)
        new_pid = services_after[TRANSITION_UNIT]["main_pid"]
        if new_pid == old_pid:
            raise GateFailure("s1c3e_pid_unchanged")
        if services_after[TRANSITION_UNIT]["nrestarts"] != preparation["services_before"][TRANSITION_UNIT]["nrestarts"]:
            raise GateFailure("s1c3e_nrestarts_changed")
        expected_environment = {
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
            "NANDO_GROUNDED_DECISION_JOURNAL": str(JOURNAL),
        }
        environment = process_environment(new_pid)
        if {key: environment.get(key) for key in expected_environment} != expected_environment:
            raise GateFailure("s1c3e_capture_environment")
        if not _startup_log(root, started_at):
            raise GateFailure("s1c3e_capture_startup_unavailable")
        journal_after = journal_snapshot()
        require_exact_empty_runtime_journal(journal_after)
        transition_rss = process_rss(new_pid)
        if max(0, transition_rss - preparation["transition_rss_before"]) > 16 * 1024 * 1024:
            raise GateFailure("s1c3e_rss_delta")
        if route_probe() != preparation["route_probe_before"]:
            raise GateFailure("s1c3e_route_probe")
        time.sleep(15)
        services_survival, health_survival = wait_for_service()
        exact_untouched(preparation["services_before"], services_survival)
        if services_survival[TRANSITION_UNIT]["main_pid"] != new_pid:
            raise GateFailure("s1c3e_survival_pid")
        if not semantic_health_equal(preparation["health_before"], health_after):
            raise GateFailure("s1c3e_health_after")
        if not semantic_health_equal(preparation["health_before"], health_survival):
            raise GateFailure("s1c3e_health_survival")
        journal_survival = journal_snapshot()
        require_runtime_journal_shape(journal_survival)
        economics = economics_snapshot()
        if economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
            raise GateFailure("s1c3e_economics_unsafe")
        pending = {
            "schema": PENDING_SCHEMA,
            "verdict": "S1C3E_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH",
            "services_after": services_after,
            "services_survival": services_survival,
            "health_after": health_after,
            "health_survival": health_survival,
            "route_probe_after": route_probe(),
            "route_probe_survival": route_probe(),
            "journal_after": journal_after,
            "journal_survival": journal_survival,
            "capture_environment": expected_environment,
            "capture_available": True,
            "startup_log_clean": True,
            "active_packages_preserved": health_after["hot"]["semantic"].get("response_active_profiles")
            == preparation["health_before"]["hot"]["semantic"].get("response_active_profiles"),
            "health_semantics_preserved": True,
            "economics": economics,
            "installed_binary_sha256": sha256_file(PRODUCTION_BINARY),
            "installed_config_sha256": sha256_file(PRODUCTION_CONFIG),
            "transition_rss_after": transition_rss,
        }
        write_json(root / "pending-receipt.json", pending, 0o600)
        write_json(
            root / "transaction-state.json",
            {"schema": STATE_SCHEMA, "state": "FINALIZE_PENDING", "transaction_id": preparation["transaction_id"]},
            0o600,
        )
        return 0
    except Exception as error:
        if stopped or JOURNAL.exists():
            rollback(root, str(error))
            raise GateFailure(f"S1C3E_ROLLBACK_PASS:{error}") from error
        raise


def abort_predeployment(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    state = read_json(root / "transaction-state.json").get("state")
    if state != "PREPARED":
        raise GateFailure(f"s1c3e_abort_state:{state}")
    preparation = read_json(root / "preparation.json")
    verify_current_production()
    if service_snapshot() != preparation["services_before"]:
        raise GateFailure("s1c3e_abort_service_drift")
    terminal = rooted(
        {
            "schema": STATE_SCHEMA,
            "state": "PREFLIGHT_FAILURE",
            "transaction_id": preparation["transaction_id"],
            "verdict": "S1C3E_PREFLIGHT_FAILURE",
            "reason": args.reason,
            "identity_consumed": True,
            "production_mutation": False,
            "capture_installed": False,
            "s1c4_state": "CLOSED",
            "scientific_authority": False,
            "model_training": False,
            "phase_mutation": False,
        },
        "state_root_sha256",
    )
    write_json(root / "s1c3e-state.json", terminal, 0o400)
    write_json(
        root / "transaction-state.json",
        {
            "schema": STATE_SCHEMA,
            "state": "PREFLIGHT_FAILURE",
            "transaction_id": preparation["transaction_id"],
            "verdict": "S1C3E_PREFLIGHT_FAILURE",
        },
        0o600,
    )
    print(json.dumps(terminal, sort_keys=True))
    return 0


def connector_failure_reasons(before: dict[str, Any], after: dict[str, Any]) -> list[str]:
    fields = ("main_pid", "nrestarts", "route_receipt_failures", "command_sha256", "active_state")
    return [f"connector_{field}" for field in fields if before.get(field) != after.get(field)]


def finalize(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    preparation = read_json(root / "preparation.json")
    pending = read_json(root / "pending-receipt.json")
    state = read_json(root / "transaction-state.json").get("state")
    if state not in {"FINALIZE_PENDING", "ROLLBACK_PENDING"}:
        raise GateFailure("s1c3e_finalize_state")
    connector_after = read_json(Path(args.connector_after))
    veto = connector_failure_reasons(preparation["connector_before"], connector_after)
    if veto and state == "FINALIZE_PENDING":
        rollback(root, ",".join(veto))
        pending = read_json(root / "pending-receipt.json")
        state = "ROLLBACK_PENDING"
    if veto:
        pending["verdict"] = "S1C3E_VETO"
    receipt = rooted(
        {
            "schema": RECEIPT_SCHEMA,
            "transaction_id": preparation["transaction_id"],
            "verdict": pending["verdict"],
            "preparation_root_sha256": preparation["preparation_root_sha256"],
            "parent": preparation["parent"],
            "candidate": preparation["candidate"],
            "services_before": preparation["services_before"],
            "services_after": pending["services_after"],
            "services_survival": pending["services_survival"],
            "health_before": preparation["health_before"],
            "health_after": pending["health_after"],
            "health_survival": pending["health_survival"],
            "route_probe_before": preparation["route_probe_before"],
            "route_probe_after": pending["route_probe_after"],
            "route_probe_survival": pending["route_probe_survival"],
            "connector_before": preparation["connector_before"],
            "connector_after": connector_after,
            "journal_before": preparation["journal_before"],
            "journal_after": pending["journal_after"],
            "journal_survival": pending.get("journal_survival", pending["journal_after"]),
            "capture_environment": pending["capture_environment"],
            "capture_available": pending["capture_available"],
            "startup_log_clean": pending["startup_log_clean"],
            "active_packages_preserved": pending["active_packages_preserved"],
            "health_semantics_preserved": pending.get("health_semantics_preserved", False),
            "false_accepts_after": pending["economics"]["false_accepts"],
            "runtime_parity_failures_after": pending["economics"]["runtime_parity_mismatches"],
            "installed_binary_sha256": pending["installed_binary_sha256"],
            "installed_config_sha256": pending["installed_config_sha256"],
            "optimization_status": "OPTIMIZATION_WATCH",
            "scientific_authority": False,
            "model_training": False,
            "phase_mutation": False,
            "veto_reasons": veto,
        },
        "receipt_root_sha256",
    )
    write_json(root / "deployment-receipt.json", receipt, 0o400)
    write_json(
        root / "transaction-state.json",
        {"schema": STATE_SCHEMA, "state": "FINAL_VERIFICATION_PENDING", "transaction_id": preparation["transaction_id"]},
        0o600,
    )
    return 0


def seal(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    if read_json(root / "transaction-state.json").get("state") != "FINAL_VERIFICATION_PENDING":
        raise GateFailure("s1c3e_seal_state")
    recorded = read_json(Path(args.final_verification))
    receipt = read_json(root / "deployment-receipt.json")
    if recorded.get("final_verification_root_sha256") != sha256_bytes(
        canonical_bytes(recorded, "final_verification_root_sha256")
    ):
        raise GateFailure("s1c3e_final_root")
    expected = {
        "valid": True,
        "authority": True,
        "verdict": receipt["verdict"],
        "receipt_root_sha256": receipt["receipt_root_sha256"],
    }
    if {key: recorded.get(key) for key in expected} != expected:
        raise GateFailure("s1c3e_final_mismatch")
    write_json(root / "final-verification.json", recorded, 0o400)
    cursor = recorded.get("s1c4_cursor")
    if cursor is not None:
        write_json(root / "s1c4-append-cursor.json", cursor, 0o400)
    state = rooted(
        {
            "schema": STATE_SCHEMA,
            "state": "COMPLETE",
            "transaction_id": receipt["transaction_id"],
            "verdict": receipt["verdict"],
            "receipt_root_sha256": receipt["receipt_root_sha256"],
            "final_verification_root_sha256": recorded["final_verification_root_sha256"],
            "capture_installed": recorded["capture_installed"],
            "s1c4_state": recorded["s1c4_state"],
            "scientific_authority": False,
            "model_training": False,
            "phase_mutation": False,
        },
        "state_root_sha256",
    )
    write_json(root / "s1c3e-state.json", state, 0o400)
    write_json(
        root / "transaction-state.json",
        {"schema": STATE_SCHEMA, "state": "COMPLETE", "transaction_id": receipt["transaction_id"], "verdict": receipt["verdict"]},
        0o600,
    )
    print(json.dumps(state, sort_keys=True))
    return 0


def locked(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    descriptor = os.open(root / ".mutation.lock", os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX)
        if args.command == "execute":
            return execute(args)
        if args.command == "abort-predeployment":
            return abort_predeployment(args)
        if args.command == "rollback":
            rollback(root, args.reason)
            return 0
        if args.command == "finalize":
            return finalize(args)
        return seal(args)
    finally:
        os.close(descriptor)


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
    abort_parser = commands.add_parser("abort-predeployment")
    abort_parser.add_argument("--transaction-directory", required=True)
    abort_parser.add_argument("--reason", required=True)
    finalize_parser = commands.add_parser("finalize")
    finalize_parser.add_argument("--transaction-directory", required=True)
    finalize_parser.add_argument("--connector-after", required=True)
    seal_parser = commands.add_parser("seal")
    seal_parser.add_argument("--transaction-directory", required=True)
    seal_parser.add_argument("--final-verification", required=True)
    args = parser.parse_args()
    if args.command == "prepare":
        return prepare(args)
    return locked(args)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(json.dumps({"schema": "nando.s1c3e-error.v1", "error": str(error)}, sort_keys=True), file=os.sys.stderr)
        raise SystemExit(1)
