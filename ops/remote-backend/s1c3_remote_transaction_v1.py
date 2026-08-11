#!/usr/bin/env python3
"""Root-only remote executor for the preregistered S1C-3 transaction."""

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
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


PAPER_COMMIT = "b3ee186d49d848b1917472f427d6afc59459c7cd"
PAPER_MANIFEST_ROOT = "ebb5067060f69722341120ae8105849cbd45f585611a30741e1db7d33ace3ab3"
PAPER_VERIFICATION_SHA256 = "41da0d1cc419690261c701133fca0c123eafdacfc9fd14a28287453af1112deb"
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


def run_test(
    source: Path,
    target: Path,
    package: str,
    test: str,
    label: str,
    evidence: Path,
    timeout: int,
) -> str:
    command = [
        "taskset", "-c", "4", "/home/e/.cargo/bin/cargo", "test", "--release",
        "-p", package, "--lib", test, "--", "--ignored", "--exact", "--nocapture", "--test-threads=1",
    ]
    completed = run(
        command,
        cwd=source,
        env={"CARGO_TARGET_DIR": str(target), "RUST_TEST_THREADS": "1"},
        as_user=True,
        timeout=timeout,
        log=evidence / f"{label}.log",
    )
    return completed.stdout.decode("utf-8", "replace")


def make_oracle(oracle_source: Path, source: Path, root: Path, label: str) -> Path:
    crate = root / f"oracle-{label}"
    (crate / "src").mkdir(parents=True)
    shutil.copyfile(oracle_source, crate / "src" / "main.rs")
    manifest = f'''[package]\nname = "s1c3-parity-{label}"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nnando-response-actor = {{ path = "{source / "crates/nando-response-actor"}" }}\nserde_json = "1"\n'''
    atomic_write(crate / "Cargo.toml", manifest.encode(), 0o644)
    return crate / "Cargo.toml"


