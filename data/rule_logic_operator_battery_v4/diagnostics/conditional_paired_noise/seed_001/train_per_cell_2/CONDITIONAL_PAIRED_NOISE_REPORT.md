# Conditional Paired-Noise Report

Date: 2026-07-02

## Verdict

`WRAPPER_TEXT_NOT_THE_CAUSE_ROLE_SURFACE_COLLISION_CONFIRMED`

This diagnostic rebuilds the conditional corpus with `OPERATOR_BATTERY_PAIRED_NOISE=1`.
The same semantic row is emitted under every noise type without advancing
`task_index`.

This is a corpus/diagnostic change only. It does not change runtime,
architecture, learning rule, or forbidden flags.

## Corpus

Build artifact:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_paired_noise/seed_001/train_per_cell_2/accepted_operator_tasks_v4.jsonl
```

Manifest:

```text
rows: 2304
train_rows: 1536
heldout_rows: 768
paired_noise: true
train_per_cell: 2
heldout_per_cell: 1
noise_types: clean, prefix_suffix, distractor, instruction_noise
correct_wrong_same_bag_milli: 1000
```

Pairing check:

```text
semantic_groups: 576
each semantic group has exactly:
  clean
  distractor
  instruction_noise
  prefix_suffix
bad_groups: 0
```

## Shortcut Gate

Shortcut report:

```text
shortcut_gate_report.json
shortcut_gate_report.log
```

Metrics:

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

## Runtime Gate

Runtime log:

```text
conditional_runtime_gate_release.log
```

Metrics:

```text
conditional_slot_ordered_sequence_accuracy_milli: 615
conditional_sequence_energy_accuracy_milli: 958
conditional_energy_pass_slot_fail: 264
conditional_output_slot_cleanup_failed_slots: 1124
conditional_slot_accuracy_by_noise:
  clean: 883
  distractor: 883
  instruction_noise: 883
  prefix_suffix: 883
conditional_slot_accuracy_by_surface:
  business: 1000
  network: 763
  ru_words: 768
  symbols: 1000
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
state_delta_edges: 0
role_binding_edges: 72917
```

Forbidden substitutions stayed false:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Core ablations stayed useful:

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

## Static Collision Diagnostic

Static collision log:

```text
conditional_static_collision_report.log
```

This uses the same Rust `SurfaceWave4096` and existing
`folded_collision_report` path as the runtime test.

Metrics:

```text
overall:
  target_impulses_checked: 491488
  multi_role_hit_milli: 174
  wrong_role_hit_milli: 174
  missing_true_role_milli: 0

by_surface:
  symbols:
    multi_role_hit_milli: 140
    wrong_role_hit_milli: 140
    missing_true_role_milli: 0
  business:
    multi_role_hit_milli: 170
    wrong_role_hit_milli: 170
    missing_true_role_milli: 0
  ru_words:
    multi_role_hit_milli: 192
    wrong_role_hit_milli: 192
    missing_true_role_milli: 0
  network:
    multi_role_hit_milli: 196
    wrong_role_hit_milli: 196
    missing_true_role_milli: 0
```

Interpretation:

```text
ru_words/network have the highest folded role collision pressure and are the
only red surface families. This supports the collision diagnosis.

But business has 170 milli collision and still reaches 1000 strict accuracy, so
raw folded collision alone is not a complete explanation. The next diagnostic
must correlate collision class with actual gap/failure by output-source pair.
```

## Runtime Collision Outcome Diagnostic

Runtime collision-outcome log:

```text
conditional_runtime_gate_release_collision_outcome.log
```

This report correlates the same folded role-collision pressure with actual
strict-slot failures and sequence-energy pass / slot-fail cases.

Top-level bucket result:

```text
high_wrong_role_hit:
  slots: 2420
  failed_slots: 376
  accuracy_milli: 845
  energy_pass_slot_fail: 292
  avg_wrong_role_hit_milli: 360273

