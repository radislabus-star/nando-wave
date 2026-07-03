# Phase-Center Core Runtime Bench C32/C64

Date: 2026-07-02

## Verdict

`PHASE_CENTER_CORE_RUNTIME_BENCH_PASS`

## Command

```bash
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_phase_center_core_runtime_benchmark_report --nocapture
```

## Runtime Path

```text
nando_core::PhaseCenterFlatRuntime
```

No Python scorer is used in the runtime loop.

## Results

```text
rows: 5312 heldout transitions
flat_records: 380
epoch_repair_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

| cells | accuracy_milli | wrong_wins | median_margin | p10_margin | p50_latency_ns | p99_latency_ns | total_eval_us | bytes_estimate |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 32 | 1000 | 0 | 0.767109 | 0.312965 | 65 | 375 | 636 | 401280 |
| 64 | 1000 | 0 | 0.848736 | 0.416739 | 190 | 651 | 1416 | 790400 |

## Interpretation

C32 remains the compact runtime point:

```text
zero wrong wins
sub-microsecond p99 scorer latency
about half the C64 memory
```

C64 buys margin reserve at roughly double bytes and higher latency:

```text
p10 margin: 0.416739 vs 0.312965
bytes: 790400 vs 401280
p99: 651 ns vs 375 ns
```

## Boundary

This benchmark covers the exported phase-center scorer only. It does not close
strict ordered decoding, text generation, or multi-seed strict robustness.

Latency values are single-run release samples. The stable comparison points are
zero wrong wins, C32 vs C64 margin reserve, and bytes estimate.
