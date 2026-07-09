#!/usr/bin/env python3
"""Run a small laboratory of transferability experiments for .nwpc profiles.

Cold-path only. Reads live traces and decision logs, writes research artifacts,
and never changes runtime, local_accept, promotion, packages, or daemon state.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import math
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


HERE = Path(__file__).resolve().parent
RICHNESS_SCRIPT = HERE / "operator-profile-richness-research.py"
spec = importlib.util.spec_from_file_location("operator_richness_research", RICHNESS_SCRIPT)
if spec is None or spec.loader is None:
    raise RuntimeError(f"cannot load {RICHNESS_SCRIPT}")
richness = importlib.util.module_from_spec(spec)
spec.loader.exec_module(richness)


DEFAULT_APPEND = richness.DEFAULT_APPEND
DEFAULT_DECISIONS = richness.DEFAULT_DECISIONS
DEFAULT_METRICS = richness.DEFAULT_METRICS
DEFAULT_OUT_DIR = Path("target/nando-wave/research")


def now_utc() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def atom_has(atoms: list[str], value: str) -> bool:
    return value in atoms


def atom_value(atoms: list[str], prefixes: tuple[str, ...], default: str = "other") -> str:
    return richness.first_atom_value(atoms, prefixes, default)


def atom_signature(atoms: list[str], prefixes: tuple[str, ...], limit: int = 6) -> str:
    return richness.signature(atoms, prefixes, limit)


def load_materialized(append_path: Path, decision_path: Path) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    append_rows = richness.read_jsonl(append_path)
    decision_rows = richness.read_jsonl(decision_path)
    append_by_index = {row.get("append_row_index", idx): row for idx, row in enumerate(append_rows)}
    rows: list[dict[str, Any]] = []
    for offset, drow in enumerate(decision_rows):
        append_idx = drow.get("append_row_index")
        arow = append_by_index.get(append_idx, {})
        row_atoms = richness.atoms_from(arow) + richness.selected_bucket_atoms(drow)
        is_positive, is_false = richness.row_truth(drow)
        tokens = int(drow.get("tokens", drow.get("row_total_tokens", 0)) or 0)
        decisions = []
        for decision in drow.get("decisions", []):
            if not isinstance(decision, dict):
                continue
            pid = decision.get("profile_id")
            if not isinstance(pid, int):
                continue
            try:
                margin = float(decision.get("margin_micro", decision.get("margin", 0)) or 0)
            except Exception:
                margin = 0.0
            decisions.append({
                "profile_id": pid,
                "score_candidate": bool(decision.get("score_candidate", decision.get("candidate", False))),
                "margin_micro": margin,
                "score_source": decision.get("score_source", "unknown"),
                "quarantined": bool(decision.get("product_hot_profile_quarantined", False)),
            })
        command = atom_value(row_atoms, ("request_command_kind:", "tool_command_kind:"), "other")
        shell = atom_value(row_atoms, ("tool_command_shell_family:",), "other")
        action_family = atom_value(row_atoms, ("action_family:",), "other")
        route_operator = atom_value(row_atoms, ("route_operator:",), "other")
        project = atom_value(row_atoms, ("project_family:", "cwd_family:"), "other")
        state_sig = atom_signature(row_atoms, ("state_", "result_exit_", "tool_exit_", "exit_code"), 5)
        surface_sig = atom_signature(row_atoms, ("request_", "tool_command_", "action_family:"), 6)
        evidence_sig = atom_signature(row_atoms, ("result_", "tool_", "stderr_", "stdout_"), 6)
        rows.append({
            "offset": offset,
            "append_row_index": append_idx,
            "tail_line_index": drow.get("tail_line_index"),
            "tokens": max(tokens, 0),
            "is_positive": is_positive,
            "is_false": is_false,
            "exact_cache_hit": bool(drow.get("row_exact_cache_hit", drow.get("exact_cache_hit", False))),
            "atoms": row_atoms,
            "command": command,
            "shell": shell,
            "action_family": action_family,
            "route_operator": route_operator,
            "project": project,
            "state_sig": state_sig,
            "surface_sig": surface_sig,
            "evidence_sig": evidence_sig,
            "decisions": decisions,
        })
    return append_rows, rows


def precision_summary(accepted: int, true_accepts: int, false_accepts: int, tokens: int) -> dict[str, Any]:
    return {
        "accepted_rows": accepted,
        "true_accept_rows": true_accepts,
        "false_accept_rows": false_accepts,
        "tokens_saved_on_true_accepts": tokens,
        "precision_milli": int(round(1000 * true_accepts / accepted)) if accepted else None,
    }


def experiment_reflex_baselines(rows: list[dict[str, Any]]) -> dict[str, Any]:
    def is_zero(row: dict[str, Any]) -> bool:
        return atom_has(row["atoms"], "state_exit_code_band:zero") or atom_has(row["atoms"], "exit_code_band:zero")

    rules = {
        "exit_zero": lambda row: is_zero(row),
        "planning_zero": lambda row: is_zero(row) and row["action_family"] == "planning",
        "shell_inspect_zero": lambda row: is_zero(row) and row["shell"] == "shell_inspect",
        "git_zero": lambda row: is_zero(row) and row["shell"] == "git",
        "rust_zero": lambda row: is_zero(row) and row["shell"] == "rust",
        "sed_zero": lambda row: is_zero(row) and row["command"] == "sed",
        "ripgrep_zero": lambda row: is_zero(row) and row["command"] == "ripgrep",
        "current_cpu_accept": lambda row: row["is_positive"],
    }
    result = {}
    for name, predicate in rules.items():
        accepted = true_accepts = false_accepts = tokens = 0
        for row in rows:
            if not predicate(row):
                continue
            accepted += 1
            true_accepts += int(row["is_positive"])
            false_accepts += int(row["is_false"])
            if row["is_positive"]:
                tokens += row["tokens"]
        result[name] = precision_summary(accepted, true_accepts, false_accepts, tokens)
    return result


def experiment_temporal_stability(
    append_rows: list[dict[str, Any]],
    decision_rows: list[dict[str, Any]],
    metrics: dict[str, Any],
    blocks: int,
) -> dict[str, Any]:
    if not decision_rows:
        return {"blocks": [], "stable_profiles": []}
    block_size = max(1, math.ceil(len(decision_rows) / blocks))
    per_profile: dict[int, Counter[str]] = defaultdict(Counter)
    block_reports = []
    for block_index in range(blocks):
        chunk = decision_rows[block_index * block_size : (block_index + 1) * block_size]
        if not chunk:
            continue
        report = richness.build_report(append_rows, chunk, metrics, "all")
        counts = report["classification_counts"]
        block_reports.append({
            "block": block_index,
            "rows": report["window_summary"]["rows"],
            "accepted_rows": report["window_summary"]["accepted_rows"],
            "false_rows": report["window_summary"]["false_rows"],
            "classes": counts,
        })
        for item in report["top_profiles"]:
            per_profile[int(item["profile_id"])][item["classification"]] += 1
    stable = []
    for pid, counter in per_profile.items():
        strong_blocks = counter["rich_operator_candidate"]
        useful_blocks = strong_blocks + counter["useful_reflex_or_medium_operator"]
        if strong_blocks or useful_blocks >= 2:
            stable.append({
                "profile_id": pid,
                "rich_blocks": strong_blocks,
                "useful_or_rich_blocks": useful_blocks,
                "dangerous_blocks": counter["dangerous_broad_or_unclean"],
                "thin_blocks": counter["thin_reflex"],
                "weak_blocks": counter["weak_or_unproven"],
            })
    stable.sort(key=lambda item: (item["rich_blocks"], item["useful_or_rich_blocks"], -item["dangerous_blocks"]), reverse=True)
    return {
        "block_count": len(block_reports),
        "blocks": block_reports,
        "stable_profile_count": len(stable),
        "top_stable_profiles": stable[:30],
    }


def profile_kind_map(metrics: dict[str, Any]) -> dict[int, str]:
    inventory = richness.profile_inventory(metrics)
    return {pid: str(info.get("kind", "unknown")) for pid, info in inventory.items()}


def experiment_symbiotic_gate(rows: list[dict[str, Any]], metrics: dict[str, Any]) -> dict[str, Any]:
    kinds = profile_kind_map(metrics)
    buckets = defaultdict(lambda: {"rows": 0, "true_accepts": 0, "false_accepts": 0, "tokens": 0})
    for row in rows:
        hidden = observable = unknown = False
        for decision in row["decisions"]:
            if not decision["score_candidate"]:
                continue
            kind = kinds.get(decision["profile_id"], "unknown")
            hidden |= kind == "hidden_state"
            observable |= kind.startswith("observable")
            unknown |= kind == "unknown"
        if hidden and observable:
            key = "hidden_and_observable"
        elif hidden:
            key = "hidden_only"
        elif observable:
            key = "observable_only"
        elif unknown:
            key = "unknown_only"
        else:
            key = "no_profile_candidate"
        item = buckets[key]
        item["rows"] += 1
        item["true_accepts"] += int(row["is_positive"])
        item["false_accepts"] += int(row["is_false"])
        if row["is_positive"]:
            item["tokens"] += row["tokens"]
    return {
        key: precision_summary(value["rows"], value["true_accepts"], value["false_accepts"], value["tokens"])
        for key, value in sorted(buckets.items())
    }


def operator_family(row: dict[str, Any]) -> str:
    if row["shell"] == "rust":
        return "rust_check_or_test_transition"
    if row["shell"] == "git":
        return "git_state_transition"
    if row["command"] in {"sed", "ripgrep", "ls", "find"} or row["shell"] == "shell_inspect":
        return "file_inspection_transition"
    if "nonzero" in row["state_sig"] or "error" in row["evidence_sig"]:
        return "failure_triage_transition"
    if row["action_family"] == "planning":
        return "agent_planning_continuation"
    return "generic_agent_loop_transition"


def experiment_operator_family_typology(rows: list[dict[str, Any]]) -> dict[str, Any]:
    families = defaultdict(lambda: {
        "rows": 0,
        "true_accepts": 0,
        "false_accepts": 0,
        "tokens": 0,
        "commands": Counter(),
        "state_signatures": Counter(),
    })
    for row in rows:
        family = operator_family(row)
        item = families[family]
        item["rows"] += 1
        item["true_accepts"] += int(row["is_positive"])
        item["false_accepts"] += int(row["is_false"])
        item["commands"][row["command"]] += 1
        item["state_signatures"][row["state_sig"]] += 1
        if row["is_positive"]:
            item["tokens"] += row["tokens"]
    out = {}
    for family, item in families.items():
        summary = precision_summary(item["rows"], item["true_accepts"], item["false_accepts"], item["tokens"])
        summary["top_commands"] = item["commands"].most_common(5)
        summary["state_signature_unique"] = len(item["state_signatures"])
        out[family] = summary
    return dict(sorted(out.items(), key=lambda kv: kv[1]["tokens_saved_on_true_accepts"], reverse=True))


def experiment_split_candidates(
    rows: list[dict[str, Any]],
    accumulated_report: dict[str, Any],
    top_n: int,
    min_child_true: int,
) -> dict[str, Any]:
    dangerous_ids = [
        int(item["profile_id"])
        for item in accumulated_report["top_profiles"]
        if item["classification"] == "dangerous_broad_or_unclean"
    ][:top_n]
    result = []
    facets = {
        "command": lambda row: row["command"],
        "shell": lambda row: row["shell"],
        "state_sig": lambda row: row["state_sig"],
        "surface_sig": lambda row: row["surface_sig"],
        "family": operator_family,
        "project": lambda row: row["project"],
    }
    for pid in dangerous_ids:
        candidate_rows = []
        for row in rows:
            if any(decision["profile_id"] == pid and decision["score_candidate"] for decision in row["decisions"]):
                candidate_rows.append(row)
        profile_result = {
            "profile_id": pid,
            "candidate_rows": len(candidate_rows),
            "true_accepts": sum(int(row["is_positive"]) for row in candidate_rows),
            "false_accepts": sum(int(row["is_false"]) for row in candidate_rows),
            "best_splits": [],
        }
        for facet_name, facet_fn in facets.items():
            groups = defaultdict(lambda: {"rows": 0, "true": 0, "false": 0, "tokens": 0})
            for row in candidate_rows:
                key = facet_fn(row)
                groups[key]["rows"] += 1
                groups[key]["true"] += int(row["is_positive"])
                groups[key]["false"] += int(row["is_false"])
                if row["is_positive"]:
                    groups[key]["tokens"] += row["tokens"]
            clean_children = []
            for key, group in groups.items():
                if group["true"] >= min_child_true and group["false"] == 0:
                    clean_children.append({
                        "facet": facet_name,
                        "value": key,
                        "rows": group["rows"],
                        "true_accepts": group["true"],
                        "tokens": group["tokens"],
                    })
            clean_children.sort(key=lambda item: (item["tokens"], item["true_accepts"]), reverse=True)
            profile_result["best_splits"].extend(clean_children[:3])
        profile_result["best_splits"].sort(key=lambda item: (item["tokens"], item["true_accepts"]), reverse=True)
        profile_result["best_splits"] = profile_result["best_splits"][:8]
        result.append(profile_result)
    return {
        "top_dangerous_profiles_checked": len(result),
        "min_child_true_accepts": min_child_true,
        "profiles_with_clean_split": sum(1 for item in result if item["best_splits"]),
        "profiles": result,
    }


def experiment_threshold_sensitivity(rows: list[dict[str, Any]], quantiles: list[float]) -> dict[str, Any]:
    margins = []
    for row in rows:
        best = None
        for decision in row["decisions"]:
            if not decision["score_candidate"]:
                continue
            best = decision["margin_micro"] if best is None else max(best, decision["margin_micro"])
        if best is not None:
            margins.append(best)
    if not margins:
        return {}
    thresholds = [richness.percentile(margins, q) for q in quantiles]
    out = {}
    for q, threshold in zip(quantiles, thresholds):
        if threshold is None:
            continue
        accepted = true_accepts = false_accepts = tokens = 0
        for row in rows:
            best = None
            for decision in row["decisions"]:
                if not decision["score_candidate"]:
                    continue
                best = decision["margin_micro"] if best is None else max(best, decision["margin_micro"])
            if best is None or best < threshold:
                continue
            accepted += 1
            true_accepts += int(row["is_positive"])
            false_accepts += int(row["is_false"])
            if row["is_positive"]:
                tokens += row["tokens"]
        out[f"q{int(q * 100):02d}"] = {"threshold_micro": threshold} | precision_summary(accepted, true_accepts, false_accepts, tokens)
    return out


def markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Transferable Operator Laboratory V1",
        "",
        f"created_utc: {report['created_utc']}",
        "",
        "## Executive Readout",
        "",
        f"- accumulated_rows: {report['accumulated_richness']['window_summary']['rows']}",
        f"- latest_rows: {report['latest_richness']['window_summary']['rows']}",
        f"- accumulated_classes: {report['accumulated_richness']['classification_counts']}",
        f"- latest_classes: {report['latest_richness']['classification_counts']}",
        "",
        "## What Counts As Operator Power",
        "",
        "- Support: repeated positive transitions.",
        "- Portability: survives multiple commands/states/surfaces, not one string.",
        "- Negative separation: sees wrong/unsafe rows and does not fire.",
        "- Margin: p10 positive margin remains above p90 negative margin.",
        "- Stability: appears across time blocks, not only one short window.",
        "- Symbiosis: hidden state and observable evidence agree.",
        "",
        "## Reflex Baseline Battle",
        "",
        "| baseline | accepted | true | false | precision_milli | tokens_true |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for name, item in report["reflex_baselines"].items():
        lines.append(
            f"| {name} | {item['accepted_rows']} | {item['true_accept_rows']} | "
            f"{item['false_accept_rows']} | {item['precision_milli']} | "
            f"{item['tokens_saved_on_true_accepts']} |"
        )
    lines.extend(["", "## Symbiotic Gate", ""])
    lines.extend(["| lane | rows | true | false | precision_milli | tokens_true |", "|---|---:|---:|---:|---:|---:|"])
    for name, item in report["symbiotic_gate"].items():
        lines.append(
            f"| {name} | {item['accepted_rows']} | {item['true_accept_rows']} | "
            f"{item['false_accept_rows']} | {item['precision_milli']} | "
            f"{item['tokens_saved_on_true_accepts']} |"
        )
    lines.extend(["", "## Operator Family Typology", ""])
    lines.extend(["| family | rows | true | false | precision_milli | tokens_true | top_commands |", "|---|---:|---:|---:|---:|---:|---|"])
    for family, item in report["operator_family_typology"].items():
        commands = ", ".join(f"{cmd}:{count}" for cmd, count in item["top_commands"][:3])
        lines.append(
            f"| {family} | {item['accepted_rows']} | {item['true_accept_rows']} | "
            f"{item['false_accept_rows']} | {item['precision_milli']} | "
            f"{item['tokens_saved_on_true_accepts']} | {commands} |"
        )
    lines.extend(["", "## Temporal Stability", ""])
    lines.append(f"- stable_profile_count: {report['temporal_stability']['stable_profile_count']}")
    for item in report["temporal_stability"]["top_stable_profiles"][:10]:
        lines.append(
            f"- profile {item['profile_id']}: rich_blocks={item['rich_blocks']} "
            f"useful_or_rich_blocks={item['useful_or_rich_blocks']} "
            f"dangerous_blocks={item['dangerous_blocks']}"
        )
    lines.extend(["", "## Dangerous Profile Split Simulation", ""])
    split = report["split_candidates"]
    lines.append(f"- checked: {split['top_dangerous_profiles_checked']}")
    lines.append(f"- profiles_with_clean_split: {split['profiles_with_clean_split']}")
    for item in split["profiles"][:10]:
        best = item["best_splits"][:3]
        best_text = "; ".join(f"{b['facet']}={b['value']} true={b['true_accepts']} tokens={b['tokens']}" for b in best) or "none"
        lines.append(
            f"- profile {item['profile_id']}: true={item['true_accepts']} false={item['false_accepts']} best={best_text}"
        )
    lines.extend(["", "## Threshold Sensitivity", ""])
    lines.extend(["| threshold | accepted | true | false | precision_milli | tokens_true |", "|---|---:|---:|---:|---:|---:|"])
    for name, item in report["threshold_sensitivity"].items():
        lines.append(
            f"| {name}:{int(item['threshold_micro'])} | {item['accepted_rows']} | "
            f"{item['true_accept_rows']} | {item['false_accept_rows']} | "
            f"{item['precision_milli']} | {item['tokens_saved_on_true_accepts']} |"
        )
    lines.extend([
        "",
        "## Lecture Draft",
        "",
        "Переносимый оператор в NANDA CPU - это compact phase center повторяемого перехода state_t + action -> state_t+1.",
        "Его мощность измеряется не жирностью токенов, а поддержкой, переносимостью, отрицательной отделимостью, margin, стабильностью и verifier-safe симбиозом.",
        "Слабый оператор может быть полезен как CPU utility reflex, но он не доказывает reasoning claim.",
        "Грязный широкий оператор обязан идти в split/quarantine, даже если он экономит много токенов.",
        "",
    ])
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--append", type=Path, default=DEFAULT_APPEND)
    parser.add_argument("--decisions", type=Path, default=DEFAULT_DECISIONS)
    parser.add_argument("--metrics", type=Path, default=DEFAULT_METRICS)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--blocks", type=int, default=8)
    parser.add_argument("--top-dangerous", type=int, default=20)
    parser.add_argument("--min-child-true", type=int, default=5)
    args = parser.parse_args()

    append_rows_raw = richness.read_jsonl(args.append)
    decision_rows_raw = richness.read_jsonl(args.decisions)
    metrics = richness.read_json(args.metrics)
    _append_rows, rows = load_materialized(args.append, args.decisions)

    latest_richness = richness.build_report(append_rows_raw, richness.latest_tail_segment(decision_rows_raw), metrics, "all")
    accumulated_richness = richness.build_report(append_rows_raw, decision_rows_raw, metrics, "all")
    report = {
        "created_utc": now_utc(),
        "append_path": str(args.append),
        "decision_path": str(args.decisions),
        "metrics_path": str(args.metrics),
        "latest_richness": latest_richness,
        "accumulated_richness": accumulated_richness,
        "reflex_baselines": experiment_reflex_baselines(rows),
        "temporal_stability": experiment_temporal_stability(append_rows_raw, decision_rows_raw, metrics, args.blocks),
        "symbiotic_gate": experiment_symbiotic_gate(rows, metrics),
        "operator_family_typology": experiment_operator_family_typology(rows),
        "split_candidates": experiment_split_candidates(rows, accumulated_richness, args.top_dangerous, args.min_child_true),
        "threshold_sensitivity": experiment_threshold_sensitivity(rows, [0.50, 0.75, 0.90, 0.95, 0.99]),
    }

    args.out_dir.mkdir(parents=True, exist_ok=True)
    json_path = args.out_dir / "operator_transferability_lab_v1.json"
    md_path = args.out_dir / "OPERATOR_TRANSFERABILITY_LAB_V1.md"
    json_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    md_path.write_text(markdown(report), encoding="utf-8")
    print(json.dumps({
        "json": str(json_path),
        "markdown": str(md_path),
        "latest_classes": latest_richness["classification_counts"],
        "accumulated_classes": accumulated_richness["classification_counts"],
        "split_profiles_with_clean_child": report["split_candidates"]["profiles_with_clean_split"],
        "stable_profile_count": report["temporal_stability"]["stable_profile_count"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
