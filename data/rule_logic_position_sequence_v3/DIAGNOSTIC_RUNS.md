# Position Sequence V3 Diagnostic Runs

This file preserves runnable commands and measured results for the v3 ordered
position-binding pressure gate. Do not overwrite v2 artifacts.

## Baseline V3

Corpus:

- path: `data/rule_logic_position_sequence_v3/accepted_position_sequence_tasks_v3.jsonl`
- rows: 2880
- train rows: 1920
- heldout rows: 960
- matrix cells: 960
- train per cell: 2
- heldout per cell: 1
- lengths: 3, 4, 5, 6, 7, 8
- rule families: 8
- surface families: 4
- noise types: 5

Shortcut gate:

```bash
python3 data/rule_logic_position_sequence_v3/run_shortcut_gates.py
```

Result:

- verdict: `VALID_POSITION_SEQUENCE_V3_CANDIDATE`
- exact/proof/surface/length/output-position/template/L2-neighbor-copy: 0
- bag-of-tokens: 500
- Markov/bigram pairwise: 500
- same-bag derangement: 1000

Runtime gate:

```bash
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
  -- --ignored ordered_position_binding_v3_balanced_matrix_must_hold_without_runtime_phase_hack --nocapture
```

Result:

- verdict: `FAIL_CURRENT_ARCHITECTURE_ON_V3`
- ordered sequence accuracy: 269 milli
- flat ordered sequence accuracy: 269 milli
- sequence energy proxy accuracy: 831 milli
- median gap: -11738
- p10 gap: -43728
- sequence energy median gap: 46908
- sequence energy p10 gap: -10438
- energy-pass but slot-fail rows: 540
- flat gap parity checked slots: 5280
- flat gap parity mismatches: 0
- per-matrix group failures: 702 / 960
- length group failures: 6
- rule group failures: 41
- surface group failures: 4
- noise group failures: 5
- output slot failures: 8

Meaning:

- flat runtime is not the cause because field and flat readout agree exactly;
- text surface/noise is not the primary cause because failures are broad;
- current action-slot binding cannot separate the dense rule/length/output-slot map.
- local all-slot gating is much weaker than sequence-level energy pressure.

## Energy Proxy Diagnostic

Command:

```bash
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
  -- --ignored ordered_position_binding_v3_balanced_matrix_must_hold_without_runtime_phase_hack --nocapture
```

Artifact:

- `diagnostics/energy_proxy_default_v3.log`

Result on default v3:

- strict all-slot ordered accuracy: 269 milli
- sequence energy proxy accuracy: 831 milli
- sequence energy median gap: 46908
- sequence energy p10 gap: -10438
- slot-pass but energy-fail rows: 0
- energy-pass but slot-fail rows: 540

Interpretation:

- this confirms a real mismatch: many rows have the correct full-sequence
  energy direction, but fail because at least one local output slot has a bad
  gap;
- this supports the diagnosis that the current kernel lacks a global
  operator/sequence objective;
- it does not prove v3 is solved, because p10 energy is still negative and
  action/operator separability remains weak.

## Static Diagnostics

Command:

```bash
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
  position_sequence_v3_static_diagnostics_report -- --nocapture
```

Artifact:

- `data/rule_logic_position_sequence_v3/static_diagnostics_report.json`

Result:

- action vectors: 48
- same rule action similarity: 1000 milli
- different rule action similarity: 553 milli
- same family different length similarity: 551 milli
- different family similarity: 553 milli
- max different rule similarity: 1000 milli
- folded target impulses checked: 236962
- folded multi-role hit: 65 milli
- folded wrong-role hit: 72 milli
- folded missing-true-role: 114 milli

Current diagnosis:

- primary suspect: action-demo motifs are not separable enough;
- secondary suspect: folded projection pressure is measurable but not dominant;
- next step: train-per-cell sweep and factor isolation before changing runtime
  architecture.

## Train-Per-Cell Sweep Commands

The generator now accepts environment configuration. Default behavior is
unchanged.

```bash
POSITION_SEQUENCE_TRAIN_PER_CELL=2 \
  python3 data/rule_logic_position_sequence_v3/build_position_sequence_tasks.py

POSITION_SEQUENCE_TRAIN_PER_CELL=4 \
  python3 data/rule_logic_position_sequence_v3/build_position_sequence_tasks.py

POSITION_SEQUENCE_TRAIN_PER_CELL=8 \
  python3 data/rule_logic_position_sequence_v3/build_position_sequence_tasks.py

POSITION_SEQUENCE_TRAIN_PER_CELL=16 \
  python3 data/rule_logic_position_sequence_v3/build_position_sequence_tasks.py
```

