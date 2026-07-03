#!/usr/bin/env python3
"""Phase center-mass operator probe for v4.

This diagnostic tests the user's intended "grok" shape:

    transition relation waves -> circular center of mass -> heldout score

It does not train epochs and does not extract an explicit out<-src program.
For every candidate sequence, it builds a transition wave from relation atoms
such as "out slot i received source slot j". Train correct transitions form a
phase center; heldout correct/wrong candidates are scored by coherence to that
center.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from collections import Counter, defaultdict
from functools import lru_cache
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parent
DEFAULT_CORPUS = ROOT / "accepted_operator_tasks_v4.jsonl"
DEFAULT_REPORT = ROOT / "diagnostics" / "phase_center_operator_probe_report.json"
DEFAULT_MD = ROOT / "diagnostics" / "PHASE_CENTER_OPERATOR_PROBE.md"

MARKER_RE = re.compile(r"marker:\s*([^;]+)")


ComplexVector = tuple[complex, ...]


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
    group = str(row["source_group"])
    if "_train_" in group:
        return "train"
    if "_heldout_" in group:
        return "heldout"
    return "unknown"


def action_marker(row: dict[str, Any]) -> str | None:
    match = MARKER_RE.search(str(row["rule_action_example"]))
    if match is None:
        return None
    return match.group(1).strip()


def normalized_action(row: dict[str, Any]) -> str:
    action = str(row["rule_action_example"])
    marker = action_marker(row)
    if marker:
        action = action.replace(marker, "<MARKER>")
    return re.sub(r"\s+", " ", action).strip()


def operator_key(row: dict[str, Any], key_mode: str) -> str:
    condition = row.get("condition_flag")
    condition_key = f"condition={condition}" if condition is not None else "condition=<none>"
    base = [
        f"class={row['operator_class']}",
        f"length={row['sequence_length']}",
        condition_key,
    ]
    if key_mode == "action":
        base.append(f"action={normalized_action(row)}")
    elif key_mode == "class_length":
        pass
    else:
        raise ValueError(f"unknown key mode: {key_mode}")
    return "|".join(base)


@lru_cache(maxsize=262_144)
def hash_phase(atom: str, cell: int) -> complex:
    digest = hashlib.blake2b(
        f"{cell}\0{atom}".encode("utf-8"),
        digest_size=8,
        person=b"nwphase",
    ).digest()
    raw = int.from_bytes(digest, "little")
    angle = (raw / float(1 << 64)) * math.tau
    return complex(math.cos(angle), math.sin(angle))


def circular_unit(value: complex) -> complex:
    magnitude = abs(value)
    if magnitude == 0:
        return 0j
    return value / magnitude


def vector_from_atoms(atoms: Iterable[str], cells: int) -> ComplexVector:
    sums = [0j] * cells
    count = 0
    for atom in atoms:
        count += 1
        for cell in range(cells):
            sums[cell] += hash_phase(atom, cell)
    if count == 0:
        return tuple(0j for _ in range(cells))
    return tuple(circular_unit(value) for value in sums)


def transition_atoms(row: dict[str, Any], candidate_tokens: list[str]) -> list[str] | None:
    source_tokens = [str(token) for token in row["source_tokens"]]
    marker = action_marker(row)
    positions: dict[str, list[int]] = defaultdict(list)
    for index, token in enumerate(source_tokens):
        positions[token].append(index)

    atoms = [
        f"class:{row['operator_class']}",
        f"src_len:{len(source_tokens)}",
        f"out_len:{len(candidate_tokens)}",
    ]

    for out_slot, raw_token in enumerate(candidate_tokens):
        token = str(raw_token)
        if token in positions and len(positions[token]) == 1:
            src_slot = positions[token][0]
            atoms.append(f"rel:o{out_slot}:s{src_slot}")
            atoms.append(f"out:o{out_slot}")
            atoms.append(f"src:s{src_slot}")
            distance = out_slot - src_slot
            atoms.append(f"delta:{distance}")
        elif marker is not None and token == marker:
            atoms.append(f"rel:o{out_slot}:marker")
            atoms.append(f"out:o{out_slot}")
            atoms.append("src:marker")
        else:
            return None
    return atoms


def transition_vector(row: dict[str, Any], candidate_tokens: list[str], cells: int) -> ComplexVector | None:
    atoms = transition_atoms(row, candidate_tokens)
    if atoms is None:
        return None
    return vector_from_atoms(atoms, cells)


def add_vec(left: list[complex], right: ComplexVector, sign: float = 1.0) -> None:
    for index, value in enumerate(right):
        left[index] += sign * value


def center_from_sum(values: list[complex]) -> ComplexVector:
    return tuple(circular_unit(value) for value in values)


def coherence(vector: ComplexVector, center: ComplexVector) -> float:
    if not vector or not center:
        return 0.0
    return sum((v * c.conjugate()).real for v, c in zip(vector, center)) / float(len(vector))


def milli(numerator: int, denominator: int) -> int:
    if denominator == 0:
        return 0
    return round(1000 * numerator / denominator)


def eval_mode(rows: list[dict[str, Any]], cells: int, key_mode: str) -> dict[str, Any]:
    train = [row for row in rows if split_of(row) == "train"]
    heldout = [row for row in rows if split_of(row) == "heldout"]

    positive_sums: dict[str, list[complex]] = defaultdict(lambda: [0j] * cells)
    negative_sums: dict[str, list[complex]] = defaultdict(lambda: [0j] * cells)
    counts: Counter[str] = Counter()
    skipped_train = 0

    for row in train:
        key = operator_key(row, key_mode)
        correct_vec = transition_vector(row, list(row["correct_tokens"]), cells)
        wrong_vec = transition_vector(row, list(row["wrong_tokens"]), cells)
        if correct_vec is None or wrong_vec is None:
            skipped_train += 1
            continue
        add_vec(positive_sums[key], correct_vec, 1.0)
        add_vec(negative_sums[key], wrong_vec, 1.0)
        counts[key] += 1

    positive_centers = {key: center_from_sum(value) for key, value in positive_sums.items()}
    negative_centers = {key: center_from_sum(value) for key, value in negative_sums.items()}

    by_class: dict[str, Counter[str]] = defaultdict(Counter)
    by_surface: dict[str, Counter[str]] = defaultdict(Counter)
    by_noise: dict[str, Counter[str]] = defaultdict(Counter)
    margins: list[float] = []
    center_gaps: list[float] = []
    correct = 0
    missing = 0
    skipped_eval = 0
    wrong_wins = 0
    failures: list[dict[str, Any]] = []

    for row in heldout:
        key = operator_key(row, key_mode)
        cls = str(row["operator_class"])
        surface = str(row["surface_family"])
        noise = str(row["noise_type"])
        by_class[cls]["rows"] += 1
        by_surface[surface]["rows"] += 1
        by_noise[noise]["rows"] += 1

        pos_center = positive_centers.get(key)
        neg_center = negative_centers.get(key)
        if pos_center is None or neg_center is None:
            missing += 1
            outcome = "missing_center"
        else:
            correct_vec = transition_vector(row, list(row["correct_tokens"]), cells)
            wrong_vec = transition_vector(row, list(row["wrong_tokens"]), cells)
            if correct_vec is None or wrong_vec is None:
                skipped_eval += 1
                outcome = "bad_candidate_vector"
            else:
                correct_pos = coherence(correct_vec, pos_center)
                wrong_pos = coherence(wrong_vec, pos_center)
                correct_neg = coherence(correct_vec, neg_center)
                wrong_neg = coherence(wrong_vec, neg_center)
                correct_score = correct_pos - correct_neg
                wrong_score = wrong_pos - wrong_neg
                margin = correct_score - wrong_score
                center_gap = correct_pos - wrong_pos
                margins.append(margin)
                center_gaps.append(center_gap)
                if margin > 0:
                    correct += 1
                    outcome = "correct"
                else:
                    wrong_wins += 1
                    outcome = "wrong_wins"

        by_class[cls][outcome] += 1
        by_surface[surface][outcome] += 1
        by_noise[noise][outcome] += 1
        if outcome != "correct" and len(failures) < 20:
            failures.append(
                {
                    "task_id": row["task_id"],
                    "operator_class": row["operator_class"],
                    "proof_rule_family": row["proof_rule_family"],
                    "sequence_length": row["sequence_length"],
                    "surface_family": row["surface_family"],
                    "noise_type": row["noise_type"],
                    "outcome": outcome,
                }
            )

    margins_sorted = sorted(margins)
    gaps_sorted = sorted(center_gaps)
    return {
        "key_mode": key_mode,
        "cells": cells,
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "compiled_phase_centers": len(positive_centers),
        "skipped_train_rows": skipped_train,
        "missing_heldout_centers": missing,
        "skipped_eval_rows": skipped_eval,
        "wrong_wins": wrong_wins,
        "heldout_correct_rows": correct,
        "heldout_accuracy_milli": milli(correct, len(heldout)),
        "median_margin": margins_sorted[len(margins_sorted) // 2] if margins_sorted else None,
        "p10_margin": margins_sorted[len(margins_sorted) // 10] if margins_sorted else None,
        "median_positive_center_gap": gaps_sorted[len(gaps_sorted) // 2] if gaps_sorted else None,
        "p10_positive_center_gap": gaps_sorted[len(gaps_sorted) // 10] if gaps_sorted else None,
        "by_class": {key: dict(value) for key, value in sorted(by_class.items())},
        "by_surface": {key: dict(value) for key, value in sorted(by_surface.items())},
        "by_noise": {key: dict(value) for key, value in sorted(by_noise.items())},
        "failure_examples": failures,
    }


def make_report(rows: list[dict[str, Any]], cells: int) -> dict[str, Any]:
    action_mode = eval_mode(rows, cells, "action")
    no_action_mode = eval_mode(rows, cells, "class_length")
    verdict = "PHASE_CENTER_OPERATOR_PROBE_WATCH"
    if (
        action_mode["heldout_accuracy_milli"] == 1000
        and action_mode["wrong_wins"] == 0
        and no_action_mode["heldout_accuracy_milli"] < action_mode["heldout_accuracy_milli"]
    ):
        verdict = "PHASE_CENTER_OPERATOR_PROBE_PASS"

    return {
        "schema_version": "phase_center_operator_probe_v1",
        "diagnostic_only": True,
        "verdict": verdict,
        "method": "transition_relation_waves_to_circular_center_of_mass",
        "not_used": [
            "proof_rule_id as key",
            "source_group as key",
            "task_id",
            "state_after_correct text",
            "state_after_wrong text",
            "epoch repair",
            "explicit out<-src program extraction",
        ],
        "claim_boundary": [
            "This is a phase/center-mass diagnostic, not a production Wave runtime proof.",
            "It tests whether candidate transition relation waves cluster around an operator center.",
            "It does not prove semantic grokking.",
            "A stronger next step must compile the phase centers into Wave field/readout weights and run flat parity/ablations.",
        ],
        "action_key_center": action_mode,
        "no_action_key_ablation": no_action_mode,
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    action = report["action_key_center"]
    no_action = report["no_action_key_ablation"]
    text = f"""# Phase Center Operator Probe

