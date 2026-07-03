# rule_logic_position_sequence_v3 plan

Purpose: stronger ordered multi-token pressure without changing runtime first.

Do not overwrite v2. V3 must use:

- path: `data/rule_logic_position_sequence_v3/`
- schema: `position_sequence_v3`
- manifest: `manifest.json`
- shortcut report: `shortcut_gate_report.json`
- baseline report: `baseline_v3_report.json`

## Fixed Matrix

V3 must be generated from an explicit balanced matrix:

- lengths: `3, 4, 5, 6, 7, 8`
- surface families: `symbols`, `ru_words`, `business`, `network`
- noise types: `clean`, `prefix_suffix`, `punctuation`, `distractor_sequence`, `instruction_noise`
- rule families:
  - full mirror
  - rotate left by 1
  - rotate right by 1
  - rotate left by 2
  - pair swap
  - block swap
  - edge-to-center reorder
  - alternating even/odd split

Every generated group must be reported by:

```text
length x rule_family x surface_family x noise_type
```

## Required Negatives

Every negative must be same-bag and wrong-order.

V3 must not use only one negative pattern. It must mix:

- sampled derangement
- adjacent slot swap
- block swap wrong answer
- inverse rotation trap
- mirror-vs-rotate trap
- correct tokens with one output-slot phase shift

For every row:

```text
sorted(correct_tokens) == sorted(wrong_tokens)
for every output slot: correct_tokens[i] != wrong_tokens[i]
```

## Legal Signal vs Shortcut

`rule_action_example` is legal input. Reading the demonstrated action is not a shortcut by itself.

Shortcut gates must target dumb paths:

- exact input/action lookup
- proof_rule_id majority
- surface_family majority
- length-only predictor
- output-position prior
- template/frame predictor that ignores `sequence:`
- bag-of-tokens predictor
- Markov/bigram predictor
- L2-neighbor predictor

Acceptance thresholds:

```text
exact_lookup_accuracy_milli == 0
bag_of_tokens_accuracy_milli == 500
same_bag_derangement_milli == 1000
proof_rule_id_majority_accuracy_milli <= 300
surface_family_majority_accuracy_milli <= 300
length_only_accuracy_milli <= 350
output_position_prior_accuracy_milli <= 350
template_without_sequence_accuracy_milli <= 350
markov_bigram_pairwise_accuracy_milli <= 650
l2_neighbor_accuracy_milli <= 650
```

## Runtime Gate

The proof command must be explicit. Ordinary ignored tests are not proof.

```bash
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
  -- --ignored ordered_position_binding_must_learn_multi_slot_sequence_not_bag_copy --nocapture
```

Required:

```text
ordered_sequence_accuracy_milli == 1000
flat_ordered_sequence_accuracy_milli == 1000
flat_gap_parity_mismatches == 0
ablation_without_binding_accuracy_milli == 0
state_delta_edges == 0
target_center_id_training_used == false
proof_rule_id_training_authority_used == false
concrete_x_lookup_used == false
local_out_t_runtime_extension_used == false
```

## Per-Group Diagnostics

Do not trust only global accuracy. V3 report must include per-group:

- accuracy
- median gap
- p10 gap
- flat-vs-field gap parity
- failures by length
- failures by rule
- failures by surface family
- failures by noise type
- failures by input slot
- failures by output slot
- positive lane hit score
- negative lane hit score
- folded-lane collision count

If any group fails, do not add `local_out_t` or a new mechanism first. Diagnose the failure mode.

## Architecture Change Rule

New architecture is allowed only after a concrete v3 failure is shown.

Forbidden as first response:

- manual `local_out_t`
- target id
- proof rule id authority
- frame id
- fixed output template id

Allowed candidate after failure:

- learned output phase center
- learned phase competition
- learned output-slot phase pressure

The new mechanism must explain the measured failure mode, not decorate the architecture.