After each build:

```bash
python3 data/rule_logic_position_sequence_v3/run_shortcut_gates.py
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
  -- --ignored ordered_position_binding_v3_balanced_matrix_must_hold_without_runtime_phase_hack --nocapture
```

The long Rust gate prints per-epoch progress:

- epoch number
- margin
- update steps
- touched edges
- margin repairs
- fixed margins
- state-delta edge count
- role-binding edge count

Record each sweep result below before moving to architecture changes.

### train_per_cell = 4

Artifacts:

- `diagnostics/train_per_cell_4/manifest.json`
- `diagnostics/train_per_cell_4/shortcut_gate_report.json`
- `diagnostics/train_per_cell_4/rust_gate.log`

Shortcut result:

- verdict: `VALID_POSITION_SEQUENCE_V3_CANDIDATE`
- rows: 4800
- train rows: 3840
- heldout rows: 960
- bag-of-tokens: 500
- Markov/bigram pairwise: 500
- all other dumb shortcuts: 0

Runtime result:

- ordered sequence accuracy: 222 milli
- flat ordered sequence accuracy: 222 milli
- median gap: -14232
- p10 gap: -40264
- flat gap parity mismatches: 0
- per-matrix group failures: 747 / 960
- role-binding edges: 6075
- flat role-binding edges: 5448

Interpretation:

- doubling train density did not improve the gate; it made accuracy worse
  (`269 -> 222`);
- this argues against a simple "not enough examples per matrix cell" diagnosis.

## Factor Isolation Runs

### One surface/noise, all rules

Configuration:

```bash
POSITION_SEQUENCE_SURFACE_FAMILIES=symbols \
POSITION_SEQUENCE_NOISE_TYPES=clean \
python3 data/rule_logic_position_sequence_v3/build_position_sequence_tasks.py
```

Artifacts:

- `diagnostics/factor_symbols_clean_all_rules/manifest.json`
- `diagnostics/factor_symbols_clean_all_rules/shortcut_gate_report.json`
- `diagnostics/factor_symbols_clean_all_rules/rust_gate.log`

Runtime result:

- rows: 144
- train rows: 96
- heldout rows: 48
- ordered sequence accuracy: 188 milli
- flat ordered sequence accuracy: 188 milli
- median gap: -5860
- p10 gap: -16728
- flat gap parity mismatches: 0
- per-matrix group failures: 39 / 48

Interpretation:

- removing cross-surface and noise variety does not rescue the current binding
  form;
- surface/noise is not the primary failure source.

### Four rule families, all contexts

Configuration:

```bash
POSITION_SEQUENCE_RULE_FAMILIES=full_mirror,rotate_left_1,rotate_right_1,rotate_left_2 \
python3 data/rule_logic_position_sequence_v3/build_position_sequence_tasks.py
```

Artifacts:

- `diagnostics/factor_4_rules_all_contexts/manifest.json`
- `diagnostics/factor_4_rules_all_contexts/shortcut_gate_report.json`
- `diagnostics/factor_4_rules_all_contexts/rust_gate.log`

Runtime result:

- rows: 1440
- train rows: 960
- heldout rows: 480
- ordered sequence accuracy: 365 milli
- flat ordered sequence accuracy: 365 milli
- median gap: -6380
- p10 gap: -27442
- flat gap parity mismatches: 0
- per-matrix group failures: 305 / 480

Interpretation:

- reducing rule-family count helps (`269 -> 365`) but remains far from a pass;
- rule separability matters, but the current binding form is still not enough.

### Lengths 3-6, all rules

Configuration:

```bash
POSITION_SEQUENCE_LENGTHS=3,4,5,6 \
python3 data/rule_logic_position_sequence_v3/build_position_sequence_tasks.py
```

Artifacts:

- `diagnostics/factor_lengths_3_6_all_rules/manifest.json`
- `diagnostics/factor_lengths_3_6_all_rules/shortcut_gate_report.json`
- `diagnostics/factor_lengths_3_6_all_rules/rust_gate.log`

Runtime result:

- rows: 1920
- train rows: 1280
- heldout rows: 640
- ordered sequence accuracy: 316 milli
- flat ordered sequence accuracy: 316 milli
- median gap: -8952
- p10 gap: -25310
- flat gap parity mismatches: 0
- per-matrix group failures: 438 / 640

