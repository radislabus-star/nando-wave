#!/usr/bin/env python3
"""S1C-3D repair over the frozen S1C-3B transaction mechanism.

The old mechanism remains immutable. This module changes only parity input
ownership and the classification of durability targets versus safety limits.
"""

from __future__ import annotations

import argparse
import contextlib
import grp
import json
import os
import re
import stat
from pathlib import Path
from typing import Any, Iterator

import s1c3b_remote_transaction_v1 as base


PAPER_COMMIT = "c3eaddc55dfcdb45060c0d61278fd115a6707639"
PAPER_TREE = "4127d54552c418baa3a9c324451a37c989a3a98f"
PAPER_PREREGISTRATION_SHA256 = "ecc8b63a1376e2c0e0007fb161f05069c8a81e13663eb73258934c43b6f7c4f1"
PAPER_CRITIQUE_SHA256 = "1604b4674be0137147b29e5fe8745a5d28e2b50febef1852b0af5c0dd594fb30"
PAPER_MANIFEST_ROOT = "75b85eeff6256e88fccc2408292f669fc32f6de538b79d5334c79af919c13e2f"
PAPER_VERIFICATION_SHA256 = "25498a5597d8a9bba6b0745426ae8f7cb08bb1553652d8400902d80eea271b62"

RESOURCE_SCHEMA = "nando.s1c3d-resource-receipt.v1"
PARITY_SCHEMA = "nando.s1c3d-parity-receipt.v1"
SNAPSHOT_SCHEMA = "nando.s1c3d-parity-snapshot.v1"
CLASSIFICATION_SCHEMA = "nando.s1c3d-resource-classification.v1"

P99_TARGET_NS = 5_000_000
DURABILITY_HARD_MAX_NS = 20_000_000
FROZEN_TRANSITION_TEST_EXECUTABLE_SHA256 = (
    "2d0a267e3d0955626ff711a3f1670a772a70103a3f9843a346684eff9aa7500b"
)
FROZEN_GROUNDED_CAPTURE_SOURCE_SHA256 = (
    "43f6de5d3df4c4a9f51f35694e8b50948c992e66b2fa1ecd846b393408cd42d4"
)
GROUNDED_CAPTURE_SOURCE = Path(
    "crates/nando-transition-serving/src/grounded_decision_capture.rs"
)
SNAPSHOT_PARENT = Path("/var/lib/nando-wave/s1c3d-parity-snapshots")

EXPECTED_TARGET_ASSERTIONS = {
    "single": "sync p99 exceeded 5 ms",
    "three-precommit": "precommit sync p99 exceeded 5 ms",
    "three-settlement": "settlement sync p99 exceeded 5 ms",
}

SNAPSHOT_PERMISSION_PROBE = r'''
import errno, hashlib, json, os, sys

directory = sys.argv[1]
paths = sys.argv[2:]
result = {"directory": {}, "files": {}}
ok = True
directory_operations = {
    "chmod": lambda: os.chmod(directory, 0o750),
    "rename": lambda: os.rename(directory, directory + ".renamed"),
}
for label, operation in directory_operations.items():
    try:
        operation()
        result["directory"][label] = {"denied": False, "errno": None}
        ok = False
    except OSError as error:
        denied = error.errno in (errno.EACCES, errno.EPERM, errno.EROFS)
        result["directory"][label] = {"denied": denied, "errno": error.errno}
        ok = ok and denied
for path in paths:
    name = os.path.basename(path)
    row = {"read_sha256": hashlib.sha256(open(path, "rb").read()).hexdigest(), "denials": {}}
    operations = {
        "chmod": lambda: os.chmod(path, 0o640),
        "write": lambda: os.close(os.open(path, os.O_WRONLY | os.O_APPEND)),
        "unlink": lambda: os.unlink(path),
        "rename": lambda: os.rename(path, path + ".renamed"),
    }
    for label, operation in operations.items():
        try:
            operation()
            row["denials"][label] = {"denied": False, "errno": None}
            ok = False
        except OSError as error:
            denied = error.errno in (errno.EACCES, errno.EPERM, errno.EROFS)
            row["denials"][label] = {"denied": denied, "errno": error.errno}
            ok = ok and denied
    result["files"][name] = row
result["all_denied"] = ok
print(json.dumps(result, sort_keys=True, separators=(",", ":")))
raise SystemExit(0 if ok else 3)
'''