def run_parity(candidate: Path, baseline: Path, oracle_source: Path, work: Path, evidence: Path) -> dict[str, Any]:
    fixture = work / "parity-fixture"
    fixture.mkdir()
    shutil.copy2(RESPONSE_REGISTRY, fixture / "response-registry.json")
    shutil.copy2(ADMISSION, fixture / "admission.json")
    outputs: dict[str, bytes] = {}
    for label, source in (("baseline", baseline), ("candidate", candidate)):
        manifest = make_oracle(oracle_source, source, work, label)
        completed = run(
            ["/home/e/.cargo/bin/cargo", "run", "--release", "--quiet", "--manifest-path", str(manifest), "--",
             str(fixture / "response-registry.json"), str(fixture / "admission.json")],
            env={
                "CARGO_TARGET_DIR": str(work / f"oracle-target-{label}"),
                "RUSTFLAGS": "-Awarnings",
            },
            as_user=True,
            timeout=1200,
            log=evidence / f"parity-{label}.log",
        )
        outputs[label] = completed.stdout
    identical = outputs["baseline"] == outputs["candidate"]
    rows = len(outputs["candidate"].splitlines())
    if not identical or rows != 16:
        raise GateFailure(f"parity_failed:identical={identical}:rows={rows}")
    receipt = {
        "schema": "nando.s1c3-parity-receipt.v1",
        "baseline_output_sha256": sha256_bytes(outputs["baseline"]),
        "candidate_output_sha256": sha256_bytes(outputs["candidate"]),
        "byte_identical": identical,
        "rows": rows,
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


def measure_isolated_rss(binary: Path, config: dict[str, str], work: Path, evidence: Path) -> dict[str, int]:
    import pwd

    account = pwd.getpwnam("e")

    def demote() -> None:
        os.initgroups(account.pw_name, account.pw_gid)
        os.setgid(account.pw_gid)
        os.setuid(account.pw_uid)

    samples: dict[str, int] = {}
    for index, capture in enumerate((False, True)):
        label = "capture_on" if capture else "capture_off"
        state = work / f"rss-{label}"
        environment = isolated_environment(config, state, 19871 + index, capture)
        log_path = evidence / f"rss-{label}.log"
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
        os.chmod(log_path, 0o400)
    delta = max(0, samples["capture_on"] - samples["capture_off"])
    if delta > 16 * 1024 * 1024:
        raise GateFailure(f"rss_delta_exceeded:{delta}")
    return {"capture_off_bytes": samples["capture_off"], "capture_on_bytes": samples["capture_on"], "delta_bytes": delta}


def run_resources(candidate: Path, target: Path, config: dict[str, str], work: Path, evidence: Path) -> dict[str, Any]:
    hot_runs = []
    single_runs = []
    three_runs = []
    for index in range(1, 4):
        output = run_test(candidate, target, "nando-response-actor", HOT_TEST, f"hot-{index}", evidence, 600)
        match = HOT_RE.search(output)
        if not match:
            raise GateFailure(f"hot_metric_missing:{index}")
        matched, no_goal, hard_max, samples = map(int, match.groups())
        hot_runs.append({"p99_ns": matched, "no_goal_p99_ns": no_goal, "hard_max_ns": hard_max, "samples": samples})
    for index in range(1, 4):
        output = run_test(candidate, target, "nando-transition-serving", SINGLE_SYNC_TEST, f"single-sync-{index}", evidence, 900)
        match = SYNC_RE.search(output)
        if not match:
            raise GateFailure(f"single_sync_metric_missing:{index}")
        p99, hard_max, samples, segments = map(int, match.groups())
        single_runs.append({"p99_ns": p99, "hard_max_ns": hard_max, "samples": samples, "segments": segments})
    for index in range(1, 4):
        output = run_test(candidate, target, "nando-transition-serving", THREE_SYNC_TEST, f"three-sync-{index}", evidence, 900)
        match = THREE_SYNC_RE.search(output)
        if not match:
            raise GateFailure(f"three_sync_metric_missing:{index}")
        p99, hard_max, samples = map(int, match.groups())
        three_runs.append({"p99_ns": p99, "hard_max_ns": hard_max, "samples": samples})
    idle_output = run_test(candidate, target, "nando-response-actor", IDLE_TEST, "idle", evidence, 180)
    idle_match = IDLE_RE.search(idle_output)
    if not idle_match:
        raise GateFailure("idle_metric_missing")
    elapsed, ticks, percent = idle_match.groups()
    idle = {"elapsed_ticks": int(elapsed), "ticks_per_second": int(ticks), "percent_of_one_core": float(percent)}
    rss = measure_isolated_rss(target / "release" / "nando-transition-serving", config, work, evidence)
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
        "schema": "nando.s1c3-resource-receipt.v1",
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
    return add_root(receipt, "resource_root_sha256")


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
        build = run(
            ["/home/e/.cargo/bin/cargo", "build", "--release", "--locked", "-p", "nando-transition-serving", "--bin", "nando-transition-serving"],
            cwd=source, env={"CARGO_TARGET_DIR": str(target)}, as_user=True, timeout=1800,
            log=root / "evidence" / "candidate-build.log",
        )
        del build
        candidate_binary = target / "release" / "nando-transition-serving"
        candidate_hash = sha256_file(candidate_binary)
        candidate_size = candidate_binary.stat().st_size
        if run(["git", "status", "--porcelain", "--untracked-files=no"],
               cwd=source, as_user=True).stdout:
            raise GateFailure("candidate_worktree_dirty_after_build")
        config = parse_env_file(config_path)
        resource = run_resources(source, target, config, work, root / "evidence")
        parity = run_parity(source, baseline, Path(args.parity_source), work, root / "evidence")
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

        rustc = run(["/home/e/.cargo/bin/rustc", "-Vv"], as_user=True).stdout.decode()
        cargo = run(["/home/e/.cargo/bin/cargo", "-V"], as_user=True).stdout.decode().strip()
        preparation = {
            "schema": "nando.s1c3-transaction-preparation.v1",
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
            "resource_root_sha256": resource["resource_root_sha256"],
            "parity_root_sha256": parity["parity_root_sha256"],
            "rollback": {"manifest_root_sha256": sha256_bytes(rollback_manifest), "entries": rollback_entries},
            "intent": ["revalidate", "arm_rollback", "stop_transition", "prove_exit",
                       "stage_pair", "fsync_pair", "rename_pair", "start_transition",
                       "post_start_gates", "survive_15_seconds", "finalize"],
        }
        preparation = add_root(preparation, "preparation_root_sha256")
        write_json(root / "preparation.json", preparation)
        write_json(root / "transaction-state.json", {"schema": "nando.s1c3-state.v1", "state": "PREPARED",
                                                       "transaction_id": args.transaction_id}, 0o600)
        fsync_directory(root)
        print(json.dumps({"state": "PREPARED", "transaction_directory": str(root),
                          "preparation_root_sha256": preparation["preparation_root_sha256"]}, sort_keys=True))
        return 0
    except Exception as error:
        write_json(root / "preflight-failure.json", {"schema": "nando.s1c3-preflight-failure.v1",
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
        "schema": "nando.s1c3-pending-receipt.v1",
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
    write_json(root / "transaction-state.json", {"schema": "nando.s1c3-state.v1", "state": "ROLLBACK_PENDING",
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
    write_json(root / "transaction-state.json", {"schema": "nando.s1c3-state.v1", "state": "ROLLBACK_ARMED",
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
            "schema": "nando.s1c3-pending-receipt.v1",
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
        write_json(root / "transaction-state.json", {"schema": "nando.s1c3-state.v1", "state": "FINALIZE_PENDING",
                                                       "transaction_id": preparation["transaction_id"]}, 0o600)
        print(json.dumps({"state": "FINALIZE_PENDING", "new_pid": new_pid}, sort_keys=True))
        return 0
    except Exception as error:
        if stopped:
            try:
                rollback(root, str(error))
            except Exception as rollback_error:
                write_json(root / "transaction-state.json", {"schema": "nando.s1c3-state.v1", "state": "VETO",
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
        "schema": "nando.s1c3-transaction-receipt.v1",
        "transaction_id": preparation["transaction_id"],
        "verdict": pending["verdict"],
        "finalized_at": utc_now(),
        "preparation_root_sha256": preparation["preparation_root_sha256"],
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
    write_json(root / "transaction-state.json", {"schema": "nando.s1c3-state.v1", "state": receipt["verdict"],
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
        print(json.dumps({"schema": "nando.s1c3-executor-error.v1", "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
