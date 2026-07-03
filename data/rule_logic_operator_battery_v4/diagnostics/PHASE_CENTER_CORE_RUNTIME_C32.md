# Phase-Center Core Runtime C32

Date: 2026-07-02

## Verdict

`PHASE_CENTER_CORE_RUNTIME_PASS`

## What Changed

The C32 phase-center scorer is now available as exported `nando-core` runtime
API:

```text
nando_core::PhaseCenterFlatRuntime
nando_core::PhaseCenterFlatRecord
nando_core::PhaseCenterEvalTask
nando_core::PhaseCenterCell
```

This moves the proof path from test-only helper structs toward a reusable Rust
runtime surface.

## Command

```bash
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_phase_center_core_runtime_report --nocapture
```

## Release Metrics

```text
rows: 10624
heldout_rows: 5312
cells: 32
flat_records: 380
skipped_train_rows: 0
missing_centers: 0
skipped_rows: 0
core_accuracy_milli: 1000
core_wrong_wins: 0
core_sign_parity_mismatches: 0
core_margin_parity_mismatches: 0
core_eval_p50_latency_ns: 69
core_eval_p99_latency_ns: 400
core_runtime_bytes_estimate: 401280
core_eval_total_us: 671
runtime_path: nando_core::PhaseCenterFlatRuntime
eval_path: precompiled_core_tasks_no_bridge_allocations
```

Forbidden substitutions:

```text
epoch_repair_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

## Boundary

This proves the exported flat phase-center runtime scorer matches the existing
compiler/test path on the current v4 C32 operator battery.

It does not prove:

```text
full strict ordered decoder
text generation
multi-step reasoning
multi-seed strict robustness
conditional train_per_cell=2 strict readout
```

The current train_per_cell=2 robustness rung remains red on conditional strict
slot readout.

Latency scope:

```text
The p50/p99 and total_us numbers measure the exported core scorer over
precompiled core eval tasks. They do not include raw text/vector construction,
strict slot decoding, or multi-step generation.
```

## Structural Gate

Worksheet:

```text
data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_STRUCTURAL_GATE.md
```

Command:

```bash
/home/ubu/.codex/skills/nanda-structural-gate/scripts/nanda-gate-md \
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_STRUCTURAL_GATE.md \
  --task-id phase-center-core-runtime --domain code --format json
```

Result:

```text
verdict: PASS
stable_triads: 8
weak_triads: 0
conflicts: 0
agent_decision.safe_to_edit: true
```

Checked boundary:

```text
core runtime PASS remains separate from train_per_cell=2 conditional RED.
```
