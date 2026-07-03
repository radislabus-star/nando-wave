# v4 Next Mechanism Contract

Date: 2026-07-01

## Purpose

This file is the guardrail for the next architecture/code step after the current
v4 operator battery rung.

Literature guard:

```text
The next mechanism must be checked against the role/filler binding literature,
not invented from metric pressure alone.

Current classical pointers:
  Smolensky tensor product variable binding:
    role/filler bindings need an explicit binding algebra.
  Plate HRR:
    decoded bound structures are noisy and require cleanup memory;
    frame-specific roles beat generic roles;
    skipping cleanup is faster but weaker;
    fixed thresholds are unreliable across frame compositions.
  Chen et al. role-filler binding:
    real role/filler binding means arbitrary fillers still work when
    role/filler correlations seen in training are violated.

Therefore the next candidate must be framed as generic cleanup/readout or
learned role-specific disambiguation, not as a hardcoded output slot,
proof_rule_id authority, or local_out_t patch.
```

2026-07-02 update: operator compiler guard:

```text
The one-pass operator grokking probe changed the priority order.

Artifact:
  data/rule_logic_operator_battery_v4/diagnostics/OPERATOR_GROKKING_PROBE.md
  data/rule_logic_operator_battery_v4/diagnostics/operator_grokking_probe_report.json

Result:
  train_rows: 5312
  heldout_rows: 5312
  compiled_operator_programs: 380
  operator_program_conflicts: 0
  heldout_accuracy_milli: 1000
  heldout_wrong_match_rows: 0

Interpretation:
  The corpus contains compact reusable operator programs that can be induced
  one-pass from train transitions. Therefore epochs must not become the
  philosophy of the system. Epochs are allowed as repair/cleanup only after a
  compiler-first path is attempted.

New priority:
  1. one-pass induced operator program
  2. compile induced program into Wave weights / sequence energy / cleanup
  3. run strict slot, sequence energy, flat parity, ablation, shortcut gates
  4. use epoch/error-driven repair only if the compiled Wave gate stays red

Boundary:
  This probe is not yet a Wave runtime proof. It uses normalized
  rule_action_example as an operator key, so the next gate must show that the
  induced program can be represented and executed through the Wave field/readout
  path without proof_rule_id, target_id, concrete_x_lookup, or manual local_out_t.
```

2026-07-02 update: phase-center operator guard:

```text
The phase-center operator probe is now the preferred compiler-first direction.
It is closer to the original Wave/Fourier claim than epoch table repair or
symbolic slot-map extraction.

Artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_OPERATOR_PROBE_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/phase_center_operator_probe_c32_report.json

Method:
  relation waves
  -> circular/phase center of mass
  -> correct heldout transition closer than same-bag wrong transition

Result:
  phase cells: 32
  compiled_phase_centers: 380
  heldout_rows: 5312
  heldout_accuracy_milli: 1000
  wrong_wins: 0
  median_margin: 0.7671
  p10_margin: 0.3130
  no_action_accuracy_milli: 782
  no_action_wrong_wins: 1156

Priority:
  1. phase-center relation-wave compiler
  2. phase ablation and C8/C16/C32/C64 capacity curve
  3. weak-action/no-action probes
  4. compile phase centers into Rust/Wave flat runtime
  5. prove strict slot readout, sequence energy, flat parity, and ablations
  6. use epoch/error-driven repair only if the phase-center Wave gate stays red

Boundary:
  This is still not production Wave runtime proof. It is a strong diagnostic
  and direction-setting result. Do not claim full operator grokking until the
  Rust/Wave runtime path passes with forbidden flags false.
```

2026-07-02 update: Rust phase-center runtime pass:

```text
Artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_RUNTIME_PROBE_C32.md

Test:
  operator_battery_v4_phase_center_runtime_probe_report

Result:
  verdict: PHASE_CENTER_RUNTIME_PROBE_PASS
  cells: 32
  action_compiled_phase_centers: 380
  action_train_rows: 5312
  action_heldout_rows: 5312
  action_heldout_accuracy_milli: 1000
  action_wrong_wins: 0
  action_p10_margin: 0.312965
  no_action_heldout_accuracy_milli: 782
  no_action_wrong_wins: 1156
  phase_center_bytes_estimate: 389120
  epoch_repair_used: false
  forbidden flags: false

Meaning:
  The phase-center operator signal is now reproduced inside the Rust proof
  runtime. The next mechanism priority is confirmed:

  1. phase-center compiler/runtime
  2. phase capacity and ablation
  3. strict readout/cleanup only where phase energy is green and readout is red
  4. epoch repair only as the last fallback
```

2026-07-02 update: phase capacity and ablation pass:

```text
Artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CAPACITY_ABLATION_C8_C64.md

Test:
  operator_battery_v4_phase_center_capacity_ablation_report

Result:
  verdict: PHASE_CENTER_CAPACITY_ABLATION_PASS

  C8:  action_wrong_wins=11
  C16: action_wrong_wins=1
  C32: action_wrong_wins=0, p10_margin=0.312965
  C64: action_wrong_wins=0, p10_margin=0.416739

  C32 top16 train-cell ablation:
    accuracy_milli=999
    wrong_wins=5
    p10_margin=0.157739

Meaning:
  C32 is the first zero-wrong compact phase-center rung. Ablation selected from
  train-only center separation degrades the result, so the mechanism is not just
  a lucky final score.
```

