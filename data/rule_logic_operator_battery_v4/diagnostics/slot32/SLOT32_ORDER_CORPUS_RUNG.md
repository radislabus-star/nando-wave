# Slot32 Order Corpus Rung

Date: 2026-07-03

This is the first real 32-slot order corpus rung after the synthetic slot32
capacity smoke. It keeps the same paged `u32` layout and flat runtime path, but
raises the corpus pressure from 64 synthetic rows to a matrix of lengths, rules,
surfaces, and noise/token families.

It is not a full 32-slot product proof: edit, conditional, composed, multi-seed
corpus robustness, and packed product p99 are still open.

## Command

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_order_corpus_must_transfer_without_lookup_or_runtime_phase_hack --nocapture
```

Saved log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_order_corpus_rung_release.log
```

## Corpus Pressure

```text
seed: 0
train_rows: 1024
heldout_rows: 1024
unique_rules: 8
unique_surfaces: 4
unique_noise_types: 2
unique_lengths: 16
lengths: 17..32
same_bag_rows: 1024
max_train_state_reuse: 8
max_heldout_state_reuse: 8
train_tokens_overlap_heldout: 0
```

Important boundary:

```text
Tokens are independent of rule_name.
The same state key is reused across 8 different rules.
Therefore input/state alone is not sufficient; action/operator-pair channel is
required for the transfer.
```

## Layout

```text
CenterId: u32
page_bits: 12
page_size: 4096
page_count: 64
total_center_count: 262144
output_slot_count: 32
role_slot_count: 32
role_top_l1_lanes: 64
```

## Result

```text
verdict: SLOT32_ORDER_CORPUS_RUNG_PASS

slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
sequence_energy_median_gap: 4203008
sequence_energy_p10_gap: 3110016
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

role_binding_edges: 1354
flat_role_binding_edges: 1354
flat_role_binding_bytes_estimate: 81792
base_mass_bytes_estimate: 524288
hot_bytes_estimate: 606080

flat_eval_rows: 1024
flat_eval_avg_ns_per_row: 185511
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
first real 32-slot order corpus rung;
lengths 17..32;
8 order/permutation families;
4 surface families;
2 noise/token families;
same-bag heldout negatives;
rule-independent tokens;
field/flat parity;
binding/action/role/active ablation collapse;
sub-4MiB hot table estimate.
```

This does not close:

```text
full 32-slot operator battery;
32-slot edit / conditional / composed gates;
32-slot multi-seed corpus robustness;
packed product runtime proof;
product p99 latency proof;
64-slot capacity;
broad product reasoning;
autonomous raw action parsing;
text generation.
```
