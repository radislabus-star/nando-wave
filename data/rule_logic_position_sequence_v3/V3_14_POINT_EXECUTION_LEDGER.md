# V3 14-Point Execution Ledger

Purpose: close the remaining Wave L3 proof debts without shortcuts.

Hard no-go rules:

- no concrete-X lookup;
- no `target_center_id` or `proof_rule_id` authority as training answer;
- no manual `local_out_t` / fixed output-time hack;
- no fixed frame/template id as proof of reasoning;
- red gates stay red until measured green.

Current best baseline:

- corpus: `data/rule_logic_position_sequence_v3/accepted_position_sequence_tasks_v3.jsonl`
- rows: 2520
- train rows: 1680
- heldout rows: 840
- matrix cells: 840
- proof rule ids: 42
- shortcut verdict: `VALID_POSITION_SEQUENCE_V3_CANDIDATE`
- output-position prior shortcut: 24 milli
- strict slot ordered accuracy: 507 milli
- sequence energy accuracy: 957 milli
- sequence energy median gap: 151224
- sequence energy p10 gap: 27348
- flat slot gap parity mismatches: 0
- ablation without binding: 0 milli
- current claim: strong sequence-level operator signal, not a full ordered decoder.

## 1. Report Consistency

Mini-plan:

- separate old v3 baseline metrics from current best metrics;
- sync JSON reports with live manifest, shortcut report, static diagnostics, and current best gate log;
- keep old failures as history, not current claim.

Plan critique:

- a blind search/replace would destroy useful historical evidence;
- the key risk is mixing old `269/831/2880/960` with current `507/957/2520/840`.

Improved plan:

- mark old v3 numbers as `historical original baseline`;
- make `baseline_v3_report.json` describe the current best report;
- keep `DIAGNOSTIC_RUNS.md` as the full run history.

Status: completed.

Evidence:

- `baseline_v3_report.json` now uses output-position prior 24 milli and current best failure breakdown.
- `l3_binding_pressure_training_report.json` now records current best static diagnostics: 42 action vectors, different rule similarity 506 milli, max different rule similarity 969 milli, folded missing true role 0 milli.
- `WAVE_LLM_LAYERS_LIVE_PLAN.md` now labels old 2880-row `269/831` data as historical/superseded and current best as `507/957`.

## 2. Sequence-Energy Runtime Parity

Mini-plan:

- add flat/runtime parity for sequence-energy decision, not only local slot readout;
- compare per-row sequence energy gap between field table and compiled flat table;
- fail the gate if energy parity diverges.

Plan critique:

- accuracy parity alone can hide margin/gap drift;
- per-row gap equality is stronger and cheaper than a new architecture change.

Improved plan:

- implement a diagnostic counter: checked rows, mismatches, max absolute gap delta;
- print it in the explicit ignored v3 gate;
- keep the old slot parity metric too.

Status: completed.

Evidence:

- Added flat sequence-energy parity to the Rust v3 gate.
- Explicit ignored v3 gate prints:
  - `flat_sequence_energy_parity_checked_rows: 840`
  - `flat_sequence_energy_parity_mismatches: 0`
  - `flat_sequence_energy_parity_max_abs_gap_delta: 0`
- The gate still fails correctly on strict ordered decoding (`507 != 1000`), but the compiled flat sequence-energy path is now proven identical to field energy for the current heldout set.

## 3. Mirror/Symmetry Operator Consistency

Mini-plan:

- isolate `full_mirror_*` and `pair_swap_len5` rows;
- report strict slot accuracy and sequence-energy accuracy only on this subset;
- test whether failures are action separability, folded collision, or symmetry ambiguity.

Plan critique:

- a tiny mirror-only corpus can become shortcut-solvable by output-position prior;
- this must be a diagnostic slice, not a proof corpus.

Improved plan:

- evaluate mirror rows inside the full valid v3 matrix;
- add per-rule/per-length energy gap diagnostics before changing mechanism.

Status: completed as a red diagnostic gate.

Evidence:

- Added symmetry/non-symmetry diagnostics to the explicit v3 gate.
- Current heldout split:
  - symmetry rows: 220
  - symmetry strict accuracy: 145 milli
  - symmetry sequence-energy accuracy: 836 milli
  - symmetry p10 energy gap: -13864
  - non-symmetry rows: 620
  - non-symmetry strict accuracy: 635 milli
  - non-symmetry sequence-energy accuracy: 1000 milli
  - non-symmetry p10 energy gap: 54150