def _identity(path: Path) -> dict[str, Any]:
    value = path.stat()
    return {
        "path": str(path),
        "size_bytes": value.st_size,
        "sha256": base.sha256_file(path),
        "uid": value.st_uid,
        "gid": value.st_gid,
        "mode_octal": f"{stat.S_IMODE(value.st_mode):04o}",
        "device": value.st_dev,
        "inode": value.st_ino,
    }


def _read_source_once(path: Path) -> tuple[bytes, dict[str, Any]]:
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode):
            raise base.GateFailure(f"parity_source_not_regular:{path}")
        chunks = []
        while True:
            chunk = os.read(descriptor, 1024 * 1024)
            if not chunk:
                break
            chunks.append(chunk)
        after = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    stable = (
        before.st_dev,
        before.st_ino,
        before.st_size,
        before.st_mtime_ns,
        before.st_ctime_ns,
    ) == (
        after.st_dev,
        after.st_ino,
        after.st_size,
        after.st_mtime_ns,
        after.st_ctime_ns,
    )
    if not stable:
        raise base.GateFailure(f"parity_source_changed_during_read:{path}")
    payload = b"".join(chunks)
    if len(payload) != before.st_size:
        raise base.GateFailure(f"parity_source_short_read:{path}")
    return payload, {
        "path": str(path),
        "size_bytes": len(payload),
        "sha256": base.sha256_bytes(payload),
        "uid": before.st_uid,
        "gid": before.st_gid,
        "mode_octal": f"{stat.S_IMODE(before.st_mode):04o}",
        "device": before.st_dev,
        "inode": before.st_ino,
        "read_stable": True,
    }


def _remove_snapshot(fixture: Path) -> None:
    if not fixture.exists():
        return
    for path in fixture.iterdir():
        if path.is_symlink() or path.is_file():
            path.unlink()
        else:
            raise base.GateFailure(f"parity_snapshot_unexpected_entry:{path}")
    fixture.rmdir()