Interpretation:

- removing lengths 7-8 helps only mildly (`269 -> 316`);
- sequence length amplifies the problem but is not the root cause.

### Single rule family rejected

Configuration:

```bash
POSITION_SEQUENCE_RULE_FAMILIES=full_mirror \
python3 data/rule_logic_position_sequence_v3/build_position_sequence_tasks.py
```

Shortcut result:

- verdict: `REJECT_OUTPUT_POSITION_PRIOR_ACCURACY_MILLI`
- output-position prior: 1000 milli

Interpretation:

- a single-rule slice is not a valid pressure corpus because a dumb
  output-position prior solves it;
- do not use this slice as proof of model ability.

## Lineage-Driven Rescue Probes

These probes follow the architecture lineage proof-debt instead of adding a
manual output-time hack.

### action lanes = 64

Reason:

- role/filler and superposition cards suggested that the action/operator motif
  may be under-separated.

Static result:

- different rule action similarity: 506 milli
- max different rule similarity: 1000 milli

Runtime result:

- ordered sequence accuracy: 393 milli
- sequence energy accuracy: 894 milli
- sequence energy median gap: 99006
- sequence energy p10 gap: -3074
- energy-pass but slot-fail rows: 481
- role-binding edges: 16206

Interpretation:

- more action evidence helps, but does not solve the gate;
- it increases memory/work sharply, so it is a useful diagnostic, not a final
  proof.

### action lanes = 64, role lanes = 32

Reason:

- folded collision diagnostics showed missing true role pressure; increasing
  role evidence tests whether slots lose their filler signal.

Static result:

- different rule action similarity: 506 milli
- max different rule similarity: 1000 milli
- folded multi-role hit: 85 milli
- folded wrong-role hit: 85 milli
- folded missing-true-role: 0 milli

Runtime result:

- ordered sequence accuracy: 501 milli
- flat ordered sequence accuracy: 501 milli
- ordered sequence median gap: 150
- ordered sequence p10 gap: -31668
- sequence energy accuracy: 905 milli
- sequence energy median gap: 131572
- sequence energy p10 gap: 1888
- energy-pass but slot-fail rows: 388
- role-binding edges: 16193
- flat role-binding edges: 14241

Interpretation:

- this is the strongest current v3 probe;
- role/filler coverage was a real bottleneck: missing-true-role dropped to zero
  and strict accuracy improved `269 -> 501`;
- the gate is still not solved because many rows have correct sequence energy
  but fail local slot margins;
- next proof-debt is a real sequence/operator energy training objective.

### naive sequence-energy objective v1

Reason:

- attractor/associative-memory lineage suggested turning the diagnostic
  sequence energy into an explicit training objective:
  `E(correct_sequence) < E(wrong_same_bag_sequence)`.

Implementation boundary:

- no `target_id`;
- no `proof_rule_id` authority;
- no concrete X lookup;
- no manual `local_out_t`;
- no runtime phase extension.

Artifact:

- `diagnostics/sequence_energy_objective_v1/rust_gate.log`

Runtime result:

- slot ordered sequence accuracy: 277 milli
- flat slot ordered sequence accuracy: 277 milli
- sequence energy accuracy: 886 milli
- sequence energy median gap: 336794
- sequence energy p10 gap: -17490
- energy-pass but slot-fail rows: 585
- role-binding edges: 15583

Interpretation:

- naive energy-only training does not solve v3;
- it is worse than the action64/role32 local slot-trained probe
  (`sequence_energy_accuracy_milli: 905`);
- conclusion: energy must not replace role/filler binding pressure. The next
  mechanism must combine local role/filler assembly with global operator
  consistency or energy cleanup.

### slot-operator action signature, action lanes = 64, role lanes = 32

Reason:

- previous action demos had complete action collisions (`max_different_rule_similarity_milli: 1000`);
- the new action text puts an abstract source-slot order first, without target
  tokens and without proof_rule_id authority.

Shortcut result:

- verdict: `VALID_POSITION_SEQUENCE_V3_CANDIDATE`
- exact/proof/surface/length/output-position/template/L2-neighbor-copy: 0
- bag-of-tokens: 500
- Markov/bigram pairwise: 500

Static result:

- different rule action similarity: 494 milli
- same family different length similarity: 488 milli
- max different rule similarity: 1000 milli
- folded missing-true-role: 0 milli

Runtime result:

- strict slot ordered accuracy: 433 milli
- sequence energy accuracy: 923 milli
- sequence energy median gap: 136758
- sequence energy p10 gap: 26978
- energy-pass but slot-fail rows: 470
- energy-failed rows by rule:
  - `full_mirror_len3`: 19
  - `full_mirror_len4`: 13
  - `full_mirror_len5`: 10
  - `full_mirror_len8`: 2
  - `pair_swap_len3`: 19
  - `pair_swap_len5`: 11
- energy-failed rows by length:
  - length 3: 38
  - length 4: 13
  - length 5: 21
  - length 8: 2
- role-binding edges: 15887

Interpretation:

- abstract operator-slot action improves whole-sequence energy beyond the
  previous best (`905 -> 923`);
- it weakens strict local slot readout (`501 -> 433`);
- this supports the split claim: L3 is closer to a sequence-level operator
  judge than to a full ordered decoder;
- next diagnostic must isolate the remaining energy-fail rows, not the much
  larger local slot-fail set.
- remaining energy failures are concentrated in symmetric / near-symmetric
  operators, not in surface/noise.

### unique operator matrix + span 4096

Reason:

- the previous matrix contained mathematically equivalent operators for some
  lengths, for example `full_mirror_len3 == pair_swap_len3` and
  `block_swap_len4 == rotate_left_2_len4`;
- equivalent operators are not useful proof pressure, because they create
  artificial action collisions without adding reasoning depth;
- the role/action folded span was increased from 2048 to 4096 and the sequence
  action base was moved to 0 so the whole u16 center space is used cleanly.

Corpus result:

- rows: 2520
- train rows: 1680
- heldout rows: 840
- matrix cells: 840
- skipped equivalent rules:
  - `block_swap_len3 -> rotate_left_1_len3`
  - `block_swap_len4 -> rotate_left_2_len4`
  - `block_swap_len5 -> rotate_left_2_len5`
  - `edge_to_center_len3 -> rotate_right_1_len3`
  - `pair_swap_len3 -> full_mirror_len3`
  - `rotate_left_2_len3 -> rotate_right_1_len3`

Static result:

- action vectors: 42
- different rule action similarity: 506 milli
- max different rule similarity: 969 milli
- folded multi-role hit: 64 milli
- folded wrong-role hit: 64 milli
- folded missing-true-role: 0 milli

Runtime result:

- strict slot ordered accuracy: 507 milli
- sequence energy accuracy: 957 milli
- sequence energy median gap: 151224
- sequence energy p10 gap: 27348
- flat sequence energy parity checked rows: 840
- flat sequence energy parity mismatches: 0
- flat sequence energy parity max abs gap delta: 0
- symmetry rows: 220
- symmetry strict accuracy: 145 milli
- symmetry sequence energy accuracy: 836 milli
- symmetry p10 energy gap: -13864
- non-symmetry rows: 620
- non-symmetry strict accuracy: 635 milli
- non-symmetry sequence energy accuracy: 1000 milli
- non-symmetry p10 energy gap: 54150
- normalized slot accuracy:
  - slot 0: 806 milli
  - slot 1: 900 milli
  - slot 2: 894 milli
  - slot 3: 891 milli
  - slot 4: 852 milli
  - slot 5: 883 milli
  - slot 6: 906 milli
  - slot 7: 1000 milli
- energy-pass but slot-fail rows: 378
- energy-failed rows by rule:
  - `full_mirror_len3`: 8
  - `full_mirror_len4`: 8
  - `full_mirror_len5`: 12
  - `full_mirror_len7`: 4
  - `pair_swap_len5`: 4

Interpretation:

- this is the current strongest honest v3 result;
- de-superposition and removal of duplicate operators both helped;
- remaining sequence-energy failures are now concentrated in mirror-like
  symmetric operators.
- non-symmetry operators are sequence-energy solved on the current heldout
  split; mirror/pair-swap consistency is the real remaining energy debt.
- slot 0 and slot 4 are the heaviest normalized local decoder debts.

### role-map action signature rejected

Reason tested:

- add explicit abstract `outK=srcJ` role-map text before the demo.

Runtime result:

- strict slot ordered accuracy: 424 milli
- sequence energy accuracy: 933 milli
- sequence energy p10 gap: 24964

Interpretation:

- explicit role-map text makes the action heavier but worse;
- do not use it as the next path.

## Current Diagnostic Conclusion

The current evidence points to this ordering:

1. Role/filler coverage and de-superposition were real bottlenecks.
2. Duplicate/equivalent operators in the corpus were a real data-quality bug.
3. Missing global sequence/operator objective remains real, but naive
   energy-only training is not sufficient.
4. Weak action/operator separability remains unresolved for symmetric rules.
5. Folded projection collision pressure is an amplifier.
6. Not primary: text surface, noise type, train density, or flat runtime readout.

Do not add a manual `local_out_t` or fixed frame/template ID. The next honest
move is to improve the task-side action demonstration pressure or test a learned
phase mechanism only after the exact failure mode is isolated.

### combined objective + channel ablations

Command:

```bash
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
  -- --ignored ordered_position_binding_v3_combined_objective_probe_must_preserve_flat_energy_parity --nocapture
```

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/rust_gate.log`

Runtime result:

- strict slot ordered accuracy: 705 milli
- flat strict slot ordered accuracy: 705 milli
- sequence energy accuracy: 988 milli
- sequence energy p10 gap: 36878
- energy-pass but slot-fail rows: 238
- symmetry sequence energy accuracy: 955 milli
- symmetry p10 energy gap: 9416
- non-symmetry sequence energy accuracy: 1000 milli
- non-symmetry p10 energy gap: 53932
- flat sequence energy parity mismatches: 0
- flat sequence energy parity max abs gap delta: 0
- flat slot-gap parity mismatches: 0
- state-delta edges: 0
- role-binding edges: 16357

Channel ablations:

- without binding: 0 milli
- without action: 0 milli
- without action energy: 0 milli
- without role: 0 milli
- without role energy: 0 milli
- without active fringe: 0 milli

Interpretation:

- combined local+global objective is a real improvement over the local-only
  current best;
- channel ablations support the intended mechanism: binding/action/role/active
  channels are all necessary for the result;
- strict ordered decoder remains open at 705 milli, so the final compact
  transferable operator claim is still not allowed.

Output-slot cleanup:

- per-slot accuracy: 933 milli
- row-level strict ordered accuracy: 705 milli
- failed local slots: 328
- accuracy by output slot:
  - slot 0: 937 milli
  - slot 1: 949 milli
  - slot 2: 904 milli
  - slot 3: 968 milli
  - slot 4: 897 milli
  - slot 5: 892 milli
  - slot 6: 966 milli
  - slot 7: 1000 milli
- symmetry accuracy by output slot:
  - slot 0: 850 milli
  - slot 1: 859 milli
  - slot 2: 727 milli
  - slot 3: 925 milli
  - slot 4: 800 milli
  - slot 5: 775 milli
  - slot 6: 950 milli
  - slot 7: 1000 milli
- non-symmetry accuracy by output slot:
  - slot 0: 968 milli
  - slot 1: 981 milli
  - slot 2: 966 milli
  - slot 3: 984 milli
  - slot 4: 930 milli
  - slot 5: 931 milli
  - slot 6: 971 milli
  - slot 7: 1000 milli
- energy-pass slot-fail by output slot:
  - slot 0: 48
  - slot 1: 36
  - slot 2: 77
  - slot 3: 24
  - slot 4: 61
  - slot 5: 52
  - slot 6: 11

Interpretation:

- the local decoder is much better than the row-level strict metric suggests:
  933 milli per-slot vs 705 milli all-slots-at-once;
- the remaining local debt is concentrated in symmetry/pair-swap rows, not in
  the non-symmetry operators;
- no manual output-time coordinate is justified by this diagnostic.

Attractor basin stability:

- clean:
  - slot accuracy: 705 milli
  - energy accuracy: 988 milli
  - p10 energy gap: 36878
- weaken_x2:
  - slot accuracy: 683 milli
  - energy accuracy: 995 milli
  - p10 energy gap: 19326
- drop_mod_11:
  - slot accuracy: 654 milli
  - energy accuracy: 985 milli
  - p10 energy gap: 26454
- drop_mod_7:
  - slot accuracy: 624 milli
  - energy accuracy: 983 milli
  - p10 energy gap: 25494
- drop_mod_5:
  - slot accuracy: 631 milli
  - energy accuracy: 980 milli
  - p10 energy gap: 23548
- drop7_distract8:
  - slot accuracy: 626 milli
  - energy accuracy: 987 milli
  - p10 energy gap: 24894
- drop5_distract16:
  - slot accuracy: 633 milli
  - energy accuracy: 980 milli
  - p10 energy gap: 23548

Interpretation:

- sequence/operator energy is robust under deterministic active-fringe
  weakening, dropout, and distractor injection;
- the worst tested energy accuracy remains 980 milli and p10 energy gap remains
  positive;
- strict slot readout degrades much earlier, confirming that the next blocker is
  local decoder crystallization rather than loss of the operator basin.

Proxy energy monotonicity:

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/proxy_energy_monotonicity_report.json`