- Conclusion: sequence-level operator energy is solved for non-symmetry rows on this corpus, but mirror/pair-swap consistency remains a real operator debt.

Next action:

- Do not add manual output phase.
- Use point 5 combined objective or point 12 collision audit to explain why symmetric inverse mappings collapse.

## 4. Full Ordered Decoder / Slot Readout

Mini-plan:

- explain why sequence energy 957 does not translate into strict slot 1000;
- identify bad output slots and bad local gaps;
- try cleanup only if it preserves no-lookup/no-phase-hack rules.

Plan critique:

- making sequence energy the only answer would dodge ordered construction;
- local slot correctness is still required for a real decoder.

Improved plan:

- treat sequence energy as global judge and slot readout as decoder debt;
- add combined local/global objective only after parity and symmetry diagnostics.

Status: completed as decoder diagnosis, not solved as final decoder.

Evidence:

- Strict slot ordered accuracy remains 507 milli while sequence energy is 957 milli.
- `energy_pass_slot_fail: 378`: many rows have the correct whole-sequence energy direction but fail at least one local output slot.
- Normalized slot accuracy:
  - slot 0: 806 milli
  - slot 1: 900 milli
  - slot 2: 894 milli
  - slot 3: 891 milli
  - slot 4: 852 milli
  - slot 5: 883 milli
  - slot 6: 906 milli
  - slot 7: 1000 milli
- Conclusion: decoder/readout debt is local-slot pressure, especially slot 0 and slot 4, while sequence-level operator energy is much stronger.

Next action:

- Use point 5 to combine local slot binding with global sequence-energy cleanup.
- Use point 7 to separate slot geometry from data artifact.

## 5. Combined Objective

Mini-plan:

- test local role/filler binding plus global sequence/operator energy cleanup;
- do not replace local binding with naive energy-only training.

Plan critique:

- naive energy-only already failed relative to current best;
- the combined objective must repair rows that energy passes but slot fails.

Improved plan:

- use local slot updates first, then a sequence-energy cleanup pass over same-bag wrong rows;
- record whether `energy_pass_slot_fail` decreases without lowering energy accuracy.

Status: completed as positive partial mechanism, not final proof.

Evidence:

- Implemented `ordered_position_binding_v3_combined_objective_probe_must_preserve_flat_energy_parity`.
- Combined training shape:
  - local role/filler slot training first;
  - sequence/operator energy cleanup second;
  - no `target_id`;
  - no `proof_rule_id` authority;
  - no concrete-X lookup;
  - no manual `local_out_t`;
  - `state_delta_edges: 0`.
- Explicit ignored probe result:
  - strict slot ordered accuracy: 705 milli
  - flat strict slot ordered accuracy: 705 milli
  - sequence energy accuracy: 988 milli
  - sequence energy p10 gap: 36878
  - energy-pass slot-fail rows: 238
  - symmetry sequence-energy accuracy: 955 milli
  - symmetry p10 energy gap: 9416
  - non-symmetry sequence-energy accuracy: 1000 milli
  - flat sequence-energy parity mismatches: 0
  - flat slot-gap parity mismatches: 0
  - role-binding edges: 16357

Delta from local-only current best:

- strict slot: 507 -> 705
- sequence energy: 957 -> 988
- symmetry sequence energy: 836 -> 955
- energy-pass slot-fail: 378 -> 238

Conclusion:

- Combined local+global objective is a real useful direction.
- It is not yet final proof because strict ordered slot readout remains below 1000.

## 6. Channel Ablations

Mini-plan:

- remove action channel, role channel, slot channel, binding edges, conflict/anti-wave separately;
- map each channel to the metric it protects.

Plan critique:

- ablation must not mutate the corpus;
- a single all-or-nothing ablation is too coarse.

Improved plan:

- run channel ablations against the current best gate and mirror subset;
- report strict slot, sequence energy, and shortcut-like collapse separately.

Status: completed for the current best combined-objective probe.

Evidence:

- Explicit ignored probe:
  - `ablation_without_binding_accuracy_milli: 0`
  - `ablation_without_action_accuracy_milli: 0`
  - `ablation_without_action_energy_accuracy_milli: 0`
  - `ablation_without_role_accuracy_milli: 0`
  - `ablation_without_role_energy_accuracy_milli: 0`
  - `ablation_without_active_fringe_accuracy_milli: 0`
- Safety guards still hold:
  - `state_delta_edges: 0`
  - `target_center_id_training_used: false`
  - `proof_rule_id_training_authority_used: false`
  - `concrete_x_lookup_used: false`
  - `local_out_t_runtime_extension_used: false`

Conclusion:

- The combined-objective result depends on the intended role/filler binding,
  action, role, and active-fringe channels.
- This closes the channel-ablation support debt for the current probe.
- It does not close the final proof because strict ordered slot readout remains
  at 705 milli.

## 7. Output-Slot Failure Cleanup

Mini-plan:

- analyze failures by output slot;
- check whether low slots fail because of data distribution, folded collisions, or slot geometry.

Plan critique:

- high slots have fewer applicable rows for shorter lengths, so raw counts are misleading.

Improved plan:

- normalize by slot availability;
- compare clean vs collision-heavy rows for each slot.

Status: completed as diagnosis; corrective mechanism still open.

Evidence:

- Combined probe local slot accuracy:
  - row-level strict ordered accuracy: 705 milli
  - per-slot accuracy: 933 milli
  - failed local slots: 328
- Accuracy by output slot:
  - slot 0: 937 milli
  - slot 1: 949 milli
  - slot 2: 904 milli
  - slot 3: 968 milli
  - slot 4: 897 milli
  - slot 5: 892 milli
  - slot 6: 966 milli
  - slot 7: 1000 milli
- Symmetry rows are the main local decoder debt:
  - symmetry slot 2: 727 milli
  - symmetry slot 4: 800 milli
  - symmetry slot 5: 775 milli
  - non-symmetry slots are all 930 milli or higher except no severe collapse.

Conclusion:

- The ordered decoder is not globally broken. It is mostly a symmetry/local-slot
  disambiguation problem.
- The next allowed fix is stronger learned symmetry/operator consistency or
  better separability/collision cleanup, not manual `local_out_t`.

## 8. Attractor Basin Stability

Mini-plan:

- perturb inputs and action demos;
- measure radius where correct sequence energy remains positive.

Plan critique:

- too much noise turns the test into a corpus/noise benchmark instead of a basin benchmark.

Improved plan:

- sweep controlled perturbation levels and report gap decay curves.

Status: completed for deterministic active-fringe perturbations.

Evidence:

- Clean combined probe:
  - slot accuracy: 705 milli
  - energy accuracy: 988 milli
  - p10 energy gap: 36878
- Perturbation sweep:
  - `weaken_x2`: slot 683, energy 995, p10 energy gap 19326
  - `drop_mod_11`: slot 654, energy 985, p10 energy gap 26454
  - `drop_mod_7`: slot 624, energy 983, p10 energy gap 25494
  - `drop_mod_5`: slot 631, energy 980, p10 energy gap 23548
  - `drop7_distract8`: slot 626, energy 987, p10 energy gap 24894
  - `drop5_distract16`: slot 633, energy 980, p10 energy gap 23548

Conclusion:

- The global sequence/operator energy basin is robust under controlled
  active-fringe weakening, dropout, and distractors.
- Strict local slot readout degrades earlier than sequence energy.
- This strengthens the diagnosis: the current learned operator exists as a
  robust energy preference, but final ordered construction still needs better
  slot crystallization.

## 9. Energy Monotonicity / Boundedness

Mini-plan:

- track proxy energy before/after training or settle-like cleanup steps;
- verify correct-vs-wrong energy improves or remains bounded.

Plan critique:

- current code has energy score, not a formal thermodynamic energy function.

Improved plan:

- call it `proxy_sequence_energy`;
- test monotonic/bounded behavior only for that proxy until a formal energy is implemented.

Status: completed as proxy-energy diagnostic.

Evidence:

- Artifact:
  `data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/proxy_energy_monotonicity_report.json`
- Cleanup trace:
  - epoch 1: energy 961, p10 gap 25148, min gap -32876
  - epoch 2: energy 961, p10 gap 30572, min gap -20566
  - epoch 3: energy 965, p10 gap 26346, min gap -5602
  - epoch 4: energy 992, p10 gap 36536, min gap -168

