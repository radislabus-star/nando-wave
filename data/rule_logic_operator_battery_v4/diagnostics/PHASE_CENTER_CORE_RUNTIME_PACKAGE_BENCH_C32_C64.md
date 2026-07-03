# Phase-Center Core Runtime Package Bench C32/C64

Date: 2026-07-02

## Verdict

`PHASE_CENTER_CORE_RUNTIME_PACKAGE_BENCH_PASS`

## Scope

This is a release benchmark for serialized phase-center scorer packages:

```text
nando_core::PhaseCenterCompiler
  -> PhaseCenterFlatRuntime::to_bytes()
  -> PhaseCenterFlatRuntime::from_bytes()
  -> loaded scorer evaluation
```

The package contains only flat positive/negative phase-center records. It does
not contain task rows, answers, source groups, proof rule ids, concrete lookup,
or manual output timing.

## Command

```bash
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_phase_center_core_runtime_package_benchmark_report --nocapture
```

## Release Metrics

```text
rows: 10624
heldout_rows: 5312
flat_records: 380
package_magic: [78, 87, 80, 67, 70, 48, 48, 49]
```

C32:

```text
package_bytes: 389136
inspected_cells: 32
inspected_records: 380
inspected_payload_bytes: 389120
package_fingerprint64: 14549306353473335964
serialized_len: 389136
core_runtime_bytes_estimate: 401280
package_load_us: 787
accuracy_milli: 1000
wrong_wins: 0
median_margin: 0.767109
p10_margin: 0.312965
package_margin_parity_mismatches: 0
package_sign_parity_mismatches: 0
p50_latency_ns: 69
p99_latency_ns: 416
total_eval_us: 720
```

C64:

```text
package_bytes: 778256
inspected_cells: 64
inspected_records: 380
inspected_payload_bytes: 778240
package_fingerprint64: 16888657547359761052
serialized_len: 778256
core_runtime_bytes_estimate: 790400
package_load_us: 1646
accuracy_milli: 1000
wrong_wins: 0
median_margin: 0.848736
p10_margin: 0.416739
package_margin_parity_mismatches: 0
package_sign_parity_mismatches: 0
p50_latency_ns: 163
p99_latency_ns: 520
total_eval_us: 1183
```

Forbidden substitutions:

```text
epoch_repair_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

## Interpretation

C32 remains the compact runtime package point for the current v4 scorer:

```text
C32 package: 389136 bytes, p99 416 ns, load 787 us
C64 package: 778256 bytes, p99 520 ns, load 1646 us
```

C64 buys more margin reserve, but nearly doubles package bytes and is slower in
this scorer benchmark.

Latency and load numbers are single-run release samples. The proof invariants
are package bytes, accuracy, wrong-wins, parity mismatches, and forbidden flags.

## Boundary

This proves a portable binary package and release benchmark for the
phase-center scorer kernel.

It does not prove:

```text
full strict ordered decoder
text generation
multi-step reasoning
conditional train_per_cell=2 strict readout
```
