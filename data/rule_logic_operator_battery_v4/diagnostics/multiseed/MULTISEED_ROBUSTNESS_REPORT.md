# v4 Multi-Seed Robustness Report

Date: 2026-07-02

## Verdict

`GREEN_MULTI_SEED_CURRENT_SCOPE`

## Scope

```text
seeds: [1, 2, 3]
classes: ['order', 'edit', 'conditional', 'composed']
runtime_gates_enabled: True
```

## Accepted Repair

```text
conditional: suppress generic action-surface by default
composed:    suppress generic action-surface by default
```

Reason:

```text
Conditional action text contains both then/else branches. Composed action text
contains an explicit demo. Keeping their raw surface centers active gives the
runtime a fuzzy action-text channel that can conflict with the selected
operator motif or weaken ablation proof.

The accepted channels are:
  conditional -> state condition + selected condition/action conjunction page
  composed    -> parsed neutral demo slot page
```

Forbidden substitutions remain false:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Rejected repairs before the accepted action-surface repair:

```text
short-token identity atoms 8:
  fixed conditional seed1 but weakened edit/composed ablation proof.

role lanes 48:
  no effect on conditional seed1 failure.

all-token candidate cleanup:
  worsened conditional strict readout from 999 to 988.
```

## Results

### Seed 1

```text
shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
composed_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
conditional_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
edit_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
order_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
composed_runtime_log_path: None
composed_test_result: True
composed_slot_accuracy: 1000
composed_energy_accuracy: 1000
composed_energy_pass_slot_fail: 0
composed_output_slot_cleanup_failed_slots: 0
composed_slot_failure_total: 0
composed_flat_energy_parity_mismatches: 0
composed_flat_gap_parity_mismatches: 0
composed_state_delta_edges: 0
conditional_runtime_log_path: None
conditional_test_result: True
conditional_slot_accuracy: 1000
conditional_energy_accuracy: 1000
conditional_energy_pass_slot_fail: 0
conditional_output_slot_cleanup_failed_slots: 0
conditional_slot_failure_total: 0
conditional_flat_energy_parity_mismatches: 0
conditional_flat_gap_parity_mismatches: 0
conditional_state_delta_edges: 0
edit_runtime_log_path: None
edit_test_result: True
edit_slot_accuracy: 1000
edit_energy_accuracy: 1000
edit_energy_pass_slot_fail: 0
edit_output_slot_cleanup_failed_slots: 0
edit_slot_failure_total: MISSING
edit_flat_energy_parity_mismatches: 0
edit_flat_gap_parity_mismatches: 0
edit_state_delta_edges: 0
order_runtime_log_path: None
order_test_result: True
order_slot_accuracy: 1000
order_energy_accuracy: 1000
order_energy_pass_slot_fail: 0
order_output_slot_cleanup_failed_slots: 0
order_slot_failure_total: 0
order_flat_energy_parity_mismatches: 0
order_flat_gap_parity_mismatches: 0
order_state_delta_edges: 0
```

### Seed 2

```text
shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
composed_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
conditional_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
edit_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
order_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
composed_runtime_log_path: None
composed_test_result: True
composed_slot_accuracy: 1000
composed_energy_accuracy: 1000
composed_energy_pass_slot_fail: 0
composed_output_slot_cleanup_failed_slots: 0
composed_slot_failure_total: 0
composed_flat_energy_parity_mismatches: 0
composed_flat_gap_parity_mismatches: 0
composed_state_delta_edges: 0
conditional_runtime_log_path: None
conditional_test_result: True
conditional_slot_accuracy: 1000
conditional_energy_accuracy: 1000
conditional_energy_pass_slot_fail: 0
conditional_output_slot_cleanup_failed_slots: 0
conditional_slot_failure_total: 0
conditional_flat_energy_parity_mismatches: 0
conditional_flat_gap_parity_mismatches: 0
conditional_state_delta_edges: 0
edit_runtime_log_path: None
edit_test_result: True
edit_slot_accuracy: 1000
edit_energy_accuracy: 1000
edit_energy_pass_slot_fail: 0
edit_output_slot_cleanup_failed_slots: 0
edit_slot_failure_total: MISSING
edit_flat_energy_parity_mismatches: 0
edit_flat_gap_parity_mismatches: 0
edit_state_delta_edges: 0
order_runtime_log_path: None
order_test_result: True
order_slot_accuracy: 1000
order_energy_accuracy: 1000
order_energy_pass_slot_fail: 0
order_output_slot_cleanup_failed_slots: 0
order_slot_failure_total: 0
order_flat_energy_parity_mismatches: 0
order_flat_gap_parity_mismatches: 0
order_state_delta_edges: 0
```

### Seed 3

```text
shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
composed_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
conditional_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
edit_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
order_shortcut_verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
composed_runtime_log_path: None
composed_test_result: True
composed_slot_accuracy: 1000
composed_energy_accuracy: 1000
composed_energy_pass_slot_fail: 0
composed_output_slot_cleanup_failed_slots: 0
composed_slot_failure_total: 0
composed_flat_energy_parity_mismatches: 0
composed_flat_gap_parity_mismatches: 0
composed_state_delta_edges: 0
conditional_runtime_log_path: None
conditional_test_result: True
conditional_slot_accuracy: 1000
conditional_energy_accuracy: 1000
conditional_energy_pass_slot_fail: 0
conditional_output_slot_cleanup_failed_slots: 0
conditional_slot_failure_total: 0
conditional_flat_energy_parity_mismatches: 0
conditional_flat_gap_parity_mismatches: 0
conditional_state_delta_edges: 0
edit_runtime_log_path: None
edit_test_result: True
edit_slot_accuracy: 1000
edit_energy_accuracy: 1000
edit_energy_pass_slot_fail: 0
edit_output_slot_cleanup_failed_slots: 0
edit_slot_failure_total: MISSING
edit_flat_energy_parity_mismatches: 0
edit_flat_gap_parity_mismatches: 0
edit_state_delta_edges: 0
order_runtime_log_path: None
order_test_result: True
order_slot_accuracy: 1000
order_energy_accuracy: 1000
order_energy_pass_slot_fail: 0
order_output_slot_cleanup_failed_slots: 0
order_slot_failure_total: 0
order_flat_energy_parity_mismatches: 0
order_flat_gap_parity_mismatches: 0
order_state_delta_edges: 0
```

## Strict Runtime Issues

```text
none
```

## Boundary

This is a robustness rung for the existing v4 mechanisms. It does not add
new architecture and does not widen the claim beyond the seeds/classes
listed above.

Do not claim robustness beyond these seeds until additional seeds are run.