mid_wrong_role_hit:
  slots: 3564
  failed_slots: 432
  accuracy_milli: 879
  energy_pass_slot_fail: 344
  avg_wrong_role_hit_milli: 177359

low_wrong_role_hit:
  slots: 2488
  failed_slots: 188
  accuracy_milli: 924
  energy_pass_slot_fail: 168
  avg_wrong_role_hit_milli: 82494

no_wrong_role_hit:
  slots: 1128
  failed_slots: 128
  accuracy_milli: 887
  energy_pass_slot_fail: 96
```

By surface:

```text
business:
  accuracy_milli: 1000
  failed_slots: 0
  avg_gap: 133505
  min_gap: 29230
  avg_wrong_role_hit_milli: 170753

symbols:
  accuracy_milli: 1000
  failed_slots: 0
  avg_gap: 129630
  min_gap: 28976
  avg_wrong_role_hit_milli: 132145

network:
  accuracy_milli: 763
  failed_slots: 568
  avg_gap: 29850
  min_gap: -45024
  avg_wrong_role_hit_milli: 241877

ru_words:
  accuracy_milli: 768
  failed_slots: 556
  avg_gap: 29090
  min_gap: -53964
  avg_wrong_role_hit_milli: 167397
```

Worst output/source pairs:

```text
out10->src7:  accuracy_milli 0,   slots 24, failed_slots 24
out1->src11:  accuracy_milli 0,   slots 8,  failed_slots 8
out12->src2:  accuracy_milli 0,   slots 8,  failed_slots 8
out12->src6:  accuracy_milli 0,   slots 8,  failed_slots 8
out12->src7:  accuracy_milli 0,   slots 8,  failed_slots 8
out14->src8:  accuracy_milli 0,   slots 8,  failed_slots 8
out2->src14:  accuracy_milli 0,   slots 8,  failed_slots 8
out6->src11:  accuracy_milli 0,   slots 8,  failed_slots 8
out6->src12:  accuracy_milli 167, slots 48, failed_slots 40
out2->src0:   accuracy_milli 208, slots 96, failed_slots 76
out1->src0:   accuracy_milli 250, slots 96, failed_slots 72
out0->src8:   accuracy_milli 250, slots 32, failed_slots 24
```

Interpretation:

```text
Wrong-role collision pressure is real and has outcome signal:
high_wrong_role_hit is worse than low_wrong_role_hit.

But collision is still not sufficient by itself. Business remains perfect even
with non-trivial wrong-role pressure, while ru_words/network fail with much
lower average gaps and negative min gaps.

The blocker is therefore narrower:
surface-token identity plus folded role collision plus weak strict-slot cleanup
on specific output/source pairs.
```

## Target/Wrong Lane Overlap Diagnostic

Static overlap log:

```text
conditional_target_wrong_overlap_report.log
```

This diagnostic compares the correct token and same-bag wrong token in folded
L1 lanes, and separately measures how much the wrong token hits the true role
slot.

By surface:

```text
business:
  slots: 7200
  avg_target_wrong_overlap_milli: 22061
  avg_wrong_hits_true_role_milli: 22395
  avg_target_hits_wrong_role_milli: 188380
  avg_target_missing_true_role_milli: 0

network:
  slots: 7200
  avg_target_wrong_overlap_milli: 28574
  avg_wrong_hits_true_role_milli: 28147
  avg_target_hits_wrong_role_milli: 226118
  avg_target_missing_true_role_milli: 0

ru_words:
  slots: 7200
  avg_target_wrong_overlap_milli: 24147
  avg_wrong_hits_true_role_milli: 23987
  avg_target_hits_wrong_role_milli: 210387
  avg_target_missing_true_role_milli: 0

symbols:
  slots: 7200
  avg_target_wrong_overlap_milli: 123698
  avg_wrong_hits_true_role_milli: 123841
  avg_target_hits_wrong_role_milli: 244140
  avg_target_missing_true_role_milli: 0
