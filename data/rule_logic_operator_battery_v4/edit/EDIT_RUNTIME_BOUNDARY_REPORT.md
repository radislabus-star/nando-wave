# v4 Edit Runtime Boundary Report

Date: 2026-07-03

## Verdict

```text
EDIT_CURRENT_SOURCE_RUNTIME_GATE_PASS
EDIT_RELEASE_SUITE_INTEGRATION_PASS
EDIT_BLUEPRINT_CLASS_PARTIAL
```

The v4 edit corpus exposed a real boundary in the original role-transfer
runtime. Order transfer can move source role fillers. Edit also needs:

```text
1. source role fillers;
2. action-supplied marker/end fillers;
3. bounded variable output length;
4. explicit pressure for length-mismatch rows.
```

Fresh current-source reruns show the edit runtime gate is green with a compact
role-binding edge set and clean shortcut flags. The bounded EDIT marker/length
profile is now also integrated into the source-verified `.nwrb/.nwreb`
role-binding release suite.

## Fresh Current-Source Runtime Gate

Command:

```bash
stdbuf -oL env \
  OPERATOR_BATTERY_V4_EDIT_CORPUS_PATH=/home/ubu/projects/nando-wave/data/rule_logic_operator_battery_v4/edit/accepted_operator_tasks_v4.jsonl \
  OPERATOR_BATTERY_V4_EDIT_LOCAL_EPOCHS=8 \
  OPERATOR_BATTERY_V4_EDIT_CLEANUP_EPOCHS=4 \
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
    --ignored operator_battery_v4_edit_marker_length_must_transfer_without_lookup_or_runtime_phase_hack --nocapture \
    2>&1 | tee data/rule_logic_operator_battery_v4/edit/edit_marker_length_runtime_gate_release.log
```

Log:

```text
data/rule_logic_operator_battery_v4/edit/edit_marker_length_runtime_gate_release.log
```

Result:

```text
test operator_battery_v4_edit_marker_length_must_transfer_without_lookup_or_runtime_phase_hack ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out; finished in 76.94s
```

## Fresh Metrics

```text
train_rows: 1536
heldout_rows: 1536
train_discriminative_slot_tasks: 13504
heldout_discriminative_slot_tasks: 13504
rows_with_full_demo_slot_map: 3072

edit_output_slot_count: 17
edit_role_slot_count: 17
edit_marker_role_slot: 16
edit_action_base: 69632
edit_demo_channel_page: 18
edit_demo_channel_base: 73728

edit_slot_ordered_sequence_accuracy_milli: 1000
edit_flat_slot_ordered_sequence_accuracy_milli: 1000
edit_sequence_energy_accuracy_milli: 1000
edit_sequence_energy_median_gap: 39424
edit_sequence_energy_p10_gap: 13056
edit_energy_pass_slot_fail: 0

edit_output_slot_cleanup_failed_slots: 0
edit_output_slot_cleanup_accuracy_by_output_slot:
  0..16 all 1000
edit_output_slot_cleanup_failed_by_output_source_pair: {}

l3_role_binding_edge_count: 135
l3_action_centers_with_edges: 68
l3_max_edges_per_action_center: 2
l3_max_slots_per_action_center: 1
l3_role_slots_covered: 17

flat_sequence_energy_parity_checked_rows: 1536
flat_sequence_energy_parity_mismatches: 0
flat_sequence_energy_parity_max_abs_gap_delta: 0
flat_gap_parity_mismatches: 0

state_delta_edges: 0
role_binding_edges: 136
```

## Ablations

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_edit_demo_accuracy_milli: 0
ablation_without_edit_demo_energy_accuracy_milli: 0
ablation_without_marker_role_accuracy_milli: 500
ablation_without_marker_role_energy_accuracy_milli: 1000
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

Interpretation:

