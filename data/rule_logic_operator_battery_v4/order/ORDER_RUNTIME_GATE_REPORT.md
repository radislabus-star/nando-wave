# v4 Order Runtime Gate Report

Date: 2026-07-01

## Verdict

`GREEN_AFTER_L1_SHORT_TOKEN_IDENTITY_FIX`

The v4 order corpus now passes the runtime gate after a general L1 fix for
short-token identity support.

The original red baseline remains recorded below because it was the diagnostic
evidence that justified the L1 change.

## Command

```bash
stdbuf -oL env \
  OPERATOR_BATTERY_V4_ORDER_CORPUS_PATH=../../data/rule_logic_operator_battery_v4/order/accepted_operator_tasks_v4.jsonl \
  POSITION_SEQUENCE_OPERATOR_PAIR_ACTION_CENTERS=1 \
  OPERATOR_BATTERY_V4_ORDER_LOCAL_EPOCHS=8 \
  OPERATOR_BATTERY_V4_ORDER_CLEANUP_EPOCHS=4 \
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_order_must_transfer_without_lookup_or_runtime_phase_hack --nocapture \
  2>&1 | tee data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_diagnostic.log
```

Log:

```text
data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_diagnostic.log
```

## Main Metrics

Original red baseline:

```text
train_rows: 2048
heldout_rows: 2048
operator_pair_action_centers_used: true

order_slot_ordered_sequence_accuracy_milli: 999
order_flat_slot_ordered_sequence_accuracy_milli: 999

order_sequence_energy_accuracy_milli: 1000
order_sequence_energy_median_gap: 3762632
order_sequence_energy_p10_gap: 1944340
order_energy_pass_slot_fail: 2

flat_sequence_energy_parity_checked_rows: 2048
flat_sequence_energy_parity_mismatches: 0
flat_sequence_energy_parity_max_abs_gap_delta: 0
flat_gap_parity_mismatches: 0

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0

state_delta_edges: 0
role_binding_edges: 197586
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

## Failure Localization

Only two strict slot failures reproduced:

```text
slot_failure source_group=operator_battery_order_heldout_order_window_reverse_3_len13
rule=order_window_reverse_3_len13
length=13
surface=symbols
noise=clean
output_slot=12
source_slot=12
gap=-6290
sequence_energy_gap=2689734
correct_token=Z12
```

```text
slot_failure source_group=operator_battery_order_heldout_order_window_reverse_3_len13
rule=order_window_reverse_3_len13
length=13
surface=symbols
noise=distractor
output_slot=12
source_slot=12
gap=-15112
sequence_energy_gap=2706618
correct_token=Z14
```

Aggregate:

```text
order_output_slot_cleanup_failed_slots: 2
order_output_slot_cleanup_accuracy_by_output_slot:
  {0: 1000, 1: 1000, 2: 1000, 3: 1000, 4: 1000, 5: 1000, 6: 1000, 7: 1000,
   8: 1000, 9: 1000, 10: 1000, 11: 1000, 12: 998, 13: 1000, 14: 1000, 15: 1000}
order_output_slot_cleanup_failed_by_output_source_pair:
  {"out12->src12": 2}
```

## Interpretation

The red gate is not a broad v4 order collapse.

The sequence-level operator energy chooses the correct sequence for all heldout
rows. The compiled flat path exactly matches the field path. All ablations
collapse to zero. The forbidden shortcut flags remain false.

The remaining failure is a strict readout problem on the tail identity/remainder
slot of `window_reverse_3` at length 13:

```text
out12 -> src12
```

That means the current model has the correct global operator preference, but the
local slot readout underweights one fixed-point tail slot in this operator
family.

## Boundary

The red baseline above is preserved as the diagnostic evidence that justified
the L1 short-token identity repair. Do not use that old 999/1000 baseline as the
current order status.

Current order status after the L1 fix:

```text
GREEN_AFTER_L1_SHORT_TOKEN_IDENTITY_FIX
```

Do not change architecture from the old red baseline alone. Any future order
change must start from the green log recorded later in this report.

## Candidate-Cleanup Attempt

After the red baseline, an existing non-architectural cleanup path was tested:
all source-token candidates were used as same-bag competitors for each output
slot.

Command delta:

```text
OPERATOR_BATTERY_V4_ORDER_CANDIDATE_CLEANUP_EPOCHS=1
```

Log:

```text
data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_candidate_cleanup.log
```

Candidate cleanup work:

```text
candidate_slot_tasks: 305152
candidate_cleanup_epoch=1/1
margin: 160
repaired_slots: 4
update_steps: 4
touched_edges: 6554
state_delta_edges: 0
role_binding_edges: 197617
```

Result:

```text
order_slot_ordered_sequence_accuracy_milli: 999
order_flat_slot_ordered_sequence_accuracy_milli: 999
order_sequence_energy_accuracy_milli: 1000
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
ablation_without_binding/action/role/active_fringe: 0
failed slots: 2
failed pair: out12->src12
```

The same two heldout slot failures remained:

```text
order_window_reverse_3_len13 / symbols / clean / out12->src12
order_window_reverse_3_len13 / symbols / distractor / out12->src12
```

Interpretation:

```text
The failure is not fixed by exposing train to all same-bag source-token
competitors. The issue is likely local slot/lane readout geometry for the
fixed-point remainder slot, not missing negative coverage.
```

## Coverage Check

The failed `out12->src12` pair is present in train. It is not a missing-cell
problem:

```text
rule: order_window_reverse_3_len13
pair: out12->src12
train coverage: symbols, ru_words, business, network
train noise coverage: clean, prefix_suffix, distractor, instruction_noise
heldout coverage: symbols, ru_words, business, network
heldout noise coverage: clean, prefix_suffix, distractor, instruction_noise
```

Only the symbolic heldout rows failed. The word-like surfaces passed.

Next diagnostic hypothesis:

```text
Short symbolic tokens such as Z12/Z14 may have weaker or more colliding L1 lane
support than word-like tokens. The next diagnostic should measure target-lane
true-role hits, wrong-role hits, and per-lane score contributions for the two
failed slots before changing architecture.
```

## Lane-Pressure Diagnostic

Log:

```text
data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_lane_pressure.log
```

The pressure diagnostic confirms the failure is local L1/readout aliasing, not
operator selection.

Clean symbolic failure:

```text
correct_token: Z12
wrong_token: Z3
target_score: -9644
wrong_score: -3354
target_wrong_cosine_milli: 182
active_action_centers: 77
worst target lanes:
  lane=166  total=-15658 roles=[(1,1),(10,1),(11,1),(12,1)]
  lane=2533 total=-15658 roles=[(1,1),(10,1),(11,1),(12,1)]