Cleanup trace:

- epoch 1: energy 961, p10 gap 25148, min gap -32876
- epoch 2: energy 961, p10 gap 30572, min gap -20566
- epoch 3: energy 965, p10 gap 26346, min gap -5602
- epoch 4: energy 992, p10 gap 36536, min gap -168

Interpretation:

- p10 energy is not strictly monotonic;
- worst-row `min_energy_gap` improves monotonically toward zero;
- energy accuracy is non-decreasing;
- p10 remains positive and ends above epoch 1;
- this closes a bounded/improving proxy diagnostic, not a formal energy theorem.

Capacity curve:

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/capacity_curve_report.json`

By length:

- len3: slot 513, energy 925, p10 3718
- len4: slot 729, energy 1000, p10 24034
- len5: slot 700, energy 971, p10 33412
- len6: slot 706, energy 1000, p10 73546
- len7: slot 663, energy 1000, p10 115548
- len8: slot 825, energy 1000, p10 165934

By rule family:

- full_mirror: slot 83, energy 917, p10 3488
- pair_swap: slot 670, energy 1000, p10 38808
- even_odd_split: slot 1000, energy 1000, p10 50996
- all rotate/block/edge non-mirror families reach sequence-energy 1000 milli.

Interpretation:

- capacity collapse is not caused by longer sequences;
- length 8 is stronger than length 3 on the current field;
- the real collapse axis is rule-family geometry, especially full_mirror;
- next mechanism work should target symmetry/operator separability, not more
  memory and not manual output phase.

Address-radius sweep:

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/address_radius_report.json`

Input-surface perturbations:

- clean: slot 705, energy 988, p10 gap 36878
- action_wrapped: slot 382, energy 904, p10 gap 1218
- source_slot0_suffix: slot 682, energy 975, p10 gap 30694
- source_all_suffix: slot 638, energy 983, p10 gap 12482
- action_wrapped_source_slot0_suffix: slot 354, energy 889, p10 gap -4358

Interpretation:

- source/role address is comparatively robust;
- action/operator address is fragile;
- this supports the earlier action-motif separability diagnosis and explains
  why mirror/operator geometry remains the hardest debt.

Collision audit:

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/collision_audit_report.json`

Findings:

- L1/action signature:
  - 42 action vectors
  - max different-rule similarity: 969 milli
  - nearest collision: `full_mirror_len3` vs `rotate_left_1_len3`
- L2/folded role pressure:
  - wrong-role hit: 64 milli
  - missing true role: 0 milli
- L3/role-binding polysemy:
  - flat nonzero role-binding edges: 14600
  - raw role-binding edges: 16357
  - action centers with edges: 1176
  - action centers with multi-slot edges: 1176
  - max slots per action center: 8

Interpretation:

- source-role recall is not the main failure;
- action/operator signatures are too close for mirror-like rules;
- compact shared L3 binding edges are doing the desired compression, but under
  near-colliding action signatures they become polysemantic pressure.

Multi-seed robustness:

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/multi_seed_robustness_report.json`

Status:

- `PARTIAL_ENERGY_ROBUST_STRICT_DECODER_NOT_CLOSED`

Runtime seeds:

- seed 1: sequence energy 992, strict slot 586, full_mirror energy 958
- seed 2: sequence energy 987, strict slot 621, full_mirror energy 908
- seed 3: sequence energy 974, strict slot 611, full_mirror energy 842

Interpretation:

- builder now supports independent seed generation;
- shortcut gates are valid for all three seed corpora;
- forbidden-authority guards remain zero for all runtime gates;
- sequence energy is robust across seeds;
- strict ordered decoder and full_mirror remain open.