```text
The edit demo channel is necessary for this current-source proof: removing it
collapses strict slot and energy accuracy to 0.

The marker/end role channel is necessary for strict marker/length decisions but
not the only source of edit evidence: removing it leaves 500 milli strict slot
accuracy and 1000 milli energy because some rows remain source-role edit
transfers.

The core mechanism is not a state_delta shortcut:
state_delta_edges: 0
```

## Fresh Boundary Gate

Command:

```bash
stdbuf -oL env \
  OPERATOR_BATTERY_V4_EDIT_CORPUS_PATH=/home/ubu/projects/nando-wave/data/rule_logic_operator_battery_v4/edit/accepted_operator_tasks_v4.jsonl \
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 -- \
    --ignored operator_battery_v4_edit_current_role_binding_runtime_boundary_must_be_explicit --nocapture \
    2>&1 | tee data/rule_logic_operator_battery_v4/edit/edit_runtime_boundary_gate.log
```

Result:

```text
test operator_battery_v4_edit_current_role_binding_runtime_boundary_must_be_explicit ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out; finished in 0.26s
```

Boundary metrics:

```text
rows: 3072
train_rows: 1536
heldout_rows: 1536
current_output_slot_count: 16
rows_output_len_over_slots: 192
rows_correct_wrong_len_mismatch: 256
rows_with_non_source_output_tokens: 1280
rows_with_marker_output_tokens: 1280
rows_representable_by_current_role_transfer: 1440
rows_not_representable_by_current_role_transfer: 1632
non_representable_by_family:
  drop_every_third: 256
  duplicate_first: 32
  duplicate_last: 32
  duplicate_middle: 32
  insert_head_marker: 256
  insert_middle_marker: 256
  insert_tail_marker: 256
  replace_first_marker: 256
  replace_last_marker: 256
```

Conclusion from the boundary gate:

```text
Order proves same-bag role/filler transfer.
It does not silently prove marker insertion, replacement, deletion,
duplication, or variable-length output.
```

## Release-Suite Integration

Command:

```bash
stdbuf -oL env \
  OPERATOR_BATTERY_V4_EDIT_CORPUS_PATH=/home/ubu/projects/nando-wave/data/rule_logic_operator_battery_v4/edit/accepted_operator_tasks_v4.jsonl \
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
    --ignored operator_battery_v4_edit_role_binding_public_sdk_must_score_loaded_package_runtime --nocapture \
    2>&1 | tee data/rule_logic_operator_battery_v4/edit/edit_role_binding_public_sdk_package_release.log
```

Result:

```text
verdict: EDIT_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG_PASS
label: sdk_edit_marker_length
seed: 0
train_rows: 1536
heldout_rows: 1536
local_margin_threshold: 1
package_path: target/nando-wave/slot32-role-binding/sdk_edit_marker_length-seed0.nwrb
package_bytes: 1664
package_fingerprint64: 15479025432367793657
inspected_edges: 135
loaded_rewrite_exact: true
slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_mismatches: 0
false_local_accepts: 0
p99_latency_ns: 153773
hot_bytes_estimate: 329308
```

Release-suite status:

```text
role-binding release-suite package_count: 7
role-binding release-suite total_sequence_count: 27648
role-binding release-suite total_sequence_false_local_accepts: 0
OPERATOR_BLUEPRINT EDIT status: PARTIAL
```

## Forbidden Shortcut Flags

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

## Boundary

Current green claim:

```text
v4 EDIT marker/length runtime gate passes on the current 3072-row edit corpus
with 17 bounded output slots, action-derived edit demo mapping, marker/end role
support, field/flat parity, ablation collapse for the essential channels, and
clean forbidden flags.
```

Do not overclaim:

```text
This proves the current bounded EDIT runtime gate and its `.nwrb/.nwreb`
release-suite integration. It does not prove unbounded text decoding,
raw-language action parsing, or the full OPERATOR_BLUEPRINT EDIT class.

The slot32 OPERATOR_BLUEPRINT gap audit lists EDIT as PARTIAL, not PROVEN,
because clear/append/prepend and the full EDIT family are still not closed as
separate product classes.
```
