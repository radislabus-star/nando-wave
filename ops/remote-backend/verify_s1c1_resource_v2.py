#!/usr/bin/env python3
"""Fail-closed verifier for the frozen S1C-1 resource protocol V2."""

from __future__ import annotations

import json
import re
import sys
from fractions import Fraction
from pathlib import Path
from typing import Any


SCHEMA = "nando.s1c1-resource-v2.measurements.v1"
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


def require_int(value: Any, label: str, *, positive: bool = False) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise InvalidMeasurements(f"{label}_not_integer")
    if positive and value <= 0:
        raise InvalidMeasurements(f"{label}_not_positive")
    return value


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


def verify_measurements(document: Any) -> tuple[dict[str, Any], int]:
    if not isinstance(document, dict):
        raise InvalidMeasurements("document_not_object")
    require_exact_keys(
        document,
        {
            "schema",
            "protocol_commit",
            "source_manifest_root",
            "targeted_runs",
            "inherited_pairs",
            "safety",
        },
        "document",
    )
    if document["schema"] != SCHEMA:
        raise InvalidMeasurements("schema_mismatch")
    require_root(document["protocol_commit"], "protocol_commit")
    if (
        require_root(document["source_manifest_root"], "source_manifest_root")
        != SOURCE_MANIFEST_ROOT
    ):
        raise InvalidMeasurements("source_manifest_root_mismatch")

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
        "schema": "nando.s1c1-resource-v2.verdict.v1",
        "protocol_commit": document["protocol_commit"],
        "source_manifest_root": document["source_manifest_root"],
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


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"usage: {Path(argv[0]).name} MEASUREMENTS.json", file=sys.stderr)
        return 2
    try:
        document = json.loads(Path(argv[1]).read_text(encoding="utf-8"))
        result, code = verify_measurements(document)
    except (OSError, json.JSONDecodeError, InvalidMeasurements) as error:
        print(
            json.dumps(
                {
                    "schema": "nando.s1c1-resource-v2.verdict.v1",
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
