# Slot32 Paged Layout Capacity Smoke

Date: 2026-07-02

This is a Rust-only capacity smoke for the next strict slot rung. It is not a
full 32-slot corpus proof and not a product latency proof.

## Command

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_paged_layout_capacity_smoke --nocapture
```

Saved log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_paged_layout_capacity_smoke_release.log
```

## Layout

```text
CenterId: u32
page_bits: 12
page_size: 4096
page_count: 64
total_center_count: 262144

role pages: 0..31
action_surface_page: 32
operator_pair_page: 33
operator_pair_source_bits: 5

output_slot_count: 32
role_slot_count: 32
role_top_l1_lanes: 64
```

Operator-pair address:

```text
operator_pair(out, src) = (33 << 12) | ((out << 5) | src)
```

## First Red Finding

With `role_top_l1_lanes = 32`, the same smoke was red:

```text
slot_accuracy_milli: 797
flat_slot_accuracy_milli: 797
sequence_energy_accuracy_milli: 1000
energy_pass_slot_fail: 13
flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_mismatches: 0
flat_failed_rows: 13
flat_failed_by_length: {20: 1, 22: 3, 24: 2, 27: 1, 29: 1, 30: 1, 31: 1, 32: 3}
flat_failed_by_rule: {"mirror": 2, "pair_swap": 6, "rotate_left": 2, "rotate_right": 3}
```

Interpretation:

```text
The 32-slot address layout was not the failure. The operator energy was already
correct, but strict slot readout was under-resolved. This is a role-lane recall
pressure problem, not a lookup/parity problem.
```

## Current Green Smoke

With `role_top_l1_lanes = 64`:

```text
verdict: SLOT32_PAGED_LAYOUT_CAPACITY_SMOKE_PASS
lengths: 17..32
train_rows: 64
heldout_rows: 64

slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
sequence_energy_median_gap: 2869504
sequence_energy_p10_gap: 593664
energy_pass_slot_fail: 0

flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_mismatches: 0
flat_sequence_energy_parity_max_abs_gap_delta: 0
flat_failed_rows: 0
flat_failed_by_length: {}
flat_failed_by_rule: {}

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0

touched_role_binding_edges: 15812
role_binding_edges: 892
flat_role_binding_edges: 892
flat_role_binding_bytes_estimate: 76248
base_mass_bytes_estimate: 524288
hot_bytes_estimate: 600536

flat_eval_rows: 64
flat_eval_total_ns: 9434598
flat_eval_avg_ns_per_row: 147415

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Engineering note:

```text
The flat readout path now prepares role strengths once per sequence row and
uses slot-scoped action grouping. This keeps field/flat parity at zero while
avoiding a full active-action scan for every output slot.

This timing is a smoke-path measurement, not a standalone p99 latency claim.
```

## Multi-Seed Smoke

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_paged_layout_multiseed_capacity_smoke --nocapture
```

Saved log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_paged_layout_multiseed_capacity_smoke_release.log
```

Result:

```text
verdict: SLOT32_PAGED_LAYOUT_MULTI_SEED_CAPACITY_SMOKE_PASS
seeds: 3
page_count: 64
total_center_count: 262144
output_slot_count: 32
role_slot_count: 32
role_top_l1_lanes: 64
lengths: 17..32

min_slot_accuracy_milli: 1000
min_flat_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
min_sequence_energy_p10_gap: 593664
total_energy_pass_slot_fail: 0
total_flat_gap_parity_mismatches: 0
total_flat_sequence_energy_parity_mismatches: 0
max_hot_bytes_estimate: 600536
max_flat_eval_avg_ns_per_row: 150392

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

## Flat Runtime Latency Smoke

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_flat_runtime_latency_smoke --nocapture
```

Saved log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_flat_runtime_latency_smoke_release.log
```

Result:

```text
verdict: SLOT32_FLAT_RUNTIME_LATENCY_SMOKE_PASS
seed: 0
lengths: 17..32
bench_repeats: 256
measured_rows: 16384
correct_rows: 16384
flat_accuracy_milli: 1000
p50_latency_ns: 135476
p99_latency_ns: 245822
max_latency_ns: 653733
avg_latency_ns: 144066
latency_gate_ns: 1000000
flat_role_binding_edges: 892
hot_bytes_estimate: 600536

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Boundary:

```text
This is a flat runtime latency smoke for the synthetic 32-slot capacity rung.
It is not a product p99 proof and not evidence for a full 32-slot corpus.
```

## Claim Boundary

This closes only:

```text
32-slot paged u32 layout smoke:
  64 pages;
  32 role slots;
  32 output slots;
  5-bit operator-pair packing;
  field/flat parity;
  binding/action/role/active ablation collapse;
  same-bag heldout transfer on synthetic order maps length 17..32;
  three seed-varied token layouts for the same capacity smoke;
  one seed0 flat runtime p50/p99 smoke over 16 384 row evaluations.
```

This does not close:

```text
full 32-slot corpus battery;
full 32-slot multi-seed corpus robustness;
packed product runtime proof;
product p99 latency proof;
64-slot capacity;
broad product reasoning;
autonomous raw action parsing;
text generation.
```

Next proof-debt:

```text
1. Build a real 32-slot corpus battery rather than this synthetic smoke.
2. Add product-grade packed/cache runtime proof for the 32-slot flat readout path.
3. Keep role_top_l1_lanes as an explicit capacity knob, not a hidden default.
```
