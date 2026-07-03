# v4 Conditional Runtime Gate Report

Date: 2026-07-01

## Verdict

`GREEN_AFTER_CONDITION_ACTION_CONJUNCTION`

The original conditional failure had two stages:

```text
1. RED_INPUT_CHANNEL_BOUNDARY_DIAGNOSED
   current_flag was copied through rule_action_example.

2. RED_STATE_CHANNEL_SLOT_READOUT_DIAGNOSED
   after removing the leak, a simple independent state-condition page produced
   strong sequence energy but weak strict slot readout.
```

The repaired conditional gate is now green after adding a proof-gated
condition/action conjunction channel.

Latest update:

```text
generic conditional action-surface is suppressed by default.
current base runtime log:
data/rule_logic_operator_battery_v4/conditional/conditional_no_action_surface_default_runtime_gate_release.log

seed1 multi-seed runtime log:
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_001/conditional/conditional_runtime_gate_release.log
```

Why:

```text
The raw conditional action text contains both then/else branches. Keeping its
surface centers active created noisy branch pressure. The accepted proof channel
is the selected condition/action conjunction, derived from rule_action_example
and state_before condition, not from target/proof_rule_id/local_out_t.
```

## What Changed

The corpus still does not copy the current branch flag through action text:

```text
rows_with_action_current_flag: 0 / 1536
rows_action_flag_matches_state_flag: 0 / 1536
rows_branch_signal_action_only_for_current_runtime: 0 / 1536
```

The condition remains in `state_before`:

```text
rows_with_state_condition_flag: 1536 / 1536
rows_source_tokens_include_condition_flag: 0 / 1536
```

`rule_action_example` now contains both branch operator maps:

```text
then_slots: src...
else_slots: src...
```

The runtime activates the selected condition/action conjunction by combining:

```text
state_before condition
+ rule_action_example branch alternatives
```

It does not read target text, proof_rule_id authority, concrete lookup, or
manual local_out_t.

Pages:

```text
state_condition_channel_page: 18
state_condition_channel_base: 73728
condition_action_conjunction_page: 19
condition_action_conjunction_base: 77824
```

## Shortcut Gate

After adding branch slot maps, the full v4 shortcut gate remains clean:

```text
verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
conditional exact_lookup_accuracy_milli: 0
conditional proof_rule_id_majority_accuracy_milli: 0
conditional proof_rule_family_majority_accuracy_milli: 0
conditional surface_family_majority_accuracy_milli: 0
conditional length_only_accuracy_milli: 0
conditional output_position_prior_accuracy_milli: 0
conditional l2_neighbor_target_copy_accuracy_milli: 0
conditional markov_bigram_pairwise_accuracy_milli: 500
conditional bayesian_cooccurrence_pairwise_accuracy_milli: 500
conditional same_bag_derangement_milli: 1000
```

Shortcut log:

```text
data/rule_logic_operator_battery_v4/shortcut_gate_report.log
data/rule_logic_operator_battery_v4/shortcut_gate_report.json
```

## Runtime Command

```bash
stdbuf -oL env \
  OPERATOR_BATTERY_V4_CONDITIONAL_CORPUS_PATH=../../data/rule_logic_operator_battery_v4/conditional/accepted_operator_tasks_v4.jsonl \
  OPERATOR_BATTERY_V4_CONDITIONAL_LOCAL_EPOCHS=8 \
  OPERATOR_BATTERY_V4_CONDITIONAL_CLEANUP_EPOCHS=4 \
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_conditional_state_channel_must_transfer_without_action_flag_leak --nocapture \
  2>&1 | tee data/rule_logic_operator_battery_v4/conditional/conditional_no_action_surface_default_runtime_gate_release.log
```

Log:

```text
data/rule_logic_operator_battery_v4/conditional/conditional_no_action_surface_default_runtime_gate_release.log
```

## Runtime Metrics

```text
rows: 1536
train_rows: 768
heldout_rows: 768
rows_same_bag: 1536
rows_all_outputs_from_source: 1536
rows_output_len_within_slots: 1536
rows_representable_as_order_transfer_if_branch_known: 1536

conditional_slot_ordered_sequence_accuracy_milli: 1000
conditional_flat_slot_ordered_sequence_accuracy_milli: 1000
conditional_sequence_energy_accuracy_milli: 1000
conditional_sequence_energy_median_gap: 1868422
conditional_sequence_energy_p10_gap: 1081834
conditional_energy_pass_slot_fail: 0
conditional_output_slot_cleanup_failed_slots: 0

conditional_output_slot_cleanup_accuracy_by_output_slot:
  {0: 1000, 1: 1000, 2: 1000, 3: 1000, 4: 1000, 5: 1000, 6: 1000, 7: 1000,
   8: 1000, 9: 1000, 10: 1000, 11: 1000, 12: 1000, 13: 1000, 14: 1000,
   15: 1000}

l3_role_binding_edge_count: 93269
l3_action_centers_with_edges: 284
l3_max_edges_per_action_center: 455
l3_max_slots_per_action_center: 16
l3_role_slots_covered: 16

state_delta_edges: 0
role_binding_edges: 114388
```

Ablations:

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_condition_accuracy_milli: 0
ablation_without_condition_energy_accuracy_milli: 0
ablation_without_condition_action_accuracy_milli: 30
ablation_without_condition_action_energy_accuracy_milli: 879
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

The condition/action ablations collapse below the same-bag chance line:

```text
chance_line_milli: 500
without_condition: 0
without_condition_action_conjunction: 30
```

The sequence-energy-only ablation without condition/action remains nonzero
because some branch pairs share partial global structure. This is not counted
as a shortcut pass because strict ordered readout falls far below chance and
below the green gate.

Parity:

```text
flat_sequence_energy_parity_checked_rows: 768
flat_sequence_energy_parity_mismatches: 0
flat_sequence_energy_parity_max_abs_gap_delta: 0
flat_gap_parity_mismatches: 0
```

Forbidden shortcut flags remain false:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

## Interpretation

Conditional now has a valid learned runtime path:

```text
state condition
+ action branch alternatives
+ learned condition/action conjunction
+ role/filler binding
-> ordered state_t+1
```

The critical evidence is:

```text
slot readout: 1000
sequence energy: 1000
flat/runtime parity: 0 mismatches
without condition/action conjunction: 30
without action: 0
without role: 0
without active fringe: 0
forbidden flags: false
```

This closes v4 conditional for the current 16-slot battery. It does not prove
edit or composed.

## Boundary

Do not generalize this result to edit or composed.

The conditional mechanism is allowed because it is derived from input state and
action branch alternatives. It is not a hand-coded answer path.