```

Worst overlap/crosstalk pairs by static risk:

```text
out9->src7
out1->src13
out4->src6
out2->src14
out14->src1
out12->src9
out0->src15
out6->src6
out3->src11
out11->src1
out5->src15
out0->src14
```

Interpretation:

```text
Pure target/wrong lane overlap is not the active cause. Symbols have the
largest target/wrong overlap and wrong-token true-role hits, but symbols remain
1000 strict-slot accurate.

The red surfaces fail when folded role collision and surface-token pressure
combine with low runtime gap. Static overlap alone is not enough to explain or
repair the conditional gate.
```

## Candidate Cleanup Probe

Runtime logs:

```text
conditional_runtime_gate_release_candidate_cleanup2.log
conditional_runtime_gate_release_candidate_cleanup8.log
```

This probe enables the existing generic candidate cleanup path:

```text
OPERATOR_BATTERY_V4_CONDITIONAL_CANDIDATE_CLEANUP_EPOCHS=2
OPERATOR_BATTERY_V4_CONDITIONAL_CANDIDATE_CLEANUP_EPOCHS=8
```

The cleanup trains each correct output slot against all other source-token
candidates from the same `state_before`. It does not use `proof_rule_id`,
`target_center_id`, concrete answer lookup, or manual `local_out_t`.

Candidate cleanup dynamics:

```text
candidate_slot_tasks: 228864

epochs=2:
  epoch 1: repaired_slots 37, touched_edges 17165, min_candidate_gap -30090
  epoch 2: repaired_slots 9,  touched_edges 5003,  min_candidate_gap -6984
  role_binding_edges: 73559

epochs=8:
  epoch 1: repaired_slots 37, touched_edges 17165, min_candidate_gap -30090
  epoch 2: repaired_slots 8,  touched_edges 4703,  min_candidate_gap -6984
  epoch 3: repaired_slots 0,  touched_edges 0,     min_candidate_gap 216
  epoch 4: repaired_slots 1,  touched_edges 300,   min_candidate_gap 216
  epoch 5: repaired_slots 0,  touched_edges 0,     min_candidate_gap 328
  epoch 6: repaired_slots 0,  touched_edges 0,     min_candidate_gap 328
  epoch 7: repaired_slots 0,  touched_edges 0,     min_candidate_gap 328
  epoch 8: repaired_slots 0,  touched_edges 0,     min_candidate_gap 328
  role_binding_edges: 73559
```

Heldout result after candidate cleanup:

```text
conditional_slot_ordered_sequence_accuracy_milli: 615
conditional_sequence_energy_accuracy_milli: 984
conditional_output_slot_cleanup_failed_slots: 972
conditional_energy_pass_slot_fail: 284

conditional_slot_accuracy_by_surface:
  business: 1000
  network: 787
  ru_words: 808
  symbols: 1000

conditional_slot_accuracy_by_noise:
  clean: 899
  distractor: 899
  instruction_noise: 899
  prefix_suffix: 899

flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
state_delta_edges: 0
role_binding_edges: 73559
```

Forbidden substitutions stayed false:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Interpretation:

```text
Candidate cleanup is a valid generic pressure path and it improves slot-level
cleanup:

  failed_slots: 1124 -> 972
  network strict-slot surface accuracy: 763 -> 787
  ru_words strict-slot surface accuracy: 768 -> 808
  sequence energy: 958 -> 984

But it does not move row-level strict ordered accuracy:

  strict row accuracy remains 615