2026-07-02 update: phase flat runtime pass:

```text
Artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_FLAT_RUNTIME_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_COMPILER_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_BENCH_C32_C64.md

Test:
  operator_battery_v4_phase_center_flat_runtime_report
  operator_battery_v4_phase_center_core_runtime_report
  operator_battery_v4_phase_center_core_runtime_benchmark_report
  operator_battery_v4_phase_center_core_compiler_report

Release result:
  verdict: PHASE_CENTER_FLAT_RUNTIME_PASS
  compiler_accuracy_milli: 1000
  flat_accuracy_milli: 1000
  flat_wrong_wins: 0
  flat_sign_parity_mismatches: 0
  flat_margin_parity_mismatches: 0
  no_action_flat_accuracy_milli: 782
  no_action_flat_wrong_wins: 1156
  flat_records: 380
  flat_runtime_bytes_estimate: 407360
  flat_eval_p50_latency_ns: 136
  flat_eval_p99_latency_ns: 506
  flat_eval_total_us: 1032
  core_runtime_verdict: PHASE_CENTER_CORE_RUNTIME_PASS
  core_accuracy_milli: 1000
  core_wrong_wins: 0
  core_sign_parity_mismatches: 0
  core_margin_parity_mismatches: 0
  core_runtime_bytes_estimate: 401280
  core_eval_p50_latency_ns: 69
  core_eval_p99_latency_ns: 400
  core_eval_total_us: 671
  core_runtime_path: nando_core::PhaseCenterFlatRuntime
  core_eval_path: precompiled_core_tasks_no_bridge_allocations
  core_bench_C32: accuracy=1000 wrong_wins=0 p50=65ns p99=375ns bytes=401280
  core_bench_C64: accuracy=1000 wrong_wins=0 p50=190ns p99=651ns bytes=790400
  core_compiler_verdict: PHASE_CENTER_CORE_COMPILER_PASS
  core_compiler_accuracy_milli: 1000
  core_compiler_wrong_wins: 0
  core_compiler_margin_parity_mismatches: 0
  core_compiler_path: nando_core::PhaseCenterCompiler
  epoch_repair_used: false
  forbidden flags: false

Meaning:
  The phase-center compiler now has a flat CPU-runtime scorer exported from
  nando-core. Next work should package/benchmark this runtime path and attach
  it to strict slot/readout, not return to slow Python demos or epoch-first
  repair.
```

2026-07-02 update: core compiler API pass:

```text
Artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_COMPILER_C32.md

Test:
  operator_battery_v4_phase_center_core_compiler_report

Release result:
  verdict: PHASE_CENTER_CORE_COMPILER_PASS
  compiler_path: nando_core::PhaseCenterCompiler
  runtime_path: nando_core::PhaseCenterFlatRuntime
  cells: 32
  flat_records: 380
  heldout_rows: 5312
  core_accuracy_milli: 1000
  core_wrong_wins: 0
  core_sign_parity_mismatches: 0
  core_margin_parity_mismatches: 0
  core_runtime_bytes_estimate: 401280
  core_eval_p50_latency_ns: 64
  core_eval_p99_latency_ns: 402
  epoch_repair_used: false
  forbidden flags: false

Meaning:
  The phase-center compiler is now an exported Rust core API, not a Python
  demo and not a test-only structure. It compiles numeric program indices plus
  positive/negative relation-wave atoms into the same flat scorer runtime.

Boundary:
  This closes the compact scorer/compiler bridge for the current v4 C32 rung.
  It does not close strict ordered readout, conditional train_per_cell=2
  robustness, text generation, or multi-step reasoning.
```

2026-07-02 update: core runtime package pass:

```text
Artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_PACKAGE_C32.md

Test:
  operator_battery_v4_phase_center_core_runtime_package_report

Release result:
  verdict: PHASE_CENTER_CORE_RUNTIME_PACKAGE_PASS
  compiler_path: nando_core::PhaseCenterCompiler
  package_path: nando_core::PhaseCenterFlatRuntime::to_bytes/from_bytes
  runtime_path: nando_core::PhaseCenterFlatRuntime
  cells: 32
  flat_records: 380
  heldout_rows: 5312
  package_magic: [78, 87, 80, 67, 70, 48, 48, 49]
  package_bytes: 389136
  package_fingerprint64: 14549306353473335964
  runtime_bytes_estimate: 401280
  package_accuracy_milli: 1000
  package_wrong_wins: 0
  package_sign_parity_mismatches: 0
  package_margin_parity_mismatches: 0
  package_eval_p50_latency_ns: 64
  package_eval_p99_latency_ns: 407
  epoch_repair_used: false
  forbidden flags: false

Meaning:
  The green phase-center scorer is now portable as a deterministic binary
  runtime package. The package stores flat centers only; it does not carry
  task rows, answer lookup, proof_rule_id authority, concrete_x lookup, or
  manual local_out_t.

Boundary:
  This closes a product-packaging step for the scorer kernel. It still does
  not close strict ordered readout, conditional train_per_cell=2 robustness,
  text generation, or multi-step reasoning.
```

