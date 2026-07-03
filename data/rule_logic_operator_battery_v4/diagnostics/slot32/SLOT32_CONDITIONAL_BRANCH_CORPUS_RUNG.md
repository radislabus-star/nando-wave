# Slot32 Conditional Branch Corpus Rung

Date: 2026-07-03

This rung closes the first 32-slot conditional branch-selection proof on the
current paged `u32` role-binding runtime.

It differs from the previous order/mixed-map rungs in one important way:

```text
The direct operator-pair page is not active.
The selected branch is carried by condition-result action pages.
```

The branch is selected from:

```text
state_flag == trigger_flag
```

The correct and wrong outputs are same-bag branch alternatives, so bag-of-tokens
cannot solve the task.

Boundary: this is still a symbolic branch-map action input proof. It does not
prove raw-language action parsing or autonomous action-tree induction.

## Command

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_conditional_branch_must_select_without_lookup_or_runtime_phase_hack --nocapture
```

Saved log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_conditional_branch_corpus_rung_release.log
```

## Corpus Pressure

```text
seed: 0
train_rows: 2048
heldout_rows: 2048
unique_operator_classes: 1
unique_rules: 8
unique_surfaces: 4
unique_noise_types: 2
unique_lengths: 16
lengths: 17..32
same_bag_rows: 2048
condition_true_rows: 1024
condition_false_rows: 1024
max_train_state_reuse: 16
max_heldout_state_reuse: 16
train_tokens_overlap_heldout: 0
```

Shortcut guards:

```text
direct_operator_pair_active_centers: 0
condition_action_active_centers: 50176
state_condition_active_centers: 120832
```

## Result

```text
verdict: SLOT32_CONDITIONAL_BRANCH_CORPUS_RUNG_PASS

page_count: 64
total_center_count: 262144
output_slot_count: 32
role_slot_count: 32
role_top_l1_lanes: 64
condition_true_action_page: 34
condition_false_action_page: 35
state_condition_page: 36

slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
sequence_energy_median_gap: 4205056
sequence_energy_p10_gap: 3122560
energy_pass_slot_fail: 0
flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_mismatches: 0
flat_failed_rows: 0

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_condition_action_accuracy_milli: 0
ablation_without_condition_action_energy_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0

state_delta_edges: 0
role_binding_edges: 2202
flat_role_binding_bytes_estimate: 157504
base_mass_bytes_estimate: 524288
hot_bytes_estimate: 681792
flat_eval_rows: 2048
flat_eval_avg_ns_per_row: 174654
flat_eval_latency_gate_ns: 1000000

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
direct_operator_pair_action_centers_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

## Claim Boundary

This closes:

```text
32-slot conditional branch selection single-seed rung;
state/action branch selection over symbolic branch-map inputs;
lengths 17..32;
8 conditional branch rules;
4 surface families;
2 noise/token families;
same-bag branch alternatives;
field/flat parity;
binding/action/condition-action/role/active ablation collapse;
no direct operator-pair action centers;
sub-4MiB hot table estimate.
```

This does not close:

```text
32-slot conditional multi-seed robustness;
full 32-slot operator battery multi-seed proof;
raw-language action parsing;
autonomous action_tree induction;
insert-new-constant edit operators;
packed product runtime proof;
product p99 latency proof;
64-slot capacity;
broad product reasoning;
text generation.
```
