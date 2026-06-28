#!/usr/bin/env python3
"""Toy phase-cell grokking probe for modular addition.

This is intentionally standalone. It does not touch the Rust core, CLI, or
existing gates. The probe tests one narrow idea:

    mod m as wave period, symbols as phases, and grokking as a phase-center
    forming across several small cells.

The model starts with random phase tables A[a, cell], B[b, cell], C[c, cell].
For each candidate c it scores:

    score(a, b, c) = sum_cell cos(A[a] + B[b] - C[c])

If the cells discover modular addition, the correct candidate should become the
phase-center: A[a] + B[b] - C[a+b mod m] clusters near zero across cells.
"""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import dataclass
from typing import Any

import numpy as np


@dataclass(frozen=True)
class Config:
    modulus: int
    cells: int
    train_frac: float
    epochs: int
    lr: float
    seed: int
    trace_interval: int


class PhaseCenterModel:
    def __init__(self, config: Config) -> None:
        self.config = config
        rng = np.random.default_rng(config.seed)
        shape = (config.modulus, config.cells)
        self.phase_a = rng.uniform(-math.pi, math.pi, shape)
        self.phase_b = rng.uniform(-math.pi, math.pi, shape)
        self.phase_c = rng.uniform(-math.pi, math.pi, shape)

    def scores(
        self,
        samples: np.ndarray,
        cell_mask: np.ndarray | None = None,
        phase_c: np.ndarray | None = None,
    ) -> np.ndarray:
        candidate_phase = self.phase_c if phase_c is None else phase_c
        z = (
            self.phase_a[samples[:, 0], None, :]
            + self.phase_b[samples[:, 1], None, :]
            - candidate_phase[None, :, :]
        )
        if cell_mask is not None:
            z = z[:, :, cell_mask]
        return np.cos(z).sum(axis=2)

    def train_epoch(self, train: np.ndarray, rng: np.random.Generator) -> None:
        m = self.config.modulus
        lr = self.config.lr
        candidates = np.arange(m)

        for index in rng.permutation(len(train)):
            a, b, target = train[index]
            z = self.phase_a[a][None, :] + self.phase_b[b][None, :] - self.phase_c
            scores = np.cos(z).sum(axis=1)
            probs = softmax(scores)
            probs[target] -= 1.0

            sin_z = np.sin(z)
            grad_ab = np.sum(probs[:, None] * -sin_z, axis=0)
            grad_c = probs[:, None] * sin_z

            self.phase_a[a] -= lr * grad_ab
            self.phase_b[b] -= lr * grad_ab
            self.phase_c[candidates] -= lr * grad_c

    def metrics(
        self,
        samples: np.ndarray,
        cell_mask: np.ndarray | None = None,
        phase_c: np.ndarray | None = None,
    ) -> dict[str, float]:
        m = self.config.modulus
        scores = self.scores(samples, cell_mask=cell_mask, phase_c=phase_c)
        predictions = np.argmax(scores, axis=1)
        targets = samples[:, 2]
        accuracy = float(np.mean(predictions == targets))

        correct_scores = scores[np.arange(len(samples)), targets]
        wrong_scores = np.where(np.arange(m)[None, :] == targets[:, None], -1.0e9, scores)
        best_wrong_scores = np.max(wrong_scores, axis=1)
        margin = float(np.mean(correct_scores - best_wrong_scores))

        center_correct, center_wrong = self.center_metrics(samples, scores, phase_c=phase_c)
        return {
            "accuracy": accuracy,
            "margin": margin,
            "center_correct": center_correct,
            "center_wrong": center_wrong,
            "center_gap": center_correct - center_wrong,
        }

    def center_metrics(
        self,
        samples: np.ndarray,
        scores: np.ndarray,
        phase_c: np.ndarray | None = None,
    ) -> tuple[float, float]:
        m = self.config.modulus
        candidate_phase = self.phase_c if phase_c is None else phase_c
        targets = samples[:, 2]
        wrong_scores = np.where(np.arange(m)[None, :] == targets[:, None], -1.0e9, scores)
        best_wrong = np.argmax(wrong_scores, axis=1)

        correct_z = (
            self.phase_a[samples[:, 0]]
            + self.phase_b[samples[:, 1]]
            - candidate_phase[targets]
        )
        wrong_z = (
            self.phase_a[samples[:, 0]]
            + self.phase_b[samples[:, 1]]
            - candidate_phase[best_wrong]
        )
        correct_center = np.abs(np.mean(np.exp(1j * correct_z), axis=1))
        wrong_center = np.abs(np.mean(np.exp(1j * wrong_z), axis=1))
        return float(np.mean(correct_center)), float(np.mean(wrong_center))


