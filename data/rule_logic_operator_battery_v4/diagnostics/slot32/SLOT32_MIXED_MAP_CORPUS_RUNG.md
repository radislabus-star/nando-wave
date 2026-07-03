# Slot32 Mixed Map Corpus Rung

Date: 2026-07-03

This is the next 32-slot rung after the order-only multi-seed proof. It keeps
the same paged `u32` layout and the same flat role-binding runtime path, but
adds transfer pressure beyond pure permutations:

```text
operator classes:
  order
  edit-map
  composed-map
```

The edit-map class covers copy/drop/duplicate-style role transfer within the
current slot-map runtime. It does not claim insertion of unseen constants or
free-form text generation.

Conditional branch selection is still open. This rung deliberately does not
pretend that a chosen slot map is a learned branch selector.

## Command

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_mixed_map_corpus_must_transfer_without_lookup_or_runtime_phase_hack --nocapture
```

Saved log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_mixed_map_corpus_rung_release.log
```

## Corpus Pressure

```text
seed: 0
train_rows: 2048
heldout_rows: 2048
unique_operator_classes: 3
unique_rules: 16
unique_surfaces: 4
unique_noise_types: 2
unique_lengths: 16
lengths: 17..32
same_bag_rows: 1536
edit_rows: 512
edit_non_same_bag_rows: 512
max_train_state_reuse: 16
max_heldout_state_reuse: 16
train_tokens_overlap_heldout: 0
```

Important boundary:

```text
Tokens are independent of rule_name.
The same state key is reused across 16 different rules.
Heldout token surfaces are disjoint from train token surfaces.
Order/composed rows use same-bag pressure.
Edit-map rows are deliberately non-same-bag because copy/drop/duplicate changes
the token bag.
```

## Result

```text
verdict: SLOT32_MIXED_MAP_CORPUS_RUNG_PASS

page_count: 64
total_center_count: 262144
output_slot_count: 32
role_slot_count: 32
role_top_l1_lanes: 64

slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
sequence_energy_median_gap: 4197248
sequence_energy_p10_gap: 3106560
energy_pass_slot_fail: 0
flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_mismatches: 0
flat_failed_rows: 0

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0

state_delta_edges: 0
role_binding_edges: 1492
flat_role_binding_bytes_estimate: 83448
base_mass_bytes_estimate: 524288
hot_bytes_estimate: 607736
flat_eval_rows: 2048
flat_eval_avg_ns_per_row: 219009
flat_eval_latency_gate_ns: 1000000

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

## Claim Boundary

This closes:

```text
32-slot mixed map corpus single-seed rung;
lengths 17..32;
order + edit-map + composed-map transfer;
16 reusable map rules;
4 surface families;
2 noise/token families;
field/flat parity;
binding/action/role/active ablation collapse;
sub-4MiB hot table estimate.
```

This does not close:

```text
32-slot conditional branch selection;
32-slot mixed-map multi-seed robustness;
full 32-slot operator battery;
insert-new-constant edit operators;
packed product runtime proof;
product p99 latency proof;
64-slot capacity;
broad product reasoning;
autonomous raw action parsing;
text generation.
```