def _create_snapshot(work: Path, evidence: Path) -> tuple[Path, dict[str, Any]]:
    SNAPSHOT_PARENT.mkdir(parents=True, exist_ok=True, mode=0o711)
    parent_stat = SNAPSHOT_PARENT.lstat()
    if not stat.S_ISDIR(parent_stat.st_mode) or stat.S_ISLNK(parent_stat.st_mode):
        raise base.GateFailure("parity_snapshot_parent_invalid")
    os.chown(SNAPSHOT_PARENT, 0, 0)
    os.chmod(SNAPSHOT_PARENT, 0o711)
    fixture = SNAPSHOT_PARENT / work.name
    if fixture.exists():
        raise base.GateFailure(f"parity_snapshot_exists:{fixture}")
    fixture.mkdir(mode=0o750)
    group_id = grp.getgrnam("e").gr_gid
    os.chown(fixture, 0, group_id)

    try:
        sources = {
            "registry": (base.legacy.RESPONSE_REGISTRY, "response-registry.json"),
            "admission": (base.legacy.ADMISSION, "admission.json"),
        }
        bindings = {}
        for label, (source_path, filename) in sources.items():
            payload, source = _read_source_once(source_path)
            destination = fixture / filename
            base.atomic_write(destination, payload, 0o440)
            os.chown(destination, 0, group_id)
            os.chmod(destination, 0o440)
            snapshot = _identity(destination)
            if source["sha256"] != snapshot["sha256"] or source["size_bytes"] != snapshot["size_bytes"]:
                raise base.GateFailure(f"parity_snapshot_binding_mismatch:{label}")
            bindings[label] = {"source": source, "snapshot": snapshot}
        os.chmod(fixture, 0o550)

        paths = [fixture / sources[label][1] for label in ("registry", "admission")]
        completed = base.run(
            [
                "/usr/bin/python3",
                "-c",
                SNAPSHOT_PERMISSION_PROBE,
                str(fixture),
                *(str(path) for path in paths),
            ],
            as_user=True,
            check=False,
            timeout=30,
        )
        base.atomic_write(evidence / "parity-snapshot-permissions.log", completed.stdout, 0o400)
        try:
            probe = json.loads(completed.stdout)
        except json.JSONDecodeError as error:
            raise base.GateFailure("parity_snapshot_probe_invalid") from error
        if completed.returncode != 0 or probe.get("all_denied") is not True:
            raise base.GateFailure("parity_snapshot_mutation_not_denied")
        for label, (_, filename) in sources.items():
            final = _identity(fixture / filename)
            if final != bindings[label]["snapshot"]:
                raise base.GateFailure(f"parity_snapshot_changed_after_probe:{label}")
            if probe["files"][filename]["read_sha256"] != final["sha256"]:
                raise base.GateFailure(f"parity_snapshot_probe_read_mismatch:{label}")

        receipt = base.add_root(
            {
                "schema": SNAPSHOT_SCHEMA,
                "directory": {
                    "path": str(fixture),
                    "uid": fixture.stat().st_uid,
                    "gid": fixture.stat().st_gid,
                    "mode_octal": f"{stat.S_IMODE(fixture.stat().st_mode):04o}",
                },
                "parent": {
                    "path": str(SNAPSHOT_PARENT),
                    "uid": SNAPSHOT_PARENT.stat().st_uid,
                    "gid": SNAPSHOT_PARENT.stat().st_gid,
                    "mode_octal": f"{stat.S_IMODE(SNAPSHOT_PARENT.stat().st_mode):04o}",
                },
                "bindings": bindings,
                "permission_probe": {
                    "returncode": completed.returncode,
                    "output_sha256": base.sha256_bytes(completed.stdout),
                    "result": probe,
                },
            },
            "snapshot_root_sha256",
        )
        return fixture, receipt
    except Exception:
        _remove_snapshot(fixture)
        raise


def run_parity(
    oracles: dict[str, Path],
    work: Path,
    evidence: Path,
    monitor: base.MeasurementMonitor,
    wrapper: Path,
) -> dict[str, Any]:
    fixture, snapshot = _create_snapshot(work, evidence)
    arguments = [str(fixture / "response-registry.json"), str(fixture / "admission.json")]
    rows = []
    outputs: dict[str, bytes] = {}
    complete = False
    try:
        for label in ("baseline", "candidate"):
            completed, output, command = base.run_measured(
                oracles[label],
                arguments,
                f"parity-{label}",
                None,
                evidence,
                1200,
                monitor,
                wrapper,
            )
            raw = output.encode("utf-8")
            outputs[label] = raw
            rows.append(
                {
                    "label": label,
                    "returncode": completed.returncode if completed is not None else None,
                    "output_sha256": base.sha256_bytes(raw),
                    "row_count": len(raw.splitlines()) - 1,
                    "command": command,
                }
            )
        complete = True
    finally:
        if not complete:
            _remove_snapshot(fixture)
    payloads = {
        label: b"\n".join(raw.splitlines()[1:]) for label, raw in outputs.items()
    }
    comparable = all(row["returncode"] == 0 and row["row_count"] == 16 for row in rows)
    identical = comparable and payloads["baseline"] == payloads["candidate"]
    return base.add_root(
        {
            "schema": PARITY_SCHEMA,
            "snapshot": snapshot,
            "snapshot_root_sha256": snapshot["snapshot_root_sha256"],
            "snapshot_retained_for_authority_verifier": True,
            "rows": rows,
            "byte_identical": identical,
            "row_count": rows[1]["row_count"],
            "baseline_output_sha256": base.sha256_bytes(payloads["baseline"]),
            "candidate_output_sha256": base.sha256_bytes(payloads["candidate"]),
        },
        "parity_root_sha256",
    )