The train-candidate problem is exhausted by epoch 3/8, yet heldout rows still
fail on the same ru_words/network surfaces. Therefore the remaining blocker is
not insufficient train-candidate negatives. It is a transfer problem in the
role-binding readout: heldout token surface/sign pressure still creates
negative runtime gaps for specific output/source pairs.
```

## Sign-Aware Collision Diagnostic

Runtime log:

```text
conditional_sign_aware_positive_collision_report_raw.log
```

This diagnostic does not change runtime behavior. It asks a narrower question:
if role matching preserved the source-lane polarity/sign for positive target
impulses, would the folded wrong-role collisions disappear?

Surface-level result:

```text
business:
  current_wrong_role_hit_milli: 170
  sign_aware_wrong_role_hit_milli: 154
  sign_erased_wrong_role_hit_milli: 16
  missing_true_signed_role_milli: 0

network:
  current_wrong_role_hit_milli: 196
  sign_aware_wrong_role_hit_milli: 178
  sign_erased_wrong_role_hit_milli: 18
  missing_true_signed_role_milli: 0

ru_words:
  current_wrong_role_hit_milli: 192
  sign_aware_wrong_role_hit_milli: 179
  sign_erased_wrong_role_hit_milli: 13
  missing_true_signed_role_milli: 0

symbols:
  current_wrong_role_hit_milli: 140
  sign_aware_wrong_role_hit_milli: 122
  sign_erased_wrong_role_hit_milli: 18
  missing_true_signed_role_milli: 0
```

Worst output/source pairs remain high even with sign-aware matching:

```text
out2->src14: current 500, sign-aware 444
out0->src15: current 474, sign-aware 461
out14->src1: current 471, sign-aware 471
out0->src10: current 355, sign-aware 345
out12->src2: current 314, sign-aware 286
```

Same-sign residual collision share:

```text
business: 154 / 170 = 906 milli of current wrong-role pressure remains
network:  178 / 196 = 908 milli remains
ru_words: 179 / 192 = 932 milli remains
symbols:  122 / 140 = 871 milli remains

out14->src1: 471 / 471 = 1000 milli remains
out0->src15: 461 / 474 = 973 milli remains
out2->src14: 444 / 500 = 888 milli remains
```

Interpretation:

```text
Preserving source sign would be safe for positive target lanes:
missing_true_signed_role_milli is 0 on every surface.

But sign-aware matching removes only 13-18 milli of wrong-role pressure by
surface and leaves the worst output/source pairs heavily collided. Therefore a
simple sign-aware role page or sign key is not enough to solve the conditional
strict-slot red gate by itself.

The remaining failure is not only sign erasure. It is same-sign folded collision
and surface identity pressure inside specific output/source role transfers. The
same-sign residual is the main measured target, not sign erasure.
```

## Residual Collision Outcome Diagnostic

Runtime log:

```text
conditional_residual_collision_outcome_cleanup8.log
```

This diagnostic reproduces the current conditional training path, enables the
existing candidate cleanup for 8 epochs, and then correlates sign-aware residual
collision with real heldout strict-slot gaps.

Boundary:

```text
The epochs here belong to the compiler/trainer proof stage, not runtime
inference. They build and audit the compact role-binding table. They are not a
production-time reasoning loop and must not be used to hide a lookup.
```

Training/compiler trace:

```text
local training:
  epochs: 8
  final role_binding_edges: 72917

cleanup:
  epochs: 4
  repaired_rows: 0 on all epochs
  train_slot_accuracy_milli: 1000
  train_energy_accuracy_milli: 1000
  min_slot_gap: 25606

candidate cleanup:
  candidate_slot_tasks: 228864
  epoch 1: repaired_slots 37, min_candidate_gap -30090
  epoch 2: repaired_slots 8,  min_candidate_gap -6984
  epoch 3: repaired_slots 0,  min_candidate_gap 216
  epoch 4: repaired_slots 1,  min_candidate_gap 216
  epoch 5-8: repaired_slots 0, min_candidate_gap 328
  final role_binding_edges: 73559
```

Heldout outcome after train/candidate cleanup is saturated:

```text
strict_row_accuracy_milli: 615
sequence_energy_accuracy_milli: 984
energy_pass_slot_fail: 284
```

Residual collision buckets:

```text
high_same_sign_residual:
  slots: 2036
  failed_slots: 288
  accuracy_milli: 859
  avg_gap: 60368
  min_gap: -26308

