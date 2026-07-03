#!/usr/bin/env python3
"""Run shortcut baselines against the materialized Wave Task V2 candidate corpus."""

from __future__ import annotations

import json
import math
import re
from collections import Counter, defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parent
TASKS_PATH = ROOT / "generated_wave_task_v2.jsonl"
REPORT_PATH = ROOT / "shortcut_gate_report.json"
MANIFEST_PATH = ROOT / "manifest.json"

EXACT_LOOKUP_MAX_MILLI = 20
L2_NEIGHBOR_MAX_MILLI = 200
BAYESIAN_PAIRWISE_MAX_MILLI = 700
MARKOV_BIGRAM_MAX_MILLI = 700
TARGET_LEAK_MAX_MILLI = 200
SINGLE_TOKEN_MAX_MILLI = 250
NEAR_NEGATIVE_MIN_MILLI = 120

TOKEN_RE = re.compile(r"[0-9A-Za-zА-Яа-яЁё_./:-]+", re.UNICODE)


def load_tasks(path: Path) -> list[dict]:
    tasks: list[dict] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if line.strip():
                tasks.append(json.loads(line))
    return tasks


def tokens(text: str) -> list[str]:
    return [match.group(0).lower() for match in TOKEN_RE.finditer(text)]


def token_count(text: str) -> int:
    return len(tokens(text))


def trigrams(text: str) -> set[str]:
    normalized = " ".join(tokens(text))
    if len(normalized) < 3:
        return {normalized} if normalized else set()
    return {normalized[index : index + 3] for index in range(len(normalized) - 2)}


def trigram_jaccard(left: str, right: str) -> float:
    left_set = trigrams(left)
    right_set = trigrams(right)
    if not left_set and not right_set:
        return 1.0
    union = left_set | right_set
    if not union:
        return 0.0
    return len(left_set & right_set) / len(union)


def atom_set(text: str) -> set[str]:
    result = set(tokens(text))
    result.update(trigrams(text))
    return result


def milli_ratio(count: int, total: int) -> int:
    if total <= 0:
        return 0
    return round(1000 * count / total)


def split_by_source_group(tasks: list[dict]) -> tuple[list[dict], list[dict], list[str]]:
    groups = sorted({task["source_group"] for task in tasks})
    heldout_groups = [group for index, group in enumerate(groups) if index % 5 == 0]
    if not heldout_groups and groups:
        heldout_groups = [groups[-1]]
    heldout_set = set(heldout_groups)
    train = [task for task in tasks if task["source_group"] not in heldout_set]
    heldout = [task for task in tasks if task["source_group"] in heldout_set]
    return train, heldout, heldout_groups


def exact_lookup_accuracy_milli(train: list[dict], heldout: list[dict]) -> int:
    train_map = {task["input"]: task["target"] for task in train}
    correct = sum(1 for task in heldout if train_map.get(task["input"]) == task["target"])
    return milli_ratio(correct, len(heldout))


def l2_neighbor_accuracy_milli(train: list[dict], heldout: list[dict]) -> int:
    correct = 0
    for task in heldout:
        nearest = max(
            train,
            key=lambda candidate: trigram_jaccard(task["input"], candidate["input"]),
            default=None,
        )
        if nearest is None:
            continue
        if nearest["target"] == task["target"]:
            correct += 1
            continue
        if trigram_jaccard(nearest["target"], task["target"]) >= 0.92:
            correct += 1
    return milli_ratio(correct, len(heldout))


class BayesianCooccurrenceBaseline:
    def __init__(self, tasks: list[dict]) -> None:
        self.input_counts: Counter[str] = Counter()
        self.pair_counts: Counter[tuple[str, str]] = Counter()
        self.target_atom_counts: Counter[str] = Counter()
        self.total_target_atoms = 0

        for task in tasks:
            input_atoms = atom_set(task["input"])
            target_atoms = atom_set(task["target"])
            for input_atom in input_atoms:
                self.input_counts[input_atom] += 1
                for target_atom in target_atoms:
                    self.pair_counts[(input_atom, target_atom)] += 1
            for target_atom in target_atoms:
                self.target_atom_counts[target_atom] += 1
                self.total_target_atoms += 1

    def score(self, input_text: str, candidate: str) -> float:
        input_atoms = atom_set(input_text)
        candidate_atoms = atom_set(candidate)
        if not input_atoms or not candidate_atoms:
            return 0.0

        score = 0.0
        denominator_floor = max(self.total_target_atoms, 1)
        for input_atom in input_atoms:
            input_count = self.input_counts[input_atom]
            for target_atom in candidate_atoms:
                pair_count = self.pair_counts[(input_atom, target_atom)]
                target_prior = self.target_atom_counts[target_atom]
                numerator = pair_count + 0.25 * target_prior + 1.0
                denominator = input_count + denominator_floor
                score += numerator / denominator
        return score / (len(input_atoms) * len(candidate_atoms))

    def prefers_target(self, task: dict) -> bool:
        return self.score(task["input"], task["target"]) > self.score(
            task["input"], task["near_negative"]
        )