Date: 2026-07-02

## Question

Can v4 operator transitions be recognized by a center-of-mass of relation
waves, without epochs and without extracting an explicit slot-map program?

## Method

Script:

```text
data/rule_logic_operator_battery_v4/run_phase_center_operator_probe.py
```

The probe converts each candidate transition into relation-wave atoms:

```text
out slot i received source slot j
output length
source length
marker insertion when applicable
```

Train correct transitions form a circular phase center. Train wrong transitions
form an anti-center. Heldout correct and wrong candidates are scored by:

```text
coherence(candidate, correct_center) - coherence(candidate, wrong_center)
```

No epoch repair is used.

## Result

Verdict:

```text
{report["verdict"]}
```

Action-key center:

```text
compiled_phase_centers: {action["compiled_phase_centers"]}
heldout_rows: {action["heldout_rows"]}
heldout_accuracy_milli: {action["heldout_accuracy_milli"]}
wrong_wins: {action["wrong_wins"]}
median_margin: {action["median_margin"]}
p10_margin: {action["p10_margin"]}
median_positive_center_gap: {action["median_positive_center_gap"]}
p10_positive_center_gap: {action["p10_positive_center_gap"]}
```

No-action key ablation:

```text
compiled_phase_centers: {no_action["compiled_phase_centers"]}
heldout_accuracy_milli: {no_action["heldout_accuracy_milli"]}
wrong_wins: {no_action["wrong_wins"]}
```

