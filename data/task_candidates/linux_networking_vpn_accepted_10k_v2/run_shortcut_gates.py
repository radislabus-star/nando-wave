#!/usr/bin/env python3
"""Run shortcut gates for the accepted 10k VPN corpus."""

from __future__ import annotations

import importlib.util
import json
import math
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
COMPACT_GATE = (
    REPO
    / "data"
    / "task_candidates"
    / "linux_networking_vpn_compact_v1"
    / "run_shortcut_gates.py"
)
TASKS_PATH = ROOT / "accepted_wave_task_v2.jsonl"
REPORT_PATH = ROOT / "shortcut_gate_report.json"
MANIFEST_PATH = ROOT / "manifest.json"
SOURCE_GROUP_PAIRWISE_MAX_MILLI = 700


def load_gate_module():
    spec = importlib.util.spec_from_file_location("wave_shortcut_gate_v1", COMPACT_GATE)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load gate module from {COMPACT_GATE}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def cached_trigrams(gate, tasks: list[dict], field: str) -> dict[str, set[str]]:
    return {task["task_id"]: gate.trigrams(task[field]) for task in tasks}


def cached_atom_sets(gate, tasks: list[dict], field: str) -> dict[str, tuple[str, ...]]:
    return {task["task_id"]: tuple(sorted(gate.atom_set(task[field]))) for task in tasks}


def jaccard(left: set[str], right: set[str]) -> float:
    if not left and not right:
        return 1.0
    union = left | right
    if not union:
        return 0.0
    return len(left & right) / len(union)


def l2_neighbor_accuracy_milli(gate, train: list[dict], heldout: list[dict]) -> int:
    train_input_tri = cached_trigrams(gate, train, "input")
    train_target_tri = cached_trigrams(gate, train, "target")
    heldout_input_tri = cached_trigrams(gate, heldout, "input")
    heldout_target_tri = cached_trigrams(gate, heldout, "target")
    train_by_id = {task["task_id"]: task for task in train}
    trigram_index: dict[str, list[str]] = defaultdict(list)
    for task in train:
        task_id = task["task_id"]
        for trigram in train_input_tri[task_id]:
            trigram_index[trigram].append(task_id)

    correct = 0
    for task in heldout:
        task_input = heldout_input_tri[task["task_id"]]
        overlap_counts: Counter[str] = Counter()
        for trigram in task_input:
            overlap_counts.update(trigram_index.get(trigram, ()))
        if overlap_counts:
            nearest_id = max(
                overlap_counts,
                key=lambda task_id: overlap_counts[task_id]
                / (len(task_input) + len(train_input_tri[task_id]) - overlap_counts[task_id]),
            )
            nearest = train_by_id[nearest_id]
        else:
            nearest = train[0] if train else None
        if nearest is None:
            continue
        if nearest["target"] == task["target"]:
            correct += 1
            continue
        if (
            jaccard(
                train_target_tri[nearest["task_id"]],
                heldout_target_tri[task["task_id"]],
            )
            >= 0.92
        ):
            correct += 1
    return gate.milli_ratio(correct, len(heldout))


