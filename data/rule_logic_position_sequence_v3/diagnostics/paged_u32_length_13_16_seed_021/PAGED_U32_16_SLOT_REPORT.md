# Paged u32 16-Slot Pressure Rung

Date: 2026-07-01

## Verdict

PASSED as a 16-slot pressure rung.

This run proves that the ordered position-binding path can hold lengths 13..16 with a paged `u32` center layout, without folded role stride, without `target_id`/`proof_rule_id` authority, without concrete-X lookup, and without manual `local_out_t`.

This is not yet a final broad LLMWave reasoning claim. It is a strong operator-transfer gate for the current ordered sequence family.

## Address Layout

The old `u16` center ceiling was removed for the sequence-pressure path.

```text
CenterId = u32
lane_id = u16
output_slot_id/source_slot_id/sign_key = u8

PAGE_BITS = 12
PAGE_SIZE = 4096
PAGE_COUNT = 32
SEQ_TOTAL_CENTER_COUNT = 131072

pages 0..15  = role slot pages
page 16      = action surface bank
page 17      = operator-pair action bank
pages 18..31 = learned reserves only; not used by this rung
```

Address formulas:

```text
center = (page << 12) | lane
role(slot, lane) = (slot << 12) | lane
action_surface(lane) = (16 << 12) | lane
operator_pair(out, src) = (17 << 12) | ((out << 4) | src)
```

Important boundary:

`operator_pair(out, src)` is allowed only as an L2/L3 action motif extracted from `rule_action_example`. It is not read from the target, correct answer, proof rule id, or `local_out_t`.

## Corpus

Artifact:

```text
data/rule_logic_position_sequence_v3/diagnostics/paged_u32_length_13_16_seed_021/accepted_position_sequence_tasks_v3.jsonl
```

Manifest summary:

```text
rows: 1920
train_rows: 1280
heldout_rows: 640
matrix_cells: 640
lengths: 13, 14, 15, 16
rule_families: 8
surface_families: symbols, ru_words, business, network
noise_types: clean, prefix_suffix, punctuation, distractor, instruction_noise
same_bag_derangement_required: true
train_heldout_overlap_by_surface: empty
```

Generator fix:

The token pools were expanded and `pick_tokens` now rejects ambiguous repeated-token sequences by selecting a stride coprime with the pool length. This prevents false slot ambiguity on length 13..16.

## Shortcut Gates

Artifact:

```text
data/rule_logic_position_sequence_v3/diagnostics/paged_u32_length_13_16_seed_021/shortcut_gate_report.json
```

Result:

```text
verdict: VALID_POSITION_SEQUENCE_V3_CANDIDATE
exact_lookup_accuracy_milli: 0
l2_neighbor_target_copy_accuracy_milli: 0
markov_bigram_pairwise_accuracy_milli: 500
bag_of_tokens_accuracy_milli: 500
proof_rule_id_majority_accuracy_milli: 0
surface_family_majority_accuracy_milli: 0
length_only_accuracy_milli: 0
output_position_prior_accuracy_milli: 0
template_without_sequence_accuracy_milli: 0
same_bag_derangement_milli: 1000
```

Interpretation:

The dumb baselines do not solve the task. Bag-of-tokens and Markov remain at pairwise chance because correct and wrong use the same token bag.

## Rust Gate

Command:

```bash
stdbuf -oL env \
  POSITION_SEQUENCE_V3_CORPUS_PATH=../../data/rule_logic_position_sequence_v3/diagnostics/paged_u32_length_13_16_seed_021/accepted_position_sequence_tasks_v3.jsonl \
  POSITION_SEQUENCE_OPERATOR_PAIR_ACTION_CENTERS=1 \
  POSITION_SEQUENCE_COMBINED_LOCAL_EPOCHS=8 \
  POSITION_SEQUENCE_COMBINED_CLEANUP_EPOCHS=4 \
  POSITION_SEQUENCE_CANDIDATE_CLEANUP_EPOCHS=0 \
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 -- \
  --ignored ordered_position_binding_v3_combined_objective_probe_must_preserve_flat_energy_parity \
  --nocapture
```