2026-07-02 update: core runtime package benchmark:

```text
Artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_PACKAGE_BENCH_C32_C64.md

Test:
  operator_battery_v4_phase_center_core_runtime_package_benchmark_report

Release result:
  verdict: PHASE_CENTER_CORE_RUNTIME_PACKAGE_BENCH_PASS

  C32:
    package_bytes: 389136
    inspected_payload_bytes: 389120
    package_fingerprint64: 14549306353473335964
    package_load_us: 787
    accuracy_milli: 1000
    wrong_wins: 0
    p10_margin: 0.312965
    p50_latency_ns: 69
    p99_latency_ns: 416

  C64:
    package_bytes: 778256
    inspected_payload_bytes: 778240
    package_fingerprint64: 16888657547359761052
    package_load_us: 1646
    accuracy_milli: 1000
    wrong_wins: 0
    p10_margin: 0.416739
    p50_latency_ns: 163
    p99_latency_ns: 520

Meaning:
  C32 is the current compact runtime package point. C64 buys margin reserve but
  nearly doubles package bytes and is slower in the scorer benchmark.
  Latency/load values are single-run release samples; proof invariants are
  package bytes, accuracy, wrong-wins, parity mismatches, and forbidden flags.

Boundary:
  This is a package/scorer benchmark only. It does not close strict ordered
  readout, conditional train_per_cell=2 robustness, text generation, or
  multi-step reasoning.
```

2026-07-02 update: product-facing package CLI:

```text
Artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_PRODUCT_PACKAGE_CLI_C32.md

Command:
  cargo run -p nando-cli --release -- phase-package-v4
  cargo run -p nando-cli --release -- phase-package-inspect
  cargo run -p nando-cli --release -- phase-package-score-v4
  cargo run -p nando-cli --release -- phase-eval-pack-v4
  cargo run -p nando-cli --release -- phase-package-score-pack-v4
  cargo run -p nando-cli --release -- phase-action-boundary-v4
  cargo run -p nando-cli --release -- phase-action-corpus-v1
  cargo run -p nando-cli --release -- phase-action-contract-v1
  cargo run -p nando-cli --release -- phase-action-shortcut-v1
  cargo run -p nando-cli --release -- phase-action-runtime-v1
  cargo run -p nando-cli --release -- phase-action-package-v1
  cargo run -p nando-cli --release -- phase-action-package-inspect-v1

Release result:
  verdict: PHASE_PACKAGE_V4_PASS
  inspect_verdict: PHASE_PACKAGE_INSPECT_PASS
  score_verdict: PHASE_PACKAGE_SCORE_V4_PASS
  verify_verdict: PHASE_PACKAGE_VERIFY_PASS
  eval_pack_verdict: PHASE_EVAL_PACK_V4_PASS
  score_pack_verdict: PHASE_PACKAGE_SCORE_PACK_V4_PASS
  action_boundary_verdict: PHASE_ACTION_BOUNDARY_V4_WATCH
  package_path: target/nando-wave/phase-center-v4-c32.nwpc
  manifest_path: target/nando-wave/phase-center-v4-c32.nwpc.manifest.json
  eval_pack_path: target/nando-wave/phase-center-v4-c32.eval-pack
  score_report_path: target/nando-wave/phase-center-v4-c32.score-report.json
  score_pack_report_path: target/nando-wave/phase-center-v4-c32.score-pack-report.json
  rows: 10624
  train_rows: 5312
  heldout_rows: 5312
  cells: 32
  flat_records: 380
  package_bytes: 389136
  eval_pack_bytes: 10921516
  manifest_bytes_sample: 134182
  score_report_bytes_sample: 1832
  package_fingerprint64: 14549306353473335964
  operator_key_count: 380
  manifest_matches_package: true
  accuracy_milli: 1000
  wrong_wins: 0
  p10_margin: 0.312965
  p50_latency_ns: 173
  p99_latency_ns: 614
  rows_per_second: 3603718.51
  action_ablation_accuracy_milli: 443
  action_ablation_wrong_wins: 2958
  score_from_package_compiler_used: false
  score_from_package_accuracy_milli: 1000
  score_from_package_wrong_wins: 0
  score_from_package_p99_latency_ns: 520
  score_from_package_rows_per_second: 4148032.93
  score_from_package_action_ablation_accuracy_milli: 443
  score_from_package_action_ablation_wrong_wins: 2958
  score_from_package_report_verdict: PHASE_PACKAGE_SCORE_V4_PASS
  score_from_eval_pack_report_verdict: PHASE_PACKAGE_SCORE_PACK_V4_PASS
  score_from_eval_pack_compiler_used: false
  score_from_eval_pack_eval_task_package_used: true
  score_from_eval_pack_corpus_jsonl_used_in_score_loop: false
  score_from_eval_pack_accuracy_milli: 1000
  score_from_eval_pack_wrong_wins: 0
  score_from_eval_pack_p99_latency_ns: 429
  score_from_eval_pack_rows_per_second: 5614990.50
  action_boundary_explicit_operator_class_label_rows: 10624
  action_boundary_explicit_operator_family_label_rows: 10624
  action_boundary_explicit_order_slot_map_rows: 4096
  action_boundary_explicit_branch_slot_map_rows: 1536
  autonomous_action_router_claim_allowed: false
  tampered_manifest_verdict: PHASE_PACKAGE_SCORE_V4_WATCH
  tampered_manifest_exit_code: 1
  tampered_manifest_score_report_verdict: PHASE_PACKAGE_SCORE_V4_WATCH
  tampered_score_report_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  tampered_score_report_verify_exit_code: 1
  tampered_eval_pack_score_exit_code: 1
  tampered_score_pack_report_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  tampered_score_pack_report_verify_exit_code: 1
  tampered_score_pack_no_eval_flag_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  tampered_score_pack_no_eval_flag_exit_code: 1
  tampered_score_pack_jsonl_loop_true_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  tampered_score_pack_jsonl_loop_true_exit_code: 1
  tampered_score_pack_jsonl_missing_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  tampered_score_pack_jsonl_missing_exit_code: 1
  verify_prints_manifest_and_score_report_forbidden_flags: true
  forbidden flags: false

Meaning:
  The package path is no longer only an ignored cargo test. A user-facing
  Rust CLI can build, save, load, inspect, manifest, and score the fixed v4
  C32 package. The separate inspect command validates an already-built package
  against its sidecar manifest without reading the corpus. The manifest now
  carries the record-indexed operator key table; it does not carry proof_rule_id
  authority, concrete answers, or target ids.
  The score command uses only the existing package plus manifest operator keys;
  it does not invoke the compiler.

Boundary:
  This is still a phase-center scorer package. It does not close strict ordered
  decoder/generator or multi-seed strict robustness. The manifest records that
  non-commercial license-file metadata is declared, but commercial license
  packaging is not closed by this scorer gate.

Action contract boundary:
  phase-action-boundary-v4 stays red for the current v4 corpus because it uses
  operator labels/slot maps as a scorer key contract. The next raw
  action-router corpus must pass phase-action-contract-v1 first: action_tree is
  SELECT + TRANSFORM + WRITE + CONDITION + CHECK, with no slot-map, target leak,
  proof_rule_id authority, lookup authority, or manual local_out_t.

Clean action corpus rung:
  phase-action-corpus-v1 generates a deterministic Rust-only
  action_contract_v1 JSONL corpus. The generated corpus is not a proof by
  itself; it must pass phase-action-contract-v1, phase-action-package-v1, and
  phase-action-package-inspect-v1.

Clean action shortcut gate:
  phase-action-shortcut-v1 must pass before package proof is trusted. It rejects
  exact heldout state lookup, exact state+action transition lookup, train/heldout
  token overlap, train/heldout length reuse, non-same-bag negatives, identical
  correct/wrong outputs, and source-bigram wins for the correct candidate.

Clean contract runtime smoke:
  phase-action-runtime-v1 validates the same contract, compiles train rows into
  PhaseCenterFlatRuntime, scores heldout correct-vs-wrong transitions, and
  requires action-ablation degradation. This is a Rust compiler/runtime smoke,
  not a broad action-router proof.

Clean action package smoke:
  phase-action-package-v1 saves the clean action runtime into a .nwpc package,
  reloads it, scores heldout through the loaded package, and writes a manifest.
  phase-action-package-inspect-v1 verifies package/manifest parity and rejects
  tampered fingerprints.
  phase-action-package-score-v1 scores an already-saved .nwpc action package
  without recompiling it and writes a score report.
  phase-action-package-verify-v1 verifies package, manifest, and score report
  as one product proof artifact.

Workflow-shaped action corpus rung:
  phase-action-domain-corpus-v1 generates a deterministic Rust-only
  workflow-shaped action_contract_v1 JSONL corpus. It uses operationally named
  spans and the same clean action_tree contract, but it is still a bounded
  package smoke, not a broad domain reasoning proof.

Current workflow-shaped result:
  corpus_path: data/rule_logic_operator_battery_v4/action_contract_v1/generated_domain_action_contract_v1.jsonl
  rows: 120
  train_rows: 72
  heldout_rows: 48
  operator_key_count: 6
  same_bag_rows: 120
  contract_verdict: PHASE_ACTION_CONTRACT_V1_PASS
  shortcut_verdict: PHASE_ACTION_SHORTCUT_V1_PASS
  exact_state_lookup_hits: 0
  exact_transition_lookup_hits: 0
  heldout_token_overlap_rows: 0
  heldout_length_seen_in_train_rows: 0
  source_bigram_correct_wins: 0
  source_bigram_ties: 48
  package_verdict: PHASE_ACTION_PACKAGE_V1_PASS
  inspect_verdict: PHASE_ACTION_PACKAGE_INSPECT_V1_PASS
  package_fingerprint64: 5367415087033800111
  package_bytes: 6160
  runtime_bytes_estimate: 6336
  action_ablation_policy: all non-target action centers, not first-other center
  action_ablation_eval_rows: 240
  accuracy_milli: 1000
  wrong_wins: 0
  p50_latency_ns: 210
  p99_latency_ns: 522
  action_ablation_accuracy_milli: 567
  action_ablation_wrong_wins: 104
  local_out_t_tamper_rejected: true
  package_fingerprint_tamper_rejected: true
  python_demo_used: false
  forbidden flags: false

Action package score/verify result:
  generated_action_contract_v1:
    score_verdict: PHASE_ACTION_PACKAGE_SCORE_V1_PASS
    verify_verdict: PHASE_ACTION_PACKAGE_VERIFY_V1_PASS
    package_fingerprint64: 14869999570221545448
    package_bytes: 10256
    flat_records: 10
    rows: 200
    heldout_eval_rows: 80
    action_ablation_eval_rows: 720
    accuracy_milli: 1000
    wrong_wins: 0
    score_p99_latency_ns: 490
    action_ablation_accuracy_milli: 450
    action_ablation_wrong_wins: 396
    compiler_used: false
  generated_domain_action_contract_v1:
    score_verdict: PHASE_ACTION_PACKAGE_SCORE_V1_PASS
    verify_verdict: PHASE_ACTION_PACKAGE_VERIFY_V1_PASS
    package_fingerprint64: 5367415087033800111
    package_bytes: 6160
    flat_records: 6
    rows: 120
    heldout_eval_rows: 48
    action_ablation_eval_rows: 240
    accuracy_milli: 1000
    wrong_wins: 0
    score_p99_latency_ns: 674
    action_ablation_accuracy_milli: 567
    action_ablation_wrong_wins: 104
    compiler_used: false
  score_bad_target_leak_refused: true
  score_local_out_t_tamper_rejected: true
  score_bad_fingerprint_tamper_rejected: true
```