def softmax(scores: np.ndarray) -> np.ndarray:
    shifted = scores - np.max(scores)
    probs = np.exp(shifted)
    return probs / np.sum(probs)


def build_dataset(config: Config) -> tuple[np.ndarray, np.ndarray]:
    rng = np.random.default_rng(config.seed ^ 0xA17CE)
    samples = np.array(
        [
            (a, b, (a + b) % config.modulus)
            for a in range(config.modulus)
            for b in range(config.modulus)
        ],
        dtype=np.int64,
    )
    rng.shuffle(samples)
    train_len = int(round(len(samples) * config.train_frac))
    train_len = min(max(train_len, 1), len(samples) - 1)
    return samples[:train_len], samples[train_len:]


def lookup_baseline(train: np.ndarray, holdout: np.ndarray, modulus: int) -> dict[str, float]:
    counts = np.bincount(train[:, 2], minlength=modulus)
    majority = int(np.argmax(counts))
    holdout_acc = float(np.mean(holdout[:, 2] == majority))
    return {
        "train_accuracy": 1.0,
        "holdout_accuracy": holdout_acc,
        "majority_target": majority,
        "random_expected_accuracy": 1.0 / float(modulus),
    }


def signal(train_metrics: dict[str, float], holdout_metrics: dict[str, float]) -> str:
    train_acc = train_metrics["accuracy"]
    holdout_acc = holdout_metrics["accuracy"]
    center_gap = holdout_metrics["center_gap"]

    if holdout_acc >= 0.95 and center_gap >= 0.25:
        return "centered_grok_candidate"
    if train_acc >= 0.95 and holdout_acc < 0.50:
        return "memorized_not_centered"
    if holdout_acc >= 0.50 and center_gap >= 0.10:
        return "center_forming"
    if train_acc >= 0.50:
        return "train_fit"
    return "warmup"


