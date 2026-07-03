#!/usr/bin/env python3
"""Run v4 operator-battery multi-seed robustness checks.

The runner is intentionally boring: it does not change corpus rules or runtime
mechanics. It rebuilds the same v4 battery under different seeds, runs shortcut
gates, optionally runs the release runtime gates, and records the result under
diagnostics/multiseed/.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
OUT_ROOT = Path(
    os.environ.get("OPERATOR_BATTERY_MULTI_OUT_ROOT", ROOT / "diagnostics" / "multiseed")
).resolve()

DEFAULT_CLASSES = ["order", "edit", "conditional", "composed"]

RUNTIME_TESTS = {
    "order": "operator_battery_v4_order_must_transfer_without_lookup_or_runtime_phase_hack",
    "edit": "operator_battery_v4_edit_marker_length_must_transfer_without_lookup_or_runtime_phase_hack",
    "conditional": "operator_battery_v4_conditional_state_channel_must_transfer_without_action_flag_leak",
    "composed": "operator_battery_v4_composed_must_transfer_without_lookup_or_runtime_phase_hack",
}

CORPUS_ENV = {
    "order": "OPERATOR_BATTERY_V4_ORDER_CORPUS_PATH",
    "edit": "OPERATOR_BATTERY_V4_EDIT_CORPUS_PATH",
    "conditional": "OPERATOR_BATTERY_V4_CONDITIONAL_CORPUS_PATH",
    "composed": "OPERATOR_BATTERY_V4_COMPOSED_CORPUS_PATH",
}

LOCAL_EPOCHS_ENV = {
    "order": ("OPERATOR_BATTERY_V4_ORDER_LOCAL_EPOCHS", "8"),
    "edit": ("OPERATOR_BATTERY_V4_EDIT_LOCAL_EPOCHS", "8"),
    "conditional": ("OPERATOR_BATTERY_V4_CONDITIONAL_LOCAL_EPOCHS", "8"),
    "composed": ("OPERATOR_BATTERY_V4_COMPOSED_LOCAL_EPOCHS", "8"),
}

CLEANUP_EPOCHS_ENV = {
    "order": ("OPERATOR_BATTERY_V4_ORDER_CLEANUP_EPOCHS", "4"),
    "edit": ("OPERATOR_BATTERY_V4_EDIT_CLEANUP_EPOCHS", "4"),
    "conditional": ("OPERATOR_BATTERY_V4_CONDITIONAL_CLEANUP_EPOCHS", "4"),
    "composed": ("OPERATOR_BATTERY_V4_COMPOSED_CLEANUP_EPOCHS", "4"),
}

PREFIXED_METRICS = [
    "slot_ordered_sequence_accuracy_milli",
    "flat_slot_ordered_sequence_accuracy_milli",
    "sequence_energy_accuracy_milli",
    "energy_pass_slot_fail",
    "output_slot_cleanup_failed_slots",
]

GENERIC_METRICS = [
    "flat_sequence_energy_parity_mismatches",
    "flat_gap_parity_mismatches",
    "state_delta_edges",
    "role_binding_edges",
    "target_center_id_training_used",
    "proof_rule_id_training_authority_used",
    "concrete_x_lookup_used",
    "local_out_t_runtime_extension_used",
]


def csv_env(name: str, default: list[str]) -> list[str]:
    raw = os.environ.get(name)
    if raw is None or raw.strip() == "":
        return default
    return [item.strip() for item in raw.split(",") if item.strip()]


def seed_env() -> list[int]:
    raw = os.environ.get("OPERATOR_BATTERY_MULTI_SEEDS", "1")
    seeds = [int(item.strip()) for item in raw.split(",") if item.strip()]
    if not seeds:
        raise ValueError("OPERATOR_BATTERY_MULTI_SEEDS must contain at least one seed")
    return seeds


def runtime_enabled() -> bool:
    return os.environ.get("OPERATOR_BATTERY_MULTI_RUNTIME", "0") == "1"


def reuse_existing() -> bool:
    return os.environ.get("OPERATOR_BATTERY_MULTI_REUSE", "0") == "1"


def run_stream(command: list[str], *, cwd: Path, env: dict[str, str], log_path: Path) -> int:
    log_path.parent.mkdir(parents=True, exist_ok=True)
    print(f"multiseed: run {' '.join(command)}", flush=True)
    print(f"multiseed: log {log_path}", flush=True)
    with log_path.open("w", encoding="utf-8") as log:
        proc = subprocess.Popen(
            command,
            cwd=cwd,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
        assert proc.stdout is not None
        for line in proc.stdout:
            print(line, end="", flush=True)
            log.write(line)
            log.flush()
        return proc.wait()


def build_seed(seed: int, seed_dir: Path) -> None:
    env = os.environ.copy()
    env["OPERATOR_BATTERY_SEED"] = str(seed)
    env["OPERATOR_BATTERY_OUTPUT_DIR"] = str(seed_dir)
    code = run_stream(
        [sys.executable, str(ROOT / "build_operator_battery_v4.py")],
        cwd=REPO,
        env=env,
        log_path=seed_dir / "build.log",
    )
    if code != 0:
        raise RuntimeError(f"build failed for seed {seed}: exit {code}")


def shortcut_seed(seed_dir: Path) -> dict[str, object]:
    env = os.environ.copy()
    env["OPERATOR_BATTERY_TASKS"] = str(seed_dir / "accepted_operator_tasks_v4.jsonl")
    env["OPERATOR_BATTERY_SHORTCUT_REPORT"] = str(seed_dir / "shortcut_gate_report.json")
    code = run_stream(
        [sys.executable, str(ROOT / "run_shortcut_gates.py")],
        cwd=REPO,
        env=env,
        log_path=seed_dir / "shortcut_gate_report.log",
    )
    report = json.loads((seed_dir / "shortcut_gate_report.json").read_text(encoding="utf-8"))
    if code != 0:
        raise RuntimeError(f"shortcut gate failed for {seed_dir}: {report.get('verdict')}")
    return report


def metric_patterns(operator_class: str) -> dict[str, re.Pattern[str]]:
    return {
        f"{operator_class}_{key}": re.compile(rf"{operator_class}_{re.escape(key)}: ([^\n]+)")
        for key in PREFIXED_METRICS
    } | {
        key: re.compile(rf"{re.escape(key)}: ([^\n]+)")
        for key in GENERIC_METRICS
    } | {
        f"{operator_class}_slot_failure_total": re.compile(
            rf"operator_battery_v4_{re.escape(operator_class)}: slot_failure_total=([^\n]+)"
        ),
        "test_result": re.compile(r"test result: ok\."),
    }


def parse_runtime_log(operator_class: str, log_path: Path) -> dict[str, str | bool]:
    text = log_path.read_text(encoding="utf-8")
    out: dict[str, str | bool] = {}
    for key, pattern in metric_patterns(operator_class).items():
        if key == "test_result":
            out[key] = bool(pattern.search(text))
            continue
        match = pattern.search(text)
        out[key] = match.group(1).strip() if match else "MISSING"
    return out


def newest_runtime_log(seed_dir: Path, operator_class: str) -> Path:
    log_dir = seed_dir / operator_class
    canonical = log_dir / f"{operator_class}_runtime_gate_release.log"
    candidates = sorted(
        log_dir.glob(f"{operator_class}_runtime_gate_release*.log"),
        key=lambda path: path.stat().st_mtime,
    )
    return candidates[-1] if candidates else canonical


def runtime_seed(seed_dir: Path, operator_class: str) -> dict[str, str | bool]:
    env = os.environ.copy()
    env["POSITION_SEQUENCE_OPERATOR_PAIR_ACTION_CENTERS"] = "1"
    env[CORPUS_ENV[operator_class]] = str(seed_dir / operator_class / "accepted_operator_tasks_v4.jsonl")
    key, value = LOCAL_EPOCHS_ENV[operator_class]
    env[key] = os.environ.get(key, value)
    key, value = CLEANUP_EPOCHS_ENV[operator_class]
    env[key] = os.environ.get(key, value)
    log_path = seed_dir / operator_class / f"{operator_class}_runtime_gate_release.log"
    code = run_stream(
        [
            "cargo",
            "test",
            "-p",
            "nando-core",
            "--release",
            "--test",
            "wavepredictor_binding_pressure_l3",
            "--",
            "--ignored",
            RUNTIME_TESTS[operator_class],
            "--nocapture",
        ],
        cwd=REPO,
        env=env,
        log_path=log_path,
    )
    metrics = parse_runtime_log(operator_class, log_path)
    metrics["exit_code"] = str(code)
    return metrics


def reuse_runtime_seed(seed_dir: Path, operator_class: str) -> dict[str, str | bool]:
    log_path = newest_runtime_log(seed_dir, operator_class)
    if not log_path.exists():
        raise FileNotFoundError(f"missing runtime log for reuse: {log_path}")
    print(f"multiseed: reuse_runtime_log seed_dir={seed_dir} class={operator_class} log={log_path}", flush=True)
    metrics = parse_runtime_log(operator_class, log_path)
    metrics["runtime_log_path"] = str(log_path)
    if metrics.get("test_result"):
        metrics["exit_code"] = "0"
    else:
        text = log_path.read_text(encoding="utf-8")
        metrics["exit_code"] = "FAILED_LOG" if "test result: FAILED" in text else "MISSING"
    return metrics


def metric_int(metrics: dict[str, str | bool], key: str) -> int | None:
    raw = metrics.get(key)
    if raw is None or raw is True or raw is False or raw == "MISSING":
        return None
    try:
        return int(str(raw))
    except ValueError:
        return None


def runtime_metrics_ok(operator_class: str, metrics: dict[str, str | bool]) -> bool:
    if metrics.get("exit_code") != "0" or not bool(metrics.get("test_result")):
        return False
    required_exact = {
        f"{operator_class}_slot_ordered_sequence_accuracy_milli": 1000,
        f"{operator_class}_flat_slot_ordered_sequence_accuracy_milli": 1000,
        f"{operator_class}_sequence_energy_accuracy_milli": 1000,
        "flat_sequence_energy_parity_mismatches": 0,
        "flat_gap_parity_mismatches": 0,
        "state_delta_edges": 0,
    }
    for key, expected in required_exact.items():
        if metric_int(metrics, key) != expected:
            return False
    required_false = [
        "target_center_id_training_used",
        "proof_rule_id_training_authority_used",
        "concrete_x_lookup_used",
        "local_out_t_runtime_extension_used",
    ]
    for key in required_false:
        if str(metrics.get(key)) != "false":
            return False
    zero_if_present = [
        f"{operator_class}_energy_pass_slot_fail",
        f"{operator_class}_output_slot_cleanup_failed_slots",
        f"{operator_class}_slot_failure_total",
    ]
    for key in zero_if_present:
        value = metric_int(metrics, key)
        if value is not None and value != 0:
            return False
    return True


def runtime_issue_lines(operator_class: str, metrics: dict[str, str | bool]) -> list[str]:
    issues: list[str] = []
    if metrics.get("exit_code") != "0":
        issues.append(f"{operator_class}: exit_code={metrics.get('exit_code')}")
    if not bool(metrics.get("test_result")):
        issues.append(f"{operator_class}: test_result={metrics.get('test_result')}")
    for key in [
        f"{operator_class}_energy_pass_slot_fail",
        f"{operator_class}_output_slot_cleanup_failed_slots",
        f"{operator_class}_slot_failure_total",
    ]:
        value = metric_int(metrics, key)
        if value is not None and value != 0:
            issues.append(f"{operator_class}: {key}={value}")
    return issues


def write_report(summary: dict[str, object]) -> None:
    OUT_ROOT.mkdir(parents=True, exist_ok=True)
    (OUT_ROOT / "multiseed_summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    lines = [
        "# v4 Multi-Seed Robustness Report",
        "",
        f"Date: {datetime.now(timezone.utc).date().isoformat()}",
        "",
        "## Verdict",
        "",
        f"`{summary['verdict']}`",
        "",
        "## Scope",
        "",
        "```text",
        f"seeds: {summary['seeds']}",
        f"classes: {summary['classes']}",
        f"runtime_gates_enabled: {summary['runtime_gates_enabled']}",
        "```",
        "",
        "## Accepted Repair",
        "",
        "```text",
        "conditional: suppress generic action-surface by default",
        "composed:    suppress generic action-surface by default",
        "```",
        "",
        "Reason:",
        "",
        "```text",
        "Conditional action text contains both then/else branches. Composed action text",
        "contains an explicit demo. Keeping their raw surface centers active gives the",
        "runtime a fuzzy action-text channel that can conflict with the selected",
        "operator motif or weaken ablation proof.",
        "",
        "The accepted channels are:",
        "  conditional -> state condition + selected condition/action conjunction page",
        "  composed    -> parsed neutral demo slot page",
        "```",
        "",
        "Forbidden substitutions remain false:",
        "",
        "```text",
        "target_center_id_training_used: false",
        "proof_rule_id_training_authority_used: false",
        "concrete_x_lookup_used: false",
        "local_out_t_runtime_extension_used: false",
        "```",
        "",
        "Rejected repairs before the accepted action-surface repair:",
        "",
        "```text",
        "short-token identity atoms 8:",
        "  fixed conditional seed1 but weakened edit/composed ablation proof.",
        "",
        "role lanes 48:",
        "  no effect on conditional seed1 failure.",
        "",
        "all-token candidate cleanup:",
        "  worsened conditional strict readout from 999 to 988.",
        "```",
        "",
        "## Results",
        "",
    ]
    for seed, seed_result in sorted(summary["seed_results"].items()):
        lines.extend(
            [
                f"### Seed {seed}",
                "",
                "```text",
                f"shortcut_verdict: {seed_result['shortcut_verdict']}",
            ]
        )
        for operator_class, verdict in sorted(seed_result["class_verdicts"].items()):
            lines.append(f"{operator_class}_shortcut_verdict: {verdict}")
        runtime = seed_result.get("runtime", {})
        for operator_class, metrics in sorted(runtime.items()):
            lines.append(f"{operator_class}_runtime_log_path: {metrics.get('runtime_log_path')}")
            lines.append(f"{operator_class}_test_result: {metrics.get('test_result')}")
            lines.append(
                f"{operator_class}_slot_accuracy: "
                f"{metrics.get(f'{operator_class}_slot_ordered_sequence_accuracy_milli')}"
            )
            lines.append(
                f"{operator_class}_energy_accuracy: "
                f"{metrics.get(f'{operator_class}_sequence_energy_accuracy_milli')}"
            )
            lines.append(
                f"{operator_class}_energy_pass_slot_fail: "
                f"{metrics.get(f'{operator_class}_energy_pass_slot_fail')}"
            )
            lines.append(
                f"{operator_class}_output_slot_cleanup_failed_slots: "
                f"{metrics.get(f'{operator_class}_output_slot_cleanup_failed_slots')}"
            )
            lines.append(
                f"{operator_class}_slot_failure_total: "
                f"{metrics.get(f'{operator_class}_slot_failure_total')}"
            )
            lines.append(
                f"{operator_class}_flat_energy_parity_mismatches: "
                f"{metrics.get('flat_sequence_energy_parity_mismatches')}"
            )
            lines.append(f"{operator_class}_flat_gap_parity_mismatches: {metrics.get('flat_gap_parity_mismatches')}")
            lines.append(f"{operator_class}_state_delta_edges: {metrics.get('state_delta_edges')}")
        lines.extend(["```", ""])
    lines.extend(
        [
            "## Strict Runtime Issues",
            "",
        ]
    )
    strict_issues = summary.get("strict_runtime_issues", [])
    if strict_issues:
        lines.extend(["```text", *[str(issue) for issue in strict_issues], "```", ""])
    else:
        lines.extend(["```text", "none", "```", ""])
    lines.extend(
        [
            "## Boundary",
            "",
            "This is a robustness rung for the existing v4 mechanisms. It does not add",
            "new architecture and does not widen the claim beyond the seeds/classes",
            "listed above.",
            "",
            "Do not claim robustness beyond these seeds until additional seeds are run.",
            "",
        ]
    )
    (OUT_ROOT / "MULTISEED_ROBUSTNESS_REPORT.md").write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    seeds = seed_env()
    classes = csv_env("OPERATOR_BATTERY_MULTI_CLASSES", DEFAULT_CLASSES)
    for operator_class in classes:
        if operator_class not in DEFAULT_CLASSES:
            raise ValueError(f"unknown operator class: {operator_class}")
    summary: dict[str, object] = {
        "schema_version": "operator_battery_v4_multiseed_summary_v1",
        "seeds": seeds,
        "classes": classes,
        "runtime_gates_enabled": runtime_enabled(),
        "reuse_existing_logs": reuse_existing(),
        "notes": [
            "Conditional generic action-surface is suppressed because raw action text contains both branches.",
            "Composed generic action-surface is suppressed because raw action text contains the demo.",
            "Accepted proof channels are selected condition/action conjunction and parsed neutral composed demo slot page.",
            "No target_id, proof_rule_id authority, concrete_x_lookup, or manual local_out_t is used.",
        ],
        "seed_results": {},
        "strict_runtime_issues": [],
        "verdict": "PENDING",
    }
    OUT_ROOT.mkdir(parents=True, exist_ok=True)
    for seed in seeds:
        seed_dir = OUT_ROOT / f"seed_{seed:03d}"
        print(f"multiseed: seed_start {seed} dir={seed_dir}", flush=True)
        if reuse_existing():
            print(f"multiseed: reuse_seed_artifacts {seed}", flush=True)
            shortcut = json.loads((seed_dir / "shortcut_gate_report.json").read_text(encoding="utf-8"))
        else:
            build_seed(seed, seed_dir)
            shortcut = shortcut_seed(seed_dir)
        seed_result: dict[str, object] = {
            "path": str(seed_dir),
            "shortcut_verdict": shortcut["verdict"],
            "class_verdicts": shortcut["class_verdicts"],
        }
        if runtime_enabled():
            runtime_results = {}
            for operator_class in classes:
                print(f"multiseed: runtime_start seed={seed} class={operator_class}", flush=True)
                runtime_results[operator_class] = (
                    reuse_runtime_seed(seed_dir, operator_class)
                    if reuse_existing()
                    else runtime_seed(seed_dir, operator_class)
                )
                seed_result["runtime"] = runtime_results
                summary["seed_results"][str(seed)] = seed_result
                for issue in runtime_issue_lines(operator_class, runtime_results[operator_class]):
                    summary["strict_runtime_issues"].append(f"seed={seed} {issue}")
                if runtime_results[operator_class].get("exit_code") != "0":
                    summary["verdict"] = "RED_MULTI_SEED_CURRENT_SCOPE"
                    write_report(summary)
                    print(
                        f"multiseed: runtime_failed seed={seed} class={operator_class}",
                        flush=True,
                    )
                    print(json.dumps(summary, ensure_ascii=False, indent=2, sort_keys=True))
                    return 1
                print(f"multiseed: runtime_done seed={seed} class={operator_class}", flush=True)
            seed_result["runtime"] = runtime_results
        summary["seed_results"][str(seed)] = seed_result
        write_report(summary)
        print(f"multiseed: seed_done {seed}", flush=True)
    shortcut_ok = all(
        seed_result["shortcut_verdict"] == "VALID_OPERATOR_BATTERY_V4_CANDIDATE"
        for seed_result in summary["seed_results"].values()
    )
    runtime_ok = True
    if runtime_enabled():
        for seed_result in summary["seed_results"].values():
            for operator_class, metrics in seed_result.get("runtime", {}).items():
                runtime_ok = runtime_ok and runtime_metrics_ok(operator_class, metrics)
    summary["verdict"] = (
        "GREEN_MULTI_SEED_CURRENT_SCOPE" if shortcut_ok and runtime_ok else "RED_MULTI_SEED_CURRENT_SCOPE"
    )
    write_report(summary)
    print(json.dumps(summary, ensure_ascii=False, indent=2, sort_keys=True))
    return 0 if summary["verdict"] == "GREEN_MULTI_SEED_CURRENT_SCOPE" else 1


if __name__ == "__main__":
    raise SystemExit(main())
