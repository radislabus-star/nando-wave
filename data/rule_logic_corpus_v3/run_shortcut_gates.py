#!/usr/bin/env python3
"""Shortcut gates for Rule Logic v3 action traces."""

from __future__ import annotations

import json
import math
import re
from collections import Counter, defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parent
TASKS = ROOT / "accepted_action_trace_tasks_v3.jsonl"
REPORT = ROOT / "shortcut_gate_report.json"
TOKEN_RE = re.compile(r"[A-Za-zА-Яа-я0-9_]+|[{}:;?=<>|,-]")


def tokens(text: str) -> list[str]:
    return [token.lower() for token in TOKEN_RE.findall(text)]


def load_rows() -> list[dict[str, object]]:
    rows = []
    with TASKS.open("r", encoding="utf-8") as handle:
        for line in handle:
            if line.strip():
                rows.append(json.loads(line))
    return rows


def split_rows(rows: list[dict[str, object]]) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    train = []
    heldout = []
    for index, row in enumerate(rows):
        (heldout if index % 5 == 0 else train).append(row)
    return train, heldout


def milli(num: int, den: int) -> int:
    return 0 if den == 0 else round(1000 * num / den)


def exact_lookup(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    table = {
        str(row["state_before"]) + "\n" + str(row["rule_action_example"]): str(row["state_after_correct"])
        for row in train
    }
    hits = 0
    for row in heldout:
        key = str(row["state_before"]) + "\n" + str(row["rule_action_example"])
        hits += int(table.get(key) == str(row["state_after_correct"]))
    return milli(hits, len(heldout))


def majority_by_key(train: list[dict[str, object]], heldout: list[dict[str, object]], key: str) -> int:
    groups: dict[str, Counter[str]] = defaultdict(Counter)
    for row in train:
        groups[str(row[key])][str(row["state_after_correct"])] += 1
    hits = 0
    seen = 0
    for row in heldout:
        group = groups.get(str(row[key]))
        if not group:
            continue
        seen += 1
        hits += int(group.most_common(1)[0][0] == str(row["state_after_correct"]))
    return milli(hits, seen)


def markov_pairwise(train: list[dict[str, object]], heldout: list[dict[str, object]]) -> int:
    counts: dict[str, Counter[str]] = defaultdict(Counter)
    vocab = set()
    for row in train:
        context_tokens = set(tokens(str(row["state_before"]) + " " + str(row["rule_action_example"])))
        for target_token in tokens(str(row["state_after_correct"])):
            vocab.add(target_token)
            for context_token in context_tokens:
                counts[context_token][target_token] += 1
    vocab_size = max(1, len(vocab))

    def score(row: dict[str, object], candidate_field: str) -> float:
        context_tokens = set(tokens(str(row["state_before"]) + " " + str(row["rule_action_example"])))
        total = 0.0
        for cand_token in tokens(str(row[candidate_field])):
            prob = 1.0
            for context_token in context_tokens:
                counter = counts.get(context_token)
                if not counter:
                    continue
                prob += (counter[cand_token] + 1) / (sum(counter.values()) + vocab_size)
            total += math.log(prob)
        return total

    hits = 0
    ties = 0
    for row in heldout:
        good = score(row, "state_after_correct")
        bad = score(row, "state_after_wrong")
        if good > bad:
            hits += 1
        elif good == bad:
            ties += 1
    return milli(hits + ties // 2, len(heldout))


def verdict(metrics: dict[str, int]) -> str:
    if metrics["exact_lookup_accuracy_milli"] > 0:
        return "REJECT_EXACT_LOOKUP_SHORTCUT"
    if metrics["source_group_majority_accuracy_milli"] > 300:
        return "REJECT_SOURCE_GROUP_SHORTCUT"
    if metrics["proof_rule_id_majority_accuracy_milli"] > 300:
        return "REJECT_RULE_ID_SHORTCUT"
    if metrics["surface_family_majority_accuracy_milli"] > 300:
        return "REJECT_SURFACE_FAMILY_SHORTCUT"
    if metrics["answer_status_majority_accuracy_milli"] > 300:
        return "REJECT_STATUS_SHORTCUT"
    if metrics["markov_pairwise_accuracy_milli"] > 650:
        return "REJECT_MARKOV_TRACE_SHORTCUT"
    return "VALID_ACTION_TRACE_OPERATOR_PRESSURE_CANDIDATE"


def main() -> int:
    rows = load_rows()
    train, heldout = split_rows(rows)
    metrics = {
        "rows": len(rows),
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "exact_lookup_accuracy_milli": exact_lookup(train, heldout),
        "source_group_majority_accuracy_milli": majority_by_key(train, heldout, "source_group"),
        "proof_rule_id_majority_accuracy_milli": majority_by_key(train, heldout, "proof_rule_id"),
        "surface_family_majority_accuracy_milli": majority_by_key(train, heldout, "surface_family"),
        "answer_status_majority_accuracy_milli": majority_by_key(train, heldout, "answer_status"),
        "markov_pairwise_accuracy_milli": markov_pairwise(train, heldout),
    }
    report = {
        "schema_version": "rule_logic_action_trace_shortcut_gate_v3",
        "metrics": metrics,
        "verdict": verdict(metrics),
    }
    REPORT.write_text(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True))
    return 0 if report["verdict"] == "VALID_ACTION_TRACE_OPERATOR_PRESSURE_CANDIDATE" else 1


if __name__ == "__main__":
    raise SystemExit(main())