def bayesian_pairwise_accuracy_milli(gate, train: list[dict], heldout: list[dict]) -> int:
    train_input_atoms = cached_atom_sets(gate, train, "input")
    train_target_atoms = cached_atom_sets(gate, train, "target")
    heldout_input_atoms = cached_atom_sets(gate, heldout, "input")
    heldout_target_atoms = cached_atom_sets(gate, heldout, "target")
    heldout_negative_atoms = cached_atom_sets(gate, heldout, "near_negative")

    candidate_atom_universe: set[str] = set()
    heldout_input_universe: set[str] = set()
    for task in heldout:
        heldout_input_universe.update(heldout_input_atoms[task["task_id"]])
        candidate_atom_universe.update(heldout_target_atoms[task["task_id"]])
        candidate_atom_universe.update(heldout_negative_atoms[task["task_id"]])

    grouped: Counter[tuple[tuple[str, ...], tuple[str, ...]]] = Counter()
    for task in train:
        grouped[(train_input_atoms[task["task_id"]], train_target_atoms[task["task_id"]])] += 1

    input_atom_ids = {atom: index for index, atom in enumerate(sorted(heldout_input_universe))}
    candidate_atom_ids = {atom: index for index, atom in enumerate(sorted(candidate_atom_universe))}
    input_counts = np.zeros(len(input_atom_ids), dtype=np.float64)
    target_atom_counts = np.zeros(len(candidate_atom_ids), dtype=np.float64)
    pair_matrix = np.zeros((len(input_atom_ids), len(candidate_atom_ids)), dtype=np.float64)
    total_target_atoms = 0

    for (input_atoms, target_atoms), count in grouped.items():
        for target_atom in target_atoms:
            target_id = candidate_atom_ids.get(target_atom)
            if target_id is not None:
                target_atom_counts[target_id] += count
        total_target_atoms += len(target_atoms) * count
        input_ids = [input_atom_ids[atom] for atom in input_atoms if atom in input_atom_ids]
        if not input_ids:
            continue
        target_ids = [candidate_atom_ids[atom] for atom in target_atoms if atom in candidate_atom_ids]
        input_counts[input_ids] += count
        if target_ids:
            pair_matrix[np.ix_(input_ids, target_ids)] += count

    denominator_floor = max(total_target_atoms, 1)
    candidate_payloads: dict[tuple[str, ...], tuple[list[int], float, int]] = {}
    for task in heldout:
        for atoms in [
            heldout_target_atoms[task["task_id"]],
            heldout_negative_atoms[task["task_id"]],
        ]:
            if atoms not in candidate_payloads:
                candidate_ids = [candidate_atom_ids[atom] for atom in atoms]
                prior_sum = float(np.sum(0.25 * target_atom_counts[candidate_ids] + 1.0))
                candidate_payloads[atoms] = (candidate_ids, prior_sum, len(atoms))

    def score(input_atoms: tuple[str, ...], candidate_atoms: tuple[str, ...]) -> float:
        if not input_atoms or not candidate_atoms:
            return 0.0
        candidate_ids, prior_sum, candidate_len = candidate_payloads[candidate_atoms]
        input_ids = [input_atom_ids[atom] for atom in input_atoms if atom in input_atom_ids]
        missing_input_atoms = len(input_atoms) - len(input_ids)
        raw_score = missing_input_atoms * (prior_sum / denominator_floor)
        if input_ids:
            pair_sums = np.sum(pair_matrix[np.ix_(input_ids, candidate_ids)], axis=1)
            denominators = input_counts[input_ids] + denominator_floor
            raw_score += float(np.sum((pair_sums + prior_sum) / denominators))
        return raw_score / (len(input_atoms) * candidate_len)

    correct = 0
    for task in heldout:
        task_id = task["task_id"]
        target_score = score(heldout_input_atoms[task_id], heldout_target_atoms[task_id])
        negative_score = score(heldout_input_atoms[task_id], heldout_negative_atoms[task_id])
        if target_score > negative_score:
            correct += 1
    return gate.milli_ratio(correct, len(heldout))


def markov_bigram_accuracy_milli(gate, train: list[dict], heldout: list[dict]) -> int:
    transition_counts: Counter[tuple[str, str]] = Counter()
    bigram_counts: Counter[tuple[str, str]] = Counter()
    unigram_counts: Counter[str] = Counter()

    token_cache: dict[tuple[str, str], list[str]] = {}
    for task in train + heldout:
        for field in ["input", "target", "near_negative"]:
            token_cache[(task["task_id"], field)] = gate.tokens(task[field])

    for task in train:
        input_tokens = token_cache[(task["task_id"], "input")]
        target_tokens = token_cache[(task["task_id"], "target")]
        if input_tokens and target_tokens:
            transition_counts[(input_tokens[-1], target_tokens[0])] += 1
        unigram_counts.update(target_tokens)
        for left, right in zip(target_tokens, target_tokens[1:]):
            bigram_counts[(left, right)] += 1

    def score(input_tokens: list[str], candidate_tokens: list[str]) -> float:
        if not candidate_tokens:
            return -math.inf
        raw_score = 0.0
        if input_tokens:
            raw_score += 4.0 * math.log(
                transition_counts[(input_tokens[-1], candidate_tokens[0])] + 1.0
            )
        for token in candidate_tokens:
            raw_score += 0.25 * math.log(unigram_counts[token] + 1.0)
        for left, right in zip(candidate_tokens, candidate_tokens[1:]):
            raw_score += math.log(bigram_counts[(left, right)] + 1.0)
        return raw_score / len(candidate_tokens)

    correct = 0
    for task in heldout:
        task_id = task["task_id"]
        input_tokens = token_cache[(task_id, "input")]
        if score(input_tokens, token_cache[(task_id, "target")]) > score(
            input_tokens, token_cache[(task_id, "near_negative")]
        ):
            correct += 1
    return gate.milli_ratio(correct, len(heldout))


def source_group_pairwise_accuracy_milli(gate, train: list[dict], heldout: list[dict]) -> int:
    group_target_atoms: dict[str, Counter[str]] = defaultdict(Counter)
    for task in train:
        group_target_atoms[task["source_group"]].update(gate.atom_set(task["target"]))

    def score(source_group: str, candidate: str) -> int:
        atoms = gate.atom_set(candidate)
        if not atoms:
            return 0
        counts = group_target_atoms[source_group]
        return sum(counts[atom] for atom in atoms)

    correct = 0
    for task in heldout:
        target_score = score(task["source_group"], task["target"])
        negative_score = score(task["source_group"], task["near_negative"])
        if target_score > negative_score:
            correct += 1
    return gate.milli_ratio(correct, len(heldout))