class MarkovBigramBaseline:
    def __init__(self, tasks: list[dict]) -> None:
        self.transition_counts: Counter[tuple[str, str]] = Counter()
        self.bigram_counts: Counter[tuple[str, str]] = Counter()
        self.unigram_counts: Counter[str] = Counter()

        for task in tasks:
            input_tokens = tokens(task["input"])
            target_tokens = tokens(task["target"])
            if input_tokens and target_tokens:
                self.transition_counts[(input_tokens[-1], target_tokens[0])] += 1
            self.unigram_counts.update(target_tokens)
            for left, right in zip(target_tokens, target_tokens[1:]):
                self.bigram_counts[(left, right)] += 1

    def score(self, input_text: str, candidate: str) -> float:
        input_tokens = tokens(input_text)
        candidate_tokens = tokens(candidate)
        if not candidate_tokens:
            return -math.inf

        score = 0.0
        if input_tokens:
            score += 4.0 * math.log(
                self.transition_counts[(input_tokens[-1], candidate_tokens[0])] + 1.0
            )
        for token in candidate_tokens:
            score += 0.25 * math.log(self.unigram_counts[token] + 1.0)
        for left, right in zip(candidate_tokens, candidate_tokens[1:]):
            score += math.log(self.bigram_counts[(left, right)] + 1.0)
        return score / len(candidate_tokens)

    def prefers_target(self, task: dict) -> bool:
        return self.score(task["input"], task["target"]) > self.score(
            task["input"], task["near_negative"]
        )


def bayesian_pairwise_accuracy_milli(train: list[dict], heldout: list[dict]) -> int:
    model = BayesianCooccurrenceBaseline(train)
    correct = sum(1 for task in heldout if model.prefers_target(task))
    return milli_ratio(correct, len(heldout))


def markov_bigram_accuracy_milli(train: list[dict], heldout: list[dict]) -> int:
    model = MarkovBigramBaseline(train)
    correct = sum(1 for task in heldout if model.prefers_target(task))
    return milli_ratio(correct, len(heldout))


def target_leak_milli(tasks: list[dict]) -> int:
    leaks = 0
    for task in tasks:
        if task["target"] in task["input"]:
            leaks += 1
            continue
        if trigram_jaccard(task["input"], task["target"]) >= 0.85:
            leaks += 1
    return milli_ratio(leaks, len(tasks))


def near_negative_similarity_milli(tasks: list[dict]) -> int:
    if not tasks:
        return 0
    return round(
        sum(trigram_jaccard(task["target"], task["near_negative"]) for task in tasks)
        * 1000
        / len(tasks)
    )


def single_token_ratio_milli(tasks: list[dict]) -> int:
    single_token = sum(
        1
        for task in tasks
        if token_count(task["input"]) <= 1 and token_count(task["target"]) <= 1
    )
    return milli_ratio(single_token, len(tasks))


def task_factory_verdict(metrics: dict) -> str:
    if metrics["single_token_ratio_milli"] > SINGLE_TOKEN_MAX_MILLI:
        return "REJECT_SINGLE_TOKEN_SEQUENCE"
    if metrics["heldout_source_groups"] == 0 or metrics["heldout_tasks"] == 0:
        return "REJECT_NO_SOURCE_GROUP_HELDOUT"
    if metrics["exact_lookup_accuracy_milli"] > EXACT_LOOKUP_MAX_MILLI:
        return "REJECT_EXACT_LOOKUP_SHORTCUT"
    if metrics["l2_neighbor_accuracy_milli"] > L2_NEIGHBOR_MAX_MILLI:
        return "REJECT_L2_NEIGHBOR_SHORTCUT"
    if metrics["bayesian_pairwise_accuracy_milli"] > BAYESIAN_PAIRWISE_MAX_MILLI:
        return "REJECT_BAYESIAN_SHORTCUT"
    if metrics["markov_bigram_accuracy_milli"] > MARKOV_BIGRAM_MAX_MILLI:
        return "REJECT_MARKOV_BIGRAM_SHORTCUT"
    if metrics["target_leak_milli"] > TARGET_LEAK_MAX_MILLI:
        return "REJECT_TARGET_LEAKAGE"
    if metrics["near_negative_similarity_milli"] < NEAR_NEGATIVE_MIN_MILLI:
        return "REJECT_NEGATIVE_TOO_ALIEN"
    return "VALID_OPERATOR_PRESSURE_CANDIDATE"


