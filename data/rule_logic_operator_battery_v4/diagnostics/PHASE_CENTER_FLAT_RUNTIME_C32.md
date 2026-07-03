# Phase Center Flat Runtime C32

Date: 2026-07-02

## Question

Can the C32 phase-center compiler be represented as flat CPU runtime records
and produce the same heldout decisions as the compiler/field path?

## Command

Release command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_flat_runtime_report --nocapture
```

Debug command also passed:

```text
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_flat_runtime_report --nocapture
```

## Method

Rust test:

```text
operator_battery_v4_phase_center_flat_runtime_report
```

Compiler stage:

```text
train relation waves
-> C32 phase centers
-> 380 flat records
```

Runtime stage:

```text
precompiled numeric center_index
+ correct/wrong transition vectors
+ flat phase-center records
-> score margin
```

No string key lookup is used inside the scoring loop.
No epoch repair is used.

## Release Result

```text
verdict: PHASE_CENTER_FLAT_RUNTIME_PASS
cells: 32
compiler_accuracy_milli: 1000
compiler_wrong_wins: 0
flat_accuracy_milli: 1000
flat_rows: 5312
flat_correct: 5312
flat_wrong_wins: 0
flat_median_margin: 0.767109
flat_p10_margin: 0.312965
flat_sign_parity_mismatches: 0
flat_margin_parity_mismatches: 0
no_action_flat_accuracy_milli: 782
no_action_flat_wrong_wins: 1156
missing_centers: 0
skipped_rows: 0
heldout_surface_groups: 4
heldout_noise_groups: 4
flat_records: 380
flat_runtime_bytes_estimate: 407360
flat_eval_p50_latency_ns: 136
flat_eval_p99_latency_ns: 506
flat_eval_total_us: 1032
runtime_path: precompiled_numeric_center_index_plus_flat_records
epoch_repair_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

## Debug Result

```text
verdict: PHASE_CENTER_FLAT_RUNTIME_PASS
flat_eval_p50_latency_ns: 2790
flat_eval_p99_latency_ns: 4439
flat_eval_total_us: 15473
```

## Interpretation

Allowed claim:

```text
The v4 C32 phase-center operator scorer has a flat Rust CPU-runtime path with
zero compiler/flat parity mismatches and zero heldout wrong wins.
```

Still not allowed:

```text
full strict ordered decoder solved
full text generation solved
multi-step LLMWave product solved
```

## Next Proof Debt

```text
1. Integrate flat phase-center scorer with strict slot/readout path.
2. Add operator-class latency breakdown.
3. Add packed/fixed-width representation if needed after the next red gate.
4. Keep epoch repair as fallback only.
```
