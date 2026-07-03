# v4 Composed Runtime Gate Report

Date: 2026-07-01

## Verdict

`GREEN_AFTER_COMPOSED_DEMO_SLOT_CHANNEL`

The original composed red gate showed a strong sequence-energy judge but weak
strict slot readout:

```text
old composed_slot_ordered_sequence_accuracy_milli: 172
old composed_sequence_energy_accuracy_milli: 982
old composed_energy_pass_slot_fail: 778
```

The composed gate is now green after adding a proof-gated composed demo slot
channel.

Latest update:

```text
generic composed action-surface is suppressed by default.
current base runtime log:
data/rule_logic_operator_battery_v4/composed/composed_no_action_surface_default_runtime_gate_release.log

seed1 multi-seed runtime log:
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_001/composed/composed_runtime_gate_release.log
```

Why:

```text
The raw composed action text contains the explicit demo. Keeping its surface
centers active let ablation without page 20 solve above chance on seed1. The
accepted proof channel is the parsed neutral demo slot page, not fuzzy
action-text surface pressure.
```

## What Changed

The composed action example now uses a full-length neutral demo:

```text
demo: d0 d1 ... dN -> after_first -> after_second
```

The runtime parses the final demo segment and derives:

```text
output_slot -> source_slot
```

from neutral demo tokens such as `d7`, `d6`, `d5`.

It does not read target text, proof_rule_id authority, concrete lookup, or
manual local_out_t.

Pages:

```text
composed_demo_channel_page: 20
composed_demo_channel_base: 81920
```

The old explicit order path remains off:

```text
operator_slots in action: 0 / 1920
operator_pair_action_centers_used: false
```

## Shortcut Gate

After adding full-length composed demos, the full v4 shortcut gate remains
clean:

```text
verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
composed exact_lookup_accuracy_milli: 0
composed proof_rule_id_majority_accuracy_milli: 0
composed proof_rule_family_majority_accuracy_milli: 0
composed surface_family_majority_accuracy_milli: 0
composed length_only_accuracy_milli: 0
composed output_position_prior_accuracy_milli: 0
composed l2_neighbor_target_copy_accuracy_milli: 0
composed markov_bigram_pairwise_accuracy_milli: 500
composed bayesian_cooccurrence_pairwise_accuracy_milli: 500
composed same_bag_derangement_milli: 1000
```

Shortcut log:

```text
data/rule_logic_operator_battery_v4/shortcut_gate_report.log
data/rule_logic_operator_battery_v4/shortcut_gate_report.json
```

## Runtime Command

```bash
stdbuf -oL env \
  OPERATOR_BATTERY_V4_COMPOSED_CORPUS_PATH=../../data/rule_logic_operator_battery_v4/composed/accepted_operator_tasks_v4.jsonl \
  OPERATOR_BATTERY_V4_COMPOSED_LOCAL_EPOCHS=8 \
  OPERATOR_BATTERY_V4_COMPOSED_CLEANUP_EPOCHS=4 \
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_composed_must_transfer_without_lookup_or_runtime_phase_hack --nocapture \
  2>&1 | tee data/rule_logic_operator_battery_v4/composed/composed_no_action_surface_default_runtime_gate_release.log
```

Log:

```text
data/rule_logic_operator_battery_v4/composed/composed_no_action_surface_default_runtime_gate_release.log
```

## Runtime Metrics

```text
rows: 1920
train_rows: 960
heldout_rows: 960
rows_with_full_demo_slot_map: 1920

composed_slot_ordered_sequence_accuracy_milli: 1000
composed_flat_slot_ordered_sequence_accuracy_milli: 1000
composed_sequence_energy_accuracy_milli: 1000
composed_sequence_energy_median_gap: 1838464
composed_sequence_energy_p10_gap: 999168
composed_energy_pass_slot_fail: 0
composed_output_slot_cleanup_failed_slots: 0

composed_output_slot_cleanup_accuracy_by_output_slot:
  {0: 1000, 1: 1000, 2: 1000, 3: 1000, 4: 1000, 5: 1000, 6: 1000,
   7: 1000, 8: 1000, 9: 1000, 10: 1000, 11: 1000, 12: 1000, 13: 1000,
   14: 1000, 15: 1000}

l3_role_binding_edge_count: 51217
l3_action_centers_with_edges: 183
l3_max_edges_per_action_center: 452
l3_max_slots_per_action_center: 16
l3_role_slots_covered: 16

state_delta_edges: 0
role_binding_edges: 65848
```

Ablations:

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_composed_demo_accuracy_milli: 0
ablation_without_composed_demo_energy_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

The composed demo channel is required for strict slot readout:

```text
chance_line_milli: 500
without_composed_demo_slot_accuracy: 0
```

Important boundary:

```text
without_composed_demo_energy_accuracy: 0
```

So the composed demo channel is now required for both strict readout and
sequence energy on the current rung.
The global sequence-energy judge already had strong signal from action surface /
demo text.

Parity:

```text
flat_sequence_energy_parity_checked_rows: 960
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

Composed now has a valid learned runtime path for the current 16-slot v4
battery:

```text
action two-step demo
+ derived composed demo slot channel
+ role/filler binding
-> ordered state_t+1
```

The critical evidence is:

```text
slot readout: 1000
sequence energy: 1000
flat/runtime parity: 0 mismatches
without composed demo channel: 0
without action: 0
without role: 0
without active fringe: 0
forbidden flags: false
```

This closes v4 composed for the current 16-slot battery. It does not prove edit.

## Boundary

Do not generalize this result to edit.

Do not claim that the composed demo channel is the source of all sequence-energy
understanding. It is the missing decoder/readout channel that turns the existing
energy signal into stable slot construction.