Conclusion:

- p10 proxy energy is not strictly monotonic.
- Worst-row `min_energy_gap` improves monotonically toward zero.
- Energy accuracy is non-decreasing and p10 remains positive/bounded.
- This is not a formal thermodynamic proof; it is a useful bounded/improving
  diagnostic for the current sequence-energy proxy.

## 10. Capacity Curve

Mini-plan:

- vary operator count, length, edges, and memory;
- find where false positives or strict slot collapse begin.

Plan critique:

- capacity must be measured on valid shortcut-resistant corpora only.

Improved plan:

- generate controlled v3 variants and keep shortcut gates mandatory for every point.

Status: completed as current-field capacity slice; full retrain sweep remains optional follow-up.

Evidence:

- Artifact:
  `data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/capacity_curve_report.json`
- By length:
  - len3: slot 513, energy 925, p10 3718
  - len4: slot 729, energy 1000, p10 24034
  - len5: slot 700, energy 971, p10 33412
  - len6: slot 706, energy 1000, p10 73546
  - len7: slot 663, energy 1000, p10 115548
  - len8: slot 825, energy 1000, p10 165934
- By rule family:
  - `full_mirror`: slot 83, energy 917, p10 3488
  - `pair_swap`: slot 670, energy 1000, p10 38808
  - `even_odd_split`: slot 1000, energy 1000, p10 50996

Conclusion:

- The current field is not primarily length-capacity limited.
- The collapse axis is rule-family geometry, especially `full_mirror`.
- More memory or manual output phase is not the next justified fix.

## 11. Address-Radius Sweep

Mini-plan:

- vary surface input while preserving operator;
- measure when center recall and sequence energy break.

Plan critique:

- address-radius failure can come from L1/L2 recall, not L3 reasoning.

Improved plan:

- record L1/L2 center recall alongside L3 energy.

Status: completed as input-surface perturbation diagnosis.

Evidence:

- Artifact:
  `data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/address_radius_report.json`
- Perturbation sweep:
  - clean: slot 705, energy 988, p10 gap 36878
  - action_wrapped: slot 382, energy 904, p10 gap 1218
  - source_slot0_suffix: slot 682, energy 975, p10 gap 30694
  - source_all_suffix: slot 638, energy 983, p10 gap 12482
  - action_wrapped_source_slot0_suffix: slot 354, energy 889, p10 gap -4358

Conclusion:

- Source/role token address has a usable radius.
- Action/operator address is fragile.
- This supports action/operator separability as a real bottleneck and explains
  part of the mirror/symmetry debt.

## 12. Full Center Collision Audit L1/L2/L3

Mini-plan:

- audit lane collisions, motif merges, and polysemantic L3 edges;
- connect collisions to failed rows.

Plan critique:

- folded collision diagnostics alone are not enough.

Improved plan:

- produce row-level collision tags and compare clean vs collision-heavy accuracy.

Status: completed as collision audit v1; learned L2 motif ownership audit remains future work.

Evidence:

- Artifact:
  `data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/collision_audit_report.json`
- L1/action signature collision:
  - max different-rule similarity: 969 milli
  - strongest pair: `full_mirror_len3` vs `rotate_left_1_len3`
- L2/folded role pressure:
  - wrong-role hit: 64 milli
  - missing true role: 0 milli
- L3/role-binding polysemy:
  - flat nonzero role-binding edges: 14600
  - raw role-binding edges: 16357
  - action centers with edges: 1176
  - action centers with multi-slot edges: 1176
  - max slots per action center: 8

Conclusion:

- The true source role is not missing; the source/role side is present.
- The main collision axis is action/operator ambiguity.
- Compact L3 binding edges are useful, but become polysemantic when action
  signatures nearly collide.

## 13. Multi-Seed Robustness

Mini-plan:

- regenerate current best corpus with different seeds/order;
- rerun shortcut and v3 gates;
- check whether 957 energy is stable.

Plan critique:

- full gates are slow, so seed count must be staged.

Improved plan:

- start with 3 seeds, then expand only if variance is high.

Status: partial; sequence-energy robust across three seeds, strict decoder still not closed.

Evidence:

- Artifact:
  `data/rule_logic_position_sequence_v3/diagnostics/multi_seed_robustness_report.json`
