#!/usr/bin/env python3
"""Research operator-profile richness from live phase-center traces.

This is a cold-path research tool. It never changes local_accept, packages,
promotion gates, or runtime state.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DEFAULT_APPEND = Path("/var/lib/nando-wave/streaming/live-agent-phase-atom-append-v1.jsonl")
DEFAULT_DECISIONS = Path("/var/lib/nando-wave/streaming/nando-phase-live-miner-tail.decisions.jsonl")
DEFAULT_METRICS = Path("/var/lib/nando-wave/streaming/metrics/nando-phase-center.metrics.json")
DEFAULT_OUT_DIR = Path("target/nando-wave/research")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not path.exists():
        return rows
    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(obj, dict):
                rows.append(obj)
    return rows


def read_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return {}
    return data if isinstance(data, dict) else {}


def latest_tail_segment(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if not rows:
        return []
    start = 0
    prev: int | None = None
    for idx, row in enumerate(rows):
        cur = row.get("tail_line_index")
        if not isinstance(cur, int):
            cur = row.get("append_row_index")
        if isinstance(cur, int) and prev is not None and cur < prev:
            start = idx
        if isinstance(cur, int):
            prev = cur
    return rows[start:]


def atoms_from(row: dict[str, Any]) -> list[str]:
    atoms: list[str] = []
    for key in (
        "action_atoms",
        "request_atoms",
        "state_atoms",
        "result_atoms",
        "route_hint_atoms",
    ):
        value = row.get(key)
        if isinstance(value, list):
            atoms.extend(str(item) for item in value)
    groups = row.get("atom_groups")
    if isinstance(groups, dict):
        for value in groups.values():
            if isinstance(value, list):
                atoms.extend(str(item) for item in value)
    return atoms


def first_atom_value(atoms: list[str], prefixes: tuple[str, ...], default: str = "other") -> str:
    for atom in atoms:
        for prefix in prefixes:
            if atom.startswith(prefix):
                return atom[len(prefix) :]
    return default


def signature(atoms: list[str], prefixes: tuple[str, ...], limit: int = 6) -> str:
    selected = []
    for atom in atoms:
        if atom.startswith(prefixes):
            selected.append(atom)
    if not selected:
        return "none"
    selected = sorted(set(selected))[:limit]
    return "|".join(selected)


def short_hash(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, ensure_ascii=True, default=str).encode("utf-8")
    return hashlib.blake2s(encoded, digest_size=6).hexdigest()


def percentile(values: list[float], pct: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    if len(ordered) == 1:
        return float(ordered[0])
    pos = (len(ordered) - 1) * pct
    lo = math.floor(pos)
    hi = math.ceil(pos)
    if lo == hi:
        return float(ordered[lo])
    return float(ordered[lo] * (hi - pos) + ordered[hi] * (pos - lo))


def profile_inventory(metrics: dict[str, Any]) -> dict[int, dict[str, Any]]:
    inventory: dict[int, dict[str, Any]] = {}
    for key in ("operator_profile_token_ranking", "top_safe_profiles", "top_quarantine_profiles"):
        rows = metrics.get(key)
        if not isinstance(rows, list):
            continue
        for row in rows:
            if not isinstance(row, dict):
                continue
            pid = row.get("profile_id")
            if isinstance(pid, int):
                inventory.setdefault(pid, {}).update(row)
    return inventory


def row_truth(row: dict[str, Any]) -> tuple[bool, bool]:
    false_count = row.get("row_false_accepts", row.get("false_accepts", 0))
    unique_accepts = row.get(
        "row_unique_cpu_accepts_over_exact_cache",
        row.get("row_verified_safe_accepts", row.get("unique_cpu_accepts_over_exact_cache", 0)),
    )
    try:
        is_false = int(false_count) > 0
    except Exception:
        is_false = False
    try:
        is_positive = int(unique_accepts) > 0 and not is_false
    except Exception:
        is_positive = False
    return is_positive, is_false


def selected_bucket_atoms(decision: dict[str, Any]) -> list[str]:
    bucket_key = decision.get("bucket_key")
    if isinstance(bucket_key, str):
        return [part.strip() for part in bucket_key.split("|") if part.strip()]
    atoms = decision.get("selected_bucket_atoms")
    if isinstance(atoms, list):
        return [str(atom) for atom in atoms]
    return []


def classify(item: dict[str, Any]) -> str:
    if item["false_candidate_negatives"] > 0:
        return "dangerous_broad_or_unclean"
    if (
        item["positive_candidate_rows"] >= 20
        and item["command_unique"] >= 3
        and item["state_signature_unique"] >= 5
        and item["max_concentration"] <= 0.75
        and item["negative_rows_seen"] >= 5
        and (item["positive_candidate_margin_p10"] or 0) > 0
        and item["richness_score"] >= 55
    ):
        return "rich_operator_candidate"
    if item["positive_candidate_rows"] >= 10 and item["max_concentration"] <= 0.90:
        return "useful_reflex_or_medium_operator"
    if item["positive_candidate_rows"] >= 3:
        return "thin_reflex"
    return "weak_or_unproven"


def build_report(
    append_rows: list[dict[str, Any]],
    decisions: list[dict[str, Any]],
    metrics: dict[str, Any],
    segment_mode: str,
) -> dict[str, Any]:
    segment = decisions if segment_mode == "all" else latest_tail_segment(decisions)
    append_by_index = {
        row.get("append_row_index", idx): row
        for idx, row in enumerate(append_rows)
    }
    inventory = profile_inventory(metrics)

    profiles: dict[int, dict[str, Any]] = defaultdict(lambda: {
        "positive_candidate_rows": 0,
        "candidate_rows": 0,
        "negative_rows_seen": 0,
        "false_candidate_negatives": 0,
        "tokens_on_positive_candidates": 0,
        "commands": Counter(),
        "cwds": Counter(),
        "state_signatures": Counter(),
        "surface_signatures": Counter(),
        "evidence_signatures": Counter(),
        "output_hashes": Counter(),
        "pos_candidate_margins": [],
        "neg_all_margins": [],
        "neg_candidate_margins": [],
        "selected_bucket_atoms": Counter(),
    })

    window = {
        "rows": len(segment),
        "candidate_rows": 0,
        "accepted_rows": 0,
        "exact_cache_rows": 0,
        "false_rows": 0,
        "total_tokens": 0,
    }
    baselines = {
        "exit_zero_rows": 0,
        "exit_zero_accepts": 0,
        "exit_zero_false_rows": 0,
        "planning_zero_rows": 0,
        "planning_zero_accepts": 0,
        "planning_zero_false_rows": 0,
    }

    for drow in segment:
        is_positive, is_false = row_truth(drow)
        tokens = int(drow.get("tokens", drow.get("row_total_tokens", 0)) or 0)
        window["total_tokens"] += max(tokens, 0)
        window["accepted_rows"] += int(is_positive)
        window["false_rows"] += int(is_false)
        window["candidate_rows"] += int(bool(drow.get("row_score_candidate", drow.get("score_candidate", True))))
        window["exact_cache_rows"] += int(bool(drow.get("row_exact_cache_hit", drow.get("exact_cache_hit", False))))

        append_idx = drow.get("append_row_index")
        arow = append_by_index.get(append_idx, {})
        atoms = atoms_from(arow) + selected_bucket_atoms(drow)
        command = first_atom_value(atoms, ("request_command_kind:", "tool_command_kind:"), "other")
        cwd = first_atom_value(atoms, ("cwd_family:", "project_family:"), "other")
        state_sig = signature(atoms, ("state_", "result_exit_", "tool_exit_", "exit_code"), 5)
        surface_sig = signature(atoms, ("request_", "tool_command_", "action_family:"), 6)
        evidence_sig = signature(atoms, ("result_", "tool_", "stderr_", "stdout_"), 6)
        output_sig = short_hash({"result": arow.get("result_atoms"), "tokens": tokens, "append": append_idx})
        exit_zero = any("state_exit_code_band:zero" == atom or "exit_code_band:zero" == atom for atom in atoms)
        planning_zero = exit_zero and any("action_family:planning" == atom for atom in atoms)
        if exit_zero:
            baselines["exit_zero_rows"] += 1
            baselines["exit_zero_accepts"] += int(is_positive)
            baselines["exit_zero_false_rows"] += int(is_false)
        if planning_zero:
            baselines["planning_zero_rows"] += 1
            baselines["planning_zero_accepts"] += int(is_positive)
            baselines["planning_zero_false_rows"] += int(is_false)

        decisions_list = drow.get("decisions", [])
        if not isinstance(decisions_list, list):
            continue
        for decision in decisions_list:
            if not isinstance(decision, dict):
                continue
            pid = decision.get("profile_id")
            if not isinstance(pid, int):
                continue
            margin = decision.get("margin_micro", decision.get("margin"))
            try:
                margin_value = float(margin)
            except Exception:
                margin_value = 0.0
            score_candidate = bool(decision.get("score_candidate", decision.get("candidate", False)))
            pdata = profiles[pid]
            pdata["commands"][command] += int(is_positive and score_candidate)
            pdata["cwds"][cwd] += int(is_positive and score_candidate)
            pdata["state_signatures"][state_sig] += int(is_positive and score_candidate)
            pdata["surface_signatures"][surface_sig] += int(is_positive and score_candidate)
            pdata["evidence_signatures"][evidence_sig] += int(is_positive and score_candidate)
            pdata["output_hashes"][output_sig] += int(is_positive and score_candidate)
            for atom in selected_bucket_atoms(decision)[:16]:
                pdata["selected_bucket_atoms"][atom] += int(is_positive and score_candidate)
            if is_false:
                pdata["negative_rows_seen"] += 1
                pdata["neg_all_margins"].append(margin_value)
                if score_candidate:
                    pdata["false_candidate_negatives"] += 1
                    pdata["neg_candidate_margins"].append(margin_value)
            if is_positive and score_candidate:
                pdata["positive_candidate_rows"] += 1
                pdata["candidate_rows"] += 1
                pdata["tokens_on_positive_candidates"] += max(tokens, 0)
                pdata["pos_candidate_margins"].append(margin_value)
            elif score_candidate:
                pdata["candidate_rows"] += 1

    top_profiles: list[dict[str, Any]] = []
    for pid, pdata in profiles.items():
        pos = int(pdata["positive_candidate_rows"])
        all_concentrations = []
        for key in ("commands", "state_signatures", "surface_signatures"):
            counter = pdata[key]
            if pos > 0 and counter:
                all_concentrations.append(max(counter.values()) / pos)
        max_concentration = max(all_concentrations) if all_concentrations else 1.0
        p10_pos = percentile(pdata["pos_candidate_margins"], 0.10)
        p90_neg = percentile(pdata["neg_all_margins"], 0.90)
        margin_gap = None if p10_pos is None or p90_neg is None else p10_pos - p90_neg
        diversity_score = min(30.0, (
            len(pdata["commands"]) * 2.0
            + len(pdata["state_signatures"]) * 0.6
            + len(pdata["surface_signatures"]) * 0.3
            + len(pdata["evidence_signatures"]) * 0.4
        ))
        support_score = min(25.0, math.log1p(pos) * 5.0)
        negative_score = min(15.0, math.log1p(pdata["negative_rows_seen"]) * 4.0)
        margin_score = 20.0 if margin_gap is not None and margin_gap > 0 else 0.0
        concentration_penalty = 20.0 * max(0.0, max_concentration - 0.75)
        false_penalty = min(50.0, pdata["false_candidate_negatives"] * 5.0)
        richness_score = max(0.0, support_score + diversity_score + negative_score + margin_score - concentration_penalty - false_penalty)
        info = inventory.get(pid, {})
        item = {
            "profile_id": pid,
            "kind": info.get("kind", "unknown"),
            "status": info.get("status", "unknown"),
            "richness_score": round(richness_score, 1),
            "positive_candidate_rows": pos,
            "candidate_rows": int(pdata["candidate_rows"]),
            "negative_rows_seen": int(pdata["negative_rows_seen"]),
            "false_candidate_negatives": int(pdata["false_candidate_negatives"]),
            "tokens_on_positive_candidates": int(pdata["tokens_on_positive_candidates"]),
            "command_unique": len(pdata["commands"]),
            "cwd_unique": len(pdata["cwds"]),
            "state_signature_unique": len(pdata["state_signatures"]),
            "surface_signature_unique": len(pdata["surface_signatures"]),
            "evidence_unique": len(pdata["evidence_signatures"]),
            "output_hash_unique": len(pdata["output_hashes"]),
            "max_concentration": round(max_concentration, 3),
            "positive_candidate_margin_median": percentile(pdata["pos_candidate_margins"], 0.50),
            "positive_candidate_margin_p10": p10_pos,
            "negative_all_margin_p90": p90_neg,
            "max_negative_candidate_margin": max(pdata["neg_candidate_margins"]) if pdata["neg_candidate_margins"] else None,
            "margin_gap_p10pos_p90neg": margin_gap,
            "top_commands": pdata["commands"].most_common(5),
            "top_cwds": pdata["cwds"].most_common(5),
            "top_state_signatures": pdata["state_signatures"].most_common(3),
            "top_selected_bucket_atoms": pdata["selected_bucket_atoms"].most_common(8),
        }
        item["classification"] = classify(item)
        top_profiles.append(item)

    top_profiles.sort(key=lambda item: (item["richness_score"], item["tokens_on_positive_candidates"]), reverse=True)
    counts = Counter(item["classification"] for item in top_profiles)
    return {
        "created_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "segment_mode": segment_mode,
        "append_path": str(DEFAULT_APPEND),
        "decision_path": str(DEFAULT_DECISIONS),
        "window_summary": window | {"profiles_scored": len(profiles)},
        "dumb_reflex_baselines": baselines,
        "classification_counts": dict(sorted(counts.items())),
        "top_profiles": top_profiles,
        "readout": {
            "rich_operator_candidate": "diverse positives, negative exposure, zero verifier-negative candidate fires, robust p10 margin",
            "useful_reflex_or_medium_operator": "useful CPU behavior but weaker support/diversity",
            "thin_reflex": "narrow surface/state pattern; utility only",
            "dangerous_broad_or_unclean": "candidate fired on verifier-negative rows; split/quarantine before claim",
        },
    }


def markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Operator Richness Research V1 Strict",
        "",
        f"created_utc: {report['created_utc']}",
        f"segment_mode: {report['segment_mode']}",
        "",
        "## Window",
    ]
    for key, value in report["window_summary"].items():
        lines.append(f"- {key}: {value}")
    lines.extend(["", "## Dumb Reflex Baselines"])
    for key, value in report["dumb_reflex_baselines"].items():
        lines.append(f"- {key}: {value}")
    lines.extend(["", "## Classification"])
    for key, value in report["classification_counts"].items():
        lines.append(f"- {key}: {value}")
    lines.extend([
        "",
        "## Top Profiles",
        "| profile | kind | class | score | pos | neg_seen | false_neg | cmd_u | state_u | conc | p10_pos | p90_neg | tokens | top_commands |",
        "|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ])
    for item in report["top_profiles"][:25]:
        top_commands = ", ".join(f"{cmd}:{count}" for cmd, count in item["top_commands"][:3])
        lines.append(
            f"| {item['profile_id']} | {item['kind']} | {item['classification']} | "
            f"{item['richness_score']} | {item['positive_candidate_rows']} | "
            f"{item['negative_rows_seen']} | {item['false_candidate_negatives']} | "
            f"{item['command_unique']} | {item['state_signature_unique']} | "
            f"{item['max_concentration']} | {item['positive_candidate_margin_p10']} | "
            f"{item['negative_all_margin_p90']} | {item['tokens_on_positive_candidates']} | "
            f"{top_commands} |"
        )
    lines.extend([
        "",
        "## Readout",
        "- Strong means: diverse candidate positives, negative exposure, no verifier-negative candidate fires, and positive candidate p10 margin above negative p90 margin.",
        "- Thin reflex means: useful CPU behavior but concentrated on one surface/state pattern.",
        "- Dangerous broad means: profile fired on verifier-negative rows and must be split/quarantined.",
        "- This report does not enable local_accept and does not claim money savings.",
        "",
    ])
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--append", type=Path, default=DEFAULT_APPEND)
    parser.add_argument("--decisions", type=Path, default=DEFAULT_DECISIONS)
    parser.add_argument("--metrics", type=Path, default=DEFAULT_METRICS)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument(
        "--segment",
        choices=("latest", "all"),
        default="latest",
        help="latest uses the last daemon segment; all uses accumulated decision history for negative exposure research",
    )
    parser.add_argument(
        "--label",
        default="strict",
        help="output label suffix: operator_richness_report_<label>.json",
    )
    args = parser.parse_args()

    append_rows = read_jsonl(args.append)
    decisions = read_jsonl(args.decisions)
    metrics = read_json(args.metrics)
    report = build_report(append_rows, decisions, metrics, args.segment)
    args.out_dir.mkdir(parents=True, exist_ok=True)
    safe_label = "".join(ch if ch.isalnum() or ch in ("-", "_") else "_" for ch in args.label)
    json_path = args.out_dir / f"operator_richness_report_{safe_label}.json"
    md_path = args.out_dir / f"OPERATOR_RICHNESS_REPORT_{safe_label.upper()}.md"
    json_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    md_path.write_text(markdown(report), encoding="utf-8")
    print(json.dumps({
        "json": str(json_path),
        "markdown": str(md_path),
        "window": report["window_summary"],
        "classification_counts": report["classification_counts"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