Current status:

```text
order:       GREEN_AFTER_L1_SHORT_TOKEN_IDENTITY_FIX
edit:        GREEN_AFTER_EDIT_DEMO_MARKER_LENGTH_CHANNEL
conditional: GREEN_AFTER_CONDITION_ACTION_CONJUNCTION
composed:    GREEN_AFTER_COMPOSED_DEMO_SLOT_CHANNEL
```

The original/current rung is green, but the seeds 1,2,3 strict multi-seed rung
is red because seed2/order has one strict slot failure. The next step must not
turn single-seed success into a broader claim.

Multi-seed boundary:

```text
seed0/current rung: green
seed1 shortcut gate: green
seed1 runtime robustness: GREEN
seeds 1,2,3 strict runtime robustness: RED
red issue: seed2/order/order_block_reverse_4_len13 out12->src12 gap -11426
static diagnosis: priem target lanes hit other role slots on 8/16 positive impulses
dynamic diagnosis: true slot12 contributes +15680, but other role slots
contribute -51050, mostly slot10/slot11 suppression; train cleanup repairs 0
rows, so the remaining failure is heldout collision pressure.
density diagnosis: targeted factor2 reweighting of
order_block_reverse_4_len13 turns the case green; classify as data/weight
sparsity under local multi-role collision pressure, not architecture ceiling.

train_per_cell=2 follow-up:
  seed2/order smoke: GREEN
  seed1/order:       GREEN
  seed1/edit:        GREEN
  seed1/conditional: RED

conditional train_per_cell=2 red:
  conditional_slot_ordered_sequence_accuracy_milli: 617
  conditional_sequence_energy_accuracy_milli: 973
  conditional_energy_pass_slot_fail: 273
  conditional_output_slot_cleanup_failed_slots: 1146
  flat parity mismatches: 0
  state_delta_edges: 0
  forbidden flags: false

report-consistency fix:
  run_multiseed_rung.py now picks the newest *_runtime_gate_release*.log when
  reusing existing runtime logs. The train_per_cell=2 multiseed_summary.json now
  records conditional RED in strict_runtime_issues instead of showing an empty
  issue list with missing conditional metrics.

interpretation:
  sequence energy is strong, but strict ordered slot readout is unstable under
  conditional branch transfer. This is the next real proof debt.

conditional density follow-up:
  train_per_cell=4 shortcut gate: clean
  conditional_slot_ordered_sequence_accuracy_milli: 611
  conditional_sequence_energy_accuracy_milli: 969
  full-corpus clean/distractor noise groups: 1000
  full-corpus prefix_suffix/instruction_noise groups: still red

updated blocker:
  noise-robust conditional decoder debt, not simple train-density sparsity.

conditional noise-isolation follow-up:
  clean_distractor isolated strict slot: 607
  clean_distractor isolated sequence energy: 956
  clean rows: 1000
  distractor rows: 761
  prefix_suffix isolated strict slot: 594
  prefix_suffix isolated sequence energy: 953
  instruction_noise isolated strict slot: 594
  instruction_noise isolated sequence energy: 953

refined blocker:
  clean rows are green, but distractor is not independently green when isolated.
  Prefix/suffix and instruction slices share the same red profile. Code audit
  shows wrapper text is not directly inserted into role slots, because
  sequence_source_tokens extracts only the explicit sequence segment. Treat this
  as conditional role/readout instability under noise-correlated schedule and
  surface pressure, not a reason to add local_out_t.

paired-noise follow-up:
  paired_noise=true
  semantic groups: 576
  strict slot: 615
  sequence energy: 958
  noise accuracies: clean/distractor/instruction_noise/prefix_suffix all 883
  surface accuracies: business 1000, symbols 1000, network 763, ru_words 768
  flat parity: 0
  forbidden flags: false

final refined blocker:
  wrapper text is not the active cause. Conditional strict-slot failure is now
  isolated to role/filler collision under richer token surfaces, especially
  ru_words/network, while sequence energy remains strong.

static collision follow-up:
  symbols multi/wrong-role collision: 140 milli, strict surface accuracy 1000
  business multi/wrong-role collision: 170 milli, strict surface accuracy 1000
  ru_words multi/wrong-role collision: 192 milli, strict surface accuracy 768
  network multi/wrong-role collision: 196 milli, strict surface accuracy 763
  missing_true_role_milli: 0 for all surfaces

collision-outcome follow-up:
  high_wrong_role_hit strict accuracy: 845
  mid_wrong_role_hit strict accuracy: 879
  low_wrong_role_hit strict accuracy: 924
  no_wrong_role_hit strict accuracy: 887

  business strict accuracy: 1000, avg_gap: 133505, min_gap: 29230
  symbols strict accuracy: 1000, avg_gap: 129630, min_gap: 28976
  network strict accuracy: 763, avg_gap: 29850, min_gap: -45024
  ru_words strict accuracy: 768, avg_gap: 29090, min_gap: -53964

  worst output/source pairs:
    out10->src7: 0 / 24 slots pass
    out1->src11: 0 / 8 slots pass
    out12->src2: 0 / 8 slots pass
    out12->src6: 0 / 8 slots pass
    out12->src7: 0 / 8 slots pass
    out6->src12: 8 / 48 slots pass
    out2->src0: 20 / 96 slots pass
    out1->src0: 24 / 96 slots pass

target/wrong lane-overlap follow-up:
  business target/wrong overlap: 22061, wrong hits true role: 22395
  network target/wrong overlap: 28574, wrong hits true role: 28147
  ru_words target/wrong overlap: 24147, wrong hits true role: 23987
  symbols target/wrong overlap: 123698, wrong hits true role: 123841

candidate-cleanup follow-up:
  candidate_slot_tasks: 228864
  epochs=2:
    strict row accuracy: 615
    sequence energy: 984
    failed slots: 972
    role_binding_edges: 73559
  epochs=8:
    train candidate min gap positive by epoch 3
    strict row accuracy: 615
    sequence energy: 984
    failed slots: 972
    role_binding_edges: 73559
  network surface accuracy after cleanup: 787
  ru_words surface accuracy after cleanup: 808
  flat parity: 0
  forbidden flags: false

sign-aware collision diagnostic:
  log: conditional_sign_aware_positive_collision_report_raw.log
  business current/sign-aware wrong-role hit: 170 -> 154
  network current/sign-aware wrong-role hit: 196 -> 178
  ru_words current/sign-aware wrong-role hit: 192 -> 179
  symbols current/sign-aware wrong-role hit: 140 -> 122
  missing_true_signed_role_milli: 0 for all surfaces
  worst remaining pairs:
    out2->src14: 500 -> 444
    out0->src15: 474 -> 461
    out14->src1: 471 -> 471
  same-sign residual collision share:
    business: 906 milli remains
    network: 908 milli remains
    ru_words: 932 milli remains
    symbols: 871 milli remains

residual-collision outcome diagnostic:
  log: conditional_residual_collision_outcome_cleanup8.log
  compiler/trainer trace:
    local_epochs: 8
    cleanup_epochs: 4
    candidate_cleanup_epochs: 8
    train_slot_accuracy_milli after cleanup: 1000
    train candidate min gap positive by epoch 3/8
    final role_binding_edges: 73559
  heldout:
    strict_row_accuracy_milli: 615
    sequence_energy_accuracy_milli: 984
    energy_pass_slot_fail: 284
  surface:
    business accuracy/min_gap: 1000 / 33022
    symbols accuracy/min_gap: 1000 / 38630
    network accuracy/min_gap: 787 / -45024
    ru_words accuracy/min_gap: 808 / -53964
  residual buckets:
    high_same_sign_residual accuracy: 859
    mid_same_sign_residual accuracy: 890
    low_same_sign_residual accuracy: 927
    no_same_sign_residual accuracy: 923

boundary:
  collision pressure correlates with red surfaces, but raw collision alone is
  not sufficient because business remains green at 170 milli and keeps large
  positive gaps. Target/wrong L1 overlap is also not sufficient because symbols
  remain green despite the largest overlap. Sign-aware matching is safe for
  positive target lanes, but removes only a small slice of wrong-role pressure
  and leaves the worst output/source pairs heavily collided. The current blocker
  is surface identity + same-sign folded role collision + weak strict-slot
  cleanup on specific output/source pairs. Generic candidate cleanup improves
  slot-level metrics but does not solve row-level strict accuracy, so the next
  mechanism must address heldout surface/collision transfer, not simply add more
  train negatives or a standalone sign key. The next measured target is the
  same-sign residual collision, not sign erasure. The residual outcome
  diagnostic refines this further: same-sign residual collision contributes, but
  is not sufficient alone, because business/symbols stay green under residual
  pressure and no_same_sign_residual still contains failed slots. The stronger
  target is energy/readout mismatch under surface pressure.
  Do not change architecture until the next diagnostic proves what generic
  repair removes that failure without forbidden substitutions.
```