Log:

```text
data/rule_logic_position_sequence_v3/diagnostics/paged_u32_length_13_16_seed_021/combined_objective_paged_u32_rust_gate.log
```

Result:

```text
test result: ok
debug diagnostic wall time: 3623.13s

combined_slot_ordered_sequence_accuracy_milli: 1000
combined_flat_slot_ordered_sequence_accuracy_milli: 1000
combined_sequence_energy_accuracy_milli: 1000
combined_symmetry_sequence_energy_accuracy_milli: 1000
combined_non_symmetry_sequence_energy_accuracy_milli: 1000

flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_checked_rows: 640
flat_sequence_energy_parity_mismatches: 0
flat_sequence_energy_parity_max_abs_gap_delta: 0

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0

state_delta_edges: 0
role_binding_edges: 143248

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Interpretation:

The model did not solve the rung through direct lane-delta lookup (`state_delta_edges: 0`). It solved it through learned role-binding edges over paged role/action/operator centers.

The flat compiled readout path matches the field path exactly for slot gaps and sequence-energy gaps.

## Symmetry / Mirror

The previous v3 weak point was mirror/symmetry consistency. In this rung:

```text
combined_symmetry_sequence_energy_accuracy_milli: 1000
combined_symmetry_p10_energy_gap: 2533782
combined_non_symmetry_sequence_energy_accuracy_milli: 1000
combined_non_symmetry_p10_energy_gap: 2598778
```

Capacity by rule family:

```text
full_mirror: slot 1000, energy 1000
pair_swap: slot 1000, energy 1000
block_swap: slot 1000, energy 1000
edge_to_center: slot 1000, energy 1000
even_odd_split: slot 1000, energy 1000
rotate_left_1: slot 1000, energy 1000
rotate_left_2: slot 1000, energy 1000
rotate_right_1: slot 1000, energy 1000
```

## Stability

Basin stability:

```text
clean: slot 1000, energy 1000
weaken_x2: slot 1000, energy 1000
drop_mod_11: slot 1000, energy 1000
drop_mod_7: slot 997, energy 1000
drop_mod_5: slot 966, energy 997
drop7_distract8: slot 997, energy 1000
drop5_distract16: slot 966, energy 997
```

Address radius:

```text
clean: slot 1000, energy 1000
action_wrapped: slot 1000, energy 1000
source_slot0_suffix: slot 998, energy 1000
source_all_suffix: slot 959, energy 1000
action_wrapped_source_slot0_suffix: slot 998, energy 1000
```

Interpretation:

Sequence energy remains very stable under perturbation. Strict slot readout is stable under moderate perturbation and degrades under broad all-token suffix mutation, which is expected and should remain a tracked robustness curve.

## Runtime / Bytes Boundary

Do not interpret `3623.13s` as inference latency. That number is the full debug diagnostic test, including training, cleanup, flat parity, basin sweeps, capacity curves, address-radius sweeps, and ablations.

Current runtime-size evidence in this rung:

```text
SEQ_TOTAL_CENTER_COUNT: 131072
role_binding_edges: 143248
state_delta_edges: 0
flat readout parity: exact
```

Remaining runtime debt:

Run a dedicated release inference/readout bench for the compiled flat table. The diagnostic path is intentionally much heavier than the intended runtime path.

## Proof Boundary

Claim allowed:

```text
Paged u32 16-slot ordered operator-transfer gate passed on lengths 13..16:
strict ordered slot readout = 1000/1000, sequence energy = 1000/1000,
flat parity exact, ablations collapse, forbidden shortcut flags false.
```

Claim not allowed yet:

```text
Final broad LLMWave reasoning solved.
Generalization beyond this ordered sequence family solved.
Release runtime latency proven for this 16-slot path.
Reserve pages 18..31 proven useful.
```

## Next Debt

1. Multi-seed repeat for length 13..16.
2. Dedicated release runtime/latency bench for compiled flat readout.
3. Wider rule families beyond the current 8 ordered sequence operators.
4. Optional optimization: grouped/offset diagnostic readout so parity sweeps do not take debug-hour scale.