mid_same_sign_residual:
  slots: 3376
  failed_slots: 372
  accuracy_milli: 890
  avg_gap: 83592
  min_gap: -45024

low_same_sign_residual:
  slots: 2784
  failed_slots: 204
  accuracy_milli: 927
  avg_gap: 89711
  min_gap: -53964

no_same_sign_residual:
  slots: 1404
  failed_slots: 108
  accuracy_milli: 923
  avg_gap: 77571
  min_gap: -42700
```

Surface outcome:

```text
business:
  accuracy_milli: 1000
  avg_gap: 130670
  min_gap: 33022

symbols:
  accuracy_milli: 1000
  avg_gap: 127026
  min_gap: 38630

network:
  accuracy_milli: 787
  avg_gap: 30599
  min_gap: -45024

ru_words:
  accuracy_milli: 808
  avg_gap: 29947
  min_gap: -53964
```

Worst output/source pairs:

```text
out2->src14: accuracy 0, avg_gap -5544
out12->src2: accuracy 0, avg_gap -22138
out12->src6: accuracy 0, avg_gap -826
out1->src11: accuracy 0, avg_gap -4512
out6->src11: accuracy 0, avg_gap -21416
out6->src12: accuracy 167, avg_gap 1910
out10->src7: accuracy 167, avg_gap -5318
out1->src0: accuracy 250, avg_gap -16047
```

Interpretation:

```text
Train and train-candidate pressure are saturated, but heldout strict readout is
still red. Therefore more epochs on the same train objective are not the answer.

Same-sign residual collision is real and often appears in the worst failures,
but it is not sufficient by itself:
  business/symbols remain 1000 even with residual pressure;
  no_same_sign_residual still has failed slots.

The strongest diagnosis is now:
  global sequence energy can select the right operator;
  local role/filler slot readout is noisy under ru_words/network surface
  pressure;
  the missing mechanism is a generic cleanup/readout stage or learned
  role-specific disambiguation, not local_out_t, target_id, or proof_rule_id
  authority.
```

## Interpretation

The key result is the identical noise accuracy:

```text
clean = distractor = instruction_noise = prefix_suffix = 883
```

Together with the code audit:

```text
sequence_source_tokens extracts only the explicit sequence segment.
```

This rules out direct wrapper-text contamination as the cause of the red gate.
The wrapper strings do not explain the strict decoder failure.

The remaining red signal is surface-specific:

```text
business: 1000
symbols: 1000
network: 763
ru_words: 768
```

So the current blocker is better classified as:

```text
conditional role/filler collision under richer token surfaces, not noise wrapper
contamination.
```

Sequence energy is still strong:

```text
sequence energy: 958
strict slot: 615
```

This preserves the earlier boundary: the operator signal exists, but strict
ordered slot readout is still not stable on ru_words/network surfaces.

## Claim Boundary

Allowed claim:

```text
Paired-noise proves that wrapper text is not the active cause of conditional
strict-slot failure. The red gate is now isolated to role/filler collision and
surface-token pressure, especially ru_words/network, while sequence energy and
field/flat parity remain strong.
```

Not allowed:

```text
conditional solved
noise wrappers caused the red gate
local_out_t needed
architecture ceiling proven
```

## Next Diagnostic

Do not change architecture yet. The next proof-debt is:

```text
1. Treat same-sign residual collision as a contributing pressure, not the sole
   cause.
2. Test a generic cleanup/readout candidate against the energy/readout mismatch:
   if sequence energy is correct but slot gap is red, cleanup must repair strict
   slot readout without knowing the answer.
3. Test learned role-specific disambiguation only if it is induced from
   rule_action/state structure and collapses under ablation.
4. Keep the compiler/runtime boundary strict: epochs are allowed for table
   induction and diagnostics, never as inference-time reasoning.
```
