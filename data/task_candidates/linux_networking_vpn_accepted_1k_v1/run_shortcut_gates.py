#!/usr/bin/env python3
"""Run shortcut gates for the accepted 1k VPN corpus."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path


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


def load_gate_module():
    spec = importlib.util.spec_from_file_location("wave_shortcut_gate_v1", COMPACT_GATE)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load gate module from {COMPACT_GATE}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


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
        "l2_neighbor_accuracy_milli": gate.l2_neighbor_accuracy_milli(train, heldout),
        "bayesian_pairwise_accuracy_milli": gate.bayesian_pairwise_accuracy_milli(
            train, heldout
        ),
        "markov_bigram_accuracy_milli": gate.markov_bigram_accuracy_milli(train, heldout),
        "target_leak_milli": gate.target_leak_milli(tasks),
        "near_negative_similarity_milli": gate.near_negative_similarity_milli(tasks),
        "single_token_ratio_milli": gate.single_token_ratio_milli(tasks),
    }
    verdict = gate.task_factory_verdict(metrics)
    accepted = verdict == "VALID_OPERATOR_PRESSURE_CANDIDATE"
    report = {
        "schema_version": "wave_shortcut_gate_report_v1",
        "corpus": str(TASKS_PATH.relative_to(REPO)),
        "quality_status": "accepted" if accepted else "candidate",
        "accepted_training_tasks": len(tasks) if accepted else 0,
        "thresholds": {
            "exact_lookup_max_milli": gate.EXACT_LOOKUP_MAX_MILLI,
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
            "step_9_done": accepted,
            "step_10_required_for_10k": accepted,
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
        boundary["step_9_done"] = accepted
        boundary["step_10_required_for_10k"] = accepted
        with MANIFEST_PATH.open("w", encoding="utf-8") as handle:
            json.dump(manifest, handle, ensure_ascii=False, indent=2)
            handle.write("\n")

    print(
        "shortcut gates 1k: "
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
    return 0 if accepted else 1


if __name__ == "__main__":
    raise SystemExit(main())
