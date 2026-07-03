# Phase-Center Core Compiler C32

Date: 2026-07-02

## Verdict

`PHASE_CENTER_CORE_COMPILER_PASS`

## What Changed

The C32 phase-center operator path is now available through exported
`nando-core` compiler and runtime APIs:

```text
nando_core::PhaseCenterCompiler
nando_core::PhaseCenterFlatRuntime
```

This proves the runtime can be built from relation/action atoms through the
core compiler surface, not only by converting test-local flat records.

## Command

```bash
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_phase_center_core_compiler_report --nocapture
```

## Release Metrics

```text
rows: 10624
heldout_rows: 5312
cells: 32
flat_records: 380
skipped_train_rows: 0
core_skipped_train_rows: 0
missing_centers: 0
skipped_rows: 0
core_accuracy_milli: 1000
core_wrong_wins: 0
core_sign_parity_mismatches: 0
core_margin_parity_mismatches: 0
core_eval_p50_latency_ns: 64
core_eval_p99_latency_ns: 402
core_runtime_bytes_estimate: 401280
core_eval_total_us: 621
compiler_path: nando_core::PhaseCenterCompiler
runtime_path: nando_core::PhaseCenterFlatRuntime
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

This proves the exported phase-center compiler can build the same C32 operator
scorer as the previous test-local compiler path.

Latency values are single-run release samples. The proof invariants are:
accuracy, wrong-wins, parity mismatches, bytes, and forbidden-substitution
flags.

It does not prove:

```text
full strict ordered decoder
text generation
multi-step reasoning
multi-seed strict robustness
```
