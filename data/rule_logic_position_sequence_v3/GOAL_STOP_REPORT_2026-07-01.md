# Goal Stop Report - 2026-07-01

User stop command received while running the operator-pair beyond-v3
multi-seed continuation. The active long Rust run was interrupted with
Ctrl-C and exited with code 130.

## Stop Boundary

- Stopped run:
  `length_9_12_seed_012_operator_pair_action`
- Command class:
  `cargo test -p nando-core --test wavepredictor_binding_pressure_l3 -- --ignored ordered_position_binding_v3_combined_objective_probe_must_preserve_flat_energy_parity --nocapture`
- Interrupted at:
  `position_sequence_v3_combined: ablation_without_binding_start`
- Artifact log:
  `data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_012_operator_pair_action/combined_objective_rust_gate.log`
- Important boundary:
  seed 012 has no final PASS/FAIL runtime verdict because the run was stopped
  before final metrics printed.

## Proven Before Stop

### Base v3 operator-pair action motif gate

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/operator_pair_action_base_v3/combined_objective_rust_gate.log`

Result:

- strict slot ordered accuracy: 1000 milli;
- flat strict slot ordered accuracy: 1000 milli;
- sequence energy accuracy: 1000 milli;
- sequence energy p10 gap: 225820;
- symmetry sequence energy accuracy: 1000 milli;
- full_mirror strict accuracy: 1000 milli;
- output-slot cleanup accuracy: 1000 milli;
- ablations without binding/action/role/active-fringe: 0 milli;
- flat sequence-energy parity mismatches: 0;
- flat gap parity mismatches: 0;
- `state_delta_edges: 0`;
- forbidden authority flags: false.

Interpretation:

- current v3 is green when L3 receives separable operator-pair action motifs
  from the rule action demonstration;
- this is not `target_id`, not `proof_rule_id` authority, not concrete token
  lookup, and not manual runtime `local_out_t`.

### Beyond-v3 length 9..12 seed 011 operator-pair gate

Artifact:

- `data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_011_operator_pair_action/combined_objective_rust_gate.log`

Result:

- heldout rows: 640;
- strict slot ordered accuracy: 1000 milli;
- flat strict slot ordered accuracy: 1000 milli;
- sequence energy accuracy: 1000 milli;
- sequence energy p10 gap: 1166244;
- symmetry sequence energy accuracy: 1000 milli;
- non-symmetry sequence energy accuracy: 1000 milli;
- output slots 0..11: all 1000 milli;
- rule families including `full_mirror`: all 1000 milli;
- ablations without binding/action/role/active-fringe: 0 milli;
- flat sequence-energy parity mismatches: 0;
- flat gap parity mismatches: 0;
- `state_delta_edges: 0`;
- forbidden authority flags: false.

Interpretation:

- the old blocker was action/operator motif separability, not sequence length,
  flat runtime, or missing target tokens;
- with separable operator-pair action motifs, L3 learns compact role/filler
  transfer on unseen heldout tokens up to length 12.

## Negative Result Preserved

All-same-bag candidate cleanup was tested and rejected:

- cleanup4 + candidate1:
  - strict slot: 587;
  - sequence energy: 981;
  - full_mirror strict: 200.
- cleanup8 + candidate1:
  - strict slot: 624;
  - sequence energy: 974;
  - full_mirror strict: 242.

Interpretation:

- pairwise pressure against every same-bag token is too blunt;
- it weakens operator-energy geometry and must not be accepted as the fix.

## Generated But Not Runtime-Proven Before Stop

### Beyond-v3 length 9..12 seed 012

Corpus:

- `data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_012/accepted_position_sequence_tasks_v3.jsonl`

Shortcut gate:

- verdict: `VALID_POSITION_SEQUENCE_V3_CANDIDATE`;
- exact lookup: 0 milli;
- L2-neighbor target copy: 0 milli;
- proof-rule-id majority: 0 milli;
- output-position prior: 0 milli;
- Markov/bigram: 500 milli;
- bag-of-tokens: 500 milli;
- same-bag derangement: 1000 milli.

Runtime partial before interruption:

- local training completed 8/8 epochs;
- cleanup completed 4/4 epochs;
- train slot accuracy reached 1000 milli;
- train energy accuracy reached 1000 milli;
- `state_delta_edges: 0`;
- role-binding edges after training: 60695.

No heldout verdict was printed before stop.

### Beyond-v3 length 9..12 seed 013

Corpus:

- `data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_013/accepted_position_sequence_tasks_v3.jsonl`

Shortcut gate:

- verdict: `VALID_POSITION_SEQUENCE_V3_CANDIDATE`;
- exact lookup: 0 milli;
- L2-neighbor target copy: 0 milli;
- proof-rule-id majority: 0 milli;
- output-position prior: 0 milli;
- Markov/bigram: 500 milli;
- bag-of-tokens: 500 milli;
- same-bag derangement: 1000 milli.

Runtime was not started before stop.

## 16-Slot Rung Status

Not executed before stop.

Accepted next plan, but not run:

- build length 13..16 corpus;
- no `u32`;
- no manual `local_out_t`;
- compare folded role profiles:
  - `slot16_stride3840`;
  - `slot16_stride3072`;
- run shortcut gates first;
- then run combined objective with operator-pair action motifs;
- required metrics:
  - strict slot ordered accuracy;
  - sequence energy accuracy;
  - `full_mirror` / `pair_swap` breakdown;
  - flat gap parity = 0;
  - flat sequence-energy parity = 0;
  - ablations without binding/action/role/active-fringe = 0;
  - forbidden authority flags false.

## Current Claim Boundary

Allowed claim:

- current v3 and one beyond-v3 length 9..12 slice pass with operator-pair
  action motifs and no forbidden authority.

Forbidden claim:

- broad compact transferable operator is fully proven.

Remaining proof debt:

- complete multi-seed beyond-v3 runtime;
- run 16-slot pressure rung;
- test new rule/token/noise families;
- prove learned L2 induction of operator-pair motifs without a test-only
  `operator_slots:` parser.