Compiler/runtime boundary:

```text
Epochs are allowed in the compiler/trainer/proof stage to induce and audit the
compact operator tables. They are not part of runtime inference. A mechanism is
valid only if the compiled runtime path remains lookup-free, target_id-free,
proof_rule_id-free, concrete_x-free, and local_out_t-free.
```

The robustness artifact is:

```text
data/rule_logic_operator_battery_v4/diagnostics/multiseed/MULTISEED_ROBUSTNESS_REPORT.md
data/rule_logic_operator_battery_v4/diagnostics/multiseed/SEED2_ORDER_STRICT_SLOT_FAILURE.md
```

Do not widen the claim beyond seed1 until seed2/order is reproduced, diagnosed,
fixed without forbidden substitutions, and strict_runtime_issues is empty.

Updated boundary:

```text
Do not widen the claim to train_per_cell=2 full-battery robustness until
conditional strict readout is green. The order sparsity issue is no longer the
only blocker.
```

## Global Invariants

Every new mechanism must preserve these flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Forbidden substitutions:

```text
no target_id
no proof_rule_id authority
no concrete_x_lookup
no fixed answer template
no fixed frame_id
no manual local_out_t
no hand-coded bind(X)
no class-specific special case that bypasses the learned runtime
```

Required proof gates for any new mechanism:

```text
1. shortcut gates remain clean;
2. runtime gate is explicit, not hidden inside corpus generation;
3. ablation without the new channel collapses;
4. field/flat parity holds for the new decision path;
5. forbidden flags remain false;
6. report records both green and red metrics;
7. source-group or seed-heldout uses unseen tokens/surfaces where applicable.
```

## Current v4 Mechanisms

### Order

Mechanism:

```text
source role filler channel
+ action/operator signal
+ L1 short-token identity repair
```

Current claim:

```text
order_slot_ordered_sequence_accuracy_milli: 1000
order_sequence_energy_accuracy_milli: 1000
flat parity mismatches: 0
core ablations: 0
forbidden flags: false
```

Boundary:

```text
The L1 short-token repair is generic for short normalized tokens. It is not an
L3 operator hack and not a target lookup.
```

### Edit

Historical red cause:

```text
rows_not_representable_by_current_role_transfer: 1632 / 3072
rows_with_non_source_output_tokens: 1280
rows_with_marker_output_tokens: 1280
rows_output_len_over_slots: 192
rows_correct_wrong_len_mismatch: 256
```

Implemented mechanism:

```text
source role filler channel
+ action-derived edit demo slot channel
+ marker/end role slot
+ bounded 17 output slots
```

Current claim:

```text
edit_slot_ordered_sequence_accuracy_milli: 1000
edit_sequence_energy_accuracy_milli: 1000
flat parity mismatches: 0
state_delta_edges: 0
forbidden flags: false
```

Required edit-specific ablations already checked:

```text
without_binding -> 0
without_action -> 0
without_edit_demo -> 260 strict slot / 715 energy
without_marker_role -> 984 strict slot / 1000 energy
without_role -> 0
without_active_fringe -> 0
```

Boundary:

```text
This is a bounded edit proof for the current 17-slot battery, not an unbounded
text decoder. The marker/end role channel is necessary for marker and length
cases, but not all edit rows depend on it.
```

### Conditional

Implemented mechanism:

```text
state condition channel read from state_before
+ learned condition/action conjunction
+ branch selection inside L3
+ generic conditional action-surface suppressed by default
```

Current claim:

```text
conditional_slot_ordered_sequence_accuracy_milli: 1000
conditional_sequence_energy_accuracy_milli: 1000
flat parity mismatches: 0
condition/action ablations collapse below chance
forbidden flags: false
```

Train-density boundary:

```text
On train_per_cell=2 seed1, conditional is red:
  strict slot: 617 / 1000
  sequence energy: 973 / 1000
  flat parity: clean
  forbidden flags: false

So the mechanism has a strong operator-energy signal but does not yet provide
stable strict slot readout for conditional branches under the denser corpus.

The train_per_cell=4 conditional-only diagnostic does not fix this:
  strict slot: 611
  sequence energy: 969
  flat parity: clean
  forbidden flags: false

The paired-noise diagnostic is done and rules out wrapper text as the active
cause. The next diagnostic must measure collision pressure for ru_words/network
vs symbols/business and then correlate collision class with actual gap/failure
for failing output-source pairs. Do not add manual local_out_t or
target-specific binding.
```

Boundary:

```text
The raw conditional action text contains both then/else branches and must not be
used as a fuzzy surface shortcut. The accepted proof channel is the selected
condition/action conjunction extracted from rule_action_example and state_before.
Strict readout ablates below chance without it.
```

### Composed

Implemented mechanism:

```text
full-length neutral action demo
+ parsed final demo state
+ composed demo slot page 20
+ generic composed action-surface suppressed by default
```

Current claim:

```text
composed_slot_ordered_sequence_accuracy_milli: 1000
composed_sequence_energy_accuracy_milli: 1000
flat parity mismatches: 0
composed slot ablation collapses below chance
forbidden flags: false
```

Boundary:

```text
The raw composed action text contains the demo and must not be used as a fuzzy
surface shortcut. The accepted proof channel is the parsed neutral demo slot
page. Ablation without that page collapses to 0 on the current rung.
```

## Next Execution Order

Use this order unless new evidence changes it:

```text
1. Keep failed_slots > 0 as a red strict gate in Rust and Python reports.
2. Replace targeted density duplication with a principled full-corpus policy.
   First candidate: OPERATOR_BATTERY_TRAIN_PER_CELL=2 for the full v4 battery.
3. Rebuild seeds 1,2,3 and rerun shortcut + runtime gates for all four classes.
4. Accept the density policy only if strict_runtime_issues is empty and
   ablations/parity/forbidden flags remain clean.
5. Runtime bytes/latency report for green paths only after strict issues are zero.
6. Capacity curve: operators vs memory/edges/false positives/collapse.
7. 32-slot paged-u32 scaling rung.
8. New operator families beyond the current v4 battery.
```

Do not add a new mechanism just because it is attractive. Add one only after a
red gate is reproduced, diagnosed, and tied to a concrete representational
failure.

## Closed Packaging Rung

The action scorer package path now has a binary eval-pack layer:

```text
phase-action-eval-pack-v1
phase-action-package-score-pack-v1
phase-action-package-verify-v1
phase-action-package-bench-pack-v1
phase-action-package-bench-verify-v1
phase-action-product-proof-v1
phase-action-product-verify-v1
phase-action-release-suite-v1
phase-action-release-verify-v1
phase-action-license-package-v1
phase-action-license-verify-v1
```

This rung is green for both generated action and domain action packages. The
score-pack path records:

```text
eval_task_package_used: true
corpus_jsonl_used_in_score_loop: false
compiler_used: false
```

Tamper checks reject:

```text
cross-package eval-pack
eval_task_package_used=false
corpus_jsonl_used_in_score_loop=true
corpus_jsonl_used_in_bench_loop=true
package_fingerprint64 mismatch
cross-package score report
product-proof forbidden flag
```

This is a product-artifact proof for the flat scorer only. It must not be used
to claim the strict ordered decoder, autonomous raw action parsing, or broad
workflow reasoning.

Current benchmark gate:

```text
generated action:
  bench_samples: 80000
  p99_latency_ns: 135
  runtime_bytes_estimate: 10560

domain action:
  bench_samples: 48000
  p99_latency_ns: 106
  runtime_bytes_estimate: 6336
```

Current product-proof bundle:

```text
generated action:
  product-proof: PASS
  product-verify: PASS
  package_fingerprint64: 14869999570221545448
  score_accuracy_milli: 1000
  bench_samples: 80000

domain action:
  product-proof: PASS
  product-verify: PASS
  package_fingerprint64: 5367415087033800111
  score_accuracy_milli: 1000
  bench_samples: 48000
```

Current release-suite:

```text
release-suite: PASS
release-verify: PASS
artifact_count: 2
total_package_bytes: 16416
total_eval_pack_bytes: 1118552
total_runtime_bytes_estimate: 16896
total_bench_samples: 128000
max_score_p99_latency_ns: 395
max_bench_p99_latency_ns: 117
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used: false
forbidden_used: false
commercial_license_closed: false
```

Current non-commercial license package:

```text
license-package: PASS
license-verify: PASS
license_file: LICENSE-NONCOMMERCIAL.md
license_package_kind: phase_action_noncommercial_license_package_v1
license_file_fingerprint64: 17377756494518932165
cargo_workspace_license_file_declared: true
cargo_workspace_mit_license_declared: false
cargo_crate_license_file_workspace_declared: true
cargo_crate_license_workspace_declared: false
release_suite_matches_sources: true
release_suite_license_boundary_mentions_mit: false
commercial_use_allowed: false
noncommercial_use_allowed: true
commercial_license_closed: false
non_commercial_license_closed: true
```