def _watch(label: str, field: str, observed: int) -> dict[str, Any]:
    return {
        "label": label,
        "field": field,
        "observed_ns": observed,
        "target_ns": P99_TARGET_NS,
        "ratio_ppm": observed * 1_000_000 // P99_TARGET_NS,
    }


def _legacy_target_assertion(
    row: dict[str, Any], kind: str, evidence: Path, watches: list[dict[str, Any]]
) -> bool:
    if row.get("returncode") != 101 or not watches:
        return False
    command = row.get("command", {})
    if (
        command.get("returncode") != 101
        or command.get("executable_sha256") != FROZEN_TRANSITION_TEST_EXECUTABLE_SHA256
        or row.get("test")
        not in {base.SINGLE_SYNC_TEST, base.THREE_SYNC_TEST}
    ):
        return False
    metrics = row.get("metrics", {})
    hard_fields = (
        ("hard_max_ns",)
        if kind == "single"
        else ("precommit_hard_max_ns", "settlement_hard_max_ns", "episode_hard_max_ns")
    )
    if any(type(metrics.get(field)) is not int or metrics[field] > DURABILITY_HARD_MAX_NS for field in hard_fields):
        return False
    if kind == "single":
        expected_message = EXPECTED_TARGET_ASSERTIONS["single"]
    elif metrics.get("precommit_p99_ns", 0) > P99_TARGET_NS:
        expected_message = EXPECTED_TARGET_ASSERTIONS["three-precommit"]
    else:
        expected_message = EXPECTED_TARGET_ASSERTIONS["three-settlement"]
    text = (evidence / f'{row["label"]}.log').read_text(encoding="utf-8", errors="replace")
    return (
        text.count("panicked at") == 1
        and expected_message in text
        and "test result: FAILED. 0 passed; 1 failed;" in text
        and "hard ceiling exceeded" not in text
        and "PermissionDenied" not in text
    )


