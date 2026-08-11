#!/usr/bin/env python3
"""Root-only remote executor for the preregistered S1C-3 V2 transaction."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


PAPER_COMMIT = "33380e3110e021a6c2d959ba7e04492e79e5093a"
PAPER_MANIFEST_ROOT = "0cdd508be964ab485a72e0984a8c424d041d3a28a88af55f36e0d72b4e25ac5c"
PAPER_VERIFICATION_SHA256 = "1e871a6acfd7067b2dda94d3c46faa053e4cfde4e801623962cf77aaa7773603"
CANDIDATE_COMMIT = "a3ea27a49af397ef79e5c9ec80089ecf53a41d59"
CANDIDATE_TREE = "670d9c4ed170a76f107db13262abcd7cc035578e"
CARGO_LOCK_SHA256 = "0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1"
CANDIDATE_CONFIG_SHA256 = "1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6"
BASELINE_COMMIT = "663959064a37caf7eb917fc99dfedb6386355fa6"
BASELINE_TREE = "05460ccbc9c44ac8b7174318903c0211de709e2e"
BASELINE_RECEIPT = Path("/var/lib/nando-wave/deployments/20260810T214210Z-663959064a37/deployment-receipt.json")
BASELINE_RECEIPT_ROOT = "785450d76037410d96baade19c2b6bb7f0fb24c6be034e2166be5533c7dd985b"
BASELINE_BINARY_SHA256 = "6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58"
BASELINE_CONFIG_SHA256 = "cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5"
UNIT_SHA256 = "6e9d2fe41b1db95f94768d1ab41dffce1f15be92e2f774832c7fe392bb77b135"
PHASE_CONFIG_SHA256 = "5c019cebbde083f963c03619ff1d938786f5b4ec58730dddd5b34adeb33cce31"
AUTHORITY_CONFIG_SHA256 = "d40b7262ff6d744a393b0fc03a5d06610d01728aa2f4603199ca8567189ec88f"

TRANSITION_UNIT = "nando-transition-serving.service"
UNTOUCHED_UNITS = (
    "nando-transport-gateway.service",
    "nando-response-learning.service",
    "nando-gateway-control.service",
    "nando-operator-certification-authority.service",
)
ALL_UNITS = (TRANSITION_UNIT, *UNTOUCHED_UNITS)
PRODUCTION_BINARY = Path("/opt/nando-wave/bin/nando-transition-serving")
PRODUCTION_CONFIG = Path("/etc/nando-wave/roles/transition-serving.env")
UNIT_FILE = Path("/etc/systemd/system/nando-transition-serving.service")
PHASE_CONFIG = Path("/etc/nando-wave/phase-center.env")
AUTHORITY_CONFIG = Path("/etc/nando-wave/authority.env")
JOURNAL = Path("/var/lib/nando-wave/transition/grounded-meaning-v1/decision-contract-precommits-v1")
ECONOMICS = Path("/var/lib/nando-wave/transition/economics-live.json")
RESPONSE_REGISTRY = Path("/var/lib/nando-wave/transition/response-registry.json")
ADMISSION = Path("/var/lib/nando-wave/transition/admission.json")

HOT_TEST = "package::tests::capture_disabled_compatibility_latency_stays_within_hot_budget"
IDLE_TEST = "package::tests::capture_disabled_executor_has_no_sustained_idle_cpu_work"
SINGLE_SYNC_TEST = "grounded_decision_capture::tests::durable_sync_path_stays_within_budget_and_rotates_exactly"
THREE_SYNC_TEST = "grounded_decision_capture::tests::three_ledger_sync_path_stays_within_eligible_budget"

HOT_RE = re.compile(
    r"S1C_HOT_LATENCY matched_p99_ns=(\d+) no_goal_p99_ns=(\d+) hard_max_ns=(\d+) samples=(\d+)"
)
SYNC_RE = re.compile(
    r"S1C_SYNC_LATENCY p99_ns=(\d+) hard_max_ns=(\d+) records=(\d+) segments=(\d+)"
)
THREE_SYNC_RE = re.compile(
    r"S1C2_SYNC_LATENCY p99_ns=(\d+) hard_max_ns=(\d+) records=(\d+)"
)
IDLE_RE = re.compile(
    r"S1C_IDLE_CPU elapsed_ticks=(\d+) ticks_per_second=(\d+) percent_of_one_core=([0-9.]+)"
)
TEST_OK_RE = re.compile(r"test result: ok\. 1 passed; 0 failed;")

QUIESCENCE_SCHEMA = "nando.s1c3-quiescence-receipt.v2"
CONTAMINATION_SCHEMA = "nando.s1c3-measurement-contamination-receipt.v2"
RESOURCE_SCHEMA = "nando.s1c3-resource-receipt.v2"
PARITY_SCHEMA = "nando.s1c3-parity-receipt.v2"
PREPARATION_SCHEMA = "nando.s1c3-transaction-preparation.v2"
RECEIPT_SCHEMA = "nando.s1c3-transaction-receipt.v2"
STATE_SCHEMA = "nando.s1c3-state.v2"
PENDING_SCHEMA = "nando.s1c3-pending-receipt.v2"

FORBIDDEN_BUILD_NAMES = (
    "cargo", "rustc", "sccache", "cc", "cc1", "cc1plus", "gcc", "g++",
    "clang", "clang++", "ld", "ld.lld", "lld", "mold", "ninja", "make",
    "cmake", "meson",
)
QUIESCENCE_MAX_WAIT_SECONDS = 1800
QUIESCENCE_REQUIRED_INTERVALS = 30
QUIESCENCE_INTERVAL_SECONDS = 1.0
QUIESCENCE_INTERVAL_MIN_SECONDS = 0.90
QUIESCENCE_INTERVAL_MAX_SECONDS = 1.50
QUIESCENCE_CPU_MAX_PERCENT = 20.0
QUIESCENCE_CPU_MEAN_MAX_PERCENT = 5.0
QUIESCENCE_IO_SOME_AVG10_MAX = 0.20
QUIESCENCE_IO_FULL_AVG10_MAX = 0.05
MONITOR_INTERVAL_SECONDS = 0.5
MONITOR_MAX_GAP_SECONDS = 2.0
MEASUREMENT_LABELS = (
    "hot-1", "hot-2", "hot-3",
    "single-sync-1", "single-sync-2", "single-sync-3",
    "three-sync-1", "three-sync-2", "three-sync-3",
    "idle", "rss-capture_off", "rss-capture_on",
    "parity-baseline", "parity-candidate",
)


class GateFailure(RuntimeError):
    pass


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path, *, limit: int | None = None) -> str:
    digest = hashlib.sha256()
    remaining = limit
    with path.open("rb") as handle:
        while True:
            size = 1024 * 1024 if remaining is None else min(1024 * 1024, remaining)
            if size <= 0:
                break
            chunk = handle.read(size)
            if not chunk:
                break
            digest.update(chunk)
            if remaining is not None:
                remaining -= len(chunk)
    return digest.hexdigest()


def forbidden_payload_bytes(path: Path, markers: tuple[bytes, ...]) -> int:
    maximum = max(map(len, markers))
    overlap = b""
    total = 0
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            payload = overlap + chunk
            total += sum(
                (payload.count(marker) - overlap.count(marker)) * len(marker)
                for marker in markers
            )
            overlap = payload[-(maximum - 1):] if maximum > 1 else b""
    return total


def canonical_bytes(value: Any, root_field: str | None = None) -> bytes:
    if root_field is not None:
        value = dict(value)
        value.pop(root_field, None)
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def add_root(value: dict[str, Any], field: str) -> dict[str, Any]:
    value = dict(value)
    value[field] = sha256_bytes(canonical_bytes(value, field))
    return value


def atomic_write(path: Path, data: bytes, mode: int = 0o400) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    descriptor = os.open(temporary, flags, 0o600)
    try:
        with os.fdopen(descriptor, "wb", closefd=False) as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
    finally:
        os.close(descriptor)
    os.chmod(temporary, mode)
    os.replace(temporary, path)
    directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def write_json(path: Path, value: Any, mode: int = 0o400) -> None:
    atomic_write(path, canonical_bytes(value), mode)


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise GateFailure(f"not_json_object:{path}")
    return value


def run(
    command: list[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    check: bool = True,
    timeout: int | None = None,
    log: Path | None = None,
    as_user: bool = False,
) -> subprocess.CompletedProcess[bytes]:
    actual = command
    actual_env = os.environ.copy()
    if env:
        actual_env.update(env)
    if as_user:
        exported = [f"{key}={value}" for key, value in sorted((env or {}).items())]
        actual = ["sudo", "-u", "e", "-H", "env", "PATH=/home/e/.cargo/bin:/usr/local/bin:/usr/bin:/bin", *exported, *command]
        actual_env = os.environ.copy()
    completed = subprocess.run(
        actual,
        cwd=cwd,
        env=actual_env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
        check=False,
    )
    if log is not None:
        atomic_write(log, completed.stdout, 0o400)
    if check and completed.returncode != 0:
        tail = completed.stdout.decode("utf-8", "replace")[-2000:]
        raise GateFailure(f"command_failed:{command[0]}:{completed.returncode}:{tail}")
    return completed


def systemctl(*args: str, check: bool = True) -> subprocess.CompletedProcess[bytes]:
    return run(["systemctl", *args], check=check, timeout=30)


def fsync_path(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def fragment_hash(unit: str) -> str:
    path = run(["systemctl", "show", unit, "-p", "FragmentPath", "--value"], timeout=5).stdout.decode().strip()
    if not path:
        raise GateFailure(f"fragment_missing:{unit}")
    return sha256_file(Path(path))


def service_snapshot() -> dict[str, Any]:
    snapshot: dict[str, Any] = {}
    for unit in ALL_UNITS:
        output = run(
            ["systemctl", "show", unit, "-p", "ActiveState", "-p", "SubState", "-p", "MainPID", "-p", "NRestarts"],
            timeout=5,
        ).stdout.decode()
        fields = dict(line.split("=", 1) for line in output.splitlines() if "=" in line)
        snapshot[unit] = {
            "active_state": fields.get("ActiveState", ""),
            "sub_state": fields.get("SubState", ""),
            "main_pid": int(fields.get("MainPID", "0")),
            "nrestarts": int(fields.get("NRestarts", "-1")),
            "fragment_sha256": fragment_hash(unit),
        }
    return snapshot


def require_active(snapshot: dict[str, Any]) -> None:
    for unit, state in snapshot.items():
        if state["active_state"] != "active" or state["main_pid"] <= 0 or state["nrestarts"] < 0:
            raise GateFailure(f"service_not_active:{unit}:{state}")


def http_json(url: str) -> dict[str, Any]:
    with urllib.request.urlopen(url, timeout=4) as response:
        value = json.loads(response.read())
    if not isinstance(value, dict) or value.get("ok") is not True:
        raise GateFailure(f"health_not_ok:{url}")
    return value


def health_snapshot() -> dict[str, Any]:
    urls = {
        "hot": "http://127.0.0.1:18789/health",
        "control": "http://127.0.0.1:18788/health",
        "gateway": "http://192.168.3.94:8787/health",
        "cpu": "http://192.168.3.94:8787/cpu-health",
    }
    result: dict[str, Any] = {}
    for label, url in urls.items():
        raw = http_json(url)
        semantic = {
            "ok": raw.get("ok"),
            "service": raw.get("service"),
            "mode": raw.get("mode"),
            "admission_verdict": raw.get("admission_verdict"),
            "transition_active_profiles": raw.get("transition_active_profiles"),
            "response_active_profiles": raw.get("response_active_profiles"),
            "response_executor_cache_ready": raw.get("response_executor_cache_ready"),
        }
        result[label] = {
            "url": url,
            "raw_sha256": sha256_bytes(canonical_bytes(raw)),
            "semantic": semantic,
            "semantic_root_sha256": sha256_bytes(canonical_bytes(semantic)),
        }
    return result


def economics_snapshot() -> dict[str, int]:
    value = read_json(ECONOMICS)
    false_accepts = value.get("false_accepts")
    parity = value.get("runtime_parity_mismatches")
    if type(false_accepts) is not int or type(parity) is not int:
        raise GateFailure("economics_counters_invalid")
    return {"false_accepts": false_accepts, "runtime_parity_mismatches": parity}


def route_probe() -> dict[str, Any]:
    body = b'{"model":"s1c3-parity-probe","input":"S1C3_ROUTE_PROBE_NO_MATCH"}'
    request = urllib.request.Request(
        "http://127.0.0.1:18789/v1/responses",
        data=body,
        headers={"content-type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=4) as response:
            status = response.status
            response_body = response.read()
    except urllib.error.HTTPError as error:
        status = error.code
        response_body = error.read()
    if status != 418:
        raise GateFailure(f"route_probe_status:{status}")
    return {"status": status, "body_sha256": sha256_bytes(response_body), "body_size": len(response_body)}


def journal_snapshot(*, prefix_reference: dict[str, Any] | None = None) -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    total = 0
    raw_payload_bytes = 0
    forbidden_markers = (
        b'"input"',
        b"request_text",
        b"raw_request",
        b"S1C_RAW_REQUEST_MARKER_MUST_NOT_PERSIST",
    )
    if JOURNAL.exists():
        for path in sorted(JOURNAL.rglob("*")):
            if path.is_symlink():
                raise GateFailure(f"journal_symlink:{path}")
            if not path.is_file():
                continue
            relative = path.relative_to(JOURNAL).as_posix()
            size = path.stat().st_size
            total += size
            raw_payload_bytes += forbidden_payload_bytes(path, forbidden_markers)
            entry = {"path": relative, "size_bytes": size, "sha256": sha256_file(path)}
            if prefix_reference is not None:
                old = next((item for item in prefix_reference.get("entries", []) if item["path"] == relative), None)
                if old is not None:
                    if size < old["size_bytes"] or sha256_file(path, limit=old["size_bytes"]) != old["sha256"]:
                        raise GateFailure(f"journal_prefix_changed:{relative}")
            entries.append(entry)
    manifest = "".join(f'{entry["sha256"]} {entry["size_bytes"]} {entry["path"]}\n' for entry in entries).encode()
    return {
        "present": JOURNAL.exists(),
        "entries": entries,
        "total_bytes": total,
        "manifest_root_sha256": sha256_bytes(manifest),
        "raw_payload_bytes": raw_payload_bytes,
        "preserved_prefixes": prefix_reference is None or all(
            any(current["path"] == old["path"] for current in entries)
            for old in prefix_reference.get("entries", [])
        ),
    }


def process_rss(pid: int) -> int:
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        if line.startswith("VmRSS:"):
            return int(line.split()[1]) * 1024
    raise GateFailure(f"rss_missing:{pid}")


def process_environment(pid: int) -> dict[str, str]:
    values: dict[str, str] = {}
    for item in Path(f"/proc/{pid}/environ").read_bytes().split(b"\0"):
        if b"=" in item:
            key, value = item.split(b"=", 1)
            values[key.decode()] = value.decode("utf-8", "strict")
    return values


def parse_env_file(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        key, value = line.split("=", 1)
        result[key] = value.strip("'\"")
    return result


def install_pair(binary: Path, config: Path) -> None:
    binary_tmp = PRODUCTION_BINARY.with_name(f".{PRODUCTION_BINARY.name}.s1c3-{os.getpid()}")
    config_tmp = PRODUCTION_CONFIG.with_name(f".{PRODUCTION_CONFIG.name}.s1c3-{os.getpid()}")
    shutil.copyfile(binary, binary_tmp)
    shutil.copyfile(config, config_tmp)
    os.chmod(binary_tmp, 0o755)
    os.chmod(config_tmp, 0o644)
    fsync_path(binary_tmp)
    fsync_path(config_tmp)
    if sha256_file(binary_tmp) != sha256_file(binary) or sha256_file(config_tmp) != sha256_file(config):
        raise GateFailure("staged_pair_hash_mismatch")
    os.replace(config_tmp, PRODUCTION_CONFIG)
    fsync_directory(PRODUCTION_CONFIG.parent)
    os.replace(binary_tmp, PRODUCTION_BINARY)
    fsync_directory(PRODUCTION_BINARY.parent)
    if sha256_file(PRODUCTION_BINARY) != sha256_file(binary) or sha256_file(PRODUCTION_CONFIG) != sha256_file(config):
        raise GateFailure("installed_pair_hash_mismatch")


def wait_for_service(timeout: float = 20.0) -> tuple[dict[str, Any], dict[str, Any]]:
    deadline = time.monotonic() + timeout
    last_error = ""
    while time.monotonic() < deadline:
        try:
            services = service_snapshot()
            require_active(services)
            health = health_snapshot()
            return services, health
        except Exception as error:  # bounded readiness poll
            last_error = str(error)
            time.sleep(0.25)
    raise GateFailure(f"service_readiness_timeout:{last_error}")


def write_evidence(root: Path, name: str, data: bytes) -> None:
    atomic_write(root / "evidence" / name, data, 0o400)


def executable_identity(path: Path, source_identity: str) -> dict[str, Any]:
    mode = path.stat().st_mode & 0o7777
    if not path.is_file() or not os.access(path, os.X_OK):
        raise GateFailure(f"measurement_executable_invalid:{path}")
    return {
        "path": str(path),
        "sha256": sha256_file(path),
        "size_bytes": path.stat().st_size,
        "mode_octal": f"{mode:04o}",
        "source_identity": source_identity,
    }


def isolate_observer_cpus() -> None:
    available = set(os.sched_getaffinity(0))
    observer = available - {4, 5}
    if not observer:
        raise GateFailure(f"observer_cpu_affinity_unavailable:{sorted(available)}")
    os.sched_setaffinity(0, observer)


def build_process_snapshot() -> dict[str, Any]:
    forbidden = set(FORBIDDEN_BUILD_NAMES)
    matches: list[dict[str, Any]] = []
    races = 0
    for entry in sorted(Path("/proc").iterdir(), key=lambda path: path.name):
        if not entry.name.isdigit():
            continue
        try:
            comm = (entry / "comm").read_text(encoding="utf-8").strip()
            executable = os.path.basename(os.readlink(entry / "exe"))
        except (FileNotFoundError, ProcessLookupError):
            races += 1
            continue
        except (OSError, UnicodeError):
            races += 1
            continue
        if comm in forbidden or executable in forbidden:
            matches.append({"pid": int(entry.name), "comm": comm, "executable_basename": executable})
    return {"matches": matches, "process_races": races}


def cpu_counters(cpu: int = 4) -> tuple[int, int]:
    prefix = f"cpu{cpu} "
    for line in Path("/proc/stat").read_text(encoding="ascii").splitlines():
        if line.startswith(prefix):
            fields = [int(value) for value in line.split()[1:]]
            total = sum(fields)
            idle = fields[3] + (fields[4] if len(fields) > 4 else 0)
            return total, idle
    raise GateFailure(f"cpu_stat_missing:{cpu}")


def io_pressure() -> dict[str, dict[str, float | int]]:
    result: dict[str, dict[str, float | int]] = {}
    for line in Path("/proc/pressure/io").read_text(encoding="ascii").splitlines():
        parts = line.split()
        values: dict[str, float | int] = {}
        for item in parts[1:]:
            key, raw = item.split("=", 1)
            values[key] = int(raw) if key == "total" else float(raw)
        result[parts[0]] = values
    if set(result) != {"some", "full"}:
        raise GateFailure("io_pressure_shape_invalid")
    return result


def host_observation(label: str) -> dict[str, Any]:
    processes = build_process_snapshot()
    total, idle = cpu_counters()
    return {
        "label": label,
        "observed_at": utc_now(),
        "monotonic_ns": time.monotonic_ns(),
        "cpu4_total": total,
        "cpu4_idle": idle,
        "io_pressure": io_pressure(),
        "loadavg": Path("/proc/loadavg").read_text(encoding="ascii").strip(),
        "build_processes": processes["matches"],
        "process_races": processes["process_races"],
    }


def interval_sample(start: dict[str, Any], end: dict[str, Any]) -> dict[str, Any]:
    elapsed = (end["monotonic_ns"] - start["monotonic_ns"]) / 1_000_000_000
    total_delta = end["cpu4_total"] - start["cpu4_total"]
    idle_delta = end["cpu4_idle"] - start["cpu4_idle"]
    if total_delta <= 0 or idle_delta < 0 or idle_delta > total_delta:
        raise GateFailure("cpu4_counter_delta_invalid")
    busy = 100.0 * (total_delta - idle_delta) / total_delta
    some = float(end["io_pressure"]["some"]["avg10"])
    full = float(end["io_pressure"]["full"]["avg10"])
    eligible = (
        not start["build_processes"]
        and not end["build_processes"]
        and QUIESCENCE_INTERVAL_MIN_SECONDS <= elapsed <= QUIESCENCE_INTERVAL_MAX_SECONDS
        and busy <= QUIESCENCE_CPU_MAX_PERCENT
        and some <= QUIESCENCE_IO_SOME_AVG10_MAX
        and full <= QUIESCENCE_IO_FULL_AVG10_MAX
    )
    return {
        "started_at": start["observed_at"],
        "ended_at": end["observed_at"],
        "start_monotonic_ns": start["monotonic_ns"],
        "end_monotonic_ns": end["monotonic_ns"],
        "interval_seconds": elapsed,
        "cpu4_busy_percent": busy,
        "io_some_avg10": some,
        "io_full_avg10": full,
        "build_processes_start": start["build_processes"],
        "build_processes_end": end["build_processes"],
        "process_races": start["process_races"] + end["process_races"],
        "loadavg_end": end["loadavg"],
        "eligible_base": eligible,
    }


def wait_for_quiescence(
    transaction_id: str,
    executables: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    started = time.monotonic()
    started_at = utc_now()
    attempted: list[dict[str, Any]] = []
    consecutive: list[dict[str, Any]] = []
    previous = host_observation("quiescence-start")
    while time.monotonic() - started < QUIESCENCE_MAX_WAIT_SECONDS:
        time.sleep(QUIESCENCE_INTERVAL_SECONDS)
        current = host_observation("quiescence-sample")
        sample = interval_sample(previous, current)
        attempted.append(sample)
        previous = current
        if sample["eligible_base"]:
            consecutive.append(sample)
        else:
            consecutive.clear()
        if len(consecutive) == QUIESCENCE_REQUIRED_INTERVALS:
            mean_busy = sum(row["cpu4_busy_percent"] for row in consecutive) / len(consecutive)
            if mean_busy <= QUIESCENCE_CPU_MEAN_MAX_PERCENT:
                window = list(consecutive)
                window_root = sha256_bytes(canonical_bytes(window))
                receipt = {
                    "schema": QUIESCENCE_SCHEMA,
                    "transaction_id": transaction_id,
                    "candidate_commit": CANDIDATE_COMMIT,
                    "candidate_tree": CANDIDATE_TREE,
                    "detector_schema": "proc-comm-exe-basename-v1",
                    "forbidden_build_names": list(FORBIDDEN_BUILD_NAMES),
                    "maximum_wait_seconds": QUIESCENCE_MAX_WAIT_SECONDS,
                    "required_intervals": QUIESCENCE_REQUIRED_INTERVALS,
                    "thresholds": {
                        "interval_min_seconds": QUIESCENCE_INTERVAL_MIN_SECONDS,
                        "interval_max_seconds": QUIESCENCE_INTERVAL_MAX_SECONDS,
                        "cpu4_max_percent": QUIESCENCE_CPU_MAX_PERCENT,
                        "cpu4_mean_max_percent": QUIESCENCE_CPU_MEAN_MAX_PERCENT,
                        "io_some_avg10_max": QUIESCENCE_IO_SOME_AVG10_MAX,
                        "io_full_avg10_max": QUIESCENCE_IO_FULL_AVG10_MAX,
                    },
                    "eligibility_started_at": started_at,
                    "eligibility_reached_at": utc_now(),
                    "attempted_samples": attempted,
                    "eligible_window": window,
                    "eligible_cpu4_mean_percent": mean_busy,
                    "eligible_window_root_sha256": window_root,
                    "executables": executables,
                }
                return add_root(receipt, "quiescence_root_sha256")
            consecutive.clear()
    raise GateFailure("INVALID_ENVIRONMENT_QUIESCENCE_TIMEOUT")


class MeasurementMonitor:
    def __init__(self, transaction_id: str, quiescence_root: str, executable_root: str) -> None:
        self.transaction_id = transaction_id
        self.quiescence_root = quiescence_root
        self.executable_root = executable_root
        self.samples: list[dict[str, Any]] = []
        self.errors: list[str] = []
        self.boundaries: list[dict[str, Any]] = []
        self.current_label = "before-first-metric"
        self.stop_event = threading.Event()
        self.lock = threading.Lock()
        self.thread = threading.Thread(target=self._loop, name="s1c3-v2-monitor", daemon=True)

    def _sample(self, kind: str) -> None:
        try:
            row = host_observation(self.current_label)
            row["kind"] = kind
            with self.lock:
                self.samples.append(row)
        except Exception as error:  # fail-closed monitoring path
            with self.lock:
                self.errors.append(f"{type(error).__name__}:{error}")

    def _loop(self) -> None:
        while not self.stop_event.wait(MONITOR_INTERVAL_SECONDS):
            self._sample("periodic")

    def start(self) -> None:
        self._sample("monitor-start")
        self.thread.start()

    def boundary(self, label: str, phase: str) -> None:
        self.current_label = label
        self._sample(f"boundary-{phase}")
        self.boundaries.append({"label": label, "phase": phase, "observed_at": utc_now()})

    def finish(self) -> dict[str, Any]:
        self.stop_event.set()
        self.thread.join(timeout=5)
        if self.thread.is_alive():
            self.errors.append("monitor_thread_did_not_stop")
        self._sample("monitor-stop")
        ordered = sorted(self.samples, key=lambda row: row["monotonic_ns"])
        gaps = [
            (right["monotonic_ns"] - left["monotonic_ns"]) / 1_000_000_000
            for left, right in zip(ordered, ordered[1:])
        ]
        matches = [
            {"sample_index": index, "process": process}
            for index, row in enumerate(ordered)
            for process in row["build_processes"]
        ]
        labels = [row["label"] for row in self.boundaries if row["phase"] == "before"]
        receipt = {
            "schema": CONTAMINATION_SCHEMA,
            "transaction_id": self.transaction_id,
            "quiescence_root_sha256": self.quiescence_root,
            "executable_set_root_sha256": self.executable_root,
            "monitor_interval_seconds": MONITOR_INTERVAL_SECONDS,
            "maximum_sample_gap_seconds": MONITOR_MAX_GAP_SECONDS,
            "observed_max_sample_gap_seconds": max(gaps, default=0.0),
            "metric_labels": labels,
            "boundaries": self.boundaries,
            "samples": ordered,
            "forbidden_process_matches": matches,
            "monitor_errors": self.errors,
            "contaminated": bool(matches or self.errors or max(gaps, default=0.0) > MONITOR_MAX_GAP_SECONDS),
        }
        return add_root(receipt, "measurement_contamination_root_sha256")


def prebuild_test_harness(
    source: Path,
    target: Path,
    package: str,
    log: Path,
) -> Path:
    completed = run(
        [
            "/home/e/.cargo/bin/cargo", "test", "--release", "--locked", "-p", package,
            "--lib", "--no-run", "--message-format=json",
        ],
        cwd=source,
        env={"CARGO_TARGET_DIR": str(target)},
        as_user=True,
        timeout=1800,
        log=log,
    )
    expected_target = package.replace("-", "_")
    matches: list[Path] = []
    for line in completed.stdout.decode("utf-8", "replace").splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        target_info = message.get("target", {})
        profile = message.get("profile", {})
        executable = message.get("executable")
        if (
            message.get("reason") == "compiler-artifact"
            and target_info.get("name") == expected_target
            and "lib" in target_info.get("kind", [])
            and profile.get("test") is True
            and isinstance(executable, str)
        ):
            matches.append(Path(executable))
    unique = sorted(set(matches))
    if len(unique) != 1:
        raise GateFailure(f"test_harness_selection_invalid:{package}:{unique}")
    return unique[0]


def run_test_binary(
    executable: Path,
    source: Path,
    test: str,
    label: str,
    evidence: Path,
    timeout: int,
    monitor: MeasurementMonitor,
) -> str:
    monitor.boundary(label, "before")
    try:
        completed = run(
            [
                "taskset", "-c", "4", str(executable), test, "--ignored", "--exact",
                "--nocapture", "--test-threads=1",
            ],
            cwd=source,
            env={"RUST_TEST_THREADS": "1"},
            as_user=True,
            timeout=timeout,
            log=evidence / f"{label}.log",
        )
    finally:
        monitor.boundary(label, "after")
    output = completed.stdout.decode("utf-8", "replace")
    if not TEST_OK_RE.search(output):
        raise GateFailure(f"direct_test_count_invalid:{label}")
    return output


def make_oracle(oracle_source: Path, source: Path, root: Path, label: str) -> Path:
    crate = root / f"oracle-{label}"
    (crate / "src").mkdir(parents=True)
    shutil.copyfile(oracle_source, crate / "src" / "main.rs")
    manifest = f'''[package]\nname = "s1c3-parity-{label}"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nnando-response-actor = {{ path = "{source / "crates/nando-response-actor"}" }}\nserde_json = "1"\n'''
    atomic_write(crate / "Cargo.toml", manifest.encode(), 0o644)
    return crate / "Cargo.toml"


def prebuild_oracle(oracle_source: Path, source: Path, work: Path, label: str, evidence: Path) -> Path:
    manifest = make_oracle(oracle_source, source, work, label)
    target = work / f"oracle-target-{label}"
    run(
        ["/home/e/.cargo/bin/cargo", "build", "--release", "--quiet", "--manifest-path", str(manifest)],
        env={"CARGO_TARGET_DIR": str(target), "RUSTFLAGS": "-Awarnings"},
        as_user=True,
        timeout=1200,
        log=evidence / f"parity-{label}-build.log",
    )
    executable = target / "release" / f"s1c3-parity-{label}"
    if not executable.is_file():
        raise GateFailure(f"parity_oracle_missing:{label}")
    return executable


def run_parity(
    oracles: dict[str, Path],
    work: Path,
    evidence: Path,
    monitor: MeasurementMonitor,
    executable_identities: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    fixture = work / "parity-fixture"
    fixture.mkdir()
    shutil.copy2(RESPONSE_REGISTRY, fixture / "response-registry.json")
    shutil.copy2(ADMISSION, fixture / "admission.json")
    outputs: dict[str, bytes] = {}
    for label in ("baseline", "candidate"):
        metric_label = f"parity-{label}"
        monitor.boundary(metric_label, "before")
        try:
            completed = run(
                [str(oracles[label]), str(fixture / "response-registry.json"), str(fixture / "admission.json")],
                as_user=True,
                timeout=1200,
                log=evidence / f"parity-{label}.log",
            )
        finally:
            monitor.boundary(metric_label, "after")
        outputs[label] = completed.stdout
    identical = outputs["baseline"] == outputs["candidate"]
    rows = len(outputs["candidate"].splitlines())
    if not identical or rows != 16:
        raise GateFailure(f"parity_failed:identical={identical}:rows={rows}")
    receipt = {
        "schema": PARITY_SCHEMA,
        "baseline_output_sha256": sha256_bytes(outputs["baseline"]),
        "candidate_output_sha256": sha256_bytes(outputs["candidate"]),
        "byte_identical": identical,
        "rows": rows,
        "direct_exec_only": True,
        "baseline_oracle_sha256": executable_identities["parity-baseline"]["sha256"],
        "candidate_oracle_sha256": executable_identities["parity-candidate"]["sha256"],
    }
    return add_root(receipt, "parity_root_sha256")


def isolated_environment(config: dict[str, str], state: Path, port: int, capture: bool) -> dict[str, str]:
    state.mkdir(parents=True)
    shutil.chown(state, user="e", group="e")
    for source in (RESPONSE_REGISTRY, ADMISSION, Path("/var/lib/nando-wave/transition/registry.json")):
        if source.exists():
            destination = state / source.name
            shutil.copy2(source, destination)
            shutil.chown(destination, user="e", group="e")
    environment = dict(config)
    environment.update({
        "NANDO_TRANSITION_STATE_DIR": str(state),
        "NANDO_TRANSITION_SERVING_BIND": f"127.0.0.1:{port}",
        "NANDO_LOCAL_ACCEPT_ENABLED": "0",
        "NANDO_CLIENT_ALLOW_LOCAL_ACCEPT": "0",
        "NANDO_GATEWAY_CPU_ROUTE_READY": "0",
        "NANDO_EMBEDDED_RESPONSE_MINER_ENABLED": "0",
        "NANDO_GENERIC_RESPONSE_MINER_ENABLED": "0",
        "NANDO_MULTI_SOURCE_RESEARCH_ENABLED": "0",
        "NANDO_K1_NATURAL_SCHEDULER_ENABLED": "0",
        "NANDO_PROVIDER_CAPTURE_ENABLED": "0",
        "NANDO_OPERATOR_GENERATION_SHADOW_ENABLED": "0",
        "NANDO_LEARNING_EVIDENCE_BRIDGE_PRODUCER_ENABLED": "0",
        "NANDO_LEARNING_EVIDENCE_BRIDGE_CONSUMER_ENABLED": "0",
        "NANDO_LEARNING_STRUCTURE_BRIDGE_PRODUCER_ENABLED": "0",
        "NANDO_LEARNING_STRUCTURE_BRIDGE_CONSUMER_ENABLED": "0",
        "NANDO_OPPORTUNITY_BRIDGE_PRODUCER_ENABLED": "0",
        "NANDO_OPPORTUNITY_BRIDGE_CONSUMER_ENABLED": "0",
        "NANDO_REMOTE_EVIDENCE_SPOOL_ENABLED": "0",
        "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1" if capture else "0",
        "NANDO_GROUNDED_DECISION_JOURNAL": str(state / "grounded-journal"),
        "HOME": "/home/e",
        "USER": "e",
        "PATH": "/home/e/.cargo/bin:/usr/local/bin:/usr/bin:/bin",
    })
    return environment


def measure_isolated_rss(
    binary: Path,
    config: dict[str, str],
    work: Path,
    evidence: Path,
    monitor: MeasurementMonitor,
) -> dict[str, int]:
    import pwd

    account = pwd.getpwnam("e")

    def demote() -> None:
        os.initgroups(account.pw_name, account.pw_gid)
        os.setgid(account.pw_gid)
        os.setuid(account.pw_uid)

    samples: dict[str, int] = {}
    for index, capture in enumerate((False, True)):
        label = "capture_on" if capture else "capture_off"
        metric_label = f"rss-{label}"
        state = work / f"rss-{label}"
        environment = isolated_environment(config, state, 19871 + index, capture)
        log_path = evidence / f"rss-{label}.log"
        monitor.boundary(metric_label, "before")
        with log_path.open("wb") as log:
            process = subprocess.Popen(
                ["taskset", "-c", "5", str(binary)],
                env=environment,
                stdout=log,
                stderr=subprocess.STDOUT,
                preexec_fn=demote,
            )
            try:
                deadline = time.monotonic() + 20
                while time.monotonic() < deadline:
                    if process.poll() is not None:
                        raise GateFailure(f"rss_process_exited:{label}:{process.returncode}")
                    try:
                        http_json(f"http://127.0.0.1:{19871 + index}/health")
                        break
                    except Exception:
                        time.sleep(0.25)
                else:
                    raise GateFailure(f"rss_health_timeout:{label}")
                for _ in range(10):
                    http_json(f"http://127.0.0.1:{19871 + index}/health")
                rss_samples = []
                for _ in range(20):
                    rss_samples.append(process_rss(process.pid))
                    time.sleep(0.1)
                samples[label] = max(rss_samples)
            finally:
                if process.poll() is None:
                    process.send_signal(signal.SIGTERM)
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
        monitor.boundary(metric_label, "after")
        os.chmod(log_path, 0o400)
    delta = max(0, samples["capture_on"] - samples["capture_off"])
    if delta > 16 * 1024 * 1024:
        raise GateFailure(f"rss_delta_exceeded:{delta}")
    return {"capture_off_bytes": samples["capture_off"], "capture_on_bytes": samples["capture_on"], "delta_bytes": delta}


def run_resources(
    candidate_binary: Path,
    source: Path,
    harnesses: dict[str, Path],
    config: dict[str, str],
    work: Path,
    evidence: Path,
    monitor: MeasurementMonitor,
) -> dict[str, Any]:
    hot_runs = []
    single_runs = []
    three_runs = []
    for index in range(1, 4):
        output = run_test_binary(
            harnesses["response-actor"], source, HOT_TEST, f"hot-{index}", evidence, 600, monitor
        )
        match = HOT_RE.search(output)
        if not match:
            raise GateFailure(f"hot_metric_missing:{index}")
        matched, no_goal, hard_max, samples = map(int, match.groups())
        hot_runs.append({"p99_ns": matched, "no_goal_p99_ns": no_goal, "hard_max_ns": hard_max, "samples": samples})
    for index in range(1, 4):
        output = run_test_binary(
            harnesses["transition-serving"], source, SINGLE_SYNC_TEST,
            f"single-sync-{index}", evidence, 900, monitor
        )
        match = SYNC_RE.search(output)
        if not match:
            raise GateFailure(f"single_sync_metric_missing:{index}")
        p99, hard_max, samples, segments = map(int, match.groups())
        single_runs.append({"p99_ns": p99, "hard_max_ns": hard_max, "samples": samples, "segments": segments})
    for index in range(1, 4):
        output = run_test_binary(
            harnesses["transition-serving"], source, THREE_SYNC_TEST,
            f"three-sync-{index}", evidence, 900, monitor
        )
        match = THREE_SYNC_RE.search(output)
        if not match:
            raise GateFailure(f"three_sync_metric_missing:{index}")
        p99, hard_max, samples = map(int, match.groups())
        three_runs.append({"p99_ns": p99, "hard_max_ns": hard_max, "samples": samples})
    idle_output = run_test_binary(
        harnesses["response-actor"], source, IDLE_TEST, "idle", evidence, 180, monitor
    )
    idle_match = IDLE_RE.search(idle_output)
    if not idle_match:
        raise GateFailure("idle_metric_missing")
    elapsed, ticks, percent = idle_match.groups()
    idle = {"elapsed_ticks": int(elapsed), "ticks_per_second": int(ticks), "percent_of_one_core": float(percent)}
    rss = measure_isolated_rss(candidate_binary, config, work, evidence, monitor)
    all_pass = (
        all(run["p99_ns"] <= 1_000_000 and run["no_goal_p99_ns"] <= 250_000 and run["hard_max_ns"] <= 2_000_000 and run["samples"] == 4096 for run in hot_runs)
        and all(run["p99_ns"] <= 5_000_000 and run["hard_max_ns"] <= 20_000_000 and run["samples"] == 1024 for run in single_runs)
        and all(run["p99_ns"] <= 5_000_000 and run["hard_max_ns"] <= 20_000_000 and run["samples"] == 256 for run in three_runs)
        and idle["percent_of_one_core"] <= 0.25
        and rss["delta_bytes"] <= 16 * 1024 * 1024
    )
    if not all_pass:
        raise GateFailure("resource_budget_failed")
    receipt = {
        "schema": RESOURCE_SCHEMA,
        "candidate_commit": CANDIDATE_COMMIT,
        "observed_at": utc_now(),
        "metrics": {"hot_latency": hot_runs, "single_ledger_sync": single_runs,
                    "three_ledger_sync": three_runs, "idle_cpu": idle, "rss": rss},
        "frozen_bounds": {
            "max_precommit_bytes": 32 * 1024,
            "max_typed_goal_bytes": 4 * 1024,
            "max_k1_actions": 256,
            "segment_bytes": 64 * 1024 * 1024,
            "journal_quota_bytes": 2 * 1024 * 1024 * 1024,
            "persisted_raw_payload_bytes": 0,
        },
        "all_pass": True,
    }
    return receipt


def verify_frozen_baseline() -> None:
    receipt = read_json(BASELINE_RECEIPT)
    checks = (
        (receipt.get("receipt_root_sha256"), BASELINE_RECEIPT_ROOT, "baseline_receipt_root"),
        (receipt.get("source", {}).get("commit"), BASELINE_COMMIT, "baseline_source_commit"),
        (receipt.get("source", {}).get("tree"), BASELINE_TREE, "baseline_source_tree"),
        (sha256_file(PRODUCTION_BINARY), BASELINE_BINARY_SHA256, "baseline_binary"),
        (sha256_file(PRODUCTION_CONFIG), BASELINE_CONFIG_SHA256, "baseline_config"),
        (sha256_file(UNIT_FILE), UNIT_SHA256, "unit"),
        (sha256_file(PHASE_CONFIG), PHASE_CONFIG_SHA256, "phase_config"),
        (sha256_file(AUTHORITY_CONFIG), AUTHORITY_CONFIG_SHA256, "authority_config"),
    )
    for actual, expected, label in checks:
        if actual != expected:
            raise GateFailure(f"STALE_BEFORE_DEPLOYMENT:{label}:{actual}")


def prepare(args: argparse.Namespace) -> int:
    if os.geteuid() != 0:
        raise GateFailure("root_required")
    root = Path(args.transaction_directory)
    if root.exists():
        raise GateFailure(f"transaction_directory_exists:{root}")
    root.mkdir(parents=True, mode=0o700)
    (root / "evidence").mkdir(mode=0o700)
    (root / "rollback").mkdir(mode=0o700)
    try:
        verify_frozen_baseline()
        before_services = service_snapshot()
        require_active(before_services)
        before_health = health_snapshot()
        before_economics = economics_snapshot()
        if before_economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
            raise GateFailure(f"baseline_economics_unsafe:{before_economics}")
        before_probe = route_probe()
        before_journal = journal_snapshot()

        config_path = Path(args.candidate_config)
        if sha256_file(config_path) != CANDIDATE_CONFIG_SHA256:
            raise GateFailure("candidate_config_identity_drift")
        connector_before = read_json(Path(args.connector_before))
        if connector_before.get("active_state") != "active":
            raise GateFailure("connector_before_inactive")

        work = Path(f"/home/e/.cache/nando-s1c3-{args.transaction_id}")
        if work.exists():
            raise GateFailure(f"work_directory_exists:{work}")
        work.mkdir(parents=True)
        shutil.chown(work, user="e", group="e")
        source = work / "candidate"
        run(["git", "clone", "--no-checkout", args.bundle, str(source)], as_user=True, timeout=300,
            log=root / "evidence" / "git-clone.log")
        run(["git", "checkout", "--detach", CANDIDATE_COMMIT], cwd=source, as_user=True, timeout=60,
            log=root / "evidence" / "candidate-checkout.log")
        if run(["git", "rev-parse", "HEAD"], cwd=source, as_user=True).stdout.decode().strip() != CANDIDATE_COMMIT:
            raise GateFailure("candidate_commit_mismatch")
        if run(["git", "rev-parse", "HEAD^{tree}"], cwd=source, as_user=True).stdout.decode().strip() != CANDIDATE_TREE:
            raise GateFailure("candidate_tree_mismatch")
        if sha256_file(source / "Cargo.lock") != CARGO_LOCK_SHA256:
            raise GateFailure("candidate_lock_mismatch")
        if run(["git", "status", "--porcelain", "--untracked-files=no"], cwd=source, as_user=True).stdout:
            raise GateFailure("candidate_worktree_dirty")
        baseline = work / "baseline"
        run(["git", "worktree", "add", "--detach", str(baseline), BASELINE_COMMIT], cwd=source, as_user=True, timeout=120,
            log=root / "evidence" / "baseline-checkout.log")
        if run(["git", "rev-parse", "HEAD^{tree}"], cwd=baseline, as_user=True).stdout.decode().strip() != BASELINE_TREE:
            raise GateFailure("baseline_tree_mismatch")

        target = work / "candidate-target"
        run(
            ["/home/e/.cargo/bin/cargo", "build", "--release", "--locked", "-p", "nando-transition-serving", "--bin", "nando-transition-serving"],
            cwd=source, env={"CARGO_TARGET_DIR": str(target)}, as_user=True, timeout=1800,
            log=root / "evidence" / "candidate-build.log",
        )
        candidate_binary = target / "release" / "nando-transition-serving"
        candidate_hash = sha256_file(candidate_binary)
        candidate_size = candidate_binary.stat().st_size
        harnesses = {
            "response-actor": prebuild_test_harness(
                source, target, "nando-response-actor", root / "evidence" / "response-actor-test-build.log"
            ),
            "transition-serving": prebuild_test_harness(
                source, target, "nando-transition-serving", root / "evidence" / "transition-serving-test-build.log"
            ),
        }
        oracle_source = Path(args.parity_source)
        oracles = {
            "baseline": prebuild_oracle(oracle_source, baseline, work, "baseline", root / "evidence"),
            "candidate": prebuild_oracle(oracle_source, source, work, "candidate", root / "evidence"),
        }
        if run(["git", "status", "--porcelain", "--untracked-files=no"],
               cwd=source, as_user=True).stdout:
            raise GateFailure("candidate_worktree_dirty_after_build")
        config = parse_env_file(config_path)
        rustc = run(["/home/e/.cargo/bin/rustc", "-Vv"], as_user=True).stdout.decode()
        cargo = run(["/home/e/.cargo/bin/cargo", "-V"], as_user=True).stdout.decode().strip()
        isolate_observer_cpus()
        executable_identities = {
            "candidate-binary": executable_identity(candidate_binary, CANDIDATE_COMMIT),
            "test-response-actor": executable_identity(harnesses["response-actor"], CANDIDATE_COMMIT),
            "test-transition-serving": executable_identity(harnesses["transition-serving"], CANDIDATE_COMMIT),
            "parity-baseline": executable_identity(oracles["baseline"], BASELINE_COMMIT),
            "parity-candidate": executable_identity(oracles["candidate"], CANDIDATE_COMMIT),
        }
        executable_root = sha256_bytes(canonical_bytes(executable_identities))
        quiescence = wait_for_quiescence(args.transaction_id, executable_identities)
        write_json(root / "quiescence-receipt.json", quiescence)
        if (root / "quiescence-receipt.json").stat().st_mode & 0o7777 != 0o400:
            raise GateFailure("quiescence_receipt_mode_invalid")
        if any(
            sha256_file(Path(identity["path"])) != identity["sha256"]
            for identity in executable_identities.values()
        ):
            raise GateFailure("measurement_executable_drift_before_first_metric")

        monitor = MeasurementMonitor(
            args.transaction_id, quiescence["quiescence_root_sha256"], executable_root
        )
        measurement_error: Exception | None = None
        resource: dict[str, Any] | None = None
        parity: dict[str, Any] | None = None
        monitor.start()
        try:
            resource = run_resources(
                candidate_binary, source, harnesses, config, work, root / "evidence", monitor
            )
            parity = run_parity(
                oracles, work, root / "evidence", monitor, executable_identities
            )
        except Exception as error:
            measurement_error = error
        finally:
            contamination = monitor.finish()
            write_json(root / "measurement-contamination-receipt.json", contamination)

        if any(
            sha256_file(Path(identity["path"])) != identity["sha256"]
            for identity in executable_identities.values()
        ):
            raise GateFailure("measurement_executable_drift_after_metrics")
        if measurement_error is not None:
            write_json(
                root / "measurement-failure.json",
                {
                    "schema": "nando.s1c3-measurement-failure.v2",
                    "observed_at": utc_now(),
                    "error": str(measurement_error),
                    "contaminated": contamination["contaminated"],
                    "measurement_contamination_root_sha256": contamination[
                        "measurement_contamination_root_sha256"
                    ],
                },
            )
        if contamination["contaminated"]:
            raise GateFailure("INVALID_ENVIRONMENT_MEASUREMENT_CONTAMINATED")
        if measurement_error is not None:
            raise measurement_error
        if resource is None or parity is None:
            raise GateFailure("measurement_result_missing")
        resource.update({
            "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
            "measurement_contamination_root_sha256": contamination[
                "measurement_contamination_root_sha256"
            ],
            "executable_set_root_sha256": executable_root,
            "direct_exec_only": True,
            "compiler_invocations_after_quiescence": 0,
        })
        resource = add_root(resource, "resource_root_sha256")
        write_json(root / "resource-receipt.json", resource)
        write_json(root / "parity-receipt.json", parity)

        rollback_binary = root / "rollback" / "nando-transition-serving"
        rollback_config = root / "rollback" / "transition-serving.env"
        rollback_unit = root / "rollback" / "nando-transition-serving.service"
        rollback_receipt = root / "rollback" / "previous-deployment-receipt.json"
        shutil.copy2(PRODUCTION_BINARY, rollback_binary)
        shutil.copy2(PRODUCTION_CONFIG, rollback_config)
        shutil.copy2(UNIT_FILE, rollback_unit)
        shutil.copy2(BASELINE_RECEIPT, rollback_receipt)
        os.chmod(rollback_binary, 0o500)
        for path in (rollback_config, rollback_unit, rollback_receipt):
            os.chmod(path, 0o400)
        rollback_entries = []
        for path in sorted((root / "rollback").iterdir()):
            rollback_entries.append({"path": path.name, "sha256": sha256_file(path), "size_bytes": path.stat().st_size})
        rollback_manifest = "".join(f'{row["sha256"]} {row["size_bytes"]} {row["path"]}\n' for row in rollback_entries).encode()
        atomic_write(root / "rollback-manifest.sha256", rollback_manifest, 0o400)

        shutil.copy2(candidate_binary, root / "candidate-binary")
        os.chmod(root / "candidate-binary", 0o500)
        shutil.copy2(config_path, root / "candidate-config")
        os.chmod(root / "candidate-config", 0o400)
        fsync_path(root / "candidate-binary")
        fsync_path(root / "candidate-config")
        if sha256_file(root / "candidate-binary") != candidate_hash:
            raise GateFailure("prepared_candidate_binary_copy_mismatch")
        if sha256_file(root / "candidate-config") != CANDIDATE_CONFIG_SHA256:
            raise GateFailure("prepared_candidate_config_copy_mismatch")

        preparation = {
            "schema": PREPARATION_SCHEMA,
            "transaction_id": args.transaction_id,
            "state": "PREPARED",
            "created_at": utc_now(),
            "paper": {"commit": PAPER_COMMIT, "manifest_root_sha256": PAPER_MANIFEST_ROOT,
                      "verification_sha256": PAPER_VERIFICATION_SHA256},
            "candidate": {"source_commit": CANDIDATE_COMMIT, "source_tree": CANDIDATE_TREE,
                          "cargo_lock_sha256": CARGO_LOCK_SHA256, "binary_sha256": candidate_hash,
                          "binary_size_bytes": candidate_size, "config_sha256": CANDIDATE_CONFIG_SHA256},
            "baseline": {"source_commit": BASELINE_COMMIT, "source_tree": BASELINE_TREE,
                         "deployment_receipt_root_sha256": BASELINE_RECEIPT_ROOT,
                         "binary_sha256": BASELINE_BINARY_SHA256,
                         "binary_size_bytes": PRODUCTION_BINARY.stat().st_size,
                         "config_sha256": BASELINE_CONFIG_SHA256},
            "toolchain": {"rustc_vv": rustc, "cargo_v": cargo, "target": "x86_64-unknown-linux-gnu"},
            "immutable": {"unit_sha256": UNIT_SHA256, "phase_config_sha256": PHASE_CONFIG_SHA256,
                          "authority_config_sha256": AUTHORITY_CONFIG_SHA256},
            "services_before": before_services,
            "health_before": before_health,
            "economics_before": before_economics,
            "route_probe_before": before_probe,
            "connector_before": connector_before,
            "journal_before": before_journal,
            "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
            "measurement_contamination_root_sha256": contamination[
                "measurement_contamination_root_sha256"
            ],
            "executable_set_root_sha256": executable_root,
            "resource_root_sha256": resource["resource_root_sha256"],
            "parity_root_sha256": parity["parity_root_sha256"],
            "rollback": {"manifest_root_sha256": sha256_bytes(rollback_manifest), "entries": rollback_entries},
            "intent": ["revalidate", "arm_rollback", "stop_transition", "prove_exit",
                       "stage_pair", "fsync_pair", "rename_pair", "start_transition",
                       "post_start_gates", "survive_15_seconds", "finalize"],
        }
        preparation = add_root(preparation, "preparation_root_sha256")
        write_json(root / "preparation.json", preparation)
        write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "PREPARED",
                                                       "transaction_id": args.transaction_id}, 0o600)
        fsync_directory(root)
        print(json.dumps({"state": "PREPARED", "transaction_directory": str(root),
                          "preparation_root_sha256": preparation["preparation_root_sha256"]}, sort_keys=True))
        return 0
    except Exception as error:
        write_json(root / "preflight-failure.json", {"schema": "nando.s1c3-preflight-failure.v2",
                                                      "observed_at": utc_now(), "error": str(error)})
        raise


def exact_untouched(before: dict[str, Any], current: dict[str, Any]) -> None:
    for unit in UNTOUCHED_UNITS:
        if current[unit] != before[unit]:
            raise GateFailure(f"untouched_service_changed:{unit}")


def semantic_health_equal(before: dict[str, Any], after: dict[str, Any]) -> bool:
    return all(before[label]["semantic"] == after[label]["semantic"] for label in before)


def rollback(root: Path, reason: str) -> None:
    preparation = read_json(root / "preparation.json")
    forward = journal_snapshot()
    systemctl("stop", TRANSITION_UNIT, check=False)
    install_pair(root / "rollback" / "nando-transition-serving", root / "rollback" / "transition-serving.env")
    if sha256_file(PRODUCTION_BINARY) != BASELINE_BINARY_SHA256 or sha256_file(PRODUCTION_CONFIG) != BASELINE_CONFIG_SHA256:
        raise GateFailure("rollback_pair_restore_failed")
    systemctl("start", TRANSITION_UNIT)
    services_after, health_after = wait_for_service()
    time.sleep(15)
    services_survival, health_survival = wait_for_service()
    journal_after = journal_snapshot(prefix_reference=forward)
    exact_untouched(preparation["services_before"], services_after)
    exact_untouched(preparation["services_before"], services_survival)
    pending = {
        "schema": PENDING_SCHEMA,
        "verdict": "S1C3_ROLLBACK_PASS",
        "rollback_reason": reason,
        "services_after": services_after,
        "services_survival": services_survival,
        "health_after": health_after,
        "health_survival": health_survival,
        "journal_after": journal_after,
        "capture_environment": {},
        "capture_available": False,
        "startup_log_clean": True,
        "health_semantics_preserved": semantic_health_equal(preparation["health_before"], health_after)
                                      and semantic_health_equal(preparation["health_before"], health_survival),
        "route_probe_equivalent": route_probe() == preparation["route_probe_before"],
        "active_packages_preserved": health_after["hot"]["semantic"].get("response_active_profiles")
                                     == preparation["health_before"]["hot"]["semantic"].get("response_active_profiles"),
        "economics": economics_snapshot(),
        "installed_binary_sha256": sha256_file(PRODUCTION_BINARY),
        "installed_config_sha256": sha256_file(PRODUCTION_CONFIG),
        "immutable_after": {"unit_sha256": sha256_file(UNIT_FILE),
                            "phase_config_sha256": sha256_file(PHASE_CONFIG),
                            "authority_config_sha256": sha256_file(AUTHORITY_CONFIG)},
    }
    write_json(root / "pending-receipt.json", pending, 0o600)
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "ROLLBACK_PENDING",
                                                   "transaction_id": preparation["transaction_id"]}, 0o600)


def execute(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    preparation = read_json(root / "preparation.json")
    state = read_json(root / "transaction-state.json")
    if state.get("state") != "PREPARED":
        raise GateFailure(f"transaction_not_prepared:{state.get('state')}")
    verify_frozen_baseline()
    current_services = service_snapshot()
    if current_services != preparation["services_before"]:
        raise GateFailure("STALE_BEFORE_DEPLOYMENT:service_snapshot")
    current_health = health_snapshot()
    if not semantic_health_equal(preparation["health_before"], current_health):
        raise GateFailure("STALE_BEFORE_DEPLOYMENT:health_semantics")
    if route_probe() != preparation["route_probe_before"]:
        raise GateFailure("STALE_BEFORE_DEPLOYMENT:route_probe")
    old_pid = current_services[TRANSITION_UNIT]["main_pid"]
    old_rss = process_rss(old_pid)
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "ROLLBACK_ARMED",
                                                   "transaction_id": preparation["transaction_id"]}, 0o600)
    stopped = False
    try:
        systemctl("stop", TRANSITION_UNIT)
        stopped = True
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline and Path(f"/proc/{old_pid}").exists():
            time.sleep(0.1)
        if Path(f"/proc/{old_pid}").exists():
            raise GateFailure("old_pid_still_alive")
        exact_untouched(preparation["services_before"], service_snapshot())
        install_pair(root / "candidate-binary", root / "candidate-config")
        if sha256_file(PRODUCTION_BINARY) != preparation["candidate"]["binary_sha256"]:
            raise GateFailure("candidate_binary_install_mismatch")
        if sha256_file(PRODUCTION_CONFIG) != CANDIDATE_CONFIG_SHA256:
            raise GateFailure("candidate_config_install_mismatch")
        started_at = time.time()
        systemctl("start", TRANSITION_UNIT)
        after_services, after_health = wait_for_service()
        new_pid = after_services[TRANSITION_UNIT]["main_pid"]
        if new_pid == old_pid or new_pid <= 0:
            raise GateFailure("new_pid_invalid")
        if after_services[TRANSITION_UNIT]["nrestarts"] != current_services[TRANSITION_UNIT]["nrestarts"]:
            raise GateFailure("transition_nrestarts_changed")
        exact_untouched(preparation["services_before"], after_services)
        environment = process_environment(new_pid)
        capture_environment = {
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": environment.get("NANDO_GROUNDED_DECISION_SHADOW_ENABLED"),
            "NANDO_GROUNDED_DECISION_JOURNAL": environment.get("NANDO_GROUNDED_DECISION_JOURNAL"),
        }
        expected_capture = {
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
            "NANDO_GROUNDED_DECISION_JOURNAL": str(JOURNAL),
        }
        if capture_environment != expected_capture:
            raise GateFailure(f"capture_environment_invalid:{capture_environment}")
        logs = run(["journalctl", "-u", TRANSITION_UNIT, "--since", f"@{started_at:.6f}", "--no-pager", "-o", "cat"], timeout=10).stdout
        write_evidence(root, "startup.log", logs)
        log_text = logs.decode("utf-8", "replace")
        startup_log_clean = "nando-grounded-decision shadow unavailable" not in log_text
        if not startup_log_clean:
            raise GateFailure("grounded_decision_startup_unavailable")
        if not JOURNAL.is_dir():
            raise GateFailure("grounded_decision_journal_not_opened")
        after_journal = journal_snapshot()
        if after_journal["total_bytes"] > 2 * 1024 * 1024 * 1024:
            raise GateFailure("journal_quota_exceeded")
        after_economics = economics_snapshot()
        if after_economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
            raise GateFailure(f"post_economics_unsafe:{after_economics}")
        new_rss = process_rss(new_pid)
        if max(0, new_rss - old_rss) > 16 * 1024 * 1024:
            raise GateFailure(f"hot_rss_delta_exceeded:{new_rss - old_rss}")
        after_probe = route_probe()
        if after_probe != preparation["route_probe_before"]:
            raise GateFailure("post_route_probe_mismatch")
        if not semantic_health_equal(preparation["health_before"], after_health):
            raise GateFailure("post_health_semantics_changed")
        time.sleep(15)
        survival_services, survival_health = wait_for_service()
        if survival_services[TRANSITION_UNIT]["main_pid"] != new_pid:
            raise GateFailure("transition_pid_changed_during_survival")
        if survival_services[TRANSITION_UNIT]["nrestarts"] != current_services[TRANSITION_UNIT]["nrestarts"]:
            raise GateFailure("transition_restart_during_survival")
        exact_untouched(preparation["services_before"], survival_services)
        if not semantic_health_equal(preparation["health_before"], survival_health):
            raise GateFailure("survival_health_semantics_changed")
        if route_probe() != preparation["route_probe_before"]:
            raise GateFailure("survival_route_probe_mismatch")
        survival_economics = economics_snapshot()
        if survival_economics != {"false_accepts": 0, "runtime_parity_mismatches": 0}:
            raise GateFailure(f"survival_economics_unsafe:{survival_economics}")
        survival_journal = journal_snapshot()
        pending = {
            "schema": PENDING_SCHEMA,
            "verdict": "S1C3_DEPLOYMENT_PASS",
            "services_after": after_services,
            "services_survival": survival_services,
            "health_after": after_health,
            "health_survival": survival_health,
            "journal_after": survival_journal,
            "capture_environment": capture_environment,
            "capture_available": True,
            "startup_log_clean": startup_log_clean,
            "health_semantics_preserved": True,
            "route_probe_equivalent": True,
            "active_packages_preserved": after_health["hot"]["semantic"].get("response_active_profiles")
                                         == preparation["health_before"]["hot"]["semantic"].get("response_active_profiles"),
            "economics": survival_economics,
            "installed_binary_sha256": sha256_file(PRODUCTION_BINARY),
            "installed_config_sha256": sha256_file(PRODUCTION_CONFIG),
            "immutable_after": {"unit_sha256": sha256_file(UNIT_FILE),
                                "phase_config_sha256": sha256_file(PHASE_CONFIG),
                                "authority_config_sha256": sha256_file(AUTHORITY_CONFIG)},
            "old_rss_bytes": old_rss,
            "new_rss_bytes": new_rss,
        }
        write_json(root / "pending-receipt.json", pending, 0o600)
        write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "FINALIZE_PENDING",
                                                       "transaction_id": preparation["transaction_id"]}, 0o600)
        print(json.dumps({"state": "FINALIZE_PENDING", "new_pid": new_pid}, sort_keys=True))
        return 0
    except Exception as error:
        if stopped:
            try:
                rollback(root, str(error))
            except Exception as rollback_error:
                write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": "VETO",
                                                               "error": str(error), "rollback_error": str(rollback_error)}, 0o600)
                raise GateFailure(f"S1C3_VETO:{error}:rollback:{rollback_error}") from rollback_error
            raise GateFailure(f"S1C3_ROLLBACK_PASS:{error}") from error
        raise


def finalize(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    preparation = read_json(root / "preparation.json")
    pending = read_json(root / "pending-receipt.json")
    state = read_json(root / "transaction-state.json")
    connector_after = read_json(Path(args.connector_after))
    connector_before = preparation["connector_before"]
    connector_fields = ("main_pid", "nrestarts", "route_receipt_failures", "command_sha256", "active_state")
    connector_ok = all(connector_after.get(field) == connector_before.get(field) for field in connector_fields)
    if not connector_ok and state.get("state") == "FINALIZE_PENDING":
        rollback(root, "connector_identity_or_failures_changed")
        pending = read_json(root / "pending-receipt.json")
        pending["verdict"] = "S1C3_VETO"
    elif state.get("state") not in {"FINALIZE_PENDING", "ROLLBACK_PENDING"}:
        raise GateFailure(f"finalize_state_invalid:{state.get('state')}")
    receipt = {
        "schema": RECEIPT_SCHEMA,
        "transaction_id": preparation["transaction_id"],
        "verdict": pending["verdict"],
        "finalized_at": utc_now(),
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "quiescence_root_sha256": preparation["quiescence_root_sha256"],
        "measurement_contamination_root_sha256": preparation[
            "measurement_contamination_root_sha256"
        ],
        "executable_set_root_sha256": preparation["executable_set_root_sha256"],
        "resource_root_sha256": preparation["resource_root_sha256"],
        "parity_root_sha256": preparation["parity_root_sha256"],
        "services_before": preparation["services_before"],
        "services_after": pending["services_after"],
        "services_survival": pending["services_survival"],
        "connector_before": connector_before,
        "connector_after": connector_after,
        "installed_binary_sha256": pending["installed_binary_sha256"],
        "installed_config_sha256": pending["installed_config_sha256"],
        "immutable_after": pending["immutable_after"],
        "capture_environment": pending["capture_environment"],
        "capture_available": pending["capture_available"],
        "startup_log_clean": pending["startup_log_clean"],
        "health_semantics_preserved": pending["health_semantics_preserved"],
        "route_probe_equivalent": pending["route_probe_equivalent"],
        "active_packages_preserved": pending["active_packages_preserved"],
        "false_accepts_after": pending["economics"]["false_accepts"],
        "runtime_parity_failures_after": pending["economics"]["runtime_parity_mismatches"],
        "journal_before": preparation["journal_before"],
        "journal_after": pending["journal_after"],
        "survival_seconds": 15,
    }
    receipt = add_root(receipt, "receipt_root_sha256")
    write_json(root / "deployment-receipt.json", receipt)
    write_json(root / "transaction-state.json", {"schema": STATE_SCHEMA, "state": receipt["verdict"],
                                                   "transaction_id": preparation["transaction_id"]})
    for path in root.rglob("*"):
        if path.is_file():
            os.chmod(path, 0o500 if os.access(path, os.X_OK) and path.name in {"candidate-binary", "nando-transition-serving"} else 0o400)
    fsync_directory(root)
    print(json.dumps({"verdict": receipt["verdict"], "receipt_root_sha256": receipt["receipt_root_sha256"],
                      "transaction_directory": str(root)}, sort_keys=True))
    return 0 if receipt["verdict"] == "S1C3_DEPLOYMENT_PASS" else 3


def rollback_command(args: argparse.Namespace) -> int:
    root = Path(args.transaction_directory)
    rollback(root, args.reason)
    print(json.dumps({"state": "ROLLBACK_PENDING", "transaction_directory": str(root)}, sort_keys=True))
    return 3


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    prepare_parser = subparsers.add_parser("prepare")
    prepare_parser.add_argument("--transaction-id", required=True)
    prepare_parser.add_argument("--transaction-directory", required=True)
    prepare_parser.add_argument("--bundle", required=True)
    prepare_parser.add_argument("--candidate-config", required=True)
    prepare_parser.add_argument("--parity-source", required=True)
    prepare_parser.add_argument("--connector-before", required=True)
    execute_parser = subparsers.add_parser("execute")
    execute_parser.add_argument("--transaction-directory", required=True)
    finalize_parser = subparsers.add_parser("finalize")
    finalize_parser.add_argument("--transaction-directory", required=True)
    finalize_parser.add_argument("--connector-after", required=True)
    rollback_parser = subparsers.add_parser("rollback")
    rollback_parser.add_argument("--transaction-directory", required=True)
    rollback_parser.add_argument("--reason", required=True)
    args = parser.parse_args()
    try:
        if args.command == "prepare":
            return prepare(args)
        if args.command == "execute":
            return execute(args)
        if args.command == "finalize":
            return finalize(args)
        return rollback_command(args)
    except GateFailure as error:
        print(json.dumps({"schema": "nando.s1c3-executor-error.v2", "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