def ablations(
    model: PhaseCenterModel,
    holdout: np.ndarray,
    rng: np.random.Generator,
) -> dict[str, Any]:
    cells = model.config.cells
    full = model.metrics(holdout)
    drop_rows = []
    for cell in range(cells):
        mask = np.ones(cells, dtype=bool)
        mask[cell] = False
        metrics = model.metrics(holdout, cell_mask=mask)
        drop_rows.append(
            {
                "cell": cell,
                "accuracy": metrics["accuracy"],
                "accuracy_drop": full["accuracy"] - metrics["accuracy"],
                "margin_drop": full["margin"] - metrics["margin"],
            }
        )
    drop_rows.sort(key=lambda row: (row["accuracy_drop"], row["margin_drop"]), reverse=True)

    keep_count = max(1, cells // 2)
    top_cells = sorted(row["cell"] for row in drop_rows[:keep_count])
    top_mask = np.zeros(cells, dtype=bool)
    top_mask[top_cells] = True
    restricted = model.metrics(holdout, cell_mask=top_mask)

    shuffled_c = model.phase_c[rng.permutation(model.config.modulus)]
    shuffled = model.metrics(holdout, phase_c=shuffled_c)

    return {
        "full_accuracy": full["accuracy"],
        "drop_top_cell": drop_rows[0],
        "restricted_top_half_cells": top_cells,
        "restricted_top_half_accuracy": restricted["accuracy"],
        "shuffled_output_phase_accuracy": shuffled["accuracy"],
    }


def run(config: Config) -> dict[str, Any]:
    train, holdout = build_dataset(config)
    model = PhaseCenterModel(config)
    rng = np.random.default_rng(config.seed ^ 0xC0FFEE)
    trace = []

    trace_epochs = {0, 1, 2, 5, 10, 20, config.epochs}
    trace_epochs.update(range(config.trace_interval, config.epochs + 1, config.trace_interval))

    for epoch in range(config.epochs + 1):
        if epoch in trace_epochs:
            train_metrics = model.metrics(train)
            holdout_metrics = model.metrics(holdout)
            trace.append(
                {
                    "epoch": epoch,
                    "train": train_metrics,
                    "holdout": holdout_metrics,
                    "signal": signal(train_metrics, holdout_metrics),
                }
            )
        if epoch == config.epochs:
            break
        model.train_epoch(train, rng)

    final_train = model.metrics(train)
    final_holdout = model.metrics(holdout)
    final_ablations = ablations(model, holdout, rng)
    lookup = lookup_baseline(train, holdout, config.modulus)

    verdict = "CENTER_MASS_PHASE_PROBE_WATCH"
    if (
        final_train["accuracy"] >= 0.95
        and final_holdout["accuracy"] >= 0.95
        and final_holdout["center_gap"] >= 0.25
        and final_ablations["shuffled_output_phase_accuracy"] <= 0.35
        and lookup["holdout_accuracy"] <= 0.35
    ):
        verdict = "CENTER_MASS_PHASE_PROBE_PASS"

    return {
        "mode": "phase-center-mass-probe",
        "verdict": verdict,
        "claim_boundary": {
            "test_model_only": True,
            "not_transformer": True,
            "not_llm": True,
            "modulus_as_wave_period": True,
            "center_mass_means_phase_attractor_not_plain_average": True,
        },
        "config": {
            "modulus": config.modulus,
            "cells": config.cells,
            "train_frac": config.train_frac,
            "epochs": config.epochs,
            "lr": config.lr,
            "seed": config.seed,
            "train_cases": int(len(train)),
            "holdout_cases": int(len(holdout)),
        },
        "lookup_memorizer_baseline": lookup,
        "final": {
            "train": final_train,
            "holdout": final_holdout,
            "ablations": final_ablations,
        },
        "trace": trace,
    }


def print_text(report: dict[str, Any]) -> None:
    config = report["config"]
    final = report["final"]
    lookup = report["lookup_memorizer_baseline"]
    ablation = final["ablations"]

    print("Nando Wave phase center-mass probe")
    print(f"verdict: {report['verdict']}")
    print(
        "config: "
        f"m={config['modulus']} cells={config['cells']} "
        f"train={config['train_cases']} holdout={config['holdout_cases']} "
        f"epochs={config['epochs']} lr={config['lr']} seed={config['seed']}"
    )
    print(
        "lookup_memorizer: "
        f"train_acc={lookup['train_accuracy']:.3f} "
        f"holdout_acc={lookup['holdout_accuracy']:.3f} "
        f"random_expected={lookup['random_expected_accuracy']:.3f}"
    )
    print(
        "final_phase_model: "
        f"train_acc={final['train']['accuracy']:.3f} "
        f"holdout_acc={final['holdout']['accuracy']:.3f} "
        f"holdout_margin={final['holdout']['margin']:.3f} "
        f"center_correct={final['holdout']['center_correct']:.3f} "
        f"center_wrong={final['holdout']['center_wrong']:.3f} "
        f"center_gap={final['holdout']['center_gap']:.3f}"
    )
    print(
        "ablations: "
        f"drop_top_cell={ablation['drop_top_cell']} "
        f"restricted_top_half_acc={ablation['restricted_top_half_accuracy']:.3f} "
        f"shuffled_output_phase_acc={ablation['shuffled_output_phase_accuracy']:.3f}"
    )
    print("trace:")
    for row in report["trace"]:
        print(
            f"  epoch={row['epoch']:>4} "
            f"train={row['train']['accuracy']:.3f} "
            f"holdout={row['holdout']['accuracy']:.3f} "
            f"center_gap={row['holdout']['center_gap']:.3f} "
            f"signal={row['signal']}"
        )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--modulus", type=int, default=13)
    parser.add_argument("--cells", type=int, default=8)
    parser.add_argument("--train-frac", type=float, default=0.70)
    parser.add_argument("--epochs", type=int, default=500)
    parser.add_argument("--lr", type=float, default=0.025)
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--trace-interval", type=int, default=50)
    parser.add_argument("--format", choices=("text", "json"), default="text")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.modulus < 3:
        raise SystemExit("--modulus must be >= 3")
    if args.cells < 1:
        raise SystemExit("--cells must be >= 1")
    if not 0.0 < args.train_frac < 1.0:
        raise SystemExit("--train-frac must be between 0 and 1")
    if args.epochs < 1:
        raise SystemExit("--epochs must be >= 1")
    if args.trace_interval < 1:
        raise SystemExit("--trace-interval must be >= 1")

    config = Config(
        modulus=args.modulus,
        cells=args.cells,
        train_frac=args.train_frac,
        epochs=args.epochs,
        lr=args.lr,
        seed=args.seed,
        trace_interval=args.trace_interval,
    )
    report = run(config)
    if args.format == "json":
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_text(report)


if __name__ == "__main__":
    main()
