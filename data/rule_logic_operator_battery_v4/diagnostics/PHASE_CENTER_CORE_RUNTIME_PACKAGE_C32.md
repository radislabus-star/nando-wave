# Phase-Center Core Runtime Package C32

Date: 2026-07-02

## Verdict

`PHASE_CENTER_CORE_RUNTIME_PACKAGE_PASS`

## What Changed

The C32 phase-center scorer can now be packaged as deterministic runtime bytes:

```text
nando_core::PhaseCenterCompiler
  -> nando_core::PhaseCenterFlatRuntime
  -> PhaseCenterFlatRuntime::to_bytes()
  -> PhaseCenterFlatRuntime::from_bytes()
  -> scoring
```

The package stores only flat positive/negative phase-center records. It does
not store corpus rows, task ids, source groups, answers, proof rule ids, or
manual output timing.

## Command

```bash
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_phase_center_core_runtime_package_report --nocapture
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
package_magic: [78, 87, 80, 67, 70, 48, 48, 49]
inspected_cells: 32
inspected_records: 380
inspected_payload_bytes: 389120
package_fingerprint64: 14549306353473335964
package_bytes: 389136
serialized_len: 389136
core_runtime_bytes_estimate: 401280
package_accuracy_milli: 1000
package_wrong_wins: 0
package_sign_parity_mismatches: 0
package_margin_parity_mismatches: 0
package_eval_p50_latency_ns: 64
package_eval_p99_latency_ns: 407
compiler_path: nando_core::PhaseCenterCompiler
runtime_path: nando_core::PhaseCenterFlatRuntime::from_bytes
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

This proves that the current v4 C32 phase-center scorer survives a real binary
runtime package roundtrip and preserves parity with the proof compiler path.

It does not prove:

```text
full strict ordered decoder
text generation
multi-step reasoning
conditional train_per_cell=2 strict readout
```

The package is a compact scorer/energy artifact. The generator/readout debt is
still separate.