```

Distractor symbolic failure:

```text
correct_token: Z14
wrong_token: Z9
target_score: -25752
wrong_score: -10640
target_wrong_cosine_milli: 169
active_action_centers: 77
worst target lanes:
  lane=166  total=-16362 roles=[(8,1),(9,1),(10,1),(11,1),(12,1)]
  lane=2533 total=-16362 roles=[(8,1),(9,1),(10,1),(11,1),(12,1)]
```

Interpretation:

```text
The failed fixed-point remainder token is not missing from the role slot.
The same L1 lanes for short symbolic tokens fire in multiple role slots.
The local readout receives strong negative binding pressure on shared lanes
166 and 2533, so the correct token's target score becomes more negative than
the wrong token's score.
```

Current diagnosis:

```text
v4 order is blocked by short-symbol L1 lane aliasing in strict local readout.
The global sequence-energy operator is already correct, but strict per-slot
readout needs cleaner token identity support for short symbolic fillers.
```

Next allowed fix candidate:

```text
Consider a general L1 short-token / token-identity atom, applied uniformly to
all tokens, not a target-specific rule and not an L3 operator hack.
It must be proved by rerunning the same v4 order gate plus L1 regression checks.
```

## L1 Short-Token Identity Fix

Implemented fix:

```text
crates/nando-core/src/wave/surface_wave.rs
SURFACE_WAVE_SHORT_TOKEN_IDENTITY_ATOMS = 4
```

The fix adds salted identity atoms for normalized tokens shorter than the
4-gram size. This is a general L1 representation repair:

```text
not target-specific
not proof_rule_id authority
not concrete_x_lookup
not manual local_out_t
not an L3 operator hack
```

L1 check:

```bash
cargo test -p nando-core --lib surface_wave -- --nocapture
```

Result:

```text
11 passed; 0 failed
```

Order runtime command:

```bash
stdbuf -oL env \
  OPERATOR_BATTERY_V4_ORDER_CORPUS_PATH=../../data/rule_logic_operator_battery_v4/order/accepted_operator_tasks_v4.jsonl \
  POSITION_SEQUENCE_OPERATOR_PAIR_ACTION_CENTERS=1 \
  OPERATOR_BATTERY_V4_ORDER_LOCAL_EPOCHS=8 \
  OPERATOR_BATTERY_V4_ORDER_CLEANUP_EPOCHS=4 \
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_order_must_transfer_without_lookup_or_runtime_phase_hack --nocapture \
  2>&1 | tee data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_l1_short_token_identity.log
```

Log:

```text
data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_l1_short_token_identity.log
```

Green result:

```text
order_slot_ordered_sequence_accuracy_milli: 1000
order_flat_slot_ordered_sequence_accuracy_milli: 1000
order_sequence_energy_accuracy_milli: 1000
order_sequence_energy_median_gap: 4765082
order_sequence_energy_p10_gap: 2646022
order_energy_pass_slot_fail: 0
order_output_slot_cleanup_failed_slots: 0
order_output_slot_cleanup_accuracy_by_output_slot:
  {0: 1000, 1: 1000, 2: 1000, 3: 1000, 4: 1000, 5: 1000, 6: 1000, 7: 1000,
   8: 1000, 9: 1000, 10: 1000, 11: 1000, 12: 1000, 13: 1000, 14: 1000, 15: 1000}
order_output_slot_cleanup_failed_by_output_source_pair: {}

flat_sequence_energy_parity_checked_rows: 2048
flat_sequence_energy_parity_mismatches: 0
flat_sequence_energy_parity_max_abs_gap_delta: 0
flat_gap_parity_mismatches: 0

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0

state_delta_edges: 0
role_binding_edges: 185112
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Interpretation:

```text
The v4 order gate is green after repairing the diagnosed L1 short-symbol lane
aliasing. The operator path still depends on action + role binding. All
shortcut/hardcode flags remain false, flat/runtime parity is exact, and all
mechanism ablations collapse to zero.
```
