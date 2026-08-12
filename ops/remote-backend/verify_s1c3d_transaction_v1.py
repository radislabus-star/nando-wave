#!/usr/bin/env python3
"""Independent verifier and authority envelope for S1C-3D."""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Iterator

import s1c3d_remote_transaction_v1 as executor
import verify_s1c3b_transaction_v1 as legacy_verifier


SCHEMA = "nando.s1c3d-authority-envelope.v1"
STATE_SCHEMA = "nando.s1c3d-state.v1"
FREEZE_SCHEMA = "nando.s1c3d-implementation-freeze.v1"
PREDEPLOYMENT_SCHEMA = "nando.s1c3d-predeployment-verification.v1"
FINAL_VERIFICATION_SCHEMA = "nando.s1c3d-final-verification.v1"
ATTEMPT_RE = re.compile(r"^\d{8}T\d{6}Z-c3eaddc55dfc-s1c3d-v1$")

IMPLEMENTATION_FILES = (
    "s1c3d_remote_transaction_v1.py",
    "verify_s1c3d_transaction_v1.py",
    "s1c3d_transaction_v1.py",
    "test_s1c3d_transaction_v1.py",
    "test_verify_s1c3d_transaction_v1.py",
    "run_s1c3d_transaction_v1.sh",
)
FROZEN_SOURCE_FILES = tuple(f"ops/remote-backend/{name}" for name in IMPLEMENTATION_FILES)
DEPENDENCY_HASHES = {
    "s1c3b_remote_transaction_v1.py": "74fde9997bb14f4064aa01303cc67cd79e2dea826f39bfc50850e49394b70523",
    "verify_s1c3b_transaction_v1.py": "72e29c6f52e3e29648a7f1bf13cc66b02ad5f0fe68db07cd9dbfa54ff86561dd",
    "s1c3_remote_transaction_v7.py": "d0a490d93cc5dbd488119d7cc721de0cf9609ab5d97c87efb8a1de69916ab971",
    "verify_s1c3_transaction_v7.py": "8e383844e7d945cd94829dfd772a68fffcb89457556a30edb41bb42b615162bc",
}


class InvalidReceipt(ValueError):
    pass


def canonical_bytes(value: Any, omit: str | None = None) -> bytes:
    if omit is not None:
        value = dict(value)
        value.pop(omit, None)
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def file_digest(path: Path) -> str:
    return digest(path.read_bytes())


def require(condition: bool, error: str) -> None:
    if not condition:
        raise InvalidReceipt(error)


def require_exact(actual: Any, expected: Any, label: str) -> None:
    require(actual == expected, f"{label}_mismatch")


def require_hash(value: Any, label: str) -> str:
    require(
        isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None,
        f"{label}_invalid",
    )
    return value


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise InvalidReceipt(f"json_invalid:{path.name}") from error
    require(isinstance(value, dict), f"json_not_object:{path.name}")
    return value


def verify_root(value: dict[str, Any], field: str, label: str) -> str:
    root = require_hash(value.get(field), f"{label}_root")
    require_exact(digest(canonical_bytes(value, field)), root, f"{label}_root")
    return root


def atomic_write(path: Path, value: dict[str, Any], mode: int = 0o400) -> None:
    payload = canonical_bytes(value)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        os.write(descriptor, payload)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    os.chmod(temporary, mode)
    os.replace(temporary, path)
    directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def verify_dependencies() -> dict[str, str]:
    directory = Path(__file__).resolve().parent
    for name, expected in DEPENDENCY_HASHES.items():
        require(file_digest(directory / name) == expected, f"dependency_drift:{name}")
    return dict(DEPENDENCY_HASHES)