By class:

```text
{json.dumps(action["by_class"], ensure_ascii=False, indent=2)}
```

## Interpretation

This is the intended "three knobs -> center of mass" diagnostic:

```text
many relation waves
-> common phase center
-> correct heldout transition closer than same-bag wrong transition
```

It does not show epoch-based learning. It shows that the operator signal can be
represented as a phase center over transition relations.

## Claim Boundary

Allowed:

```text
The current v4 battery has a zero-epoch phase-center operator signal.
```

Not allowed:

```text
semantic grokking proven
production Wave runtime solved
flat CPU parity proven by this probe
```

Next:

```text
compile these phase centers into actual Wave field/readout weights
and compare against current epoch repair.
```
"""
    path.write_text(text, encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--cells", type=int, default=16)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--markdown", type=Path, default=DEFAULT_MD)
    args = parser.parse_args()

    if args.cells < 2:
        raise SystemExit("--cells must be >= 2")

    rows = load_rows(args.corpus)
    report = make_report(rows, args.cells)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    write_markdown(args.markdown, report)

    action = report["action_key_center"]
    no_action = report["no_action_key_ablation"]
    print("phase_center_operator_probe:")
    print(f"  verdict: {report['verdict']}")
    print(f"  corpus: {args.corpus}")
    print(f"  report: {args.report}")
    print(f"  cells: {args.cells}")
    print(f"  action_compiled_phase_centers: {action['compiled_phase_centers']}")
    print(f"  action_heldout_accuracy_milli: {action['heldout_accuracy_milli']}")
    print(f"  action_wrong_wins: {action['wrong_wins']}")
    print(f"  action_median_margin: {action['median_margin']}")
    print(f"  action_p10_margin: {action['p10_margin']}")
    print(f"  no_action_heldout_accuracy_milli: {no_action['heldout_accuracy_milli']}")
    print(f"  no_action_wrong_wins: {no_action['wrong_wins']}")


if __name__ == "__main__":
    main()