- Builder now supports `POSITION_SEQUENCE_SEED` and `POSITION_SEQUENCE_OUTPUT_DIR`.
- Runtime seeds:
  - seed 1: sequence energy 992, strict slot 586, full_mirror energy 958
  - seed 2: sequence energy 987, strict slot 621, full_mirror energy 908
  - seed 3: sequence energy 974, strict slot 611, full_mirror energy 842
- All shortcut gates are valid.
- All forbidden-authority guards remain zero.

Conclusion:

- Multi-seed strengthens the sequence-energy operator claim.
- Multi-seed does not close the final proof because strict ordered decoder and
  `full_mirror` remain unstable.

## 14. Generalization Beyond Current V3 Family

Mini-plan:

- add new rule families, lengths > 8, token families, noise families, and same-bag traps;
- require all shortcut gates before runtime claims.

Plan critique:

- adding everything at once will hide the causal failure.

Improved plan:

- add one axis of novelty at a time, with per-axis diagnostics.

Status: partial; learned output-slot key improves length 9..12 strict decoder, but does not close it.

Evidence:

- Artifact:
  `data/rule_logic_position_sequence_v3/diagnostics/generalization_beyond_v3_report.json`
- New artifact slice:
  `data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_011/`
- Corpus:
  - lengths: 9, 10, 11, 12;
  - rows: 1920;
  - train rows: 1280;
  - heldout rows: 640;
  - matrix cells: 640;
  - seed: 11;
  - shortcut verdict: `VALID_POSITION_SEQUENCE_V3_CANDIDATE`.
- Shortcut metrics:
  - exact lookup: 0 milli;
  - L2-neighbor target copy: 0 milli;
  - Markov/bigram pairwise: 500 milli;
  - bag-of-tokens pairwise: 500 milli;
  - same-bag derangement: 1000 milli;
  - output-position prior: 0 milli.
- Static diagnostics:
  - max different-rule action similarity: 882 milli;
  - folded wrong-role hit: 108 milli;
  - folded missing true role: 239 milli.
- Runtime combined-objective gate:
  - strict slot ordered accuracy: 0 milli;
  - flat strict slot ordered accuracy: 0 milli;
  - sequence energy accuracy: 1000 milli;
  - sequence energy p10 gap: 131766;
  - symmetry sequence energy accuracy: 1000 milli;
  - non-symmetry sequence energy accuracy: 1000 milli;
  - output-slot cleanup accuracy: 693 milli;
  - output slots 8, 9, 10, 11: 0 milli;
  - flat sequence-energy parity mismatches: 0;
  - flat gap parity mismatches: 0;
  - ablations without binding/action/role/active-fringe: 0 milli;
  - `state_delta_edges: 0`;
  - forbidden authority flags: false.
- Learned output-slot key follow-up:
  - mechanism:
    `(action_center, output_slot_id, source_role_slot_id, sign_key) -> learned weight`;
  - base v3 preserved: strict 705, sequence energy 988, flat parity 0;
  - length 9..12 strict slot: 734 milli;
  - length 9..12 flat strict slot: 734 milli;
  - length 9..12 sequence energy: 1000 milli;
  - length 9..12 sequence energy p10 gap: 233720;
  - length 9..12 symmetry sequence energy: 1000 milli;
  - output-slot cleanup: 970 milli;
  - output slots 8..11: 955 / 894 / 969 / 1000 milli;
  - ablations without binding/action/role/active-fringe: 0 milli;
  - flat parity mismatches: 0;
  - `state_delta_edges: 0`;
  - forbidden authority flags: false.
- Cleanup8 diagnostic:
  - length 9..12 strict slot: 778 milli;
  - length 9..12 flat strict slot: 778 milli;
  - length 9..12 sequence energy: 1000 milli;
  - length 9..12 sequence energy p10 gap: 267892;
  - output-slot cleanup: 976 milli;
  - output slots 8..11: 958 / 946 / 994 / 1000 milli;
  - length strict: 9 -> 825, 10 -> 638, 11 -> 750, 12 -> 900;
  - rule strict: full_mirror -> 300, rotate_left_1 -> 988,
    rotate_left_2 -> 938, even_odd_split -> 950;
  - ablations without binding/action/role/active-fringe: 0 milli;
  - flat parity mismatches: 0;
  - `state_delta_edges: 0`;
  - forbidden authority flags: false.
