#!/usr/bin/env python3
"""Pure, no-side-effect schema preflight for the S1C-3C successor."""

from __future__ import annotations

import hashlib
import json
import sys
from dataclasses import dataclass
from typing import Any, Callable

import s1c3b_remote_transaction_v1 as mechanism


SCHEMA = "nando.s1c3c-schema-preflight.v1"
PAPER_COMMIT = "2a1505055ce98b3f6bed5cb440a0faa345fb78cb"


class SchemaVeto(ValueError):
    pass


@dataclass(frozen=True)
class MetricSpec:
    name: str
    pattern: Any
    fields: tuple[str, ...]
    kinds: tuple[str, ...]
    fixture: str


SPECS = (
    MetricSpec(
        "hot",
        mechanism.HOT_RE,
        ("p99_ns", "no_goal_p99_ns", "hard_max_ns", "samples"),
        ("int", "int", "int", "int"),
        "S1C_HOT_LATENCY matched_p99_ns=1000 no_goal_p99_ns=100 "
        "hard_max_ns=2000 samples=4096",
    ),
    MetricSpec(
        "single_sync",
        mechanism.SYNC_RE,
        ("p99_ns", "hard_max_ns", "samples", "segments"),
        ("int", "int", "int", "int"),
        "S1C_SYNC_LATENCY p99_ns=1000 hard_max_ns=2000 records=1024 segments=2",
    ),
    MetricSpec(
        "three_sync",
        mechanism.STAGE_SYNC_RE,
        (
            "precommit_p99_ns",
            "precommit_hard_max_ns",
            "settlement_p99_ns",
            "settlement_hard_max_ns",
            "episode_p99_ns",
            "episode_hard_max_ns",
            "samples",
        ),
        ("int", "int", "int", "int", "int", "int", "int"),
        "S1C3_STAGE_SYNC_LATENCY precommit_p99_ns=1000 "
        "precommit_hard_max_ns=2000 settlement_p99_ns=1000 "
        "settlement_hard_max_ns=2000 episode_p99_ns=1000 "
        "episode_hard_max_ns=2000 records=256",
    ),
    MetricSpec(
        "idle",
        mechanism.IDLE_RE,
        mechanism.IDLE_METRIC_FIELDS,
        ("int", "int", "float"),
        "S1C_IDLE_CPU elapsed_ticks=0 ticks_per_second=100 "
        "percent_of_one_core=0.000000",
    ),
)


def canonical_bytes(value: Any, omit: str | None = None) -> bytes:
    if omit is not None:
        value = dict(value)
        value.pop(omit, None)
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def require(condition: bool, error: str) -> None:
    if not condition:
        raise SchemaVeto(error)


def convert(kind: str, raw: str) -> int | float:
    if kind == "int":
        return int(raw)
    if kind == "float":
        return float(raw)
    raise SchemaVeto(f"unknown_metric_kind:{kind}")


def parse_metric(spec: MetricSpec, line: str) -> dict[str, int | float]:
    match = spec.pattern.fullmatch(line)
    require(match is not None, f"{spec.name}:fixture_fullmatch")
    require(spec.pattern.groups == len(spec.fields), f"{spec.name}:regex_field_count")
    require(len(spec.fields) == len(spec.kinds), f"{spec.name}:field_kind_count")
    values = tuple(
        convert(kind, raw) for kind, raw in zip(spec.kinds, match.groups(), strict=True)
    )
    parsed = dict(zip(spec.fields, values, strict=True))
    require(tuple(parsed) == spec.fields, f"{spec.name}:parsed_key_order")
    return parsed


def evaluate_hot(metric: dict[str, int | float]) -> dict[str, Any]:
    failures = []
    instruments = []
    if metric["samples"] != 4096:
        instruments.append("denominator")
    if metric["p99_ns"] > 1_000_000:
        failures.append("matched_p99")
    if metric["no_goal_p99_ns"] > 250_000:
        failures.append("no_goal_p99")
    if metric["hard_max_ns"] > 2_000_000:
        failures.append("hard_max")
    return evaluation(metric, failures, instruments, ())


def evaluate_single_sync(metric: dict[str, int | float]) -> dict[str, Any]:
    failures = []
    instruments = []
    if metric["samples"] != 1024:
        instruments.append("denominator")
    if metric["p99_ns"] > 5_000_000:
        failures.append("p99")
    if metric["hard_max_ns"] > 20_000_000:
        failures.append("hard_max")
    return evaluation(metric, failures, instruments, ("segments",))


