#!/usr/bin/env python3
"""Build narrow seed2/order density-sweep corpora.

This is a diagnostic generator, not a new proof corpus. It preserves the seed2
order heldout rows and duplicates only train rows for one target rule. The goal
is to distinguish data/weight sparsity from strict readout geometry debt.
"""

from __future__ import annotations

import json
import os
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SOURCE = ROOT / "diagnostics" / "multiseed" / "seed_002" / "order" / "accepted_operator_tasks_v4.jsonl"
OUT_ROOT = ROOT / "diagnostics" / "density_sweep" / "seed_002" / "order"
TARGET_RULE = os.environ.get("OPERATOR_BATTERY_DENSITY_RULE", "order_block_reverse_4_len13")


def factor_env() -> list[int]:
    raw = os.environ.get("OPERATOR_BATTERY_DENSITY_FACTORS", "1,4,16")
    factors = [int(item.strip()) for item in raw.split(",") if item.strip()]
    if not factors or any(factor < 1 for factor in factors):
        raise ValueError("OPERATOR_BATTERY_DENSITY_FACTORS must contain positive integers")
    return factors


def read_jsonl(path: Path) -> list[dict[str, object]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_jsonl(path: Path, rows: list[dict[str, object]]) -> None:
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def is_target_train(row: dict[str, object]) -> bool:
    return (
        row.get("proof_rule_id") == TARGET_RULE
        and f"_train_{TARGET_RULE}" in str(row.get("source_group", ""))
    )


def augmented_rows(rows: list[dict[str, object]], factor: int) -> tuple[list[dict[str, object]], int]:
    out = list(rows)
    target_train = [row for row in rows if is_target_train(row)]
    duplicate_count = 0
    for copy_index in range(1, factor):
        for row_index, row in enumerate(target_train):
            duplicate = dict(row)
            duplicate["task_id"] = f"{row['task_id']}_density_x{factor}_copy{copy_index}_{row_index}"
            duplicate["density_sweep_duplicate_of"] = row["task_id"]
            duplicate["density_sweep_factor"] = factor
            out.append(duplicate)
            duplicate_count += 1
    return out, duplicate_count


def manifest(rows: list[dict[str, object]], factor: int, duplicate_count: int) -> dict[str, object]:
    train = [row for row in rows if "_train_" in str(row.get("source_group", ""))]
    heldout = [row for row in rows if "_heldout_" in str(row.get("source_group", ""))]
    rules = Counter(str(row.get("proof_rule_id")) for row in rows)
    target_train = [row for row in train if row.get("proof_rule_id") == TARGET_RULE]
    return {
        "schema_version": "operator_battery_v4_seed2_order_density_sweep_v1",
        "source": str(SOURCE),
        "target_rule": TARGET_RULE,
        "factor": factor,
        "rows": len(rows),
        "train_rows": len(train),
        "heldout_rows": len(heldout),
        "duplicated_train_rows": duplicate_count,
        "target_rule_train_rows": len(target_train),
        "rules": dict(sorted(rules.items())),
        "forbidden_runtime_changes": {
            "target_id": False,
            "proof_rule_id_authority": False,
            "concrete_x_lookup": False,
            "manual_local_out_t": False,
        },
        "diagnostic_warning": (
            "This corpus reweights existing target-rule train rows. Passing it "
            "would classify the failure as data/weight sparsity, not as a new "
            "generalization proof."
        ),
    }


def main() -> int:
    rows = read_jsonl(SOURCE)
    if not rows:
        raise RuntimeError(f"empty source corpus: {SOURCE}")
    if not any(is_target_train(row) for row in rows):
        raise RuntimeError(f"no target train rows for {TARGET_RULE}")

    OUT_ROOT.mkdir(parents=True, exist_ok=True)
    summary: dict[str, object] = {
        "schema_version": "operator_battery_v4_density_sweep_summary_v1",
        "source": str(SOURCE),
        "target_rule": TARGET_RULE,
        "factors": {},
    }
    for factor in factor_env():
        factor_dir = OUT_ROOT / f"factor_{factor:03d}"
        factor_dir.mkdir(parents=True, exist_ok=True)
        out_rows, duplicate_count = augmented_rows(rows, factor)
        write_jsonl(factor_dir / "accepted_operator_tasks_v4.jsonl", out_rows)
        factor_manifest = manifest(out_rows, factor, duplicate_count)
        (factor_dir / "manifest.json").write_text(
            json.dumps(factor_manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        summary["factors"][str(factor)] = factor_manifest
        print(
            "density_sweep_build: "
            f"factor={factor} rows={factor_manifest['rows']} "
            f"train_rows={factor_manifest['train_rows']} "
            f"duplicated_train_rows={duplicate_count}",
            flush=True,
        )
    (OUT_ROOT / "density_sweep_summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