Beyond-v3 generalization:

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/generalization_beyond_v3_report.json`
- `data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_011/`

Status:

- `PARTIAL_LENGTH_9_12_LEARNED_OUTPUT_SLOT_STRICT_DECODER_IMPROVED_NOT_CLOSED`

Validated beyond-v3 length slice:

- lengths: 9, 10, 11, 12
- rows: 1920
- train rows: 1280
- heldout rows: 640
- seed: 11
- shortcut verdict: `VALID_POSITION_SEQUENCE_V3_CANDIDATE`
- exact lookup: 0 milli
- L2-neighbor target copy: 0 milli
- Markov/bigram pairwise: 500 milli
- bag-of-tokens pairwise: 500 milli
- same-bag derangement: 1000 milli

Static diagnostics:

- max different-rule action similarity: 882 milli
- folded wrong-role hit: 108 milli
- folded missing true role: 239 milli

Runtime combined-objective result:

- strict slot ordered accuracy: 0 milli
- flat strict slot ordered accuracy: 0 milli
- sequence energy accuracy: 1000 milli
- sequence energy p10 gap: 131766
- symmetry sequence energy accuracy: 1000 milli
- non-symmetry sequence energy accuracy: 1000 milli
- output-slot cleanup accuracy: 693 milli
- output slots 8, 9, 10, 11: 0 milli
- flat sequence-energy parity mismatches: 0
- flat gap parity mismatches: 0
- ablations without binding/action/role/active-fringe: 0 milli
- state_delta_edges: 0
- forbidden authority flags: false

Learned output-slot key follow-up:

- artifact:
  `data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_011_learned_output_slot/`
- mechanism:
  `(action_center, output_slot_id, source_role_slot_id, sign_key) -> learned weight`
- manual `local_out_t`: false
- base v3 preserved:
  - strict slot ordered accuracy: 705 milli
  - sequence energy accuracy: 988 milli
  - flat sequence-energy parity mismatches: 0
- beyond length 9..12:
  - strict slot ordered accuracy: 734 milli
  - flat strict slot ordered accuracy: 734 milli
  - sequence energy accuracy: 1000 milli
  - sequence energy p10 gap: 233720
  - symmetry sequence energy accuracy: 1000 milli
  - output-slot cleanup accuracy: 970 milli
  - output slot 8: 955 milli
  - output slot 9: 894 milli
  - output slot 10: 969 milli
  - output slot 11: 1000 milli
  - flat sequence-energy parity mismatches: 0
  - flat gap parity mismatches: 0
  - ablations without binding/action/role/active-fringe: 0 milli
  - state_delta_edges: 0
  - forbidden authority flags: false
- cleanup8 diagnostic:
  - strict slot ordered accuracy: 778 milli
  - flat strict slot ordered accuracy: 778 milli
  - sequence energy accuracy: 1000 milli
  - sequence energy p10 gap: 267892
  - output-slot cleanup accuracy: 976 milli
  - output slot 8: 958 milli
  - output slot 9: 946 milli
  - output slot 10: 994 milli
  - output slot 11: 1000 milli
  - length 9 strict: 825 milli
  - length 10 strict: 638 milli
  - length 11 strict: 750 milli
  - length 12 strict: 900 milli
  - full_mirror strict: 300 milli
  - flat parity mismatches: 0
  - ablations without binding/action/role/active-fringe: 0 milli
  - state_delta_edges: 0
  - forbidden authority flags: false

Not validated:

- new rule families;
- new token/noise families;
- multi-seed beyond-v3;
- strict ordered decoder at 1000 milli;
- learned readout for output slots greater than 8;
- fixed full_mirror family on base v3 multi-seed.

Interpretation:

- length 9..12 generalizes at sequence-energy/operator-judge level;
- learned output-slot key breaks the previous 8-slot ceiling symptom;
- two direct slot-bank expansions were tested and rejected as defaults:
  - `slot16_span2048`: base v3 strict 679, energy 985, symmetry energy 941;
  - `slot12_span2730`: base v3 strict 649, energy 983, symmetry energy 936.
- both expansions remove beyond-v3 missing-true-role pressure, but increase
  wrong-role collision pressure and regress the base v3 current best;
- the next architecture debt is strict slot cleanup from 734 to 1000, especially
  length 10/11 and full_mirror-like cases;
- final compact transferable operator claim is still forbidden until strict
  ordered decoder reaches the green gate.

## Operator-Pair Action Motif Probe

Mini-plan:

- do not add manual `local_out_t`;
- improve the L2 -> L3 action representation instead;
- derive operator-pair action centers from the provided `rule_action_example`
  demonstration, not from `proof_rule_id`, target tokens, or a stored answer.

Plan critique:

- if this is treated as a final proof, it overclaims: the current test still
  uses a deterministic extractor for the formal `operator_slots:` text;
- the correct claim is narrower: L3 binding works when L2 supplies separable
  operator-pair motifs.

Improved plan:

- keep the extractor behind `POSITION_SEQUENCE_OPERATOR_PAIR_ACTION_CENTERS=1`;
- run base v3 and beyond-v3 length 9..12;
- require strict slot, sequence energy, flat parity, symmetry, and ablations to
  pass together.

Negative control first:

- tried `all-same-bag candidate cleanup`;
- cleanup4 + candidate1 regressed base v3:
  - strict slot: 587;
  - sequence energy: 981;
  - full_mirror strict: 200.
- cleanup8 + candidate1 also regressed:
  - strict slot: 624;
  - sequence energy: 974;
  - full_mirror strict: 242.
- verdict: rejected. Pairwise pressure against every same-bag token is too
  blunt and weakens the operator-energy geometry.

Base v3 command:

```bash
POSITION_SEQUENCE_OPERATOR_PAIR_ACTION_CENTERS=1 \
POSITION_SEQUENCE_COMBINED_CLEANUP_EPOCHS=4 \
POSITION_SEQUENCE_CANDIDATE_CLEANUP_EPOCHS=0 \
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
  -- --ignored ordered_position_binding_v3_combined_objective_probe_must_preserve_flat_energy_parity --nocapture
