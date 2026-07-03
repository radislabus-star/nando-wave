#!/usr/bin/env python3
"""One-pass operator induction probe for v4 operator battery.

This is a diagnostic stand, not a proof gate. It checks whether train examples
can be compressed into reusable operator programs and applied to heldout rows
without epoch-based repair.
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent
DEFAULT_CORPUS = ROOT / "accepted_operator_tasks_v4.jsonl"
DEFAULT_REPORT = ROOT / "diagnostics" / "operator_grokking_probe_report.json"


MARKER_RE = re.compile(r"marker:\s*([^;]+)")


def load_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, 1):
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            row["_line_no"] = line_no
            rows.append(row)
    return rows


def split_of(row: dict[str, Any]) -> str:
    source_group = str(row["source_group"])
    if "_train_" in source_group:
        return "train"
    if "_heldout_" in source_group:
        return "heldout"
    return "unknown"


def action_marker(row: dict[str, Any]) -> str | None:
    match = MARKER_RE.search(str(row["rule_action_example"]))
    if not match:
        return None
    return match.group(1).strip()


def normalized_action(row: dict[str, Any]) -> str:
    action = str(row["rule_action_example"])
    marker = action_marker(row)
    if marker:
        action = action.replace(marker, "<MARKER>")
    action = re.sub(r"\s+", " ", action).strip()
    return action


def operator_key(row: dict[str, Any]) -> str:
    condition = row.get("condition_flag")
    condition_key = f"condition={condition}" if condition is not None else "condition=<none>"
    return "|".join(
        [
            f"class={row['operator_class']}",
            f"length={row['sequence_length']}",
            condition_key,
            f"action={normalized_action(row)}",
        ]
    )


def induce_program(row: dict[str, Any]) -> tuple[tuple[str, int | str], ...] | None:
    source_tokens = list(row["source_tokens"])
    correct_tokens = list(row["correct_tokens"])
    marker = action_marker(row)
    positions: dict[str, list[int]] = defaultdict(list)
    for index, token in enumerate(source_tokens):
        positions[str(token)].append(index)

    program: list[tuple[str, int | str]] = []
    for token in correct_tokens:
        token = str(token)
        if token in positions and len(positions[token]) == 1:
            program.append(("src", positions[token][0]))
        elif marker is not None and token == marker:
            program.append(("marker", "<MARKER>"))
        else:
            return None
    return tuple(program)


def apply_program(
    program: tuple[tuple[str, int | str], ...], row: dict[str, Any]
) -> list[str] | None:
    source_tokens = list(row["source_tokens"])
    marker = action_marker(row)
    output: list[str] = []
    for kind, value in program:
        if kind == "src":
            index = int(value)
            if index < 0 or index >= len(source_tokens):
                return None
            output.append(str(source_tokens[index]))
        elif kind == "marker":
            if marker is None:
                return None
            output.append(marker)
        else:
            return None
    return output


def milli(numerator: int, denominator: int) -> int:
    if denominator == 0:
        return 0
    return round(1000 * numerator / denominator)


def make_report(rows: list[dict[str, Any]]) -> dict[str, Any]:
    train = [row for row in rows if split_of(row) == "train"]
    heldout = [row for row in rows if split_of(row) == "heldout"]
    unknown = [row for row in rows if split_of(row) == "unknown"]

    grouped_programs: dict[str, Counter[tuple[tuple[str, int | str], ...]]] = defaultdict(Counter)
    skipped_train = 0
    for row in train:
        program = induce_program(row)
        if program is None:
            skipped_train += 1
            continue
        grouped_programs[operator_key(row)][program] += 1

    compiled: dict[str, tuple[tuple[str, int | str], ...]] = {}
    conflicts: dict[str, dict[str, int]] = {}
    for key, counter in grouped_programs.items():
        program, _ = counter.most_common(1)[0]
        compiled[key] = program
        if len(counter) > 1:
            conflicts[key] = {repr(item): count for item, count in counter.most_common()}

    by_class: dict[str, Counter[str]] = defaultdict(Counter)
    by_surface: dict[str, Counter[str]] = defaultdict(Counter)
    by_noise: dict[str, Counter[str]] = defaultdict(Counter)
    by_length: dict[str, Counter[str]] = defaultdict(Counter)
    failure_examples: list[dict[str, Any]] = []

    known = 0
    predicted = 0
    correct = 0
    wrong_match = 0
    same_bag_correct = 0
    skipped_heldout = 0

    for row in heldout:
        key = operator_key(row)
        cls = str(row["operator_class"])
        surface = str(row["surface_family"])
        noise = str(row["noise_type"])
        length = str(row["sequence_length"])
        by_class[cls]["rows"] += 1
        by_surface[surface]["rows"] += 1
        by_noise[noise]["rows"] += 1
        by_length[length]["rows"] += 1

        program = compiled.get(key)
        if program is None:
            skipped_heldout += 1
            outcome = "missing_program"
        else:
            known += 1
            output = apply_program(program, row)
            if output is None:
                outcome = "apply_failed"
            else:
                predicted += 1
                if output == list(row["correct_tokens"]):
                    correct += 1
                    outcome = "correct"
                elif output == list(row["wrong_tokens"]):
                    wrong_match += 1
                    outcome = "wrong_match"
                else:
                    outcome = "other_wrong"

                if Counter(output) == Counter(row["correct_tokens"]):
                    same_bag_correct += 1

        by_class[cls][outcome] += 1
        by_surface[surface][outcome] += 1
        by_noise[noise][outcome] += 1
        by_length[length][outcome] += 1

        if outcome != "correct" and len(failure_examples) < 20:
            failure_examples.append(
                {
                    "task_id": row["task_id"],
                    "operator_class": row["operator_class"],
                    "proof_rule_id": row["proof_rule_id"],
                    "surface_family": row["surface_family"],
                    "noise_type": row["noise_type"],
                    "sequence_length": row["sequence_length"],
                    "outcome": outcome,
                }
            )

    return {
        "schema_version": "operator_grokking_probe_v1",
        "diagnostic_only": True,
        "method": "one_pass_program_induction_from_train_transitions",
        "operator_key_uses": [
            "operator_class",
            "sequence_length",
            "condition_flag",
            "normalized_rule_action_example",
        ],
        "forbidden_fields_not_used_for_key_or_program": [
            "proof_rule_id",
            "source_group",
            "task_id",
            "state_after_correct",
            "state_after_wrong",
            "why_target_is_correct",
            "why_negative_is_wrong",
        ],
        "rows": len(rows),
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "unknown_split_rows": len(unknown),
        "compiled_operator_programs": len(compiled),
        "operator_program_conflicts": len(conflicts),
        "skipped_train_rows": skipped_train,
        "known_heldout_program_rows": known,
        "predicted_heldout_rows": predicted,
        "skipped_heldout_rows": skipped_heldout,
        "heldout_correct_rows": correct,
        "heldout_wrong_match_rows": wrong_match,
        "heldout_same_bag_as_correct_rows": same_bag_correct,
        "heldout_accuracy_milli": milli(correct, len(heldout)),
        "known_program_accuracy_milli": milli(correct, known),
        "same_bag_output_milli": milli(same_bag_correct, predicted),
        "compression": {
            "train_rows_per_compiled_program_milli": milli(len(train), len(compiled)),
            "heldout_rows_per_compiled_program_milli": milli(len(heldout), len(compiled)),
        },
        "by_class": {key: dict(value) for key, value in sorted(by_class.items())},
        "by_surface": {key: dict(value) for key, value in sorted(by_surface.items())},
        "by_noise": {key: dict(value) for key, value in sorted(by_noise.items())},
        "by_length": {key: dict(value) for key, value in sorted(by_length.items())},
        "conflict_examples": dict(list(conflicts.items())[:10]),
        "failure_examples": failure_examples,
        "claim_boundary": [
            "This proves only that compact operator programs can be induced from train transitions for this corpus.",
            "It is not a Wave runtime proof and not a semantic grokking claim.",
            "It does not parse proof_rule_id and does not use target rows at heldout time.",
            "The next step is to compile these programs into Nando Wave weights/energy/cleanup and compare against epoch repair.",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    args = parser.parse_args()

    rows = load_rows(args.corpus)
    report = make_report(rows)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    print("operator_grokking_probe:")
    print(f"  corpus: {args.corpus}")
    print(f"  report: {args.report}")
    print(f"  rows: {report['rows']}")
    print(f"  train_rows: {report['train_rows']}")
    print(f"  heldout_rows: {report['heldout_rows']}")
    print(f"  compiled_operator_programs: {report['compiled_operator_programs']}")
    print(f"  operator_program_conflicts: {report['operator_program_conflicts']}")
    print(f"  heldout_accuracy_milli: {report['heldout_accuracy_milli']}")
    print(f"  heldout_wrong_match_rows: {report['heldout_wrong_match_rows']}")
    print(f"  skipped_train_rows: {report['skipped_train_rows']}")
    print(f"  skipped_heldout_rows: {report['skipped_heldout_rows']}")


if __name__ == "__main__":
    main()