def bundle_identity(bundle: Path, source_commit: str) -> tuple[str, dict[str, str]]:
    require(bundle.is_file(), "source_bundle_missing")
    with tempfile.TemporaryDirectory(prefix="nando-s1c3d-bundle-") as directory:
        repository = Path(directory) / "repository.git"
        completed = subprocess.run(
            ["git", "clone", "--quiet", "--bare", str(bundle), str(repository)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        require(completed.returncode == 0, "source_bundle_clone_failed")
        tree = subprocess.run(
            ["git", "-C", str(repository), "rev-parse", f"{source_commit}^{{tree}}"],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        require(tree.returncode == 0, "source_commit_missing_from_bundle")
        files = {}
        for name in FROZEN_SOURCE_FILES:
            content = subprocess.run(
                ["git", "-C", str(repository), "show", f"{source_commit}:{name}"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            require(content.returncode == 0, f"frozen_source_file_missing:{name}")
            files[name] = digest(content.stdout)
    return tree.stdout.strip(), files


def create_implementation_freeze(
    source_commit: str,
    source_tree: str,
    bundle: Path,
    implementation_directory: Path,
) -> dict[str, Any]:
    require(re.fullmatch(r"[0-9a-f]{40}", source_commit) is not None, "source_commit")
    require(re.fullmatch(r"[0-9a-f]{40}", source_tree) is not None, "source_tree")
    bundle_tree, source_files = bundle_identity(bundle, source_commit)
    require_exact(bundle_tree, source_tree, "source_bundle_tree")
    implementation_files = {}
    for name in IMPLEMENTATION_FILES:
        path = implementation_directory / name
        require(path.is_file(), f"implementation_file_missing:{name}")
        implementation_files[name] = file_digest(path)
        require_exact(
            source_files[f"ops/remote-backend/{name}"],
            implementation_files[name],
            f"implementation_file_bundle:{name}",
        )
    receipt = {
        "schema": FREEZE_SCHEMA,
        "paper_commit": executor.PAPER_COMMIT,
        "source_commit": source_commit,
        "source_tree": source_tree,
        "source_bundle_sha256": file_digest(bundle),
        "source_files": source_files,
        "implementation_files": implementation_files,
    }
    receipt["implementation_freeze_root_sha256"] = digest(canonical_bytes(receipt))
    return receipt


def verify_implementation_freeze(
    path: Path, implementation_directory: Path, bundle: Path | None = None
) -> dict[str, Any]:
    receipt = load_json(path)
    require_exact(receipt.get("schema"), FREEZE_SCHEMA, "implementation_freeze_schema")
    verify_root(receipt, "implementation_freeze_root_sha256", "implementation_freeze")
    require_exact(receipt.get("paper_commit"), executor.PAPER_COMMIT, "implementation_freeze_paper")
    require(
        set(receipt.get("implementation_files", {})) == set(IMPLEMENTATION_FILES),
        "implementation_freeze_file_set",
    )
    require(
        set(receipt.get("source_files", {})) == set(FROZEN_SOURCE_FILES),
        "implementation_freeze_source_set",
    )
    for name in IMPLEMENTATION_FILES:
        require_exact(
            file_digest(implementation_directory / name),
            receipt["implementation_files"][name],
            f"implementation_file_drift:{name}",
        )
    if bundle is not None:
        require_exact(file_digest(bundle), receipt.get("source_bundle_sha256"), "implementation_bundle")
        tree, files = bundle_identity(bundle, receipt["source_commit"])
        require_exact(tree, receipt.get("source_tree"), "implementation_bundle_tree")
        require_exact(files, receipt.get("source_files"), "implementation_bundle_files")
    return receipt


def _verify_snapshot(snapshot: dict[str, Any], directory: Path) -> str:
    require_exact(snapshot.get("schema"), executor.SNAPSHOT_SCHEMA, "snapshot_schema")
    root = verify_root(snapshot, "snapshot_root_sha256", "snapshot")
    fixture = snapshot.get("directory", {})
    require_exact(fixture.get("uid"), 0, "snapshot_directory_uid")
    require_exact(fixture.get("gid"), 1000, "snapshot_directory_gid")
    require_exact(fixture.get("mode_octal"), "0550", "snapshot_directory_mode")
    require(isinstance(fixture.get("path"), str), "snapshot_directory_path")
    parent = snapshot.get("parent", {})
    require_exact(parent.get("path"), str(executor.SNAPSHOT_PARENT), "snapshot_parent_path")
    require_exact(parent.get("uid"), 0, "snapshot_parent_uid")
    require_exact(parent.get("gid"), 0, "snapshot_parent_gid")
    require_exact(parent.get("mode_octal"), "0711", "snapshot_parent_mode")
    expected = {
        "registry": (
            str(executor.base.legacy.RESPONSE_REGISTRY),
            "response-registry.json",
        ),
        "admission": (str(executor.base.legacy.ADMISSION), "admission.json"),
    }
    bindings = snapshot.get("bindings")
    require(isinstance(bindings, dict) and set(bindings) == set(expected), "snapshot_bindings")
    for label, (source_path, filename) in expected.items():
        binding = bindings[label]
        source = binding.get("source", {})
        bounded = binding.get("snapshot", {})
        require_exact(source.get("path"), source_path, f"snapshot_{label}_source_path")
        require_exact(source.get("read_stable"), True, f"snapshot_{label}_source_stable")
        require_exact(bounded.get("path"), f'{fixture["path"]}/{filename}', f"snapshot_{label}_path")
        require_exact(bounded.get("uid"), 0, f"snapshot_{label}_uid")
        require_exact(bounded.get("gid"), 1000, f"snapshot_{label}_gid")
        require_exact(bounded.get("mode_octal"), "0440", f"snapshot_{label}_mode")
        require_exact(bounded.get("sha256"), source.get("sha256"), f"snapshot_{label}_sha")
        require_exact(bounded.get("size_bytes"), source.get("size_bytes"), f"snapshot_{label}_size")
        require_hash(source.get("sha256"), f"snapshot_{label}_source_sha")
        # The authority artifacts are mutable production projections.  Their
        # one-read fstat/hash binding is frozen in this receipt; rereading the
        # live path here would compare the snapshot with a later generation.
        snapshot_path = Path(bounded["path"])
        if os.geteuid() == 0:
            require(snapshot_path.is_file(), f"snapshot_{label}_runtime_missing")
            runtime = snapshot_path.stat()
            require_exact(runtime.st_uid, 0, f"snapshot_{label}_runtime_uid")
            require_exact(runtime.st_gid, 1000, f"snapshot_{label}_runtime_gid")
            require_exact(
                f"{stat.S_IMODE(runtime.st_mode):04o}",
                "0440",
                f"snapshot_{label}_runtime_mode",
            )
            require_exact(runtime.st_dev, bounded.get("device"), f"snapshot_{label}_runtime_device")
            require_exact(runtime.st_ino, bounded.get("inode"), f"snapshot_{label}_runtime_inode")
            require_exact(file_digest(snapshot_path), bounded.get("sha256"), f"snapshot_{label}_runtime_sha")
    probe = snapshot.get("permission_probe", {})
    require_exact(probe.get("returncode"), 0, "snapshot_probe_returncode")
    result = probe.get("result")
    require(isinstance(result, dict), "snapshot_probe_result")
    require_exact(result.get("all_denied"), True, "snapshot_probe_all_denied")
    directory_denials = result.get("directory", {})
    require(
        set(directory_denials) == {"chmod", "rename"},
        "snapshot_probe_directory_operations",
    )
    for operation, verdict in directory_denials.items():
        require_exact(verdict.get("denied"), True, f"snapshot_probe_directory_{operation}")
        require(
            verdict.get("errno") in {1, 13, 30},
            f"snapshot_probe_directory_{operation}_errno",
        )
    require(set(result.get("files", {})) == {name for _, name in expected.values()}, "snapshot_probe_files")
    log = directory / "evidence" / "parity-snapshot-permissions.log"
    require(log.is_file(), "snapshot_probe_log_missing")
    require_exact(digest(log.read_bytes()), probe.get("output_sha256"), "snapshot_probe_output")
    require_exact(json.loads(log.read_text()), result, "snapshot_probe_result_log")
    for filename, row in result["files"].items():
        label = "registry" if filename == "response-registry.json" else "admission"
        require_exact(row.get("read_sha256"), bindings[label]["snapshot"]["sha256"], f"snapshot_probe_{label}_read")
        denials = row.get("denials", {})
        require(set(denials) == {"chmod", "write", "unlink", "rename"}, f"snapshot_probe_{label}_operations")
        for operation, verdict in denials.items():
            require_exact(verdict.get("denied"), True, f"snapshot_probe_{label}_{operation}")
            require(verdict.get("errno") in {1, 13, 30}, f"snapshot_probe_{label}_{operation}_errno")
    return root


def _verify_parity(
    directory: Path, resource: dict[str, Any], commands: dict[str, Any]
) -> tuple[str, dict[str, Any]]:
    parity = load_json(directory / "parity-receipt.json")
    require_exact(parity.get("schema"), executor.PARITY_SCHEMA, "parity_schema")
    root = verify_root(parity, "parity_root_sha256", "parity")
    snapshot = parity.get("snapshot")
    require(isinstance(snapshot, dict), "parity_snapshot_shape")
    snapshot_root = _verify_snapshot(snapshot, directory)
    require_exact(parity.get("snapshot_root_sha256"), snapshot_root, "parity_snapshot_root")
    require_exact(
        parity.get("snapshot_retained_for_authority_verifier"),
        True,
        "parity_snapshot_retained",
    )
    rows = parity.get("rows")
    require(isinstance(rows, list) and len(rows) == 2, "parity_rows")
    payloads = {}
    for row, label in zip(rows, ("baseline", "candidate"), strict=True):
        require_exact(row.get("label"), label, f"parity_{label}_label")
        require_exact(row.get("returncode"), 0, f"parity_{label}_returncode")
        require_exact(row.get("command"), commands[f"parity-{label}"], f"parity_{label}_command")
        log = directory / "evidence" / f"parity-{label}.log"
        raw = log.read_bytes()
        lines = raw.splitlines()
        require_exact(row.get("output_sha256"), digest(raw), f"parity_{label}_raw")
        require_exact(row.get("row_count"), len(lines) - 1, f"parity_{label}_rows")
        require_exact(row.get("row_count"), 16, f"parity_{label}_denominator")
        payloads[label] = b"\n".join(lines[1:])
        require_exact(parity.get(f"{label}_output_sha256"), digest(payloads[label]), f"parity_{label}_payload")
    require_exact(payloads["baseline"], payloads["candidate"], "parity_payload")
    require_exact(parity.get("byte_identical"), True, "parity_byte_identity")
    require_exact(parity.get("row_count"), 16, "parity_row_count")
    require_exact(resource.get("parity_root_sha256"), root, "resource_parity_root")
    return root, parity


def _watch(label: str, field: str, observed: int) -> dict[str, Any]:
    return {
        "label": label,
        "field": field,
        "observed_ns": observed,
        "target_ns": executor.P99_TARGET_NS,
        "ratio_ppm": observed * 1_000_000 // executor.P99_TARGET_NS,
    }


def _legacy_assertion(
    directory: Path,
    row: dict[str, Any],
    kind: str,
    watches: list[dict[str, Any]],
) -> bool:
    if row.get("returncode") != 101 or not watches:
        return False
    command = row.get("command", {})
    if (
        command.get("returncode") != 101
        or command.get("executable_sha256")
        != executor.FROZEN_TRANSITION_TEST_EXECUTABLE_SHA256
    ):
        return False
    metric = row.get("metrics", {})
    hard_fields = (
        ("hard_max_ns",)
        if kind == "single"
        else (
            "precommit_hard_max_ns",
            "settlement_hard_max_ns",
            "episode_hard_max_ns",
        )
    )
    if any(metric.get(field, executor.DURABILITY_HARD_MAX_NS + 1) > executor.DURABILITY_HARD_MAX_NS for field in hard_fields):
        return False
    if kind == "single":
        message = executor.EXPECTED_TARGET_ASSERTIONS["single"]
    elif metric.get("precommit_p99_ns", 0) > executor.P99_TARGET_NS:
        message = executor.EXPECTED_TARGET_ASSERTIONS["three-precommit"]
    else:
        message = executor.EXPECTED_TARGET_ASSERTIONS["three-settlement"]
    text = (directory / "evidence" / f'{row["label"]}.log').read_text(errors="replace")
    return (
        text.count("panicked at") == 1
        and message in text
        and "test result: FAILED. 0 passed; 1 failed;" in text
        and "hard ceiling exceeded" not in text
        and "PermissionDenied" not in text
    )


def _classify(
    directory: Path,
    resource: dict[str, Any],
    executable_drift: bool,
    monitor_pass: bool,
) -> dict[str, Any]:
    correctness = []
    safety = []
    watches = []
    legacy_assertions = []
    if resource.get("classification", {}).get("frozen_capture_source_sha256") != executor.FROZEN_GROUNDED_CAPTURE_SOURCE_SHA256:
        correctness.append("legacy_target_assertion_source_identity")
    for row in resource["floor_probes"]:
        if row["records"] != executor.base.FLOOR_RECORDS or row["returncode"] != 0 or row["error"]:
            correctness.append(f'{row["label"]}:floor_incomplete')
    for row in resource["metrics"]["hot_latency"]:
        label = row["label"]
        metric = row.get("metrics")
        if metric is None:
            correctness.append(f"{label}:metric_missing")
            continue
        if row["returncode"] != 0 or not row["test_assertion_pass"]:
            correctness.append(f"{label}:test_assertion_failed")
        if row["command"].get("observed_affinity") != [executor.base.MEASUREMENT_CPU]:
            correctness.append(f"{label}:affinity_invalid")
        if metric["samples"] != 4096:
            correctness.append(f"{label}:denominator")
        for field, limit, suffix in (
            ("p99_ns", 1_000_000, "matched_p99"),
            ("no_goal_p99_ns", 250_000, "no_goal_p99"),
            ("hard_max_ns", 2_000_000, "hard_max"),
        ):
            if metric[field] > limit:
                safety.append(f"{label}:{suffix}")
    for name, kind, denominator in (
        ("single_ledger_sync", "single", 1024),
        ("three_ledger_sync", "three", 256),
    ):
        for row in resource["metrics"][name]:
            label = row["label"]
            metric = row.get("metrics")
            if metric is None:
                correctness.append(f"{label}:metric_missing")
                continue
            if row["command"].get("observed_affinity") != [executor.base.MEASUREMENT_CPU]:
                correctness.append(f"{label}:affinity_invalid")
            if metric["samples"] != denominator:
                correctness.append(f"{label}:denominator")
            target_fields = ("p99_ns",) if kind == "single" else ("precommit_p99_ns", "settlement_p99_ns")
            hard_fields = ("hard_max_ns",) if kind == "single" else (
                "precommit_hard_max_ns",
                "settlement_hard_max_ns",
                "episode_hard_max_ns",
            )
            row_watches = [_watch(label, field, metric[field]) for field in target_fields if metric[field] > executor.P99_TARGET_NS]
            watches.extend(row_watches)
            safety.extend(f"{label}:{field}" for field in hard_fields if metric[field] > executor.DURABILITY_HARD_MAX_NS)
            if row["returncode"] == 0 and row["test_assertion_pass"] and not row_watches:
                continue
            if _legacy_assertion(directory, row, kind, row_watches):
                legacy_assertions.append(label)
            else:
                correctness.append(f"{label}:test_assertion_failed")
    idle = resource["metrics"]["idle_cpu"]
    metric = idle.get("metrics")
    if metric is None:
        correctness.append("idle:metric_missing")
    elif metric["percent_of_one_core"] > 0.25:
        safety.append("idle:percent_of_one_core")
    if idle["returncode"] != 0 or not idle["test_assertion_pass"]:
        correctness.append("idle:test_assertion_failed")
    if idle["command"].get("observed_affinity") != [executor.base.MEASUREMENT_CPU]:
        correctness.append("idle:affinity_invalid")
    rss = resource["metrics"]["rss"]
    if any(row["rss_bytes"] is None or row["sample_count"] != 20 or row["error"] for row in rss["rows"]):
        correctness.append("rss:incomplete")
    elif rss["delta_bytes"] > 16 * 1024 * 1024:
        safety.append("rss:delta")
    if executable_drift:
        correctness.append("executable_drift_after_measurement")
    if not monitor_pass:
        correctness.append("monitor_instrument_failure")
    value = {
        "schema": executor.CLASSIFICATION_SCHEMA,
        "correctness_failures": sorted(set(correctness)),
        "operational_safety_failures": sorted(set(safety)),
        "optimization_watches": sorted(watches, key=lambda row: (row["label"], row["field"])),
        "legacy_target_assertions": sorted(legacy_assertions),
        "target_p99_ns": executor.P99_TARGET_NS,
        "durability_hard_max_ns": executor.DURABILITY_HARD_MAX_NS,
        "frozen_test_executable_sha256": executor.FROZEN_TRANSITION_TEST_EXECUTABLE_SHA256,
        "frozen_capture_source_sha256": executor.FROZEN_GROUNDED_CAPTURE_SOURCE_SHA256,
    }
    value["correctness_pass"] = not value["correctness_failures"]
    value["operational_safety_pass"] = not value["operational_safety_failures"]
    value["optimization_pass"] = not value["optimization_watches"]
    value["optimization_status"] = "PASS" if value["optimization_pass"] else "OPTIMIZATION_WATCH"
    value["hard_gate_status"] = "PASS" if value["correctness_pass"] and value["operational_safety_pass"] else "VETO"
    value["classification_root_sha256"] = digest(canonical_bytes(value))
    return value


def verify_resource(
    directory: Path,
    resource: dict[str, Any],
    monitor: dict[str, Any],
    executables: dict[str, Any],
    executable_root: str,
    ownership_root: str,
    executable_drift: bool,
) -> str:
    require_exact(resource.get("schema"), executor.RESOURCE_SCHEMA, "resource_schema")
    root = verify_root(resource, "resource_root_sha256", "resource")
    require_exact(resource.get("candidate_commit"), executor.base.CANDIDATE_COMMIT, "resource_candidate")
    require_exact(resource.get("measurement_cpu"), executor.base.MEASUREMENT_CPU, "resource_cpu")
    require_exact(resource.get("round_count"), executor.base.ROUND_COUNT, "resource_rounds")
    require_exact(resource.get("executable_set_root_sha256"), executable_root, "resource_executable_root")
    require_exact(resource.get("oracle_ownership_root_sha256"), ownership_root, "resource_ownership_root")
    require_exact(resource.get("monitor_root_sha256"), monitor["monitor_root_sha256"], "resource_monitor_root")
    commands_list = monitor.get("commands")
    require(isinstance(commands_list, list) and len(commands_list) == len(executor.base.MEASUREMENT_LABELS), "monitor_command_count")
    commands = {row["label"]: legacy_verifier.verify_command(row, row["label"], executables) for row in commands_list}
    require_exact(
        executables["test-transition-serving"].get("source_identity"),
        executor.base.CANDIDATE_COMMIT,
        "transition_test_source_identity",
    )
    floors = resource.get("floor_probes")
    require(isinstance(floors, list) and len(floors) == executor.base.ROUND_COUNT * 2, "floor_count")
    for index, row in enumerate(floors):
        legacy_verifier.verify_floor(directory, row, index, commands)
    metrics = resource.get("metrics")
    require(isinstance(metrics, dict), "metrics_shape")
    for name, prefix, test in (
        ("hot_latency", "hot", executor.base.HOT_TEST),
        ("single_ledger_sync", "single-sync", executor.base.SINGLE_SYNC_TEST),
        ("three_ledger_sync", "three-sync", executor.base.THREE_SYNC_TEST),
    ):
        rows = metrics.get(name)
        require(isinstance(rows, list) and len(rows) == executor.base.ROUND_COUNT, f"{name}_count")
        for index, row in enumerate(rows, start=1):
            legacy_verifier.verify_metric_row(directory, row, f"{prefix}-{index}", test, commands)
    legacy_verifier.verify_metric_row(directory, metrics.get("idle_cpu"), "idle", executor.base.IDLE_TEST, commands)
    rss = metrics.get("rss")
    require(isinstance(rss, dict) and len(rss.get("rows", [])) == 2, "rss_shape")
    for row, label in zip(rss["rows"], ("capture_off", "capture_on"), strict=True):
        require_exact(row.get("label"), label, f"rss_{label}_label")
        legacy_verifier.verify_command(row.get("command"), f"rss-{label}", executables)
    if all(row.get("rss_bytes") is not None and row.get("sample_count") == 20 and row.get("error") is None for row in rss["rows"]):
        require_exact(rss.get("delta_bytes"), max(0, rss["rows"][1]["rss_bytes"] - rss["rows"][0]["rss_bytes"]), "rss_delta")
    _verify_parity(directory, resource, commands)
    classification = _classify(directory, resource, executable_drift, bool(monitor.get("instrument_pass")))
    require_exact(resource.get("classification"), classification, "resource_classification")
    require_exact(resource.get("resource_failures"), classification["operational_safety_failures"], "resource_safety_failures")
    require_exact(resource.get("instrument_failures"), classification["correctness_failures"], "resource_correctness_failures")
    require_exact(resource.get("all_pass"), classification["hard_gate_status"] == "PASS", "resource_all_pass")
    initial_correctness = [
        value
        for value in classification["correctness_failures"]
        if value not in {"executable_drift_after_measurement", "monitor_instrument_failure"}
    ]
    require_exact(
        resource.get("all_pass_before_monitor"),
        not initial_correctness and not classification["operational_safety_failures"],
        "resource_pre_monitor_pass",
    )
    return root


@contextlib.contextmanager
def patched_legacy_verifier() -> Iterator[None]:
    fields = {
        "PAPER_COMMIT": executor.PAPER_COMMIT,
        "PAPER_TREE": executor.PAPER_TREE,
        "PAPER_MANIFEST_ROOT": executor.PAPER_MANIFEST_ROOT,
        "PAPER_VERIFICATION_SHA256": executor.PAPER_VERIFICATION_SHA256,
        "PAPER_CRITIQUE_SHA256": executor.PAPER_CRITIQUE_SHA256,
        "RESOURCE_SCHEMA": executor.RESOURCE_SCHEMA,
        "PARITY_SCHEMA": executor.PARITY_SCHEMA,
        "verify_resource": verify_resource,
    }
    old = {name: getattr(legacy_verifier, name) for name in fields}
    try:
        for name, value in fields.items():
            setattr(legacy_verifier, name, value)
        yield
    finally:
        for name, value in old.items():
            setattr(legacy_verifier, name, value)


def verify_preparation(directory: Path) -> dict[str, Any]:
    with patched_legacy_verifier():
        legacy = legacy_verifier.verify_preparation(directory)
    resource = load_json(directory / "resource-receipt.json")
    classification = resource["classification"]
    result = {
        "schema": PREDEPLOYMENT_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": (
            "S1C3D_PREPARATION_PASS"
            if classification["optimization_pass"]
            else "S1C3D_PREPARATION_PASS_WITH_OPTIMIZATION_WATCH"
        ),
        "preparation_root_sha256": legacy["preparation_root_sha256"],
        "oracle_ownership_root_sha256": legacy["oracle_ownership_root_sha256"],
        "monitor_root_sha256": legacy["monitor_root_sha256"],
        "resource_root_sha256": legacy["resource_root_sha256"],
        "parity_root_sha256": legacy["parity_root_sha256"],
        "classification_root_sha256": classification["classification_root_sha256"],
        "optimization_status": classification["optimization_status"],
        "optimization_watches": classification["optimization_watches"],
        "scientific_authority": False,
        "model_training": False,
        "phase_mutation": False,
    }
    result["predeployment_verification_root_sha256"] = digest(canonical_bytes(result))
    return result


def verify_resource_veto(directory: Path) -> dict[str, Any]:
    with patched_legacy_verifier():
        legacy = legacy_verifier.verify_resource_veto(directory)
    resource = load_json(directory / "resource-receipt.json")
    classification = resource["classification"]
    require_exact(classification.get("hard_gate_status"), "VETO", "resource_veto_hard_gate")
    if classification["correctness_failures"]:
        verdict = "S1C3D_CORRECTNESS_VETO"
    else:
        require(bool(classification["operational_safety_failures"]), "resource_veto_without_failure")
        verdict = "S1C3D_SAFETY_VETO"
    result = {
        "schema": "nando.s1c3d-terminal-verification.v1",
        "valid": True,
        "authority": False,
        "verdict": verdict,
        "resource_root_sha256": legacy["resource_root_sha256"],
        "classification_root_sha256": classification[
            "classification_root_sha256"
        ],
        "correctness_failures": classification["correctness_failures"],
        "operational_safety_failures": classification[
            "operational_safety_failures"
        ],
        "optimization_watches": classification["optimization_watches"],
        "production_mutation": False,
        "capture_installed": False,
        "s1c4_state": "CLOSED",
        "scientific_authority": False,
        "model_training": False,
        "phase_mutation": False,
    }
    result["terminal_verification_root_sha256"] = digest(canonical_bytes(result))
    return result


def _legacy_predeployment_projection(directory: Path, verification: dict[str, Any]) -> dict[str, Any]:
    preparation = load_json(directory / "preparation.json")
    result = {
        "schema": executor.base.PREDEPLOYMENT_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": "S1C3B_PREPARATION_PASS",
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "oracle_ownership_root_sha256": preparation["oracle_ownership_root_sha256"],
        "monitor_root_sha256": preparation["monitor_root_sha256"],
        "resource_root_sha256": preparation["resource_root_sha256"],
        "parity_root_sha256": preparation["parity_root_sha256"],
    }
    result["predeployment_verification_root_sha256"] = digest(canonical_bytes(result))
    require(verification.get("authority") is True, "predeployment_projection_without_authority")
    return result


def verify_final(directory: Path) -> dict[str, Any]:
    with patched_legacy_verifier():
        legacy = legacy_verifier.verify_final(directory)
    resource = load_json(directory / "resource-receipt.json")
    classification = resource["classification"]
    require(classification["hard_gate_status"] == "PASS", "final_hard_gate_veto")
    verdict_map = {
        "S1C3B_DEPLOYMENT_PASS": (
            "S1C3D_DEPLOYMENT_PASS"
            if classification["optimization_pass"]
            else "S1C3D_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH"
        ),
        "S1C3B_ROLLBACK_PASS": "S1C3D_ROLLBACK_PASS",
        "S1C3B_VETO": "S1C3D_VETO",
    }
    require(legacy.get("verdict") in verdict_map, "final_legacy_verdict")
    deployment = legacy["verdict"] == "S1C3B_DEPLOYMENT_PASS"
    preparation = load_json(directory / "preparation.json")
    deployment_receipt = load_json(directory / "deployment-receipt.json")
    cursor = {
        "schema": "nando.s1c4-append-cursor.v1",
        "transaction_id": preparation["transaction_id"],
        "deployment_receipt_root_sha256": deployment_receipt[
            "receipt_root_sha256"
        ],
        "journal_at_open": deployment_receipt["journal_after"],
        "retroactive_rows_allowed": False,
    }
    cursor["cursor_root_sha256"] = digest(canonical_bytes(cursor))
    result = {
        "schema": FINAL_VERIFICATION_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": verdict_map[legacy["verdict"]],
        "receipt_root_sha256": legacy["receipt_root_sha256"],
        "preparation_root_sha256": legacy["preparation_root_sha256"],
        "classification_root_sha256": classification["classification_root_sha256"],
        "optimization_status": classification["optimization_status"],
        "optimization_watches": classification["optimization_watches"],
        "capture_installed": deployment,
        "s1c4_state": "COLLECTING" if deployment else "CLOSED",
        "s1c4_cursor": cursor if deployment else None,
        "scientific_authority": False,
        "model_training": False,
        "phase_mutation": False,
    }
    result["final_verification_root_sha256"] = digest(canonical_bytes(result))
    return result


def transaction_id(directory: Path) -> str:
    for name in ("preparation.json", "transaction-state.json"):
        path = directory / name
        if path.is_file():
            value = load_json(path).get("transaction_id")
            if isinstance(value, str):
                require(ATTEMPT_RE.fullmatch(value) is not None, "attempt_id_invalid")
                return value
    raise InvalidReceipt("attempt_id_missing")


def build_envelope(
    directory: Path,
    implementation_freeze_path: Path,
    *,
    predeployment: bool,
    recorded_verification: Path | None = None,
) -> dict[str, Any]:
    dependencies = verify_dependencies()
    freeze = verify_implementation_freeze(
        implementation_freeze_path, Path(__file__).resolve().parent
    )
    state = load_json(directory / "transaction-state.json").get("state")
    if predeployment:
        verification = verify_preparation(directory)
    elif state == "RESOURCE_VETO":
        verification = verify_resource_veto(directory)
    else:
        verification = verify_final(directory)
    if recorded_verification is not None:
        recorded = load_json(recorded_verification)
        if recorded.get("schema") == SCHEMA:
            recorded = recorded.get("verification")
        require_exact(recorded, verification, "recorded_verification")
    result = {
        "schema": SCHEMA,
        "valid": True,
        "authority": verification["authority"],
        "verdict": verification["verdict"],
        "transaction_id": transaction_id(directory),
        "protocol": {
            "commit": executor.PAPER_COMMIT,
            "tree": executor.PAPER_TREE,
            "preregistration_sha256": executor.PAPER_PREREGISTRATION_SHA256,
            "critique_sha256": executor.PAPER_CRITIQUE_SHA256,
            "paper_manifest_root": executor.PAPER_MANIFEST_ROOT,
        },
        "implementation_freeze_root_sha256": freeze[
            "implementation_freeze_root_sha256"
        ],
        "implementation_source": {
            "commit": freeze["source_commit"],
            "tree": freeze["source_tree"],
            "bundle_sha256": freeze["source_bundle_sha256"],
        },
        "dependencies": dependencies,
        "verification": verification,
        "verification_sha256": digest(canonical_bytes(verification)),
        "production_mutation": verification.get(
            "production_mutation", False if predeployment else True
        ),
        "production_restored": verification["verdict"]
        in {"S1C3D_ROLLBACK_PASS", "S1C3D_VETO"},
        "capture_installed": verification.get("capture_installed", False),
        "s1c4_state": verification.get("s1c4_state", "CLOSED"),
        "scientific_authority": False,
        "model_training": False,
        "phase_mutation": False,
    }
    result["authority_envelope_root_sha256"] = digest(canonical_bytes(result))
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    verify = subparsers.add_parser("verify")
    verify.add_argument("transaction_directory", type=Path)
    verify.add_argument("--implementation-freeze", type=Path, required=True)
    verify.add_argument("--recorded-verification", type=Path)
    verify.add_argument("--pre-deployment", action="store_true")
    freeze = subparsers.add_parser("create-freeze")
    freeze.add_argument("--source-commit", required=True)
    freeze.add_argument("--source-tree", required=True)
    freeze.add_argument("--bundle", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "create-freeze":
            print(
                json.dumps(
                    create_implementation_freeze(
                        args.source_commit,
                        args.source_tree,
                        args.bundle,
                        Path(__file__).resolve().parent,
                    ),
                    sort_keys=True,
                )
            )
        else:
            print(
                json.dumps(
                    build_envelope(
                        args.transaction_directory,
                        args.implementation_freeze,
                        predeployment=args.pre_deployment,
                        recorded_verification=args.recorded_verification,
                    ),
                    sort_keys=True,
                )
            )
        return 0
    except (InvalidReceipt, legacy_verifier.InvalidReceipt, OSError, ValueError) as error:
        print(json.dumps({"schema": SCHEMA, "valid": False, "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
