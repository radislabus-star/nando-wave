# Conditional Noise Isolation Report

Date: 2026-07-02

## Verdict

`CONDITIONAL_NOISE_SCHEDULE_INSTABILITY_CONFIRMED`

This diagnostic isolates the conditional red gate by noise channel. It does not
change architecture, learning rule, runtime readout, corpus semantics, shortcut
policy, or forbidden flags.

The earlier full-corpus breakdown showed clean rows green and noisy rows weak.
This isolation pass refines that claim:

```text
clean alone inside the clean+distractor run is green;
distractor isolated from prefix_suffix/instruction_noise support is red;
prefix_suffix is red;
instruction_noise is red;
```

Code audit after the run found an important boundary:

```text
sequence_source_tokens(row.state_before) extracts only the segment after
`sequence:` and before `;`.
```

Therefore wrapper text such as `note_*`, `tail operator_probe`, `please
transform carefully`, or `distractor:` is not directly inserted into role slots.
The current diagnostic is still useful, but it is not a pure wrapper-center A/B
test. It is a noise-correlated schedule/density slice.

## Corpora

Artifacts:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_noise_isolation/seed_001/clean_distractor/accepted_operator_tasks_v4.jsonl
data/rule_logic_operator_battery_v4/diagnostics/conditional_noise_isolation/seed_001/prefix_suffix/accepted_operator_tasks_v4.jsonl
data/rule_logic_operator_battery_v4/diagnostics/conditional_noise_isolation/seed_001/instruction_noise/accepted_operator_tasks_v4.jsonl
```

Rows:

```text
clean_distractor: 1152 rows, 768 train, 384 heldout
prefix_suffix:    576 rows, 384 train, 192 heldout
instruction_noise:576 rows, 384 train, 192 heldout
```

## Shortcut Gates

All three isolated corpora pass the shortcut gate:

```text
exact_lookup_accuracy_milli: 0
l2_neighbor_target_copy_accuracy_milli: 0
proof_rule_id_majority_accuracy_milli: 0
markov_bigram_pairwise_accuracy_milli: 500
bayesian_cooccurrence_pairwise_accuracy_milli: 500
bag_of_tokens_accuracy_milli: 500
same_bag_derangement_milli: 1000
output_position_prior_accuracy_milli: 21
```

Reports:

```text
clean_distractor/shortcut_gate_report.json
prefix_suffix/shortcut_gate_report.json
instruction_noise/shortcut_gate_report.json
```

## Runtime Results

### clean_distractor

Log:

```text
clean_distractor/conditional_runtime_gate_release.log
```

Metrics:

```text
conditional_slot_ordered_sequence_accuracy_milli: 607
conditional_sequence_energy_accuracy_milli: 956
conditional_energy_pass_slot_fail: 134
conditional_output_slot_cleanup_failed_slots: 573
conditional_slot_accuracy_by_noise:
  clean: 1000
  distractor: 761
conditional_slot_accuracy_by_surface:
  business: 880
  network: 881
  ru_words: 876
  symbols: 886
role_binding_edges: 79220
state_delta_edges: 0
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
```

### prefix_suffix

Log:

```text
prefix_suffix/conditional_runtime_gate_release.log
```

Metrics:

```text
conditional_slot_ordered_sequence_accuracy_milli: 594
conditional_sequence_energy_accuracy_milli: 953
conditional_energy_pass_slot_fail: 69
conditional_output_slot_cleanup_failed_slots: 293
conditional_slot_accuracy_by_noise:
  prefix_suffix: 878
conditional_slot_accuracy_by_surface:
  business: 1000
  network: 752
  ru_words: 760
  symbols: 1000
role_binding_edges: 73590
state_delta_edges: 0
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
```

### instruction_noise

Log:

```text
instruction_noise/conditional_runtime_gate_release.log
```

Metrics:

```text
conditional_slot_ordered_sequence_accuracy_milli: 594
conditional_sequence_energy_accuracy_milli: 953
conditional_energy_pass_slot_fail: 69
conditional_output_slot_cleanup_failed_slots: 293
conditional_slot_accuracy_by_noise:
  instruction_noise: 878
conditional_slot_accuracy_by_surface:
  business: 1000
  network: 752
  ru_words: 760
  symbols: 1000
role_binding_edges: 73590
state_delta_edges: 0
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
```

Forbidden substitutions stayed false in all three runtime runs:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Core ablations stayed useful in all three runtime runs:

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

## Interpretation

The conditional operator signal is present but not yet a stable strict decoder:

```text
sequence energy remains high: 953..956
strict slot remains red: 594..607
flat parity remains exact: 0 mismatches
```

The isolated `prefix_suffix` and `instruction_noise` runs are numerically
identical in the key metrics. They also share the same surface weakness:

```text
business: 1000
symbols: 1000
ru_words: ~760
network: ~752
```

That points to a role/filler collision problem in richer token surfaces, not a
failure of the rule-action branch alone. It does not prove that wrapper text
itself entered the active fringe.

The isolated `clean_distractor` result is also important. The clean rows are
green, but distractor rows fall to 761. However, this is not a same-row wrapper
comparison: the generator increments `task_index` across `noise_type`, so the
different noise slices also carry different source-token schedules. The clean
rows also receive extra training density from the paired distractor rows.
Therefore:

```text
full-corpus mixing can help some noise-correlated slices;
isolated slices expose decoder fragility;
simple train-density increase is not enough;
```

## Updated Claim Boundary

Allowed claim:

```text
Conditional v4 has strong operator-energy signal and exact field/flat parity,
but strict slot readout is unstable under isolated noise-correlated slices. The
red gate is best classified as conditional role/readout instability under
noise-correlated schedule and surface pressure, with failures concentrated in
ru_words/network and early output slots.
```

## Paired-Noise Follow-Up

The proper paired-noise corpus was built after this report:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_paired_noise/seed_001/train_per_cell_2/CONDITIONAL_PAIRED_NOISE_REPORT.md
```

Result:

```text
rows: 2304
semantic_groups: 576
each semantic group has clean/distractor/instruction_noise/prefix_suffix
shortcut gate: clean
strict slot: 615
sequence energy: 958
noise accuracy:
  clean: 883
  distractor: 883
  instruction_noise: 883
  prefix_suffix: 883
surface accuracy:
  business: 1000
  symbols: 1000
  network: 763
  ru_words: 768
flat parity mismatches: 0
forbidden flags: false
```

This supersedes any claim that wrapper text itself is the active cause. The
paired run shows identical behavior across noise variants of the same semantic
row. The remaining blocker is role/filler collision under richer token
surfaces, not wrapper contamination.

Not allowed:

```text
conditional solved
all distractor rows are inherently green
prefix_suffix is the only failing noise type
wrapper centers proven to enter role slots
architecture ceiling proven
local_out_t needed
```

## Next Diagnostic

Do not change architecture yet. The next runnable proof-debt is:

```text
1. build a paired-noise corpus where the same semantic row/source_tokens is
   emitted under every noise_type without advancing task_index;
2. compare active centers for identical rows across clean/distractor/
   prefix_suffix/instruction_noise;
3. confirm whether the active fringe is identical across wrappers, as the code
   audit predicts;
4. measure role/filler lane collisions by surface family and output slot;
5. test any filtering/gating channel only if the paired-noise gate proves it;
6. keep local_out_t, target_id, proof_rule_id authority, and lookup forbidden.
```