def classify_resource(resource: dict[str, Any], source: Path, evidence: Path) -> dict[str, Any]:
    correctness: list[str] = []
    safety: list[str] = []
    watches: list[dict[str, Any]] = []
    legacy_assertions: list[str] = []
    metrics = resource["metrics"]

    source_hash = base.sha256_file(source / GROUNDED_CAPTURE_SOURCE)
    if source_hash != FROZEN_GROUNDED_CAPTURE_SOURCE_SHA256:
        correctness.append("legacy_target_assertion_source_identity")

    for row in resource["floor_probes"]:
        if row["records"] != base.FLOOR_RECORDS or row["returncode"] != 0 or row["error"]:
            correctness.append(f'{row["label"]}:floor_incomplete')

    for row in metrics["hot_latency"]:
        label = row["label"]
        metric = row.get("metrics")
        if metric is None:
            correctness.append(f"{label}:metric_missing")
            continue
        if row["returncode"] != 0 or not row["test_assertion_pass"]:
            correctness.append(f"{label}:test_assertion_failed")
        if row["command"].get("observed_affinity") != [base.MEASUREMENT_CPU]:
            correctness.append(f"{label}:affinity_invalid")
        if metric["samples"] != 4096:
            correctness.append(f"{label}:denominator")
        if metric["p99_ns"] > 1_000_000:
            safety.append(f"{label}:matched_p99")
        if metric["no_goal_p99_ns"] > 250_000:
            safety.append(f"{label}:no_goal_p99")
        if metric["hard_max_ns"] > 2_000_000:
            safety.append(f"{label}:hard_max")

    for name, kind, denominator in (
        ("single_ledger_sync", "single", 1024),
        ("three_ledger_sync", "three", 256),
    ):
        for row in metrics[name]:
            label = row["label"]
            metric = row.get("metrics")
            if metric is None:
                correctness.append(f"{label}:metric_missing")
                continue
            if row["command"].get("observed_affinity") != [base.MEASUREMENT_CPU]:
                correctness.append(f"{label}:affinity_invalid")
            if metric["samples"] != denominator:
                correctness.append(f"{label}:denominator")
            target_fields = ("p99_ns",) if kind == "single" else ("precommit_p99_ns", "settlement_p99_ns")
            hard_fields = ("hard_max_ns",) if kind == "single" else (
                "precommit_hard_max_ns",
                "settlement_hard_max_ns",
                "episode_hard_max_ns",
            )
            row_watches = [_watch(label, field, metric[field]) for field in target_fields if metric[field] > P99_TARGET_NS]
            watches.extend(row_watches)
            for field in hard_fields:
                if metric[field] > DURABILITY_HARD_MAX_NS:
                    safety.append(f"{label}:{field}")
            if row["returncode"] == 0 and not row_watches and row["test_assertion_pass"]:
                continue
            if (
                source_hash == FROZEN_GROUNDED_CAPTURE_SOURCE_SHA256
                and _legacy_target_assertion(row, kind, evidence, row_watches)
            ):
                legacy_assertions.append(label)
            else:
                correctness.append(f"{label}:test_assertion_failed")

    idle = metrics["idle_cpu"]
    idle_metric = idle.get("metrics")
    if idle_metric is None:
        correctness.append("idle:metric_missing")
    else:
        if idle_metric["percent_of_one_core"] > 0.25:
            safety.append("idle:percent_of_one_core")
    if idle["returncode"] != 0 or not idle["test_assertion_pass"]:
        correctness.append("idle:test_assertion_failed")
    if idle["command"].get("observed_affinity") != [base.MEASUREMENT_CPU]:
        correctness.append("idle:affinity_invalid")

    rss = metrics["rss"]
    if any(row["rss_bytes"] is None or row["sample_count"] != 20 or row["error"] for row in rss["rows"]):
        correctness.append("rss:incomplete")
    elif rss["delta_bytes"] > 16 * 1024 * 1024:
        safety.append("rss:delta")

    parity = resource.pop("_s1c3d_parity")
    if not parity["byte_identical"] or parity["row_count"] != 16:
        correctness.append("parity:byte_identity")

    classification = {
        "schema": CLASSIFICATION_SCHEMA,
        "correctness_failures": sorted(set(correctness)),
        "operational_safety_failures": sorted(set(safety)),
        "optimization_watches": sorted(watches, key=lambda row: (row["label"], row["field"])),
        "legacy_target_assertions": sorted(legacy_assertions),
        "target_p99_ns": P99_TARGET_NS,
        "durability_hard_max_ns": DURABILITY_HARD_MAX_NS,
        "frozen_test_executable_sha256": FROZEN_TRANSITION_TEST_EXECUTABLE_SHA256,
        "frozen_capture_source_sha256": source_hash,
    }
    classification["correctness_pass"] = not classification["correctness_failures"]
    classification["operational_safety_pass"] = not classification[
        "operational_safety_failures"
    ]
    classification["optimization_pass"] = not classification["optimization_watches"]
    classification["optimization_status"] = (
        "PASS" if classification["optimization_pass"] else "OPTIMIZATION_WATCH"
    )
    classification["hard_gate_status"] = (
        "PASS"
        if classification["correctness_pass"]
        and classification["operational_safety_pass"]
        else "VETO"
    )
    classification["classification_root_sha256"] = base.sha256_bytes(
        base.canonical_bytes(classification)
    )
    return classification


_BASE_EVALUATE = base.evaluate_measurement
_BASE_ADD_ROOT = base.add_root