def main() -> int:
    tasks = load_tasks(TASKS_PATH)
    train, heldout, heldout_groups = split_by_source_group(tasks)
    task_kinds = sorted({task["task_kind"] for task in tasks})

    metrics = {
        "tasks_total": len(tasks),
        "train_tasks": len(train),
        "heldout_tasks": len(heldout),
        "source_groups": len({task["source_group"] for task in tasks}),
        "heldout_source_groups": len(heldout_groups),
        "task_kinds": len(task_kinds),
        "exact_lookup_accuracy_milli": exact_lookup_accuracy_milli(train, heldout),
        "l2_neighbor_accuracy_milli": l2_neighbor_accuracy_milli(train, heldout),
        "bayesian_pairwise_accuracy_milli": bayesian_pairwise_accuracy_milli(train, heldout),
        "markov_bigram_accuracy_milli": markov_bigram_accuracy_milli(train, heldout),
        "target_leak_milli": target_leak_milli(tasks),
        "near_negative_similarity_milli": near_negative_similarity_milli(tasks),
        "single_token_ratio_milli": single_token_ratio_milli(tasks),
    }
    verdict = task_factory_verdict(metrics)
    report = {
        "schema_version": "wave_shortcut_gate_report_v1",
        "corpus": str(TASKS_PATH.relative_to(ROOT.parents[2])),
        "quality_status": "candidate",
        "accepted_training_tasks": 0,
        "thresholds": {
            "exact_lookup_max_milli": EXACT_LOOKUP_MAX_MILLI,
            "l2_neighbor_max_milli": L2_NEIGHBOR_MAX_MILLI,
            "bayesian_pairwise_max_milli": BAYESIAN_PAIRWISE_MAX_MILLI,
            "markov_bigram_max_milli": MARKOV_BIGRAM_MAX_MILLI,
            "target_leak_max_milli": TARGET_LEAK_MAX_MILLI,
            "single_token_max_milli": SINGLE_TOKEN_MAX_MILLI,
            "near_negative_min_milli": NEAR_NEGATIVE_MIN_MILLI,
        },
        "heldout_groups": heldout_groups,
        "task_kinds_list": task_kinds,
        "metrics": metrics,
        "verdict": verdict,
        "useful_task_yield": {
            "method": "corpus_level_shortcut_gate_v1",
            "candidate_tasks": len(tasks),
            "useful_candidate_tasks": len(tasks)
            if verdict == "VALID_OPERATOR_PRESSURE_CANDIDATE"
            else 0,
            "useful_task_yield_milli": 1000
            if verdict == "VALID_OPERATOR_PRESSURE_CANDIDATE"
            else 0,
            "target_yield_milli": 300,
        },
        "boundary": {
            "gates_run": True,
            "candidate_only": True,
            "not_runtime_authority": True,
            "not_training_acceptance": True,
            "step_8_done": verdict == "VALID_OPERATOR_PRESSURE_CANDIDATE",
            "step_9_required_for_accepted_training_scale": True,
        },
    }
    with REPORT_PATH.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, ensure_ascii=False, indent=2)
        handle.write("\n")

    if MANIFEST_PATH.exists():
        with MANIFEST_PATH.open("r", encoding="utf-8") as handle:
            manifest = json.load(handle)
        manifest["shortcut_gate_report"] = str(REPORT_PATH.relative_to(ROOT.parents[2]))
        manifest["shortcut_gate_verdict"] = verdict
        manifest["useful_task_yield_milli"] = report["useful_task_yield"][
            "useful_task_yield_milli"
        ]
        manifest["useful_candidate_tasks"] = report["useful_task_yield"][
            "useful_candidate_tasks"
        ]
        manifest["accepted_training_tasks"] = 0
        boundary = manifest.setdefault("boundary", {})
        boundary["shortcut_gates_pending"] = False
        boundary["shortcut_gates_run"] = True
        boundary["step_7_done"] = True
        boundary["step_8_required_for_yield"] = False
        boundary["step_8_done"] = verdict == "VALID_OPERATOR_PRESSURE_CANDIDATE"
        boundary["step_9_required_for_accepted_training_scale"] = True
        with MANIFEST_PATH.open("w", encoding="utf-8") as handle:
            json.dump(manifest, handle, ensure_ascii=False, indent=2)
            handle.write("\n")

    print(
        "shortcut gates: "
        f"verdict={verdict} "
        f"tasks={metrics['tasks_total']} "
        f"heldout={metrics['heldout_tasks']} "
        f"exact={metrics['exact_lookup_accuracy_milli']} "
        f"l2={metrics['l2_neighbor_accuracy_milli']} "
        f"bayes={metrics['bayesian_pairwise_accuracy_milli']} "
        f"markov={metrics['markov_bigram_accuracy_milli']} "
        f"leak={metrics['target_leak_milli']} "
        f"nearneg={metrics['near_negative_similarity_milli']}"
    )
    return 0 if verdict == "VALID_OPERATOR_PRESSURE_CANDIDATE" else 1


if __name__ == "__main__":
    raise SystemExit(main())
