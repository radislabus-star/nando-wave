# Conditional Density Sweep Report

Date: 2026-07-02

## Verdict

`CONDITIONAL_TRAIN_DENSITY_4_DOES_NOT_FIX_STRICT_SLOT`

This diagnostic tests whether the seed1 conditional red gate is caused by
simple train-density sparsity. It does not change architecture, learning rule,
runtime readout, shortcut policy, or forbidden flags.

## Baseline: train_per_cell=2

Artifact:

```text
data/rule_logic_operator_battery_v4/diagnostics/multiseed_train_per_cell_2/seed_001/conditional/conditional_runtime_gate_release_with_breakdown.log
```

Metrics:

```text
rows: 2304
train_rows: 1536
heldout_rows: 768
conditional_slot_ordered_sequence_accuracy_milli: 617
conditional_sequence_energy_accuracy_milli: 973
conditional_energy_pass_slot_fail: 273
conditional_output_slot_cleanup_failed_slots: 1146
conditional_slot_failure_by_noise:
  clean: 0
  distractor: 0
  prefix_suffix: 573
  instruction_noise: 573
conditional_slot_accuracy_by_noise:
  clean: 1000
  distractor: 1000
  prefix_suffix: 761
  instruction_noise: 761
state_delta_edges: 0
role_binding_edges: 81739
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
```

## Diagnostic: train_per_cell=4

Build command:

```bash
env \
  OPERATOR_BATTERY_OUTPUT_DIR=/home/ubu/projects/nando-wave/data/rule_logic_operator_battery_v4/diagnostics/conditional_density_sweep/seed_001/train_per_cell_4 \
  OPERATOR_BATTERY_CLASSES=conditional \
  OPERATOR_BATTERY_TRAIN_PER_CELL=4 \
  OPERATOR_BATTERY_SEED=1 \
  python3 data/rule_logic_operator_battery_v4/build_operator_battery_v4.py
```

Shortcut gate:

```text
verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
exact_lookup_accuracy_milli: 0
proof_rule_id_majority_accuracy_milli: 0
markov_bigram_pairwise_accuracy_milli: 500
bayesian_cooccurrence_pairwise_accuracy_milli: 500
l2_neighbor_target_copy_accuracy_milli: 0
same_bag_derangement_milli: 1000
```

Runtime log:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_density_sweep/seed_001/train_per_cell_4/conditional_runtime_gate_release.log
```

Metrics:

```text
rows: 3840
train_rows: 3072
heldout_rows: 768
conditional_slot_ordered_sequence_accuracy_milli: 611
conditional_sequence_energy_accuracy_milli: 969
conditional_energy_pass_slot_fail: 275
conditional_output_slot_cleanup_failed_slots: 1052
conditional_slot_failure_by_noise:
  clean: 0
  distractor: 0
  prefix_suffix: 531
  instruction_noise: 521
conditional_slot_accuracy_by_noise:
  clean: 1000
  distractor: 1000
  prefix_suffix: 779
  instruction_noise: 783
state_delta_edges: 0
role_binding_edges: 86562
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
```

Forbidden substitutions stayed false:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Core ablations remain useful:

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_condition_accuracy_milli: 0
ablation_without_condition_action_accuracy_milli: 44
ablation_without_role_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

## Comparison

```text
train_per_cell=2:
  strict slot: 617
  sequence energy: 973
  output-slot failures: 1146
  prefix_suffix accuracy: 761
  instruction_noise accuracy: 761
  role_binding_edges: 81739

train_per_cell=4:
  strict slot: 611
  sequence energy: 969
  output-slot failures: 1052
  prefix_suffix accuracy: 779
  instruction_noise accuracy: 783
  role_binding_edges: 86562
```

## Interpretation

Increasing conditional train density from 2 to 4 examples per matrix cell does
not close the strict ordered slot gate. It slightly improves noisy-slot
accuracy but does not improve row-level strict accuracy or sequence energy.

The clean and distractor cases are already perfect:

```text
clean: 1000
distractor: 1000
```

The remaining failure is specifically noise-wrapper sensitive:

```text
prefix_suffix
instruction_noise
```

This points away from simple corpus sparsity and toward conditional branch
readout instability when extra state/action surface atoms are present. The
operator energy remains strong, but slot-local binding is still contaminated by
noise-conditioned role collisions.

## Claim Boundary

Allowed claim:

```text
Doubling full-corpus conditional train density from 2 to 4 does not fix seed1
conditional strict readout. The red gate is now best classified as
noise-robust conditional decoder debt, not simple train-density sparsity.
```

Not allowed:

```text
all density options exhausted
conditional solved
v4 complete
architecture must change immediately
```

## Next Diagnostic

The next check should isolate the noise channel rather than increase all
training density blindly:

```text
1. run conditional clean+distractor only as a green control;
2. run prefix_suffix only and instruction_noise only as red controls;
3. inspect which active centers are introduced by the noisy wrappers;
4. test whether filtering non-operator wrapper centers before L3 preserves
   shortcut gates and ablation proof.
```

## Follow-Up: Noise Isolation

The noise-isolation diagnostic was run after this density sweep:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_noise_isolation/seed_001/CONDITIONAL_NOISE_ISOLATION_REPORT.md
```

Result:

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

Updated diagnosis after code audit:

```text
The isolated runs are red, but wrapper text is not proven to enter role slots:
sequence_source_tokens extracts only the explicit sequence segment. The
diagnosis is therefore conditional role/readout instability under
noise-correlated schedule and surface pressure, not direct wrapper-center
contamination. A paired-noise corpus is required before adding any wrapper
filter/gating mechanism.
```
