# Phase Center Runtime Probe C32

Date: 2026-07-02

## Question

Can the phase-center operator signal be reproduced inside the Rust proof
runtime, not only in the Python diagnostic?

## Command

```text
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_runtime_probe_report --nocapture
```

## Method

Rust test:

```text
operator_battery_v4_phase_center_runtime_probe_report
```

The test uses the same mechanism as the Python C32 probe:

```text
transition relation atoms
-> BLAKE2b digest8 with person=nwphase
-> circular phase cells
-> correct phase center and wrong anti-center
-> heldout correct-vs-wrong coherence margin
```

No epoch repair is used.

## Result

```text
verdict: PHASE_CENTER_RUNTIME_PROBE_PASS
cells: 32
action_compiled_phase_centers: 380
action_train_rows: 5312
action_heldout_rows: 5312
action_heldout_surface_groups: 4
action_heldout_noise_groups: 4
action_heldout_accuracy_milli: 1000
action_wrong_wins: 0
action_median_margin: 0.767109
action_p10_margin: 0.312965
action_median_positive_center_gap: 0.537150
action_p10_positive_center_gap: 0.242992
no_action_compiled_phase_centers: 40
no_action_heldout_accuracy_milli: 782
no_action_wrong_wins: 1156
phase_center_bytes_estimate: 389120
epoch_repair_used: false
explicit_out_src_program_extraction_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
diagnostic_only: true
```

## Interpretation

This closes the Python-only weakness of the phase-center diagnostic.

Allowed claim:

```text
The v4 corpus has a zero-epoch compact phase-center operator signal, and the
signal is reproduced in the Rust proof runtime at C32.
```

Still not allowed:

```text
production flat CPU operator runtime solved
strict ordered decoder solved for every v4/v5 family
semantic grokking claim
```

## Next Proof Debt

```text
1. C8/C16/C32/C64 capacity curve.
2. Phase ablation: remove dominant phase components and require predictable drop.
3. Compile phase centers into the final flat runtime representation.
4. Keep epoch/error-driven repair as fallback only.
```