- Capacity packing attempts:
  - `slot16_span2048`: base v3 strict 679, energy 985, symmetry energy 941,
    output cleanup 924; rejected as default because it regresses current best.
  - `slot12_span2730`: base v3 strict 649, energy 983, symmetry energy 936,
    output cleanup 910; rejected as default because it regresses current best.
  - both remove beyond-v3 missing-true-role pressure, but raise wrong-role
    collision pressure and do not close strict ordered decoding.
- Not validated:
  - new rule families;
  - new token/noise families;
  - multi-seed beyond-v3;
  - strict ordered decoder at 1000 milli;
  - learned readout for output slots greater than 8;
  - fixed full_mirror family on base v3 multi-seed.

Conclusion:

- Beyond-v3 length generalization is partially proven at both sequence-energy
  and strict-readout levels.
- Learned output-slot key solved the previous 8-slot ceiling symptom without
  manual `local_out_t`.
- Beyond-v3 strict ordered construction is still not fully proven because row
  accuracy is 778 milli, not 1000.
- A blind slot-bank expansion is not the fix: it trades the missing-slot problem
  for folded collision pressure and base-v3 regression.
- Final compact transferable operator claim remains forbidden.

## 15. Operator-Pair Action Motif Follow-Up

Mini-plan:

- attack the measured `full_mirror`/strict-readout blocker by improving action
  separability;
- do not add manual output time, target ids, proof-rule authority, or lookup;
- test both base v3 and a beyond-v3 length 9..12 slice.

Plan critique:

- a direct all-same-bag negative cleanup sounds attractive, but it can
  over-regularize the local decoder and harm operator energy;
- an operator-pair motif extractor is not a final broad-language proof unless a
  later L2 induction gate learns those motifs from less formal demonstrations.

Improved plan:

- preserve all-same-bag cleanup as a negative experiment;
- accept only the operator-pair action motif path if flat parity, ablations,
  strict readout, sequence energy, and mirror/symmetry all pass.

Status: green for current v3 and length 9..12 slice; broad proof still open.

Rejected experiment:

- all-same-bag candidate cleanup:
  - cleanup4 + candidate1: strict 587, energy 981, full_mirror strict 200;
  - cleanup8 + candidate1: strict 624, energy 974, full_mirror strict 242;
  - verdict: rejected as default because it regresses base v3.

Accepted mechanism:

- operator-pair action motifs from `rule_action_example`;
- edge key remains learned:
  `(action_center, output_slot_id, source_role_slot_id, sign_key) -> learned weight`;
- no target-center training;
- no proof-rule-id authority;
- no concrete token lookup;
- no manual `local_out_t` runtime extension.

Base v3 result:

- artifact:
  `data/rule_logic_position_sequence_v3/diagnostics/operator_pair_action_base_v3/combined_objective_rust_gate.log`
- strict slot ordered accuracy: 1000 milli;
- flat strict slot ordered accuracy: 1000 milli;
- sequence energy accuracy: 1000 milli;
- symmetry sequence energy accuracy: 1000 milli;
- full_mirror strict: 1000 milli;
- output-slot cleanup: 1000 milli;
- ablations without binding/action/role/active-fringe: 0 milli;
- flat parity mismatches: 0;
- `state_delta_edges: 0`.

Beyond-v3 length 9..12 result:

- artifact:
  `data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_011_operator_pair_action/combined_objective_rust_gate.log`
- heldout rows: 640;
- strict slot ordered accuracy: 1000 milli;
- flat strict slot ordered accuracy: 1000 milli;
- sequence energy accuracy: 1000 milli;
- every output slot 0..11: 1000 milli;
- every rule family, including `full_mirror`: 1000 milli;
- basin tests: energy stays 1000; source-all-suffix slot accuracy is 983;
- ablations without binding/action/role/active-fringe: 0 milli;
- flat parity mismatches: 0;
- `state_delta_edges: 0`.

Conclusion:

- the previous blocker was action/operator motif separability, not sequence
  length and not flat runtime;
- with separable operator-pair action motifs, L3 learns compact role/filler
  transfer and applies it to unseen heldout tokens up to length 12;
- final claim is still bounded: next proof-debts are multi-seed beyond-v3,
  new rule/token/noise families, length > 12 or dynamic slot capacity, and
  learned L2 induction of operator-pair motifs without a test-only parser.
