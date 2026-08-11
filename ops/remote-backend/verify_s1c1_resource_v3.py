#!/usr/bin/env python3
"""Fail-closed verifier for the frozen S1C-1 resource protocol V3."""

from __future__ import annotations

import json
import hashlib
import re
import sys
from datetime import datetime
from fractions import Fraction
from pathlib import Path
from typing import Any


SCHEMA = "nando.s1c1-resource-v3.measurements.v1"
VERDICT_SCHEMA = "nando.s1c1-resource-v3.verdict.v1"
PROTOCOL_PARENT_COMMIT = "335696e903e58c3710e7f813ed79805fec5b26cc"
PROTOCOL_EPOCH_ROOT = "2a21bc5d99a0dd8181ec105a2bdb449f66715674ffb109e3d8941a0bf9a47590"
SOURCE_MANIFEST_ROOT = "aa046add5048987c744ca25db89d1510d5f99105305d72bcfc4bed7be805b6b2"
BASELINE_BINARY_SHA256 = "ab31fde97776084de499e8d70ff3ade6d20a9d05dba912e69e5d069c777e6656"
CANDIDATE_BINARY_SHA256 = "99c8b9fe8c8e192c418aa1057bec0380c568f666166d40674685aa2132982277"
TARGETED_BINARY_SHA256 = "dd785c1c96122aa1c6aa33f5f637d92636346b15d55902659cfe067c127a124b"

TARGETED_TEST = (
    "package::tests::capture_disabled_compatibility_latency_stays_within_hot_budget"
)
INHERITED_TEST = "performance::full_generation_shadow_latency_stays_within_traffic_budget"
PAIR_ORDERS = ("baseline_candidate", "candidate_baseline", "baseline_candidate")

MATCHED_BUDGET_NS = 1_000_000
NO_MATCH_BUDGET_NS = 250_000
HARD_MAX_BUDGET_NS = 2_000_000
MAX_REGRESSION = Fraction(11, 10)
MAX_SINGLE_PAIR_REGRESSION = Fraction(2, 1)

ROOT_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
EVIDENCE_DIR_RE = re.compile(r"^s1c1-v3-[0-9a-f]{8}$")
TARGET_METRIC_RE = re.compile(
    r"S1C_HOT_LATENCY matched_p99_ns=(\d+) no_goal_p99_ns=(\d+) "
    r"hard_max_ns=(\d+) samples=(\d+)$",
    re.MULTILINE,
)
INHERITED_METRIC_RE = re.compile(
    r"F7E_LATENCY matched_p99_ns=(\d+) no_match_p99_ns=(\d+) "
    r"hard_max_ns=(\d+) samples=(\d+)$",
    re.MULTILINE,
)
RUN_LABELS = ("T1", "P1B", "P1C", "T2", "P2C", "P2B", "T3", "P3B", "P3C")
SERVICE_UNITS = (
    "nando-transport-gateway.service",
    "nando-transition-serving.service",
    "nando-response-learning.service",
    "nando-gateway-control.service",
    "nando-operator-certification-authority.service",
)


class InvalidMeasurements(ValueError):
    pass


def require_exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    actual = set(value)
    if actual != expected:
        raise InvalidMeasurements(
            f"{label}_keys_invalid:missing={sorted(expected - actual)}:extra={sorted(actual - expected)}"
        )


def require_root(value: Any, label: str) -> str:
    if not isinstance(value, str) or ROOT_RE.fullmatch(value) is None:
        raise InvalidMeasurements(f"{label}_invalid")
    return value


def require_commit(value: Any, label: str) -> str:
    if not isinstance(value, str) or COMMIT_RE.fullmatch(value) is None:
        raise InvalidMeasurements(f"{label}_invalid")
    return value