```

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/operator_pair_action_base_v3/combined_objective_rust_gate.log`

Base v3 result:

- strict slot ordered accuracy: 1000 milli;
- flat strict slot ordered accuracy: 1000 milli;
- sequence energy accuracy: 1000 milli;
- sequence energy p10 gap: 225820;
- symmetry sequence energy accuracy: 1000 milli;
- non-symmetry sequence energy accuracy: 1000 milli;
- full_mirror strict: 1000 milli;
- output-slot cleanup: 1000 milli;
- ablations without binding/action/role/active-fringe: 0 milli;
- flat sequence-energy parity mismatches: 0;
- flat gap parity mismatches: 0;
- `state_delta_edges: 0`;
- forbidden authority flags: false.

Beyond-v3 length 9..12 command:

```bash
POSITION_SEQUENCE_V3_CORPUS_PATH=../../data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_011/accepted_position_sequence_tasks_v3.jsonl \
POSITION_SEQUENCE_OPERATOR_PAIR_ACTION_CENTERS=1 \
POSITION_SEQUENCE_COMBINED_CLEANUP_EPOCHS=4 \
POSITION_SEQUENCE_CANDIDATE_CLEANUP_EPOCHS=0 \
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
  -- --ignored ordered_position_binding_v3_combined_objective_probe_must_preserve_flat_energy_parity --nocapture
```

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_011_operator_pair_action/combined_objective_rust_gate.log`

Beyond-v3 result:

- lengths: 9, 10, 11, 12;
- heldout rows: 640;
- strict slot ordered accuracy: 1000 milli;
- flat strict slot ordered accuracy: 1000 milli;
- sequence energy accuracy: 1000 milli;
- sequence energy p10 gap: 1166244;
- symmetry sequence energy accuracy: 1000 milli;
- non-symmetry sequence energy accuracy: 1000 milli;
- every output slot 0..11: 1000 milli;
- every rule family, including `full_mirror`: 1000 milli;
- ablations without binding/action/role/active-fringe: 0 milli;
- flat sequence-energy parity mismatches: 0;
- flat gap parity mismatches: 0;
- `state_delta_edges: 0`;
- forbidden authority flags: false.

Interpretation:

- the old blocker was not sequence length, not flat runtime, and not missing
  target tokens;
- the old blocker was action/operator motif separability;
- when L2 supplies explicit operator-pair action motifs from the rule action
  demonstration, L3 learns a compact role/filler transfer operator and applies
  it to unseen heldout tokens and longer sequences;
- this still does not prove broad LLMWave reasoning. Remaining debt:
  multi-seed beyond-v3, new rule/token/noise families, lengths greater than 12
  or dynamic slot capacity, and learned L2 induction of operator-pair motifs
  from less formal demonstrations without a test-only parser.

## Stop Marker

2026-07-01 user stop command interrupted the next multi-seed continuation.

Canonical stop report:

- `data/rule_logic_position_sequence_v3/GOAL_STOP_REPORT_2026-07-01.md`

Boundary:

- seed 012 runtime was stopped at `ablation_without_binding_start`;
- seed 012 has shortcut PASS and partial train/cleanup logs, but no final
  heldout verdict;
- seed 013 has shortcut PASS, but runtime was not started;
- 16-slot rung was planned but not executed.