def evaluate_three_sync(metric: dict[str, int | float]) -> dict[str, Any]:
    failures = []
    instruments = []
    if metric["samples"] != 256:
        instruments.append("denominator")
    for field in ("precommit_p99_ns", "settlement_p99_ns"):
        if metric[field] > 5_000_000:
            failures.append(field)
    for field in (
        "precommit_hard_max_ns",
        "settlement_hard_max_ns",
        "episode_hard_max_ns",
    ):
        if metric[field] > 20_000_000:
            failures.append(field)
    return evaluation(metric, failures, instruments, ("episode_p99_ns",))


def evaluate_idle(metric: dict[str, int | float]) -> dict[str, Any]:
    failures = []
    if metric["percent_of_one_core"] > 0.25:
        failures.append("percent_of_one_core")
    return evaluation(metric, failures, (), ("elapsed_ticks", "ticks_per_second"))


def evaluation(
    metric: dict[str, int | float],
    failures: list[str] | tuple[str, ...],
    instruments: list[str] | tuple[str, ...],
    diagnostics: tuple[str, ...],
) -> dict[str, Any]:
    consumed = tuple(metric)
    require(set(diagnostics).issubset(metric), "diagnostic_field_missing")
    return {
        "consumed_fields": list(consumed),
        "diagnostic_fields": list(diagnostics),
        "resource_failures": sorted(failures),
        "instrument_failures": sorted(instruments),
        "retained_metric_root_sha256": digest(canonical_bytes(metric)),
    }


EVALUATORS: dict[str, Callable[[dict[str, int | float]], dict[str, Any]]] = {
    "hot": evaluate_hot,
    "single_sync": evaluate_single_sync,
    "three_sync": evaluate_three_sync,
    "idle": evaluate_idle,
}


def mutated_value(value: int | float) -> int | float:
    if isinstance(value, float):
        return value + 1.0
    return value + 1


def validate_spec(spec: MetricSpec) -> dict[str, Any]:
    parsed = parse_metric(spec, spec.fixture)
    evaluator = EVALUATORS[spec.name]
    result = evaluator(parsed)
    require(
        result["consumed_fields"] == list(spec.fields),
        f"{spec.name}:evaluator_field_set",
    )
    mutation_roots = {}
    for field in spec.fields:
        changed = dict(parsed)
        changed[field] = mutated_value(changed[field])
        changed_result = evaluator(changed)
        require(
            changed_result["retained_metric_root_sha256"]
            != result["retained_metric_root_sha256"],
            f"{spec.name}:field_not_retained:{field}",
        )
        require(
            changed_result["consumed_fields"] == list(spec.fields),
            f"{spec.name}:mutation_field_set:{field}",
        )
        mutation_roots[field] = changed_result["retained_metric_root_sha256"]
    return {
        "name": spec.name,
        "regex_sha256": digest(spec.pattern.pattern.encode()),
        "capture_groups": spec.pattern.groups,
        "fields": list(spec.fields),
        "kinds": list(spec.kinds),
        "fixture_sha256": digest(spec.fixture.encode()),
        "parsed_root_sha256": result["retained_metric_root_sha256"],
        "mutation_roots": mutation_roots,
        "valid": True,
    }


def run_preflight() -> dict[str, Any]:
    require(
        mechanism.IDLE_METRIC_FIELDS
        == ("elapsed_ticks", "ticks_per_second", "percent_of_one_core"),
        "idle_declared_field_tuple_drift",
    )
    rows = [validate_spec(spec) for spec in SPECS]
    require([row["name"] for row in rows] == list(EVALUATORS), "metric_family_order")
    receipt = {
        "schema": SCHEMA,
        "valid": True,
        "authority": False,
        "paper_commit": PAPER_COMMIT,
        "side_effects": False,
        "remote_attempt_created": False,
        "metric_families": rows,
    }
    receipt["schema_preflight_root_sha256"] = digest(canonical_bytes(receipt))
    return receipt


def main() -> int:
    try:
        print(json.dumps(run_preflight(), sort_keys=True))
        return 0
    except (SchemaVeto, KeyError, TypeError, ValueError) as error:
        print(
            json.dumps(
                {"schema": SCHEMA, "valid": False, "authority": False, "error": str(error)},
                sort_keys=True,
            )
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())