def evaluate_measurement(*args: Any, **kwargs: Any) -> tuple[dict[str, Any], dict[str, Any]]:
    resource, parity = _BASE_EVALUATE(*args, **kwargs)
    source = args[1]
    evidence = args[5]
    resource["_s1c3d_parity"] = parity
    classification = classify_resource(resource, source, evidence)
    resource["schema"] = RESOURCE_SCHEMA
    resource["classification"] = classification
    resource["resource_failures"] = classification["operational_safety_failures"]
    resource["instrument_failures"] = classification["correctness_failures"]
    resource["all_pass_before_monitor"] = not resource["resource_failures"] and not resource["instrument_failures"]
    return resource, parity


def add_root(value: dict[str, Any], field: str) -> dict[str, Any]:
    if value.get("schema") == RESOURCE_SCHEMA and field == "resource_root_sha256":
        classification = dict(value["classification"])
        late_instruments = sorted(
            set(value.get("instrument_failures", ()))
            - set(classification["correctness_failures"])
        )
        correctness = sorted(
            set(classification["correctness_failures"]) | set(late_instruments)
        )
        classification["correctness_failures"] = correctness
        classification["correctness_pass"] = not correctness
        classification["operational_safety_pass"] = not classification[
            "operational_safety_failures"
        ]
        classification["optimization_pass"] = not classification[
            "optimization_watches"
        ]
        classification["optimization_status"] = (
            "PASS" if classification["optimization_pass"] else "OPTIMIZATION_WATCH"
        )
        classification["hard_gate_status"] = (
            "PASS"
            if classification["correctness_pass"]
            and classification["operational_safety_pass"]
            else "VETO"
        )
        classification["classification_root_sha256"] = base.sha256_bytes(
            base.canonical_bytes(classification, "classification_root_sha256")
        )
        value["classification"] = classification
        value["resource_failures"] = classification["operational_safety_failures"]
        value["instrument_failures"] = correctness
        value["all_pass"] = (
            classification["correctness_pass"]
            and classification["operational_safety_pass"]
        )
    return _BASE_ADD_ROOT(value, field)


@contextlib.contextmanager
def _patched_mechanism() -> Iterator[None]:
    old_parity = base.run_parity
    old_evaluate = base.evaluate_measurement
    old_add_root = base.add_root
    old_parity_schema = base.PARITY_SCHEMA
    old_paper = (
        base.PAPER_COMMIT,
        base.PAPER_TREE,
        base.PAPER_MANIFEST_ROOT,
        base.PAPER_VERIFICATION_SHA256,
        base.PAPER_CRITIQUE_SHA256,
    )
    try:
        base.run_parity = run_parity
        base.evaluate_measurement = evaluate_measurement
        base.add_root = add_root
        base.PARITY_SCHEMA = PARITY_SCHEMA
        base.PAPER_COMMIT = PAPER_COMMIT
        base.PAPER_TREE = PAPER_TREE
        base.PAPER_MANIFEST_ROOT = PAPER_MANIFEST_ROOT
        base.PAPER_VERIFICATION_SHA256 = PAPER_VERIFICATION_SHA256
        base.PAPER_CRITIQUE_SHA256 = PAPER_CRITIQUE_SHA256
        yield
    finally:
        base.run_parity = old_parity
        base.evaluate_measurement = old_evaluate
        base.add_root = old_add_root
        base.PARITY_SCHEMA = old_parity_schema
        (
            base.PAPER_COMMIT,
            base.PAPER_TREE,
            base.PAPER_MANIFEST_ROOT,
            base.PAPER_VERIFICATION_SHA256,
            base.PAPER_CRITIQUE_SHA256,
        ) = old_paper


def prepare(args: argparse.Namespace) -> int:
    with _patched_mechanism():
        return base.prepare(args)


# The mutation mechanism itself is unchanged after preparation.
execute = base.execute
rollback = base.rollback
finalize = base.finalize
seal = base.seal
read_json = base.read_json
write_json = base.write_json
verify_current_production = base.verify_current_production
service_snapshot = base.service_snapshot
GateFailure = base.GateFailure
STATE_SCHEMA = base.STATE_SCHEMA
PREDEPLOYMENT_SCHEMA = base.PREDEPLOYMENT_SCHEMA