def main() -> int:
    gate = load_gate_module()
    tasks = gate.load_tasks(TASKS_PATH)
    train, heldout, heldout_groups = gate.split_by_source_group(tasks)
    task_kinds = sorted({task["task_kind"] for task in tasks})
    metrics = {
        "tasks_total": len(tasks),
        "train_tasks": len(train),
        "heldout_tasks": len(heldout),
        "source_groups": len({task["source_group"] for task in tasks}),
        "heldout_source_groups": len(heldout_groups),
        "task_kinds": len(task_kinds),
        "exact_lookup_accuracy_milli": gate.exact_lookup_accuracy_milli(train, heldout),
        "source_group_pairwise_accuracy_milli": source_group_pairwise_accuracy_milli(
            gate, train, heldout
        ),
        "l2_neighbor_accuracy_milli": l2_neighbor_accuracy_milli(gate, train, heldout),
        "bayesian_pairwise_accuracy_milli": bayesian_pairwise_accuracy_milli(
            gate, train, heldout
        ),
        "markov_bigram_accuracy_milli": markov_bigram_accuracy_milli(gate, train, heldout),
        "target_leak_milli": gate.target_leak_milli(tasks),
        "near_negative_similarity_milli": gate.near_negative_similarity_milli(tasks),
        "single_token_ratio_milli": gate.single_token_ratio_milli(tasks),
    }
    verdict = gate.task_factory_verdict(metrics)
    if metrics["source_group_pairwise_accuracy_milli"] > SOURCE_GROUP_PAIRWISE_MAX_MILLI:
        verdict = "REJECT_SOURCE_GROUP_SHORTCUT"
    accepted = verdict == "VALID_OPERATOR_PRESSURE_CANDIDATE"
    report = {
        "schema_version": "wave_shortcut_gate_report_v1",
        "corpus": str(TASKS_PATH.relative_to(REPO)),
        "quality_status": "accepted" if accepted else "candidate",
        "accepted_training_tasks": len(tasks) if accepted else 0,
        "thresholds": {
            "exact_lookup_max_milli": gate.EXACT_LOOKUP_MAX_MILLI,
            "source_group_pairwise_max_milli": SOURCE_GROUP_PAIRWISE_MAX_MILLI,
            "l2_neighbor_max_milli": gate.L2_NEIGHBOR_MAX_MILLI,
            "bayesian_pairwise_max_milli": gate.BAYESIAN_PAIRWISE_MAX_MILLI,
            "markov_bigram_max_milli": gate.MARKOV_BIGRAM_MAX_MILLI,
            "target_leak_max_milli": gate.TARGET_LEAK_MAX_MILLI,
            "single_token_max_milli": gate.SINGLE_TOKEN_MAX_MILLI,
            "near_negative_min_milli": gate.NEAR_NEGATIVE_MIN_MILLI,
        },
        "heldout_groups": heldout_groups,
        "task_kinds_list": task_kinds,
        "metrics": metrics,
        "verdict": verdict,
        "boundary": {
            "gates_run": True,
            "not_runtime_authority": True,
            "accepted_training_scale": accepted,
            "step_10_done": accepted,
            "step_11_required_for_wavepredictor": accepted,
            "optimized_gate_used": True,
        },
    }
    with REPORT_PATH.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, ensure_ascii=False, indent=2)
        handle.write("\n")

    for task in tasks:
        task["quality_status"] = "accepted" if accepted else "candidate"
    with TASKS_PATH.open("w", encoding="utf-8") as handle:
        for task in tasks:
            handle.write(json.dumps(task, ensure_ascii=False, separators=(",", ":")) + "\n")

    if MANIFEST_PATH.exists():
        with MANIFEST_PATH.open("r", encoding="utf-8") as handle:
            manifest = json.load(handle)
        manifest["shortcut_gate_report"] = str(REPORT_PATH.relative_to(REPO))
        manifest["shortcut_gate_verdict"] = verdict
        manifest["quality_status"] = "accepted" if accepted else "candidate"
        manifest["accepted_training_tasks"] = len(tasks) if accepted else 0
        boundary = manifest.setdefault("boundary", {})
        boundary["shortcut_gates_run"] = True
        boundary["accepted_training_scale"] = accepted
        boundary["step_10_done"] = accepted
        boundary["step_11_required_for_wavepredictor"] = accepted
        boundary["optimized_gate_used"] = True
        with MANIFEST_PATH.open("w", encoding="utf-8") as handle:
            json.dump(manifest, handle, ensure_ascii=False, indent=2)
            handle.write("\n")

    print(
        "shortcut gates 10k: "
        f"verdict={verdict} "
        f"tasks={metrics['tasks_total']} "
        f"heldout={metrics['heldout_tasks']} "
        f"exact={metrics['exact_lookup_accuracy_milli']} "
        f"source_group={metrics['source_group_pairwise_accuracy_milli']} "
        f"l2={metrics['l2_neighbor_accuracy_milli']} "
        f"bayes={metrics['bayesian_pairwise_accuracy_milli']} "
        f"markov={metrics['markov_bigram_accuracy_milli']} "
        f"leak={metrics['target_leak_milli']} "
        f"nearneg={metrics['near_negative_similarity_milli']}"
    )
    return 0 if accepted else 1


if __name__ == "__main__":
    raise SystemExit(main())
