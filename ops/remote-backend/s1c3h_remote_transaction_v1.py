#!/usr/bin/env python3
"""Transactional S1C-3H runtime-authority compatibility installation."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import pwd
import re
import shutil
import stat
import subprocess
import time
import urllib.request
from pathlib import Path
from typing import Any

import s1c3g_remote_transaction_v1 as previous


PAPER_COMMIT = "1c50fea7119a123379bb7dca5a0eccbda63a9a7b"
PREFLIGHT_COMMIT = "be77371dff05b5eade9841e6612a59937648c2c8"
CANDIDATE_SOURCE_COMMIT = "03e3dd00c90206e2f705371318c50dd50537d6d8"
CANDIDATE_SOURCE_TREE = "06a9df51797dffc127fec41672bddae29c38bb92"
BASELINE_TRANSITION_SHA256 = "6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58"
BASELINE_AUTHORITY_SHA256 = "634f4aaeadeb5815ea1bf67a5ea76d63aae782dae63f8505d40f810e21ad2c3a"
BASELINE_CONFIG_SHA256 = "cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5"
BASELINE_RUNTIME_CONTRACT = "f8d955826f258388f7225086dc57e6557a0fa75bb94809ae7da2cb0e428bd55a"

TRANSITION_BINARY = Path("/opt/nando-wave/bin/nando-transition-serving")
AUTHORITY_BINARY = Path("/opt/nando-wave/bin/nando-response-admission")
TRANSITION_CONFIG = Path("/etc/nando-wave/roles/transition-serving.env")
STATE_DIR = Path("/var/lib/nando-wave/transition")
JOURNAL = STATE_DIR / "grounded-meaning-v1/decision-contract-precommits-v1"
GATE = Path("/opt/nando-wave/ops/phase-center-test-server/bin/nando-live-transition-gate")
GATE_PROFILE = Path(
    "/opt/nando-wave/ops/phase-center-test-server/gates/"
    "nando-live-transition-gate.profile.json"
)
STRUCTURAL_RECEIPT = GATE_PROFILE.parent / "receipts/STRUCTURAL_GATE_V2.json"
STRUCTURAL_RECEIPT_SHA256 = "e2da35480472aa246398b7d09d3a30a9feaefb15ff43be9fdc9d1ae68ae26ce8"
TRANSITION_UNIT = "nando-transition-serving.service"
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
COMPATIBILITY_FILES = (
    "response-registry.json",
    "response-admission-controller.json",
    "response-authority-candidate.json",
    "response-admission-controller.marker.json",
    "response-authority-sidecar-current-v2.json",
    "response-admission-controller-report.json",
    "admission.json",
)

PREPARATION_SCHEMA = "nando.s1c3h-preparation.v1"
PREDEPLOYMENT_SCHEMA = "nando.s1c3h-predeployment-verification.v1"
PENDING_SCHEMA = "nando.s1c3h-pending-receipt.v1"
RECEIPT_SCHEMA = "nando.s1c3h-deployment-receipt.v1"
STATE_SCHEMA = "nando.s1c3h-state.v1"
DIAGNOSTIC_SCHEMA = "nando.s1c3h-candidate-diagnostic.v1"
TRANSACTION_RE = re.compile(r"^\d{8}T\d{6}Z-[0-9a-f]{12}-s1c3h-v1$")
EXECUTION_STAGING_PARENT = STATE_DIR / ".s1c3h-authority-staging-v1"

GateFailure = previous.base.GateFailure
read_json = previous.base.read_json
write_json = previous.base.write_json
canonical_bytes = previous.base.canonical_bytes
sha256_bytes = previous.base.sha256_bytes
sha256_file = previous.base.sha256_file
fsync_directory = previous.base.fsync_directory
service_snapshot = previous.base.service_snapshot
health_snapshot = previous.base.health_snapshot
economics_snapshot = previous.base.economics_snapshot
process_environment = previous.base.process_environment
systemctl = previous.base.systemctl
journal_snapshot = previous.parent.journal_snapshot
require_valid_journal = previous.parent.require_valid_survival
require_prefix_preserved = previous.parent.require_prefix_preserved


def rooted(value: dict[str, Any], field: str) -> dict[str, Any]:
    value[field] = sha256_bytes(canonical_bytes(value, field))
    return value


def fsync_file(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def http_json(url: str) -> dict[str, Any]:
    with urllib.request.urlopen(url, timeout=4) as response:
        value = json.load(response)
    if not isinstance(value, dict):
        raise GateFailure(f"s1c3h_http_object:{url}")
    return value


def atomic_install(
    source: Path,
    destination: Path,
    mode: int | None = None,
    ownership: tuple[int, int] | None = None,
) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(f".{destination.name}.s1c3h-{os.getpid()}")
    shutil.copyfile(source, temporary)
    source_stat = source.stat()
    if ownership is not None:
        uid, gid = ownership
        selected_mode = stat.S_IMODE(source_stat.st_mode) if mode is None else mode
    elif destination.exists():
        target_stat = destination.stat()
        uid, gid = target_stat.st_uid, target_stat.st_gid
        selected_mode = stat.S_IMODE(target_stat.st_mode) if mode is None else mode
    else:
        uid, gid = source_stat.st_uid, source_stat.st_gid
        selected_mode = stat.S_IMODE(source_stat.st_mode) if mode is None else mode
    os.chown(temporary, uid, gid)
    os.chmod(temporary, selected_mode)
    fsync_file(temporary)
    if sha256_file(temporary) != sha256_file(source):
        temporary.unlink(missing_ok=True)
        raise GateFailure(f"s1c3h_staged_file_hash:{destination.name}")
    os.replace(temporary, destination)
    fsync_directory(destination.parent)
    if sha256_file(destination) != sha256_file(source):
        raise GateFailure(f"s1c3h_installed_file_hash:{destination.name}")


def copy_tree_atomic(source: Path, destination: Path) -> None:
    if destination.exists():
        if tree_manifest(source) != tree_manifest(destination):
            raise GateFailure(f"s1c3h_generation_collision:{destination.name}")
        return
    temporary = destination.with_name(f".{destination.name}.s1c3h-{os.getpid()}")
    shutil.copytree(source, temporary)
    for source_path in (source, *sorted(source.rglob("*"))):
        relative = source_path.relative_to(source)
        target_path = temporary if relative == Path(".") else temporary / relative
        source_stat = source_path.stat()
        os.chown(target_path, source_stat.st_uid, source_stat.st_gid)
        os.chmod(target_path, stat.S_IMODE(source_stat.st_mode))
        if target_path.is_file():
            fsync_file(target_path)
        elif target_path.is_dir():
            fsync_directory(target_path)
    fsync_directory(temporary)
    os.replace(temporary, destination)
    fsync_directory(destination.parent)
    if tree_manifest(source) != tree_manifest(destination):
        raise GateFailure("s1c3h_generation_install_mismatch")


def remove_directory(path: Path) -> None:
    if not path.exists():
        return
    if path.is_symlink() or not path.is_dir():
        raise GateFailure(f"s1c3h_directory_invalid:{path}")
    shutil.rmtree(path)
    fsync_directory(path.parent)


def tree_manifest(root: Path) -> list[dict[str, Any]]:
    if not root.is_dir() or root.is_symlink():
        raise GateFailure(f"s1c3h_tree_invalid:{root}")
    rows: list[dict[str, Any]] = []
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise GateFailure(f"s1c3h_tree_symlink:{path}")
        if path.is_file():
            value = path.stat()
            rows.append(
                {
                    "path": str(path.relative_to(root)),
                    "size_bytes": value.st_size,
                    "sha256": sha256_file(path),
                    "mode_octal": f"{stat.S_IMODE(value.st_mode):04o}",
                    "uid": value.st_uid,
                    "gid": value.st_gid,
                }
            )
    return rows


def file_manifest(paths: dict[str, Path]) -> dict[str, dict[str, Any]]:
    rows: dict[str, dict[str, Any]] = {}
    for name, path in sorted(paths.items()):
        value = path.stat()
        rows[name] = {
            "path": str(path),
            "size_bytes": value.st_size,
            "sha256": sha256_file(path),
            "mode_octal": f"{stat.S_IMODE(value.st_mode):04o}",
            "uid": value.st_uid,
            "gid": value.st_gid,
        }
    return rows


def compatibility_manifest(root: Path) -> dict[str, dict[str, Any]]:
    rows: dict[str, dict[str, Any]] = {}
    for name, path in sorted(compatibility_paths(root).items()):
        value = path.stat()
        rows[name] = {
            "size_bytes": value.st_size,
            "sha256": sha256_file(path),
            "mode_octal": f"{stat.S_IMODE(value.st_mode):04o}",
            "uid": value.st_uid,
            "gid": value.st_gid,
        }
    return rows


def runtime_contract(binary: Path) -> str:
    completed = subprocess.run(
        [str(binary), "--print-runtime-contract-sha256"],
        check=True,
        capture_output=True,
        timeout=10,
    )
    value = completed.stdout.decode("ascii", "strict").strip()
    if len(value) != 64 or any(character not in "0123456789abcdef" for character in value):
        raise GateFailure(f"s1c3h_runtime_contract_invalid:{binary.name}")
    return value


def pair_identity(transition: Path, authority: Path) -> dict[str, Any]:
    transition_contract = runtime_contract(transition)
    authority_contract = runtime_contract(authority)
    return {
        "transition_sha256": sha256_file(transition),
        "authority_sha256": sha256_file(authority),
        "transition_runtime_contract_sha256": transition_contract,
        "authority_runtime_contract_sha256": authority_contract,
        "pair_contract_equal": transition_contract == authority_contract,
    }


def unit_state(unit: str) -> dict[str, Any]:
    completed = subprocess.run(
        ["systemctl", "show", unit, "-p", "ActiveState", "-p", "SubState", "-p", "MainPID", "-p", "NRestarts"],
        check=True,
        capture_output=True,
        timeout=10,
    )
    fields = dict(
        line.split("=", 1)
        for line in completed.stdout.decode().splitlines()
        if "=" in line
    )
    return {
        "active_state": fields.get("ActiveState", ""),
        "sub_state": fields.get("SubState", ""),
        "main_pid": int(fields.get("MainPID", "0")),
        "nrestarts": int(fields.get("NRestarts", "-1")),
    }


def trigger_snapshot() -> dict[str, dict[str, Any]]:
    return {unit: unit_state(unit) for unit in (*TRIGGER_UNITS, *ONESHOT_UNITS)}


def require_expected_trigger_baseline(value: dict[str, dict[str, Any]]) -> None:
    for unit in TRIGGER_UNITS:
        if value[unit]["active_state"] != "active":
            raise GateFailure(f"s1c3h_trigger_baseline:{unit}")
    for unit in ONESHOT_UNITS:
        if value[unit]["active_state"] != "inactive":
            raise GateFailure(f"s1c3h_oneshot_baseline:{unit}")


def stable_trigger_baseline(timeout: float = 30.0) -> dict[str, dict[str, Any]]:
    deadline = time.monotonic() + timeout
    last: dict[str, dict[str, Any]] = {}
    while time.monotonic() < deadline:
        last = trigger_snapshot()
        try:
            require_expected_trigger_baseline(last)
            return last
        except GateFailure:
            time.sleep(0.2)
    raise GateFailure(f"s1c3h_trigger_baseline_timeout:{last}")


def nginx_pid() -> int:
    completed = subprocess.run(
        ["pgrep", "-o", "-x", "nginx"], check=False, capture_output=True, timeout=5
    )
    if completed.returncode != 0:
        return 0
    try:
        return int(completed.stdout.decode().strip().splitlines()[0])
    except (IndexError, ValueError):
        return 0


def pause_authority_triggers() -> None:
    for unit in TRIGGER_UNITS:
        systemctl("stop", unit, check=False)
    for unit in ONESHOT_UNITS:
        systemctl("stop", unit, check=False)
    for unit in (*TRIGGER_UNITS, *ONESHOT_UNITS):
        if unit_state(unit)["active_state"] not in {"inactive", "failed"}:
            raise GateFailure(f"s1c3h_trigger_not_stopped:{unit}")


def restore_authority_triggers(before: dict[str, dict[str, Any]]) -> None:
    for unit in TRIGGER_UNITS:
        if before[unit]["active_state"] == "active":
            systemctl("start", unit)
    for unit in TRIGGER_UNITS:
        expected = before[unit]["active_state"]
        actual = unit_state(unit)["active_state"]
        if actual != expected:
            raise GateFailure(f"s1c3h_trigger_restore:{unit}:{actual}:{expected}")


def require_trigger_state_restored(before: dict[str, dict[str, Any]]) -> None:
    after = trigger_snapshot()
    for unit in (*TRIGGER_UNITS, *ONESHOT_UNITS):
        if after[unit]["active_state"] != before[unit]["active_state"]:
            raise GateFailure(f"s1c3h_trigger_final_state:{unit}")


def wait_for_oneshots(timeout: float = 30.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        states = [unit_state(unit)["active_state"] for unit in ONESHOT_UNITS]
        if all(state in {"inactive", "failed"} for state in states):
            return
        time.sleep(0.2)
    raise GateFailure("s1c3h_oneshot_settle_timeout")


def stable_health() -> dict[str, Any]:
    return previous.stable_health_projection(health_snapshot())


def health_contract(snapshot: dict[str, Any], expected_runtime_contract: str) -> None:
    hot = http_json("http://127.0.0.1:18789/health")
    cpu = http_json("http://192.168.3.94:8787/cpu-health")
    expected = {
        "ok": True,
        "service": "nando-transition-serving",
        "mode": "CPU",
        "admission_verdict": "PASS",
        "response_executor_cache_ready": True,
        "response_active_profiles": 2,
        "response_runtime_contract_sha256": expected_runtime_contract,
    }
    for label, value in (("hot", hot), ("cpu", cpu)):
        actual = {field: value.get(field) for field in expected}
        if actual != expected:
            raise GateFailure(f"s1c3h_health_contract:{label}:{actual}")
    if snapshot["hot"]["stable"] != snapshot["cpu"]["stable"]:
        raise GateFailure("s1c3h_hot_cpu_projection_mismatch")


def wait_for_runtime(expected_contract: str, timeout: float = 30.0) -> tuple[dict[str, Any], dict[str, Any]]:
    deadline = time.monotonic() + timeout
    last_error = ""
    while time.monotonic() < deadline:
        try:
            services = service_snapshot()
            if services[TRANSITION_UNIT]["active_state"] != "active":
                raise GateFailure("transition_not_active")
            projected = stable_health()
            health_contract(projected, expected_contract)
            return services, projected
        except Exception as error:
            last_error = str(error)
            time.sleep(0.25)
    raise GateFailure(f"s1c3h_runtime_ready_timeout:{last_error}")


def compatibility_paths(root: Path) -> dict[str, Path]:
    return {name: root / name for name in COMPATIBILITY_FILES}


def verify_compatibility_files(root: Path) -> None:
    for name, path in compatibility_paths(root).items():
        if not path.is_file() or path.is_symlink():
            raise GateFailure(f"s1c3h_compatibility_file_missing:{name}")


def snapshot_compatibility(destination: Path) -> None:
    if destination.exists():
        raise GateFailure(f"s1c3h_compatibility_snapshot_exists:{destination.name}")
    temporary = destination.with_name(f".{destination.name}.s1c3h-{os.getpid()}")
    remove_directory(temporary)
    temporary.mkdir(mode=0o700)
    verify_compatibility_files(STATE_DIR)
    pointer = read_json(STATE_DIR / "response-authority-sidecar-current-v2.json")
    generation_root = pointer.get("generation_root_sha256")
    if not isinstance(generation_root, str) or len(generation_root) != 64:
        raise GateFailure("s1c3h_production_generation_root")
    generation_source = (
        STATE_DIR / "response-authority-sidecar-generations-v2" / generation_root
    )
    for name, source in compatibility_paths(STATE_DIR).items():
        target = temporary / name
        shutil.copy2(source, target)
        source_stat = source.stat()
        os.chown(target, source_stat.st_uid, source_stat.st_gid)
        os.chmod(target, stat.S_IMODE(source_stat.st_mode))
        fsync_file(target)
    generation_target = temporary / "generation"
    shutil.copytree(generation_source, generation_target)
    for source_path in (generation_source, *sorted(generation_source.rglob("*"))):
        relative = source_path.relative_to(generation_source)
        target_path = generation_target if relative == Path(".") else generation_target / relative
        source_stat = source_path.stat()
        os.chown(target_path, source_stat.st_uid, source_stat.st_gid)
        os.chmod(target_path, stat.S_IMODE(source_stat.st_mode))
        if target_path.is_file():
            fsync_file(target_path)
        elif target_path.is_dir():
            fsync_directory(target_path)
    fsync_directory(generation_target)
    manifest = rooted(
        {
            "schema": "nando.s1c3h-compatibility-snapshot.v1",
            "generation_root_sha256": generation_root,
            "compatibility_files": compatibility_manifest(temporary),
            "generation_manifest": tree_manifest(generation_target),
        },
        "snapshot_root_sha256",
    )
    write_json(temporary / "snapshot.json", manifest, 0o400)
    fsync_directory(temporary)
    os.replace(temporary, destination)
    fsync_directory(destination.parent)


def snapshot_installed_unit(root: Path) -> dict[str, dict[str, Any]]:
    destination = root / "evidence" / "installed-unit"
    remove_directory(destination)
    destination.mkdir(mode=0o700)
    sources = {
        "nando-transition-serving": TRANSITION_BINARY,
        "nando-response-admission": AUTHORITY_BINARY,
        "transition-serving.env": TRANSITION_CONFIG,
    }
    for name, source in sources.items():
        target = destination / name
        shutil.copy2(source, target)
        os.chown(target, 0, 0)
        os.chmod(target, 0o500 if not name.endswith(".env") else 0o400)
        fsync_file(target)
    fsync_directory(destination)
    return file_manifest({name: destination / name for name in sources})


def verify_compatibility_snapshot(source: Path) -> dict[str, Any]:
    value = read_json(source / "snapshot.json")
    if value.get("snapshot_root_sha256") != sha256_bytes(
        canonical_bytes(value, "snapshot_root_sha256")
    ):
        raise GateFailure("s1c3h_compatibility_snapshot_root")
    if value.get("compatibility_files") != compatibility_manifest(source):
        raise GateFailure("s1c3h_compatibility_snapshot_files")
    if value.get("generation_manifest") != tree_manifest(source / "generation"):
        raise GateFailure("s1c3h_compatibility_snapshot_generation")
    pointer = read_json(source / "response-authority-sidecar-current-v2.json")
    if pointer.get("generation_root_sha256") != value.get("generation_root_sha256"):
        raise GateFailure("s1c3h_compatibility_snapshot_pointer")
    return value


def backup_production(root: Path) -> None:
    rollback = root / "rollback"
    binaries = {
        "nando-transition-serving": TRANSITION_BINARY,
        "nando-response-admission": AUTHORITY_BINARY,
        "transition-serving.env": TRANSITION_CONFIG,
    }
    for name, source in binaries.items():
        destination = rollback / name
        shutil.copy2(source, destination)
        os.chmod(destination, 0o500 if name != "transition-serving.env" else 0o400)
        fsync_file(destination)
    snapshot_compatibility(rollback / "compatibility-prepared")
    fsync_directory(rollback)


def refresh_dynamic_rollback_backup(root: Path) -> None:
    snapshot_compatibility(root / "rollback" / "compatibility-frozen")


def run_as_e(arguments: list[str], environment: dict[str, str]) -> subprocess.CompletedProcess[bytes]:
    command = ["runuser", "-u", "e", "--", "env"]
    command.extend(f"{key}={value}" for key, value in sorted(environment.items()))
    command.extend(arguments)
    return subprocess.run(command, check=False, capture_output=True, timeout=60)


def execution_staging(root: Path) -> Path:
    return EXECUTION_STAGING_PARENT / root.name


def reset_staging(root: Path) -> Path:
    user = pwd.getpwnam("e")
    if EXECUTION_STAGING_PARENT.is_symlink():
        raise GateFailure("s1c3h_staging_parent_symlink")
    EXECUTION_STAGING_PARENT.mkdir(mode=0o700, exist_ok=True)
    if EXECUTION_STAGING_PARENT.is_symlink():
        raise GateFailure("s1c3h_staging_parent_symlink")
    os.chown(EXECUTION_STAGING_PARENT, user.pw_uid, user.pw_gid)
    os.chmod(EXECUTION_STAGING_PARENT, 0o700)
    staging = execution_staging(root)
    remove_directory(staging)
    staging.mkdir(mode=0o700)
    os.chown(staging, user.pw_uid, user.pw_gid)
    candidate = staging / "nando-response-admission"
    shutil.copyfile(root / "candidate-nando-response-admission", candidate)
    os.chown(candidate, user.pw_uid, user.pw_gid)
    os.chmod(candidate, 0o500)
    fsync_file(candidate)
    for name in (
        "response-registry.json",
        "response-admission-controller.json",
        "response-authority-candidate.json",
        "response-admission-controller.marker.json",
    ):
        destination = staging / name
        shutil.copy2(STATE_DIR / name, destination)
        os.chown(destination, user.pw_uid, user.pw_gid)
        os.chmod(destination, 0o600)
    fsync_directory(staging)
    return staging


def staged_profile(staging: Path, candidate_authority: Path) -> Path:
    profile = read_json(GATE_PROFILE)
    profile["response_runtime"]["registry"] = str(staging / "response-registry.json")
    profile["runtime"]["admission_status"] = str(staging / "admission.json")
    profile["deployment"]["response_runtime_build"] = str(candidate_authority)
    destination = staging / "live-transition-gate.profile.json"
    write_json(destination, profile, 0o600)
    user = pwd.getpwnam("e")
    os.chown(destination, user.pw_uid, user.pw_gid)
    receipt = read_json(STRUCTURAL_RECEIPT)
    if (
        sha256_file(STRUCTURAL_RECEIPT) != STRUCTURAL_RECEIPT_SHA256
        or receipt.get("verdict") != "PASS"
        or receipt.get("pass_count") != 4
        or receipt.get("route_count") != 4
        or receipt.get("blocked_routes") != []
    ):
        raise GateFailure("s1c3h_structural_receipt_invalid")
    receipt_directory = staging / "receipts"
    receipt_directory.mkdir(mode=0o700)
    os.chown(receipt_directory, user.pw_uid, user.pw_gid)
    receipt_destination = receipt_directory / "STRUCTURAL_GATE_V2.json"
    shutil.copyfile(STRUCTURAL_RECEIPT, receipt_destination)
    os.chown(receipt_destination, user.pw_uid, user.pw_gid)
    os.chmod(receipt_destination, 0o400)
    fsync_file(receipt_destination)
    fsync_directory(receipt_directory)
    return destination


def freeze_staging_evidence(root: Path, staging: Path, phase: str) -> None:
    destination = root / "evidence" / f"staged-authority-{phase}"
    remove_directory(destination)
    shutil.copytree(staging, destination, ignore=shutil.ignore_patterns("nando-response-admission"))
    for path in sorted(destination.rglob("*")):
        if path.is_file():
            os.chown(path, 0, 0)
            os.chmod(path, 0o400)
            fsync_file(path)
        elif path.is_dir():
            os.chown(path, 0, 0)
            os.chmod(path, 0o500)
            fsync_directory(path)
    os.chown(destination, 0, 0)
    os.chmod(destination, 0o500)
    fsync_directory(destination)
    fsync_directory(destination.parent)


def stage_candidate_authority(root: Path, phase: str) -> dict[str, Any]:
    staging = reset_staging(root)
    candidate_authority = staging / "nando-response-admission"
    environment = {
        "NANDO_TRANSITION_STATE_DIR": str(STATE_DIR),
        "NANDO_RESPONSE_REGISTRY": str(staging / "response-registry.json"),
        "NANDO_RESPONSE_CONTROLLER_ADMISSION_JSON": str(staging / "response-admission-controller.json"),
        "NANDO_RESPONSE_AUTHORITY_CANDIDATE": str(staging / "response-authority-candidate.json"),
        "NANDO_RESPONSE_ADMISSION_REPORT": str(staging / "response-admission-controller-report.json"),
        "NANDO_RESPONSE_ADMISSION_MARKER": str(staging / "response-admission-controller.marker.json"),
        "NANDO_RUNTIME_PACKAGE_REVOCATIONS": str(STATE_DIR / "runtime-package-revocations.json"),
        "NANDO_LIVE_TRANSITION_GATE_BUILD": str(GATE),
    }
    controller = run_as_e([str(candidate_authority)], environment)
    for stream, payload in (
        ("candidate-controller.stdout", controller.stdout),
        ("candidate-controller.stderr", controller.stderr),
    ):
        previous.base.atomic_write(root / "evidence" / stream, payload, 0o400)
    if controller.returncode != 0:
        raise GateFailure(f"s1c3h_staged_controller_exit:{controller.returncode}")
    profile = staged_profile(staging, candidate_authority)
    gate_environment = {
        "NANDO_LIVE_GATE_PROFILE": str(profile),
        "NANDO_RESPONSE_ADMISSION_BUILD": str(candidate_authority),
        "NANDO_TRANSITION_ADMISSION_JSON": str(staging / "admission.json"),
    }
    gate = run_as_e([str(GATE), "--status-mode", "--project-root", "/opt/nando-wave"], gate_environment)
    for stream, payload in (
        ("candidate-gate.stdout", gate.stdout),
        ("candidate-gate.stderr", gate.stderr),
    ):
        previous.base.atomic_write(root / "evidence" / stream, payload, 0o400)
    fsync_directory(root / "evidence")
    if gate.returncode != 0:
        raise GateFailure(f"s1c3h_staged_gate_exit:{gate.returncode}")
    contract = runtime_contract(candidate_authority)
    controller_admission = read_json(staging / "response-admission-controller.json")
    admission = read_json(staging / "admission.json")
    report = read_json(staging / "response-admission-controller-report.json")
    if report.get("verdict") != "PASS" or report.get("active_packages") != 2:
        raise GateFailure(f"s1c3h_staged_controller:{report}")
    for label, value in (("controller", controller_admission), ("admission", admission)):
        authority = value.get("response_authority", {})
        if (
            value.get("verdict") != "PASS"
            or value.get("eligible_for_local_accept") is not True
            or authority.get("runtime_build_sha256") != contract
            or len(authority.get("packages", [])) != 2
        ):
            raise GateFailure(f"s1c3h_staged_{label}_invalid")
    pointer = read_json(staging / "response-authority-sidecar-current-v2.json")
    generation_root = pointer.get("generation_root_sha256")
    generation = staging / "response-authority-sidecar-generations-v2" / str(generation_root)
    if not isinstance(generation_root, str) or len(generation_root) != 64:
        raise GateFailure("s1c3h_staged_generation_root")
    generation_manifest = tree_manifest(generation)
    freeze_staging_evidence(root, staging, phase)
    value = rooted(
        {
            "schema": "nando.s1c3h-staged-authority.v1",
            "runtime_contract_sha256": contract,
            "registry_revision": authority.get("registry_revision"),
            "registry_sha256": authority.get("registry_sha256"),
            "active_packages": len(authority.get("packages", [])),
            "generation_root_sha256": generation_root,
            "generation_manifest": generation_manifest,
            "staging_manifest": tree_manifest(staging),
            "structural_receipt_sha256": STRUCTURAL_RECEIPT_SHA256,
            "controller_report": report,
            "admission_expires_at_unix": admission.get("expires_at_unix"),
        },
        "staged_authority_root_sha256",
    )
    write_json(root / f"staged-authority-{phase}.json", value, 0o400)
    return value


def install_staged_authority(root: Path) -> None:
    staging = execution_staging(root)
    pointer = read_json(staging / "response-authority-sidecar-current-v2.json")
    generation_root = pointer["generation_root_sha256"]
    source_generation = staging / "response-authority-sidecar-generations-v2" / generation_root
    destination_generation = STATE_DIR / "response-authority-sidecar-generations-v2" / generation_root
    copy_tree_atomic(source_generation, destination_generation)
    for name in (
        "response-registry.json",
        "response-admission-controller.json",
        "response-authority-candidate.json",
        "response-admission-controller.marker.json",
        "response-admission-controller-report.json",
    ):
        atomic_install(staging / name, STATE_DIR / name, 0o600)
    atomic_install(
        staging / "response-authority-sidecar-current-v2.json",
        STATE_DIR / "response-authority-sidecar-current-v2.json",
        0o600,
    )
    atomic_install(staging / "admission.json", STATE_DIR / "admission.json", 0o600)


def restore_compatibility_files(root: Path) -> None:
    frozen = root / "rollback" / "compatibility-frozen"
    source = frozen if frozen.is_dir() else root / "rollback" / "compatibility-prepared"
    snapshot = verify_compatibility_snapshot(source)
    generation_root = snapshot["generation_root_sha256"]
    copy_tree_atomic(
        source / "generation",
        STATE_DIR / "response-authority-sidecar-generations-v2" / generation_root,
    )
    for name in (
        "response-registry.json",
        "response-admission-controller.json",
        "response-authority-candidate.json",
        "response-admission-controller.marker.json",
        "response-admission-controller-report.json",
    ):
        source_stat = (source / name).stat()
        atomic_install(
            source / name,
            STATE_DIR / name,
            stat.S_IMODE(source_stat.st_mode),
            (source_stat.st_uid, source_stat.st_gid),
        )
    pointer_stat = (source / "response-authority-sidecar-current-v2.json").stat()
    atomic_install(
        source / "response-authority-sidecar-current-v2.json",
        STATE_DIR / "response-authority-sidecar-current-v2.json",
        stat.S_IMODE(pointer_stat.st_mode),
        (pointer_stat.st_uid, pointer_stat.st_gid),
    )
    admission_stat = (source / "admission.json").stat()
    atomic_install(
        source / "admission.json",
        STATE_DIR / "admission.json",
        stat.S_IMODE(admission_stat.st_mode),
        (admission_stat.st_uid, admission_stat.st_gid),
    )


def validate_build_receipt(value: dict[str, Any]) -> None:
    root = value.get("build_receipt_root_sha256")
    if root != sha256_bytes(canonical_bytes(value, "build_receipt_root_sha256")):
        raise GateFailure("s1c3h_build_receipt_root")
    if value.get("source") != {
        "commit": CANDIDATE_SOURCE_COMMIT,
        "tree": CANDIDATE_SOURCE_TREE,
    }:
        raise GateFailure("s1c3h_build_source")
    pair = value.get("pair")
    if not isinstance(pair, dict) or pair.get("pair_contract_equal") is not True:
        raise GateFailure("s1c3h_build_pair")


def verify_current_production() -> dict[str, Any]:
    checks = (
        (sha256_file(TRANSITION_BINARY), BASELINE_TRANSITION_SHA256, "transition"),
        (sha256_file(AUTHORITY_BINARY), BASELINE_AUTHORITY_SHA256, "authority"),
        (sha256_file(TRANSITION_CONFIG), BASELINE_CONFIG_SHA256, "config"),
    )
    for actual, expected, label in checks:
        if actual != expected:
            raise GateFailure(f"STALE_BEFORE_MUTATION:{label}:{actual}")
    pair = pair_identity(TRANSITION_BINARY, AUTHORITY_BINARY)
    if not pair["pair_contract_equal"] or pair["transition_runtime_contract_sha256"] != BASELINE_RUNTIME_CONTRACT:
        raise GateFailure("STALE_BEFORE_MUTATION:runtime_contract")
    journal = journal_snapshot()
    require_valid_journal(journal)
    return {"pair": pair, "journal": journal}


def verify_implementation_freeze(value: dict[str, Any]) -> None:
    root = value.get("implementation_freeze_root_sha256")
    if root != sha256_bytes(canonical_bytes(value, "implementation_freeze_root_sha256")):
        raise GateFailure("s1c3h_implementation_freeze_root")
    paper = value.get("paper")
    preflight = value.get("preflight")
    if (
        not isinstance(paper, dict)
        or paper.get("commit") != PAPER_COMMIT
        or not isinstance(preflight, dict)
        or preflight.get("commit") != PREFLIGHT_COMMIT
    ):
        raise GateFailure("s1c3h_implementation_freeze_commit")


def prepare(args: argparse.Namespace) -> int:
    if os.geteuid() != 0:
        raise GateFailure("root_required")
    root = Path(args.transaction_directory)
    if not TRANSACTION_RE.fullmatch(args.transaction_id) or root.name != args.transaction_id:
        raise GateFailure("s1c3h_transaction_identity")
    if root.exists():
        raise GateFailure("s1c3h_transaction_exists")
    root.mkdir(parents=True, mode=0o700)
    (root / "rollback").mkdir(mode=0o700)
    (root / "evidence").mkdir(mode=0o700)
    try:
        baseline = verify_current_production()
        implementation_freeze = read_json(Path(args.implementation_freeze))
        verify_implementation_freeze(implementation_freeze)
        build_receipt = read_json(Path(args.build_receipt))
        validate_build_receipt(build_receipt)
        candidate_paths = {
            "candidate-nando-transition-serving": Path(args.candidate_transition),
            "candidate-nando-response-admission": Path(args.candidate_authority),
            "candidate-transition-serving.env": Path(args.candidate_config),
        }
        for name, source in candidate_paths.items():
            destination = root / name
            shutil.copy2(source, destination)
            os.chmod(destination, 0o500 if not name.endswith(".env") else 0o400)
            fsync_file(destination)
        candidate_pair = pair_identity(
            root / "candidate-nando-transition-serving",
            root / "candidate-nando-response-admission",
        )
        if candidate_pair != build_receipt.get("pair") or not candidate_pair["pair_contract_equal"]:
            raise GateFailure("s1c3h_candidate_build_binding")
        if sha256_file(root / "candidate-transition-serving.env") != build_receipt.get("config_sha256"):
            raise GateFailure("s1c3h_candidate_config_binding")
        services = service_snapshot()
        triggers = stable_trigger_baseline()
        projected_health = stable_health()
        health_contract(projected_health, BASELINE_RUNTIME_CONTRACT)
        economics = economics_snapshot()
        if economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
            raise GateFailure("s1c3h_baseline_economics")
        backup_production(root)
        shutil.copy2(Path(args.implementation_freeze), root / "implementation-freeze.json")
        shutil.copy2(Path(args.build_receipt), root / "candidate-build-receipt.json")
        os.chmod(root / "implementation-freeze.json", 0o400)
        os.chmod(root / "candidate-build-receipt.json", 0o400)
        staged = stage_candidate_authority(root, "prepared")
        preparation = rooted(
            {
                "schema": PREPARATION_SCHEMA,
                "transaction_id": args.transaction_id,
                "paper_commit": PAPER_COMMIT,
                "preflight_commit": PREFLIGHT_COMMIT,
                "implementation_freeze_root_sha256": implementation_freeze[
                    "implementation_freeze_root_sha256"
                ],
                "candidate_build_receipt_root_sha256": build_receipt[
                    "build_receipt_root_sha256"
                ],
                "candidate_source": build_receipt["source"],
                "baseline": baseline,
                "candidate_pair": candidate_pair,
                "candidate_config_sha256": build_receipt["config_sha256"],
                "staged_authority_root_sha256": staged["staged_authority_root_sha256"],
                "services_before": services,
                "triggers_before": triggers,
                "health_before": projected_health,
                "economics_before": economics,
                "nginx_pid_before": nginx_pid(),
                "prepared_at_unix": int(time.time()),
            },
            "preparation_root_sha256",
        )
        write_json(root / "preparation.json", preparation, 0o400)
        write_json(
            root / "transaction-state.json",
            {"schema": STATE_SCHEMA, "state": "PREPARED", "transaction_id": args.transaction_id},
            0o600,
        )
        fsync_directory(root)
        print(json.dumps({"state": "PREPARED", "preparation_root_sha256": preparation["preparation_root_sha256"]}, sort_keys=True))
        return 0
    except Exception as error:
        failure = rooted(
            {
                "schema": "nando.s1c3h-preflight-failure.v1",
                "transaction_id": args.transaction_id,
                "error": str(error),
                "production_mutation": False,
                "observed_at_unix": int(time.time()),
            },
            "preflight_failure_root_sha256",
        )
        write_json(root / "preflight-failure.json", failure, 0o400)
        write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "PREFLIGHT_FAILURE", "transaction_id": args.transaction_id}, 0o600)
        fsync_directory(root)
        remove_directory(execution_staging(root))
        raise


def verify_predeployment(root: Path, path: Path) -> dict[str, Any]:
    value = read_json(path)
    preparation = read_json(root / "preparation.json")
    if value.get("predeployment_verification_root_sha256") != sha256_bytes(
        canonical_bytes(value, "predeployment_verification_root_sha256")
    ):
        raise GateFailure("s1c3h_predeployment_root")
    expected = {
        "schema": PREDEPLOYMENT_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": "S1C3H_PREPARATION_PASS",
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "implementation_freeze_root_sha256": preparation["implementation_freeze_root_sha256"],
        "candidate_build_receipt_root_sha256": preparation["candidate_build_receipt_root_sha256"],
    }
    if {key: value.get(key) for key in expected} != expected:
        raise GateFailure("s1c3h_predeployment_mismatch")
    return value


def persist_candidate_diagnostic(root: Path, stage: str, error: str) -> dict[str, Any]:
    health: dict[str, Any]
    try:
        health = {
            "projection": stable_health(),
            "hot": http_json("http://127.0.0.1:18789/health"),
        }
    except Exception as health_error:
        health = {"error": str(health_error)}
    try:
        logs = subprocess.run(
            ["journalctl", "-u", TRANSITION_UNIT, "--since", "-2 minutes", "--no-pager", "-o", "cat"],
            check=False,
            capture_output=True,
            timeout=10,
        ).stdout
        previous.base.atomic_write(root / "evidence" / "candidate-startup.log", logs, 0o400)
    except Exception as log_error:
        previous.base.atomic_write(
            root / "evidence" / "candidate-startup-log-error.txt",
            str(log_error).encode(),
            0o400,
        )
    pair: dict[str, Any]
    try:
        pair = pair_identity(TRANSITION_BINARY, AUTHORITY_BINARY)
    except Exception as pair_error:
        pair = {"error": str(pair_error)}
    diagnostic = rooted(
        {
            "schema": DIAGNOSTIC_SCHEMA,
            "stage": stage,
            "error": error,
            "observed_at_unix": int(time.time()),
            "installed_pair": pair,
            "installed_config_sha256": sha256_file(TRANSITION_CONFIG),
            "health": health,
            "journal": journal_snapshot(),
            "staged_authority": read_json(root / "staged-authority-frozen.json")
            if (root / "staged-authority-frozen.json").is_file()
            else read_json(root / "staged-authority-prepared.json")
            if (root / "staged-authority-prepared.json").is_file()
            else None,
        },
        "diagnostic_root_sha256",
    )
    write_json(root / "candidate-diagnostic.json", diagnostic, 0o400)
    fsync_directory(root)
    return diagnostic


def persist_minimal_diagnostic(
    root: Path, stage: str, error: str, diagnostic_error: str
) -> dict[str, Any]:
    value = rooted(
        {
            "schema": DIAGNOSTIC_SCHEMA,
            "stage": stage,
            "error": error,
            "diagnostic_error": diagnostic_error,
            "observed_at_unix": int(time.time()),
            "installed_files": file_manifest(
                {
                    "transition": TRANSITION_BINARY,
                    "authority": AUTHORITY_BINARY,
                    "config": TRANSITION_CONFIG,
                }
            ),
            "extended_diagnostic_available": False,
        },
        "diagnostic_root_sha256",
    )
    write_json(root / "candidate-diagnostic.json", value, 0o400)
    fsync_directory(root)
    return value


def fault_after(stage: str) -> None:
    if os.environ.get("NANDO_S1C3H_FAULT_AFTER") == stage:
        raise GateFailure(f"s1c3h_injected_fault:{stage}")


def rollback(root: Path, reason: str, diagnostic: dict[str, Any] | None = None) -> None:
    state = read_json(root / "transaction-state.json").get("state")
    if state not in {
        "ROLLBACK_ARMED",
        "MUTATION_STARTED",
        "AUTHORITY_INSTALLED",
        "RUNTIME_INSTALLED",
        "FINALIZE_PENDING",
        "FINAL_VERIFICATION_PENDING",
    }:
        raise GateFailure(f"s1c3h_rollback_state:{state}")
    preparation = read_json(root / "preparation.json")
    if diagnostic is None:
        diagnostic = persist_candidate_diagnostic(root, f"manual_rollback:{state}", reason)
    pause_authority_triggers()
    if state != "ROLLBACK_ARMED":
        systemctl("stop", TRANSITION_UNIT, check=False)
        atomic_install(root / "rollback" / "nando-response-admission", AUTHORITY_BINARY, 0o755)
        restore_compatibility_files(root)
        atomic_install(root / "rollback" / "transition-serving.env", TRANSITION_CONFIG, 0o644)
        atomic_install(root / "rollback" / "nando-transition-serving", TRANSITION_BINARY, 0o755)
    pair = pair_identity(TRANSITION_BINARY, AUTHORITY_BINARY)
    if (
        pair["transition_sha256"] != BASELINE_TRANSITION_SHA256
        or pair["authority_sha256"] != BASELINE_AUTHORITY_SHA256
        or pair["transition_runtime_contract_sha256"] != BASELINE_RUNTIME_CONTRACT
        or not pair["pair_contract_equal"]
    ):
        raise GateFailure("s1c3h_rollback_pair_mismatch")
    if state != "ROLLBACK_ARMED":
        systemctl("start", TRANSITION_UNIT)
    services_after, health_after = wait_for_runtime(BASELINE_RUNTIME_CONTRACT)
    restore_authority_triggers(preparation["triggers_before"])
    wait_for_oneshots()
    require_trigger_state_restored(preparation["triggers_before"])
    services_survival, health_survival = wait_for_runtime(BASELINE_RUNTIME_CONTRACT)
    journal_after = journal_snapshot(preparation["baseline"]["journal"])
    require_prefix_preserved(preparation["baseline"]["journal"], journal_after)
    economics = economics_snapshot()
    if economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
        raise GateFailure("s1c3h_rollback_economics")
    pending = rooted(
        {
            "schema": PENDING_SCHEMA,
            "verdict": "S1C3H_ROLLBACK_PASS",
            "rollback_reason": reason,
            "diagnostic_root_sha256": diagnostic["diagnostic_root_sha256"],
            "installed_pair": pair,
            "installed_config_sha256": sha256_file(TRANSITION_CONFIG),
            "services_after": services_after,
            "services_survival": services_survival,
            "health_after": health_after,
            "health_survival": health_survival,
            "triggers_after": trigger_snapshot(),
            "journal_after": journal_after,
            "economics": economics,
            "nginx_pid_after": nginx_pid(),
            "capture_installed": False,
            "scientific_authority": False,
        },
        "pending_root_sha256",
    )
    write_json(root / "pending-receipt.json", pending, 0o600)
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "ROLLBACK_PENDING", "transaction_id": preparation["transaction_id"]}, 0o600)


def execute(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    state = read_json(root / "transaction-state.json").get("state")
    if state != "PREPARED":
        raise GateFailure(f"s1c3h_execute_state:{state}")
    verify_predeployment(root, Path(args.predeployment_verification))
    preparation = read_json(root / "preparation.json")
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "ROLLBACK_ARMED", "transaction_id": preparation["transaction_id"]}, 0o600)
    fsync_directory(root)
    stage = "rollback_armed"
    try:
        pause_authority_triggers()
        stage = "triggers_paused"
        refresh_dynamic_rollback_backup(root)
        staged = stage_candidate_authority(root, "frozen")
        stage = "authority_staged"
        write_json(
            root / "transaction-state.json",
            {
                "schema": STATE_SCHEMA,
                "state": "MUTATION_STARTED",
                "transaction_id": preparation["transaction_id"],
            },
            0o600,
        )
        fsync_directory(root)
        systemctl("stop", TRANSITION_UNIT)
        stage = "runtime_stopped"
        fault_after(stage)
        atomic_install(root / "candidate-nando-response-admission", AUTHORITY_BINARY, 0o755)
        stage = "authority_binary_installed"
        fault_after(stage)
        install_staged_authority(root)
        write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "AUTHORITY_INSTALLED", "transaction_id": preparation["transaction_id"]}, 0o600)
        stage = "authority_installed"
        fault_after(stage)
        atomic_install(root / "candidate-transition-serving.env", TRANSITION_CONFIG, 0o644)
        stage = "config_installed"
        fault_after(stage)
        atomic_install(root / "candidate-nando-transition-serving", TRANSITION_BINARY, 0o755)
        stage = "transition_binary_installed"
        fault_after(stage)
        installed_pair = pair_identity(TRANSITION_BINARY, AUTHORITY_BINARY)
        if installed_pair != preparation["candidate_pair"]:
            raise GateFailure("s1c3h_installed_candidate_pair")
        write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "RUNTIME_INSTALLED", "transaction_id": preparation["transaction_id"]}, 0o600)
        stage = "runtime_installed"
        fault_after(stage)
        systemctl("start", TRANSITION_UNIT)
        stage = "runtime_started"
        fault_after(stage)
        services_after, health_after = wait_for_runtime(
            installed_pair["transition_runtime_contract_sha256"]
        )
        new_pid = services_after[TRANSITION_UNIT]["main_pid"]
        environment = process_environment(new_pid)
        expected_environment = {
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
            "NANDO_GROUNDED_DECISION_JOURNAL": str(JOURNAL),
        }
        if {key: environment.get(key) for key in expected_environment} != expected_environment:
            raise GateFailure("s1c3h_capture_environment")
        restore_authority_triggers(preparation["triggers_before"])
        wait_for_oneshots()
        require_trigger_state_restored(preparation["triggers_before"])
        stage = "triggers_restored"
        fault_after(stage)
        opening_expiry = staged["admission_expires_at_unix"]
        deadline = time.monotonic() + 50
        renewed_admission: dict[str, Any] | None = None
        while time.monotonic() < deadline:
            candidate = read_json(STATE_DIR / "admission.json")
            authority = candidate.get("response_authority", {})
            if (
                candidate.get("verdict") == "PASS"
                and candidate.get("expires_at_unix", 0) > opening_expiry
                and authority.get("runtime_build_sha256")
                == installed_pair["transition_runtime_contract_sha256"]
                and len(authority.get("packages", [])) == 2
            ):
                renewed_admission = candidate
                break
            time.sleep(0.5)
        if renewed_admission is None:
            raise GateFailure("s1c3h_authority_renewal_timeout")
        stage = "authority_renewed"
        time.sleep(15)
        services_survival, health_survival = wait_for_runtime(
            installed_pair["transition_runtime_contract_sha256"]
        )
        if services_survival[TRANSITION_UNIT]["main_pid"] != new_pid:
            raise GateFailure("s1c3h_runtime_pid_changed")
        pause_authority_triggers()
        stage = "installed_authority_quiesced"
        installed_snapshot_path = root / "evidence" / "installed-compatibility"
        snapshot_compatibility(installed_snapshot_path)
        installed_snapshot = verify_compatibility_snapshot(installed_snapshot_path)
        installed_unit = snapshot_installed_unit(root)
        restore_authority_triggers(preparation["triggers_before"])
        wait_for_oneshots()
        require_trigger_state_restored(preparation["triggers_before"])
        stage = "installed_authority_snapshotted"
        journal_after = journal_snapshot(preparation["baseline"]["journal"])
        require_prefix_preserved(preparation["baseline"]["journal"], journal_after)
        economics = economics_snapshot()
        if economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
            raise GateFailure("s1c3h_candidate_economics")
        diagnostic = persist_candidate_diagnostic(root, "candidate_pass", "")
        pending = rooted(
            {
                "schema": PENDING_SCHEMA,
                "verdict": "S1C3H_DEPLOYMENT_PASS",
                "staged_authority_root_sha256": staged["staged_authority_root_sha256"],
                "diagnostic_root_sha256": diagnostic["diagnostic_root_sha256"],
                "installed_compatibility_snapshot_root_sha256": installed_snapshot[
                    "snapshot_root_sha256"
                ],
                "installed_unit": installed_unit,
                "installed_pair": installed_pair,
                "installed_config_sha256": sha256_file(TRANSITION_CONFIG),
                "services_after": services_after,
                "services_survival": services_survival,
                "health_after": health_after,
                "health_survival": health_survival,
                "triggers_after": trigger_snapshot(),
                "authority_renewal": {
                    "opening_expires_at_unix": opening_expiry,
                    "renewed_expires_at_unix": renewed_admission["expires_at_unix"],
                    "runtime_contract_sha256": renewed_admission["response_authority"]["runtime_build_sha256"],
                },
                "capture_environment": expected_environment,
                "journal_after": journal_after,
                "natural_record_count": journal_after.get("record_count", 0),
                "economics": economics,
                "nginx_pid_after": nginx_pid(),
                "capture_installed": True,
                "scientific_authority": False,
            },
            "pending_root_sha256",
        )
        write_json(root / "pending-receipt.json", pending, 0o600)
        write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "FINALIZE_PENDING", "transaction_id": preparation["transaction_id"]}, 0o600)
        return 0
    except Exception as error:
        try:
            diagnostic = persist_candidate_diagnostic(root, stage, str(error))
        except Exception as diagnostic_error:
            previous.base.atomic_write(
                root / "diagnostic-persistence-error.txt",
                f"stage={stage}\nerror={error}\ndiagnostic_error={diagnostic_error}\n".encode(),
                0o400,
            )
            diagnostic = persist_minimal_diagnostic(
                root, stage, str(error), str(diagnostic_error)
            )
        rollback(root, str(error), diagnostic)
        raise GateFailure(f"S1C3H_ROLLBACK_PASS:{error}") from error


def abort_predeployment(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    state = read_json(root / "transaction-state.json").get("state")
    if state not in {"PREPARED", "PREFLIGHT_FAILURE"}:
        raise GateFailure(f"s1c3h_abort_state:{state}")
    production = verify_current_production()
    if state == "PREFLIGHT_FAILURE":
        failure_path = root / "preflight-failure.json"
        failure = read_json(failure_path)
        if "preflight_failure_root_sha256" not in failure:
            if set(failure) != {"schema", "error"} or failure.get("schema") != "nando.s1c3h-preflight-failure.v1":
                raise GateFailure("s1c3h_legacy_preflight_failure_shape")
            original = root / "preflight-failure.unrooted.json"
            if not original.exists():
                shutil.copy2(failure_path, original)
                os.chmod(original, 0o400)
                fsync_file(original)
            failure = rooted(
                {
                    "schema": "nando.s1c3h-preflight-failure.v1",
                    "transaction_id": read_json(root / "transaction-state.json")[
                        "transaction_id"
                    ],
                    "error": failure["error"],
                    "production_mutation": False,
                    "legacy_unrooted_sha256": sha256_file(original),
                    "observed_at_unix": int(time.time()),
                },
                "preflight_failure_root_sha256",
            )
            write_json(failure_path, failure, 0o400)
            fsync_directory(root)
        if failure.get("preflight_failure_root_sha256") != sha256_bytes(
            canonical_bytes(failure, "preflight_failure_root_sha256")
        ):
            raise GateFailure("s1c3h_preflight_failure_root")
        transaction_id = failure["transaction_id"]
        reason = failure["error"]
        failure_root = failure["preflight_failure_root_sha256"]
    else:
        transaction_id = read_json(root / "preparation.json")["transaction_id"]
        reason = args.reason
        failure_root = None
    terminal = rooted(
        {
            "schema": STATE_SCHEMA,
            "state": "COMPLETE",
            "transaction_id": transaction_id,
            "verdict": "S1C3H_PREFLIGHT_FAILURE",
            "reason": reason,
            "preflight_failure_root_sha256": failure_root,
            "production_mutation": False,
            "production_pair": production["pair"],
            "journal": production["journal"],
            "capture_installed": False,
            "scientific_authority": False,
        },
        "state_root_sha256",
    )
    write_json(root / "s1c3h-state.json", terminal, 0o400)
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "COMPLETE", "transaction_id": terminal["transaction_id"]}, 0o600)
    fsync_directory(root)
    remove_directory(execution_staging(root))
    return 0


def finalize(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    state = read_json(root / "transaction-state.json").get("state")
    if state not in {"FINALIZE_PENDING", "ROLLBACK_PENDING"}:
        raise GateFailure(f"s1c3h_finalize_state:{state}")
    preparation = read_json(root / "preparation.json")
    pending = read_json(root / "pending-receipt.json")
    connector_before = read_json(Path(args.connector_before))
    connector_after = read_json(Path(args.connector_after))
    connector_fields = ("active_state", "main_pid", "nrestarts", "command_sha256")
    if any(connector_before.get(field) != connector_after.get(field) for field in connector_fields):
        if state == "FINALIZE_PENDING":
            diagnostic = persist_candidate_diagnostic(root, "connector_veto", "connector_changed")
            rollback(root, "connector_changed", diagnostic)
            pending = read_json(root / "pending-receipt.json")
            state = "ROLLBACK_PENDING"
        else:
            raise GateFailure("s1c3h_connector_changed_after_rollback")
    if connector_after.get("route_receipt_failures") != connector_before.get("route_receipt_failures"):
        if state == "FINALIZE_PENDING":
            diagnostic = persist_candidate_diagnostic(root, "connector_veto", "route_receipt_failures_changed")
            rollback(root, "route_receipt_failures_changed", diagnostic)
            pending = read_json(root / "pending-receipt.json")
        else:
            raise GateFailure("s1c3h_route_receipt_failures_after_rollback")
    receipt = rooted(
        {
            "schema": RECEIPT_SCHEMA,
            "transaction_id": preparation["transaction_id"],
            "verdict": pending["verdict"],
            "source": {
                "implementation_commit": read_json(root / "implementation-freeze.json")["source_commit"],
                "implementation_tree": read_json(root / "implementation-freeze.json")["source_tree"],
                "candidate_commit": CANDIDATE_SOURCE_COMMIT,
                "candidate_tree": CANDIDATE_SOURCE_TREE,
            },
            "preparation_root_sha256": preparation["preparation_root_sha256"],
            "pending_root_sha256": pending["pending_root_sha256"],
            "diagnostic_root_sha256": pending["diagnostic_root_sha256"],
            "installed_compatibility_snapshot_root_sha256": pending.get(
                "installed_compatibility_snapshot_root_sha256"
            ),
            "installed_pair": pending["installed_pair"],
            "installed_unit": pending.get("installed_unit"),
            "installed_config_sha256": pending["installed_config_sha256"],
            "capture_installed": pending["capture_installed"],
            "capture_environment": pending.get("capture_environment"),
            "authority_renewal": pending.get("authority_renewal"),
            "natural_record_count": pending.get("natural_record_count", 0),
            "scientific_authority": False,
            "journal_after": pending["journal_after"],
            "economics": pending["economics"],
            "nginx_pid_before": preparation["nginx_pid_before"],
            "nginx_pid_after": pending["nginx_pid_after"],
            "connector_before": connector_before,
            "connector_after": connector_after,
            "triggers_after": pending["triggers_after"],
            "services_survival": pending["services_survival"],
            "finalized_at_unix": int(time.time()),
        },
        "receipt_root_sha256",
    )
    write_json(root / "deployment-receipt.json", receipt, 0o400)
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "FINAL_VERIFICATION_PENDING", "transaction_id": preparation["transaction_id"]}, 0o600)
    return 0


def seal(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    state = read_json(root / "transaction-state.json").get("state")
    if state != "FINAL_VERIFICATION_PENDING":
        raise GateFailure(f"s1c3h_seal_state:{state}")
    receipt = read_json(root / "deployment-receipt.json")
    verification = read_json(Path(args.final_verification))
    if verification.get("final_verification_root_sha256") != sha256_bytes(
        canonical_bytes(verification, "final_verification_root_sha256")
    ):
        raise GateFailure("s1c3h_final_verification_root")
    if (
        verification.get("valid") is not True
        or verification.get("authority") is not True
        or verification.get("receipt_root_sha256") != receipt["receipt_root_sha256"]
        or verification.get("verdict") != receipt["verdict"]
    ):
        raise GateFailure("s1c3h_final_verification_mismatch")
    shutil.copy2(Path(args.final_verification), root / "final-verification.json")
    os.chmod(root / "final-verification.json", 0o400)
    terminal = rooted(
        {
            "schema": STATE_SCHEMA,
            "state": "COMPLETE",
            "transaction_id": receipt["transaction_id"],
            "verdict": receipt["verdict"],
            "receipt_root_sha256": receipt["receipt_root_sha256"],
            "final_verification_root_sha256": verification["final_verification_root_sha256"],
            "capture_installed": receipt["capture_installed"],
            "natural_record_count": receipt["natural_record_count"],
            "scientific_authority": False,
        },
        "state_root_sha256",
    )
    write_json(root / "s1c3h-state.json", terminal, 0o400)
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "COMPLETE", "transaction_id": receipt["transaction_id"]}, 0o600)
    remove_directory(execution_staging(root))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    prepare_parser = commands.add_parser("prepare")
    prepare_parser.add_argument("--transaction-id", required=True)
    prepare_parser.add_argument("--transaction-directory", required=True)
    prepare_parser.add_argument("--candidate-transition", required=True)
    prepare_parser.add_argument("--candidate-authority", required=True)
    prepare_parser.add_argument("--candidate-config", required=True)
    prepare_parser.add_argument("--build-receipt", required=True)
    prepare_parser.add_argument("--implementation-freeze", required=True)
    execute_parser = commands.add_parser("execute")
    execute_parser.add_argument("--transaction-directory", required=True)
    execute_parser.add_argument("--predeployment-verification", required=True)
    rollback_parser = commands.add_parser("rollback")
    rollback_parser.add_argument("--transaction-directory", required=True)
    rollback_parser.add_argument("--reason", required=True)
    finalize_parser = commands.add_parser("finalize")
    finalize_parser.add_argument("--transaction-directory", required=True)
    finalize_parser.add_argument("--connector-before", required=True)
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
        print(json.dumps({"schema": "nando.s1c3h-error.v1", "error": str(error)}, sort_keys=True), file=os.sys.stderr)
        raise SystemExit(1)
