# Conditional Seed1 train_per_cell=2 Red After Contract Fix

Date: 2026-07-01

## Verdict

`CONDITIONAL_STRICT_SLOT_RED_SEQUENCE_ENERGY_STRONG`

This is the first true conditional runtime result for the
`train_per_cell=2` full-corpus policy after removing an obsolete hardcoded
row-count assert.

The earlier full run stopped before training conditional because the test
expected `report.rows == 1536`. Under `train_per_cell=2`, the conditional
corpus has:

```text
rows: 2304
train_rows: 1536
heldout_rows: 768
```

The test contract was corrected to assert the current corpus size instead of
the old constant. After that, the conditional gate ran to completion and failed
on real heldout metrics.

## Command

```bash
env \
  OPERATOR_BATTERY_V4_CONDITIONAL_CORPUS_PATH=/home/ubu/projects/nando-wave/data/rule_logic_operator_battery_v4/diagnostics/multiseed_train_per_cell_2/seed_001/conditional/accepted_operator_tasks_v4.jsonl \
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- \
  --ignored operator_battery_v4_conditional_state_channel_must_transfer_without_action_flag_leak --nocapture
```

Log:

```text
data/rule_logic_operator_battery_v4/diagnostics/multiseed_train_per_cell_2/seed_001/conditional/conditional_runtime_gate_release_after_contract_fix.log
```

## Metrics

```text
conditional_slot_ordered_sequence_accuracy_milli: 617
conditional_flat_slot_ordered_sequence_accuracy_milli: 617
conditional_sequence_energy_accuracy_milli: 973
conditional_sequence_energy_median_gap: 1351198
conditional_sequence_energy_p10_gap: 79876
conditional_energy_pass_slot_fail: 273
conditional_output_slot_cleanup_failed_slots: 1146
conditional_slot_failure_total: 1146
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
state_delta_edges: 0
role_binding_edges: 81739
```

Forbidden substitutions stayed false:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Ablations stayed useful:

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_condition_accuracy_milli: 0
ablation_without_condition_energy_accuracy_milli: 0
ablation_without_condition_action_accuracy_milli: 49
ablation_without_condition_action_energy_accuracy_milli: 587
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

## Interpretation

This is not lookup and not a runtime/flat mismatch:

```text
flat parity: clean
state_delta_edges: 0
forbidden flags: false
shortcut gate: clean
```

The current conditional mechanism has a strong sequence-level operator signal,
but the strict slot decoder is not stable:

```text
sequence energy: 973 / 1000
strict ordered slot readout: 617 / 1000
```

The failure concentrates on conditional branch transfer under heldout surfaces,
especially early output slots in length-9 branch cases. Example failure:

```text
rule: conditional_if_alpha_mirror_else_rotate_left_len9
output_slot: 5
source_slot: 3
gap: -6996
sequence_energy_gap: 184856
```

Observed pressure pattern:

```text
target lanes receive negative binding pressure from the intended role plus
other active roles; wrong lanes receive positive pressure from competing roles.
```

This is a decoder/slot-cleanup failure under conditional branch ambiguity, not
a proof that the operator signal is absent.

## Claim Boundary

`train_per_cell=2` is a principled full-corpus density policy, not a targeted
fix. It made seed2/order green and seed1/order/edit green, but it does not close
the full v4 operator battery because seed1/conditional remains red.

Do not claim:

```text
multi-seed train_per_cell=2 green
conditional robust
v4 operator battery complete
```

Allowed claim:

```text
train_per_cell=2 fixes the known seed2/order sparsity issue and keeps seed1
order/edit green, but exposes a real conditional strict-slot readout failure:
energy is strong, decoder is weak.
```

## Next Diagnostic

Before adding architecture, diagnose conditional specifically:

```text
1. conditional branch-family breakdown;
2. output-slot breakdown for length 9..16;
3. condition/action conjunction separability;
4. role collision pressure by output slot;
5. whether extra conditional train density fixes strict readout;
6. whether a combined energy cleanup can repair strict slots without hurting
   ablations.
```

## Density Follow-Up

The train-density diagnostic was run after this red gate:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_density_sweep/seed_001/CONDITIONAL_DENSITY_SWEEP_REPORT.md
```

Result:

```text
train_per_cell=4 did not fix conditional strict readout.
strict slot: 611
sequence energy: 969
clean/distractor: 1000
prefix_suffix: 779
instruction_noise: 783
```

Updated classification:

```text
noise-robust conditional decoder debt, not simple train-density sparsity
```

## Noise-Isolation Follow-Up

The noise-isolation diagnostic refined the density-sweep claim:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_noise_isolation/seed_001/CONDITIONAL_NOISE_ISOLATION_REPORT.md
```

Key result:

```text
clean_distractor isolated:
  strict slot: 607
  sequence energy: 956
  clean: 1000
  distractor: 761

prefix_suffix isolated:
  strict slot: 594
  sequence energy: 953
  prefix_suffix: 878

instruction_noise isolated:
  strict slot: 594
  sequence energy: 953
  instruction_noise: 878
```

Updated classification after code audit:

```text
conditional role/readout instability under noise-correlated schedule and
surface pressure. Clean rows are green, but distractor is not independently
green when isolated. Prefix/suffix and instruction slices share the same red
profile, but wrapper text is not proven to enter role slots because
sequence_source_tokens extracts only the explicit sequence segment. Sequence
energy remains strong and parity remains exact, so this is still a strict
slot/readout instability, not a missing operator signal.
```