def require_int(value: Any, label: str, *, positive: bool = False) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise InvalidMeasurements(f"{label}_not_integer")
    if positive and value <= 0:
        raise InvalidMeasurements(f"{label}_not_positive")
    return value


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(64 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_evidence_manifest(evidence_dir: Path) -> tuple[str, str]:
    expected = {"environment.txt", "final.snapshot", "local_connector.before", "local_connector.after"}
    for label in RUN_LABELS:
        expected.update(
            {
                f"{label}.log",
                f"{label}.exit",
                f"{label}.before.snapshot",
                f"{label}.after.snapshot",
            }
        )
    if not evidence_dir.is_dir():
        raise InvalidMeasurements("evidence_directory_missing")
    entries = list(evidence_dir.iterdir())
    allowed = expected | {"SHA256SUMS"}
    if {path.name for path in entries} - allowed:
        raise InvalidMeasurements("evidence_entries_unexpected")
    for path in entries:
        if path.name == "SHA256SUMS":
            if not path.is_file() or path.is_symlink():
                raise InvalidMeasurements("evidence_manifest_file_invalid")
        elif not path.is_file() or path.is_symlink():
            raise InvalidMeasurements(f"evidence_entry_type_invalid:{path.name}")
    actual = {path.name for path in entries if path.name != "SHA256SUMS"}
    if actual != expected:
        raise InvalidMeasurements(
            f"evidence_files_invalid:missing={sorted(expected - actual)}:extra={sorted(actual - expected)}"
        )
    manifest = "".join(
        f"{file_sha256(evidence_dir / name)}  {name}\n" for name in sorted(expected)
    )
    return hashlib.sha256(manifest.encode("ascii")).hexdigest(), manifest


def parse_snapshot(path: Path, expected_label: str) -> tuple[datetime, tuple[tuple[str, int, int], ...]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if len(lines) < 5 or lines[0] != f"label={expected_label}":
        raise InvalidMeasurements(f"snapshot_{expected_label}_label_invalid")
    try:
        observed_at = datetime.fromisoformat(lines[1].replace(",", ".", 1))
    except ValueError as error:
        raise InvalidMeasurements(f"snapshot_{expected_label}_timestamp_invalid") from error
    if not lines[2].startswith("loadavg="):
        raise InvalidMeasurements(f"snapshot_{expected_label}_loadavg_missing")

    services: dict[str, dict[str, str]] = {}
    current_unit: str | None = None
    health: dict[str, Any] | None = None
    for line in lines[3:]:
        if line.startswith("unit="):
            current_unit = line.removeprefix("unit=")
            if current_unit in services:
                raise InvalidMeasurements(f"snapshot_{expected_label}_service_duplicate")
            services[current_unit] = {}
        elif line.startswith("{"):
            try:
                health = json.loads(line)
            except json.JSONDecodeError as error:
                raise InvalidMeasurements(f"snapshot_{expected_label}_health_invalid") from error
        elif "=" in line and current_unit is not None:
            key, value = line.split("=", 1)
            services[current_unit][key] = value
        elif line:
            raise InvalidMeasurements(f"snapshot_{expected_label}_line_invalid")

    if set(services) != set(SERVICE_UNITS):
        raise InvalidMeasurements(f"snapshot_{expected_label}_service_set_invalid")
    service_state = []
    for unit in SERVICE_UNITS:
        state = services[unit]
        if set(state) != {"ActiveState", "MainPID", "NRestarts"}:
            raise InvalidMeasurements(f"snapshot_{expected_label}_{unit}_fields_invalid")
        if state["ActiveState"] != "active":
            raise InvalidMeasurements(f"snapshot_{expected_label}_{unit}_inactive")
        try:
            pid = int(state["MainPID"])
            restarts = int(state["NRestarts"])
        except ValueError as error:
            raise InvalidMeasurements(f"snapshot_{expected_label}_{unit}_integer_invalid") from error
        if pid <= 0 or restarts < 0:
            raise InvalidMeasurements(f"snapshot_{expected_label}_{unit}_state_invalid")
        service_state.append((unit, pid, restarts))

    if not isinstance(health, dict) or health.get("ok") is not True:
        raise InvalidMeasurements(f"snapshot_{expected_label}_health_not_ok")
    if require_int(
        health.get("transition_false_accepts"),
        f"snapshot_{expected_label}_false_accepts",
    ) != 0:
        raise InvalidMeasurements(f"snapshot_{expected_label}_false_accepts_nonzero")
    return observed_at, tuple(service_state)


def parse_connector_snapshot(
    path: Path, expected_label: str, expected_commit: str
) -> tuple[datetime, int, int]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise InvalidMeasurements(f"connector_{expected_label}_json_invalid") from error
    if not isinstance(value, dict):
        raise InvalidMeasurements(f"connector_{expected_label}_not_object")
    require_exact_keys(
        value,
        {
            "schema",
            "label",
            "observed_at",
            "protocol_commit",
            "protocol_epoch_root",
            "active_state",
            "main_pid",
            "nrestarts",
            "route_receipt_failures",
        },
        f"connector_{expected_label}",
    )
    if value["schema"] != "nando.s1c1-resource-v3.connector-snapshot.v1":
        raise InvalidMeasurements(f"connector_{expected_label}_schema_invalid")
    if value["label"] != expected_label:
        raise InvalidMeasurements(f"connector_{expected_label}_label_invalid")
    if value["protocol_commit"] != expected_commit:
        raise InvalidMeasurements(f"connector_{expected_label}_protocol_commit_mismatch")
    if value["protocol_epoch_root"] != PROTOCOL_EPOCH_ROOT:
        raise InvalidMeasurements(f"connector_{expected_label}_protocol_epoch_mismatch")
    if value["active_state"] != "active":
        raise InvalidMeasurements(f"connector_{expected_label}_inactive")
    if require_int(
        value["route_receipt_failures"],
        f"connector_{expected_label}_route_receipt_failures",
    ) != 0:
        raise InvalidMeasurements(f"connector_{expected_label}_receipt_failures")
    try:
        observed_at = datetime.fromisoformat(value["observed_at"].replace(",", ".", 1))
    except (AttributeError, ValueError) as error:
        raise InvalidMeasurements(f"connector_{expected_label}_timestamp_invalid") from error
    restarts = require_int(value["nrestarts"], f"connector_{expected_label}_restarts")
    if restarts < 0:
        raise InvalidMeasurements(f"connector_{expected_label}_restarts_invalid")
    return (
        observed_at,
        require_int(value["main_pid"], f"connector_{expected_label}_pid", positive=True),
        restarts,
    )


def absolute_pass(run: dict[str, Any], no_match_field: str) -> bool:
    return (
        run["matched_p99_ns"] <= MATCHED_BUDGET_NS
        and run[no_match_field] <= NO_MATCH_BUDGET_NS
        and run["hard_max_ns"] <= HARD_MAX_BUDGET_NS
    )


def validate_run(
    run: Any,
    *,
    label: str,
    ordinal: int,
    test: str,
    binary_sha256: str,
    no_match_field: str,
) -> dict[str, Any]:
    if not isinstance(run, dict):
        raise InvalidMeasurements(f"{label}_not_object")
    require_exact_keys(
        run,
        {
            "ordinal",
            "test",
            "binary_sha256",
            "samples",
            "matched_p99_ns",
            no_match_field,
            "hard_max_ns",
            "exit_code",
        },
        label,
    )
    if require_int(run["ordinal"], f"{label}_ordinal") != ordinal:
        raise InvalidMeasurements(f"{label}_ordinal_mismatch")
    if run["test"] != test:
        raise InvalidMeasurements(f"{label}_test_mismatch")
    if require_root(run["binary_sha256"], f"{label}_binary") != binary_sha256:
        raise InvalidMeasurements(f"{label}_binary_mismatch")
    if require_int(run["samples"], f"{label}_samples", positive=True) != 4_096:
        raise InvalidMeasurements(f"{label}_sample_count_mismatch")
    require_int(run["matched_p99_ns"], f"{label}_matched", positive=True)
    require_int(run[no_match_field], f"{label}_{no_match_field}", positive=True)
    require_int(run["hard_max_ns"], f"{label}_hard_max", positive=True)
    require_int(run["exit_code"], f"{label}_exit_code")
    passed = absolute_pass(run, no_match_field)
    if (run["exit_code"] == 0) != passed:
        raise InvalidMeasurements(f"{label}_exit_code_budget_mismatch")
    return run


def median(values: list[Any]) -> Any:
    if len(values) != 3:
        raise InvalidMeasurements("median_requires_three_values")
    return sorted(values)[1]


def fraction_json(value: Fraction) -> dict[str, int]:
    return {"numerator": value.numerator, "denominator": value.denominator}


def document_run(document: dict[str, Any], label: str) -> tuple[dict[str, Any], str, str]:
    if label.startswith("T"):
        ordinal = int(label[1:])
        return document["targeted_runs"][ordinal - 1], "no_goal_p99_ns", TARGETED_TEST
    ordinal = int(label[1])
    side = label[2]
    key = "baseline" if side == "B" else "candidate"
    return document["inherited_pairs"][ordinal - 1][key], "no_match_p99_ns", INHERITED_TEST


def validate_evidence(document: dict[str, Any], evidence_dir: Path) -> str:
    commit = document["protocol_commit"]
    expected_directory_name = f"s1c1-v3-{commit[:8]}"
    directory_name = document["evidence_directory_name"]
    if not isinstance(directory_name, str) or EVIDENCE_DIR_RE.fullmatch(directory_name) is None:
        raise InvalidMeasurements("evidence_directory_name_invalid")
    if directory_name != expected_directory_name or evidence_dir.name != directory_name:
        raise InvalidMeasurements("evidence_directory_binding_mismatch")

    manifest_root, _ = canonical_evidence_manifest(evidence_dir)
    if manifest_root != document["evidence_manifest_sha256"]:
        raise InvalidMeasurements("evidence_manifest_mismatch")

    environment = (evidence_dir / "environment.txt").read_text(encoding="utf-8")
    required_environment_lines = (
        f"protocol_commit={commit}",
        f"protocol_parent_commit={PROTOCOL_PARENT_COMMIT}",
        f"protocol_epoch_root={PROTOCOL_EPOCH_ROOT}",
        f"source_manifest_root={SOURCE_MANIFEST_ROOT}",
        f"{BASELINE_BINARY_SHA256}  /home/e/.cache/nando-wave-s1c1-baseline-target/release/deps/f7_generation_shadow_v3-257d2fa93e7c240e",
        f"{CANDIDATE_BINARY_SHA256}  /home/e/.cache/nando-wave-s1c1-target/release/deps/f7_generation_shadow_v3-257d2fa93e7c240e",
        f"{TARGETED_BINARY_SHA256}  /home/e/.cache/nando-wave-s1c1-target/release/deps/nando_response_actor-94c534b357a046f6",
    )
    for line in required_environment_lines:
        if line not in environment.splitlines():
            raise InvalidMeasurements("environment_binding_missing")
    boot_ids = re.findall(r"^boot_id=([0-9a-f-]+)$", environment, re.MULTILINE)
    if len(boot_ids) != 1:
        raise InvalidMeasurements("environment_boot_id_invalid")

    first_before: datetime | None = None
    previous_after: datetime | None = None
    frozen_services: tuple[tuple[str, int, int], ...] | None = None
    for label in RUN_LABELS:
        run, no_match_field, expected_test = document_run(document, label)
        log = (evidence_dir / f"{label}.log").read_text(encoding="utf-8")
        metric_re = TARGET_METRIC_RE if label.startswith("T") else INHERITED_METRIC_RE
        metrics = metric_re.findall(log)
        if len(metrics) != 1:
            raise InvalidMeasurements(f"evidence_{label}_metrics_count_invalid")
        matched, no_match, hard_max, samples = (int(value) for value in metrics[0])
        if (
            matched != run["matched_p99_ns"]
            or no_match != run[no_match_field]
            or hard_max != run["hard_max_ns"]
            or samples != run["samples"]
        ):
            raise InvalidMeasurements(f"evidence_{label}_metrics_mismatch")
        if f"test {expected_test} ..." not in log:
            raise InvalidMeasurements(f"evidence_{label}_test_name_mismatch")
        try:
            raw_exit = int((evidence_dir / f"{label}.exit").read_text(encoding="ascii").strip())
        except ValueError as error:
            raise InvalidMeasurements(f"evidence_{label}_exit_invalid") from error
        if raw_exit != run["exit_code"]:
            raise InvalidMeasurements(f"evidence_{label}_exit_mismatch")

        before, before_services = parse_snapshot(
            evidence_dir / f"{label}.before.snapshot", f"{label}.before"
        )
        after, after_services = parse_snapshot(
            evidence_dir / f"{label}.after.snapshot", f"{label}.after"
        )
        if before >= after:
            raise InvalidMeasurements(f"evidence_{label}_chronology_invalid")
        if first_before is None:
            first_before = before
        if previous_after is not None and (before - previous_after).total_seconds() < 1.9:
            raise InvalidMeasurements(f"evidence_{label}_fixed_gap_missing")
        previous_after = after
        for service_state in (before_services, after_services):
            if frozen_services is None:
                frozen_services = service_state
            elif service_state != frozen_services:
                raise InvalidMeasurements(f"evidence_{label}_service_drift")

    final_at, final_services = parse_snapshot(evidence_dir / "final.snapshot", "final")
    if previous_after is None or (final_at - previous_after).total_seconds() < 1.9:
        raise InvalidMeasurements("evidence_final_gap_missing")
    if final_services != frozen_services:
        raise InvalidMeasurements("evidence_final_service_drift")

    connector_before = parse_connector_snapshot(
        evidence_dir / "local_connector.before", "before", commit
    )
    connector_after = parse_connector_snapshot(
        evidence_dir / "local_connector.after", "after", commit
    )
    if first_before is None or connector_before[0] > first_before or connector_after[0] < final_at:
        raise InvalidMeasurements("connector_chronology_invalid")
    if connector_before[1:] != connector_after[1:]:
        raise InvalidMeasurements("connector_state_drift")
    return manifest_root


def verify_measurement_math(document: Any) -> tuple[dict[str, Any], int]:
    if not isinstance(document, dict):
        raise InvalidMeasurements("document_not_object")
    require_exact_keys(
        document,
        {
            "schema",
            "protocol_epoch_root",
            "protocol_parent_commit",
            "protocol_commit",
            "source_manifest_root",
            "evidence_directory_name",
            "evidence_manifest_sha256",
            "targeted_runs",
            "inherited_pairs",
            "safety",
        },
        "document",
    )
    if document["schema"] != SCHEMA:
        raise InvalidMeasurements("schema_mismatch")
    if require_root(document["protocol_epoch_root"], "protocol_epoch_root") != PROTOCOL_EPOCH_ROOT:
        raise InvalidMeasurements("protocol_epoch_root_mismatch")
    if require_commit(document["protocol_parent_commit"], "protocol_parent_commit") != PROTOCOL_PARENT_COMMIT:
        raise InvalidMeasurements("protocol_parent_commit_mismatch")
    protocol_commit = require_commit(document["protocol_commit"], "protocol_commit")
    if protocol_commit == PROTOCOL_PARENT_COMMIT:
        raise InvalidMeasurements("protocol_commit_not_post_parent")
    if (
        require_root(document["source_manifest_root"], "source_manifest_root")
        != SOURCE_MANIFEST_ROOT
    ):
        raise InvalidMeasurements("source_manifest_root_mismatch")
    require_root(document["evidence_manifest_sha256"], "evidence_manifest_sha256")

    targeted = document["targeted_runs"]
    if not isinstance(targeted, list) or len(targeted) != 3:
        raise InvalidMeasurements("targeted_run_count_mismatch")
    targeted_results = [
        validate_run(
            run,
            label=f"targeted_{index}",
            ordinal=index,
            test=TARGETED_TEST,
            binary_sha256=TARGETED_BINARY_SHA256,
            no_match_field="no_goal_p99_ns",
        )
        for index, run in enumerate(targeted, start=1)
    ]
    targeted_passes = sum(
        absolute_pass(run, "no_goal_p99_ns") for run in targeted_results
    )

    pairs = document["inherited_pairs"]
    if not isinstance(pairs, list) or len(pairs) != 3:
        raise InvalidMeasurements("inherited_pair_count_mismatch")

    pair_factors: list[Fraction] = []
    baseline_runs: list[dict[str, Any]] = []
    candidate_runs: list[dict[str, Any]] = []
    for index, pair in enumerate(pairs, start=1):
        label = f"pair_{index}"
        if not isinstance(pair, dict):
            raise InvalidMeasurements(f"{label}_not_object")
        require_exact_keys(pair, {"ordinal", "order", "baseline", "candidate"}, label)
        if require_int(pair["ordinal"], f"{label}_ordinal") != index:
            raise InvalidMeasurements(f"{label}_ordinal_mismatch")
        if pair["order"] != PAIR_ORDERS[index - 1]:
            raise InvalidMeasurements(f"{label}_order_mismatch")
        baseline = validate_run(
            pair["baseline"],
            label=f"{label}_baseline",
            ordinal=index,
            test=INHERITED_TEST,
            binary_sha256=BASELINE_BINARY_SHA256,
            no_match_field="no_match_p99_ns",
        )
        candidate = validate_run(
            pair["candidate"],
            label=f"{label}_candidate",
            ordinal=index,
            test=INHERITED_TEST,
            binary_sha256=CANDIDATE_BINARY_SHA256,
            no_match_field="no_match_p99_ns",
        )
        baseline_runs.append(baseline)
        candidate_runs.append(candidate)
        factors = [
            Fraction(candidate[field], baseline[field])
            for field in ("matched_p99_ns", "no_match_p99_ns", "hard_max_ns")
        ]
        pair_factors.append(max(factors))

    baseline_absolute_passes = sum(
        absolute_pass(run, "no_match_p99_ns") for run in baseline_runs
    )
    candidate_absolute_passes = sum(
        absolute_pass(run, "no_match_p99_ns") for run in candidate_runs
    )
    median_factor = median(pair_factors)
    median_checks = {
        field: Fraction(
            median([run[field] for run in candidate_runs]),
            median([run[field] for run in baseline_runs]),
        )
        <= MAX_REGRESSION
        for field in ("matched_p99_ns", "no_match_p99_ns", "hard_max_ns")
    }
    inherited_pass = (
        candidate_absolute_passes >= baseline_absolute_passes
        and median_factor <= MAX_REGRESSION
        and all(factor <= MAX_SINGLE_PAIR_REGRESSION for factor in pair_factors)
        and all(median_checks.values())
        and not (baseline_absolute_passes == 3 and candidate_absolute_passes != 3)
    )

    safety = document["safety"]
    if not isinstance(safety, dict):
        raise InvalidMeasurements("safety_not_object")
    require_exact_keys(
        safety,
        {
            "production_service_survival",
            "connector_survival",
            "false_accepts",
            "runtime_parity_failures",
            "serving_parity_equal",
            "structural_verdict",
        },
        "safety",
    )
    for field in (
        "production_service_survival",
        "connector_survival",
        "serving_parity_equal",
    ):
        if not isinstance(safety[field], bool):
            raise InvalidMeasurements(f"safety_{field}_not_boolean")
    require_int(safety["false_accepts"], "safety_false_accepts")
    require_int(safety["runtime_parity_failures"], "safety_runtime_parity_failures")
    if safety["structural_verdict"] not in {"PASS", "WATCH", "VETO"}:
        raise InvalidMeasurements("safety_structural_verdict_invalid")
    safety_pass = (
        safety["production_service_survival"]
        and safety["connector_survival"]
        and safety["false_accepts"] == 0
        and safety["runtime_parity_failures"] == 0
        and safety["serving_parity_equal"]
        and safety["structural_verdict"] == "PASS"
    )

    targeted_pass = targeted_passes == 3
    verdict = "PASS" if targeted_pass and inherited_pass and safety_pass else "VETO"
    result = {
        "schema": VERDICT_SCHEMA,
        "protocol_epoch_root": document["protocol_epoch_root"],
        "protocol_parent_commit": document["protocol_parent_commit"],
        "protocol_commit": document["protocol_commit"],
        "source_manifest_root": document["source_manifest_root"],
        "evidence_manifest_sha256": document["evidence_manifest_sha256"],
        "targeted_absolute_passes": targeted_passes,
        "baseline_inherited_absolute_passes": baseline_absolute_passes,
        "candidate_inherited_absolute_passes": candidate_absolute_passes,
        "pair_regression_factors": [fraction_json(value) for value in pair_factors],
        "median_pair_regression_factor": fraction_json(median_factor),
        "median_metric_checks": median_checks,
        "targeted_gate": "PASS" if targeted_pass else "VETO",
        "inherited_regression_gate": "PASS" if inherited_pass else "VETO",
        "safety_gate": "PASS" if safety_pass else "VETO",
        "inherited_absolute_environment": (
            "PASS"
            if baseline_absolute_passes == 3 and candidate_absolute_passes == 3
            else "FAIL"
        ),
        "verdict": verdict,
        "authority_ready": False,
        "deployment_allowed": False,
    }
    return result, 0 if verdict == "PASS" else 1


def verify_measurements(document: Any, evidence_dir: Path) -> tuple[dict[str, Any], int]:
    result, code = verify_measurement_math(document)
    manifest_root = validate_evidence(document, evidence_dir)
    result["evidence_manifest_sha256"] = manifest_root
    result["evidence_gate"] = "PASS"
    return result, code


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(f"usage: {Path(argv[0]).name} MEASUREMENTS.json EVIDENCE_DIR", file=sys.stderr)
        return 2
    try:
        document = json.loads(Path(argv[1]).read_text(encoding="utf-8"))
        result, code = verify_measurements(document, Path(argv[2]))
    except (OSError, json.JSONDecodeError, InvalidMeasurements) as error:
        print(
            json.dumps(
                {
                    "schema": VERDICT_SCHEMA,
                    "verdict": "INVALID_ENVIRONMENT",
                    "reason": str(error),
                    "authority_ready": False,
                    "deployment_allowed": False,
                },
                sort_keys=True,
            )
        )
        return 2
    print(json.dumps(result, sort_keys=True))
    return code


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
