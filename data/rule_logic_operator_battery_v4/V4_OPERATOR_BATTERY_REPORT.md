# v4 Operator Battery Report

Date: 2026-07-01

## Verdict

`V4_OPERATOR_BATTERY_CURRENT_RUNG_STRONG_BUT_MULTI_SEED_STRICT_RED`

The first v4 operator battery is green for the single-seed/current rung:

```text
order:       GREEN_AFTER_L1_SHORT_TOKEN_IDENTITY_FIX
edit:        GREEN_AFTER_EDIT_DEMO_MARKER_LENGTH_CHANNEL
conditional: GREEN_AFTER_CONDITION_ACTION_CONJUNCTION
composed:    GREEN_AFTER_COMPOSED_DEMO_SLOT_CHANNEL
```

This proves the current proof-gated operator battery on the original/current
seed. It does not prove multi-seed strict ordered readout, an unbounded decoder,
or every future operator family.

Multi-seed boundary:

```text
seed0/current rung: green
seed1 shortcut gate: green
seed1 runtime robustness: GREEN
seeds 1,2,3 strict runtime robustness: RED
red issue: seed2/order has 1 strict slot failure hidden by rounded 1000 milli
```

The multi-seed robustness artifact is recorded here:

```text
data/rule_logic_operator_battery_v4/diagnostics/multiseed/MULTISEED_ROBUSTNESS_REPORT.md
data/rule_logic_operator_battery_v4/diagnostics/multiseed/SEED2_ORDER_STRICT_SLOT_FAILURE.md
```

Do not claim multi-seed strict robustness until the seed2/order failure is
reproduced, diagnosed, fixed without shortcuts, and the strict issue count is
zero.

Train-density follow-up:

```text
train_per_cell=2:
  seed2/order smoke: GREEN
  seed1/order:       GREEN
  seed1/edit:        GREEN
  seed1/conditional: RED
```

The seed2/order failure was fixed by principled full-corpus density, not a
targeted rule patch. But the full battery is still not green: conditional
strict slot readout fails after the stale row-count contract is fixed.

Artifact:

```text
data/rule_logic_operator_battery_v4/diagnostics/multiseed_train_per_cell_2/CONDITIONAL_SEED1_RED_AFTER_CONTRACT_FIX.md
data/rule_logic_operator_battery_v4/diagnostics/multiseed_train_per_cell_2/MULTISEED_ROBUSTNESS_REPORT.md
data/rule_logic_operator_battery_v4/diagnostics/multiseed_train_per_cell_2/multiseed_summary.json
```

Current conditional train_per_cell=2 result:

```text
conditional_slot_ordered_sequence_accuracy_milli: 617
conditional_sequence_energy_accuracy_milli: 973
conditional_energy_pass_slot_fail: 273
conditional_output_slot_cleanup_failed_slots: 1146
flat parity mismatches: 0
state_delta_edges: 0
forbidden flags: false
```

Interpretation:

```text
sequence energy is strong; strict decoder is weak under conditional branch
transfer. Do not call v4 complete.
```

Report-consistency fix:

```text
run_multiseed_rung.py now reuses the newest runtime log for each class.
The train_per_cell=2 machine summary now records the conditional red gate as
strict_runtime_issues instead of leaving it as MISSING/empty.
```

Conditional density follow-up:

```text
train_per_cell=4 conditional-only diagnostic:
  shortcut gate: clean
  strict slot: 611
  sequence energy: 969
  flat parity: 0
  state_delta_edges: 0
  forbidden flags: false
```

Artifact:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_density_sweep/seed_001/CONDITIONAL_DENSITY_SWEEP_REPORT.md
```

Updated diagnosis:

```text
The conditional red is not fixed by simply doubling train density from 2 to 4.
Noise isolation refines the blocker: clean rows are green, but distractor is
not independently green when isolated from the full corpus. Prefix_suffix and
instruction_noise remain weak with nearly identical profiles. Code audit shows
wrapper text is not directly inserted into role slots, so classify current
blocker as conditional role/readout instability under noise-correlated schedule
and surface pressure, not simple train-density sparsity.
```

Noise-isolation follow-up:

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

Important boundary:

```text
sequence_source_tokens extracts only the explicit sequence segment. The
noise-isolation run does not prove wrapper centers entered role slots. A
paired-noise corpus is required to isolate wrapper effect from token schedule.
```

Artifact:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_noise_isolation/seed_001/CONDITIONAL_NOISE_ISOLATION_REPORT.md
```

Paired-noise follow-up:

```text
paired_noise=true
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
flat parity: 0
forbidden flags: false
```

Artifact:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_paired_noise/seed_001/train_per_cell_2/CONDITIONAL_PAIRED_NOISE_REPORT.md
```

Final refined diagnosis for this rung:

```text
Wrapper text is not the active cause. Conditional strict-slot failure is now
isolated to role/filler collision under richer token surfaces, especially
ru_words/network, while sequence energy remains strong.
```

One-pass operator grokking probe:

```text
artifact:
  data/rule_logic_operator_battery_v4/diagnostics/OPERATOR_GROKKING_PROBE.md
  data/rule_logic_operator_battery_v4/diagnostics/operator_grokking_probe_report.json

method:
  train transitions -> compact operator program -> heldout application

result:
  rows: 10624
  train_rows: 5312
  heldout_rows: 5312
  compiled_operator_programs: 380
  operator_program_conflicts: 0
  heldout_accuracy_milli: 1000
  heldout_wrong_match_rows: 0

by class:
  order: 2048 / 2048
  edit: 1536 / 1536
  conditional: 768 / 768
  composed: 960 / 960
```

Interpretation:

```text
The v4 corpus admits compact one-pass operator program induction. This is a
strong argument for compiler-first operator induction: epochs should be repair
passes, not the primary philosophy.

Claim boundary: this is not yet a Wave runtime proof. The probe uses
normalized rule_action_example as an operator key, so the next gate must compile
the induced programs into Wave weights / energy / cleanup and compare against
epoch repair under the same forbidden-substitution flags.
```

Phase-center operator probe C32:

```text
artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_OPERATOR_PROBE_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/phase_center_operator_probe_c32_report.json

method:
  relation waves
  -> circular/phase center of mass
  -> correct heldout transition closer than same-bag wrong transition

result:
  verdict: PHASE_CENTER_OPERATOR_PROBE_PASS
  phase cells: 32
  compiled_phase_centers: 380
  heldout_rows: 5312
  heldout_accuracy_milli: 1000
  wrong_wins: 0
  median_margin: 0.7671
  p10_margin: 0.3130
  median_positive_center_gap: 0.5371
  p10_positive_center_gap: 0.2430

by class:
  order: 2048 / 2048
  edit: 1536 / 1536
  conditional: 768 / 768
  composed: 960 / 960

no-action ablation:
  heldout_accuracy_milli: 782
  wrong_wins: 1156
```

Interpretation:

```text
This is the strongest current v4 signal and the closest current result to the
original Wave/Fourier direction. The primary path is now:

relation waves -> phase-center operator compiler -> Wave runtime readout.

Epoch/error-driven repair remains allowed only as a fallback after the
phase-center Wave gate is red and diagnosed.

Claim boundary: this diagnostic alone is not a production Wave runtime proof.
The follow-up Rust flat runtime and core compiler gates below now close the
scorer/compiler part of that bridge. They still do not close the strict
ordered decoder or generator path.
```

Rust phase-center runtime probe C32:

```text
artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_RUNTIME_PROBE_C32.md

test:
  operator_battery_v4_phase_center_runtime_probe_report

command:
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_runtime_probe_report --nocapture

result:
  verdict: PHASE_CENTER_RUNTIME_PROBE_PASS
  cells: 32
  action_compiled_phase_centers: 380
  action_train_rows: 5312
  action_heldout_rows: 5312
  action_heldout_surface_groups: 4
  action_heldout_noise_groups: 4
  action_heldout_accuracy_milli: 1000
  action_wrong_wins: 0
  action_median_margin: 0.767109
  action_p10_margin: 0.312965
  no_action_compiled_phase_centers: 40
  no_action_heldout_accuracy_milli: 782
  no_action_wrong_wins: 1156
  phase_center_bytes_estimate: 389120
  epoch_repair_used: false
  explicit_out_src_program_extraction_used: false
  forbidden flags: false
```

Interpretation:

```text
The phase-center signal is no longer Python-only. It is reproduced in the Rust
proof runtime with the same C32 numbers. This becomes the primary Wave/Fourier
path. Epoch/error-driven repair stays a fallback after a red phase-center gate,
not the default philosophy.
```

Phase-center capacity and ablation:

```text
artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CAPACITY_ABLATION_C8_C64.md

test:
  operator_battery_v4_phase_center_capacity_ablation_report

command:
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_capacity_ablation_report --nocapture

result:
  verdict: PHASE_CENTER_CAPACITY_ABLATION_PASS

  C8:  action_accuracy_milli=998, action_wrong_wins=11, p10_margin=0.214822
  C16: action_accuracy_milli=1000, action_wrong_wins=1, p10_margin=0.246618
  C32: action_accuracy_milli=1000, action_wrong_wins=0, p10_margin=0.312965
  C64: action_accuracy_milli=1000, action_wrong_wins=0, p10_margin=0.416739

  C32 top4 train-cell ablation:  accuracy=1000, wrong_wins=0, p10=0.276477
  C32 top8 train-cell ablation:  accuracy=1000, wrong_wins=0, p10=0.248453
  C32 top16 train-cell ablation: accuracy=999,  wrong_wins=5, p10=0.157739
```

Interpretation:

```text
C32 is the first compact zero-wrong phase-center rung. C64 increases margin
reserve. Train-only phase-cell ablation weakens the score and creates wrong
wins at top16, so the result is tied to a removable phase mechanism rather than
only final accuracy.
```

Phase-center flat runtime:

```text
artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_FLAT_RUNTIME_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_COMPILER_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_PACKAGE_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_PACKAGE_BENCH_C32_C64.md
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CORE_RUNTIME_BENCH_C32_C64.md

test:
  operator_battery_v4_phase_center_flat_runtime_report
  operator_battery_v4_phase_center_core_runtime_report
  operator_battery_v4_phase_center_core_compiler_report
  operator_battery_v4_phase_center_core_runtime_package_report
  operator_battery_v4_phase_center_core_runtime_package_benchmark_report
  operator_battery_v4_phase_center_core_runtime_benchmark_report

release command:
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_flat_runtime_report --nocapture
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_core_runtime_report --nocapture
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_core_compiler_report --nocapture
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_core_runtime_package_report --nocapture
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_core_runtime_package_benchmark_report --nocapture
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_core_runtime_benchmark_report --nocapture

result:
  verdict: PHASE_CENTER_FLAT_RUNTIME_PASS
  cells: 32
  compiler_accuracy_milli: 1000
  flat_accuracy_milli: 1000
  flat_wrong_wins: 0
  flat_sign_parity_mismatches: 0
  flat_margin_parity_mismatches: 0
  no_action_flat_accuracy_milli: 782
  no_action_flat_wrong_wins: 1156
  flat_records: 380
  flat_runtime_bytes_estimate: 407360
  release_p50_latency_ns: 136
  release_p99_latency_ns: 506
  release_total_eval_us: 1032
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
  core_runtime_package_verdict: PHASE_CENTER_CORE_RUNTIME_PACKAGE_PASS
  core_runtime_package_magic: [78, 87, 80, 67, 70, 48, 48, 49]
  core_runtime_package_bytes: 389136
  core_runtime_package_fingerprint64: 14549306353473335964
  core_runtime_package_accuracy_milli: 1000
  core_runtime_package_wrong_wins: 0
  core_runtime_package_sign_parity_mismatches: 0
  core_runtime_package_margin_parity_mismatches: 0
  core_runtime_package_p50_latency_ns: 64
  core_runtime_package_p99_latency_ns: 407
  core_runtime_package_path: PhaseCenterFlatRuntime::to_bytes/from_bytes
  core_runtime_package_bench_C32: package_bytes=389136 payload=389120 fingerprint64=14549306353473335964 load=787us p50=69ns p99=416ns wrong_wins=0
  core_runtime_package_bench_C64: package_bytes=778256 payload=778240 fingerprint64=16888657547359761052 load=1646us p50=163ns p99=520ns wrong_wins=0
  product_package_cli_verdict: PHASE_PACKAGE_V4_PASS
  product_package_inspect_verdict: PHASE_PACKAGE_INSPECT_PASS
  product_package_score_verdict: PHASE_PACKAGE_SCORE_V4_PASS
  product_package_verify_verdict: PHASE_PACKAGE_VERIFY_PASS
  product_package_cli_path: target/nando-wave/phase-center-v4-c32.nwpc
  product_package_manifest_path: target/nando-wave/phase-center-v4-c32.nwpc.manifest.json
  product_package_eval_pack_path: target/nando-wave/phase-center-v4-c32.eval-pack
  product_package_score_report_path: target/nando-wave/phase-center-v4-c32.score-report.json
  product_package_score_pack_report_path: target/nando-wave/phase-center-v4-c32.score-pack-report.json
  product_package_cli_bytes: 389136
  product_package_eval_pack_bytes: 10921516
  product_package_manifest_bytes_sample: 134182
  product_package_score_report_bytes_sample: 1832
  product_package_cli_fingerprint64: 14549306353473335964
  product_package_operator_key_count: 380
  product_package_manifest_matches_package: true
  product_package_cli_accuracy_milli: 1000
  product_package_cli_wrong_wins: 0
  product_package_cli_p50_latency_ns: 173
  product_package_cli_p99_latency_ns: 614
  product_package_cli_rows_per_second: 3603718.51
  product_package_action_ablation_accuracy_milli: 443
  product_package_action_ablation_wrong_wins: 2958
  product_package_score_compiler_used: false
  product_package_score_accuracy_milli: 1000
  product_package_score_wrong_wins: 0
  product_package_score_p99_latency_ns: 520
  product_package_score_rows_per_second: 4148032.93
  product_package_score_action_ablation_accuracy_milli: 443
  product_package_score_action_ablation_wrong_wins: 2958
  product_package_score_report_verdict: PHASE_PACKAGE_SCORE_V4_PASS
  product_package_eval_pack_verdict: PHASE_EVAL_PACK_V4_PASS
  product_package_score_pack_verdict: PHASE_PACKAGE_SCORE_PACK_V4_PASS
  product_package_score_pack_compiler_used: false
  product_package_score_pack_eval_task_package_used: true
  product_package_score_pack_corpus_jsonl_used_in_score_loop: false
  product_package_score_pack_accuracy_milli: 1000
  product_package_score_pack_wrong_wins: 0
  product_package_score_pack_p99_latency_ns: 429
  product_package_score_pack_rows_per_second: 5614990.50
  product_package_score_pack_action_ablation_accuracy_milli: 443
  product_package_score_pack_action_ablation_wrong_wins: 2958
  product_package_action_boundary_verdict: PHASE_ACTION_BOUNDARY_V4_WATCH
  product_package_action_boundary_exit_code: 1
  product_package_action_boundary_explicit_operator_class_label_rows: 10624
  product_package_action_boundary_explicit_operator_family_label_rows: 10624
  product_package_action_boundary_explicit_order_slot_map_rows: 4096
  product_package_action_boundary_explicit_branch_slot_map_rows: 1536
  autonomous_action_router_claim_allowed: false
  product_package_tampered_manifest_verdict: PHASE_PACKAGE_SCORE_V4_WATCH
  product_package_tampered_manifest_exit_code: 1
  product_package_tampered_score_report_verdict: PHASE_PACKAGE_SCORE_V4_WATCH
  product_package_tampered_score_report_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  product_package_tampered_score_report_verify_exit_code: 1
  product_package_tampered_eval_pack_score_exit_code: 1
  product_package_tampered_score_pack_report_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  product_package_tampered_score_pack_report_verify_exit_code: 1
  product_package_tampered_score_pack_no_eval_flag_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  product_package_tampered_score_pack_no_eval_flag_exit_code: 1
  product_package_tampered_score_pack_jsonl_loop_true_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  product_package_tampered_score_pack_jsonl_loop_true_exit_code: 1
  product_package_tampered_score_pack_jsonl_missing_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH
  product_package_tampered_score_pack_jsonl_missing_exit_code: 1
  product_package_verify_prints_manifest_and_score_report_forbidden_flags: true
  epoch_repair_used: false
  forbidden flags: false
```

Interpretation:

```text
The phase-center scorer now has a flat Rust CPU-runtime path. The runtime loop
uses precompiled numeric center_index values and flat records, not source_group,
task_id, target_id, proof_rule_id authority, or manual local_out_t.

The exported `nando_core::PhaseCenterFlatRuntime` path now passes the same C32
heldout/parity gate, so the scorer is no longer only a test-local helper.
Its core gate now measures precompiled eval tasks without bridge allocations in
the hot loop; the separate C32/C64 benchmark tracks capacity-vs-latency shape.
The exported `nando_core::PhaseCenterCompiler` now builds the same C32 runtime
surface directly from phase-center atoms.
The exported `PhaseCenterFlatRuntime::to_bytes/from_bytes` path now packages
and reloads that scorer as deterministic runtime bytes without carrying task
rows, answers, proof rule ids, concrete lookup, or manual output timing.
`nando-cli phase-package-v4` now exposes the same package path as a
product-facing Rust CLI harness. It writes the `.nwpc` package plus
`.manifest.json`, reloads the package, inspects `fingerprint64`, and scores the
fixed v4 heldout batch. `nando-cli phase-package-inspect` validates the
manifest against the already-built package without reading the corpus.
`nando-cli phase-package-score-v4` scores the v4 heldout rows using only the
already-built package plus manifest operator keys; `compiler_used: false`.
`nando-cli phase-eval-pack-v4` precompiles the v4 heldout/action-ablation
phase vectors into a binary eval-pack, and `nando-cli
phase-package-score-pack-v4` scores that eval-pack without JSONL rebuild or
compiler use. This is the current product/benchmark scoring path for the C32
phase-center scorer package.
```

Static collision follow-up:

```text
symbols multi/wrong-role collision: 140 milli, strict surface accuracy 1000
business multi/wrong-role collision: 170 milli, strict surface accuracy 1000
ru_words multi/wrong-role collision: 192 milli, strict surface accuracy 768
network multi/wrong-role collision: 196 milli, strict surface accuracy 763
missing_true_role_milli: 0 for all surfaces
```

Boundary:

```text
Collision pressure correlates with the red surfaces, but raw collision alone is
not sufficient because business remains green at 170 milli. The next proof-debt
is collision-class vs actual gap/failure, not an architecture change.
```

Collision-outcome follow-up:

```text
paired-noise runtime collision outcome:
  strict slot: 615
  sequence energy: 958
  flat parity: 0

by collision bucket:
  high_wrong_role_hit accuracy: 845
  mid_wrong_role_hit accuracy: 879
  low_wrong_role_hit accuracy: 924
  no_wrong_role_hit accuracy: 887

by surface:
  business accuracy: 1000, avg_gap: 133505, min_gap: 29230
  symbols accuracy: 1000, avg_gap: 129630, min_gap: 28976
  network accuracy: 763, avg_gap: 29850, min_gap: -45024
  ru_words accuracy: 768, avg_gap: 29090, min_gap: -53964

worst output-source pairs:
  out10->src7: 0 / 24 slots pass
  out1->src11: 0 / 8 slots pass
  out12->src2: 0 / 8 slots pass
  out12->src6: 0 / 8 slots pass
  out12->src7: 0 / 8 slots pass
  out6->src12: 8 / 48 slots pass
  out2->src0: 20 / 96 slots pass
  out1->src0: 24 / 96 slots pass
```

Refined proof-debt:

```text
The red gate is not a generic noise problem and not raw collision alone.
It is a surface-sensitive strict-slot cleanup problem: ru_words/network have
low gaps and negative min gaps under folded role collision, while
business/symbols keep large positive gaps under similar collision pressure.
Next mechanism work must explain surface identity plus role collision plus
output/source pair cleanup without target_id, proof_rule_id authority,
concrete_x_lookup, manual local_out_t, or a class-specific hardcode.
```

Target/wrong lane-overlap follow-up:

```text
static target/wrong overlap by surface:
  business target/wrong overlap: 22061, wrong hits true role: 22395
  network target/wrong overlap: 28574, wrong hits true role: 28147
  ru_words target/wrong overlap: 24147, wrong hits true role: 23987
  symbols target/wrong overlap: 123698, wrong hits true role: 123841

symbols remain strict green despite the largest static overlap. Therefore L1
target/wrong overlap and wrong-token true-role hits are not the standalone
cause. The next mechanism must target runtime gap cleanup under surface-specific
role collision, not simply lower overlap.
```

Candidate-cleanup follow-up:

```text
conditional candidate cleanup:
  candidate_slot_tasks: 228864
  epochs=2 role_binding_edges: 73559
  epochs=8 role_binding_edges: 73559
  train candidate min gap becomes positive by epoch 3/8

heldout after candidate cleanup:
  strict row accuracy: 615
  sequence energy: 984
  failed slots: 972
  network surface accuracy: 787
  ru_words surface accuracy: 808
  business/symbols: 1000
  flat parity: 0
  forbidden flags: false
```

Boundary:

```text
Candidate cleanup is a valid generic repair and improves slot-level metrics,
but it does not solve the row-level strict decoder. The current red gate is not
caused by too few train candidate negatives. The next proof-debt is
surface/sign-conditioned role-binding transfer: heldout token surfaces still
produce negative runtime gaps on specific output/source pairs even after all
train candidates have positive margin.
```

Sign-aware collision diagnostic:

```text
conditional_sign_aware_positive_collision_report_raw.log

business current/sign-aware wrong-role hit: 170 -> 154
network  current/sign-aware wrong-role hit: 196 -> 178
ru_words current/sign-aware wrong-role hit: 192 -> 179
symbols  current/sign-aware wrong-role hit: 140 -> 122
missing_true_signed_role_milli: 0 for all surfaces

worst pairs remain high:
  out2->src14: 500 -> 444
  out0->src15: 474 -> 461
  out14->src1: 471 -> 471

same-sign residual collision share:
  business: 154 / 170 = 906 milli remains
  network:  178 / 196 = 908 milli remains
  ru_words: 179 / 192 = 932 milli remains
  symbols:  122 / 140 = 871 milli remains
```

Boundary update:

```text
Sign-aware role matching is safe for positive target lanes, but it is too weak
as a standalone fix. The red gate is not only sign erasure; it also contains
same-sign folded collision and surface identity pressure. The next measured
target is the same-sign residual collision, not the erased-sign slice. Do not
add a sign channel and call v4 solved unless strict row accuracy, ablations, and
parity turn green.
```

Residual collision outcome diagnostic:

```text
conditional_residual_collision_outcome_cleanup8.log

compiler/trainer stage:
  local_epochs: 8
  cleanup_epochs: 4
  candidate_cleanup_epochs: 8
  train_slot_accuracy_milli: 1000 after cleanup
  train candidate min gap positive by epoch 3/8
  final role_binding_edges: 73559

heldout:
  strict row accuracy: 615
  sequence energy: 984
  energy_pass_slot_fail: 284

surface:
  business: 1000, min_gap 33022
  symbols: 1000, min_gap 38630
  network: 787, min_gap -45024
  ru_words: 808, min_gap -53964

residual buckets:
  high_same_sign_residual accuracy: 859
  mid_same_sign_residual accuracy: 890
  low_same_sign_residual accuracy: 927
  no_same_sign_residual accuracy: 923
```

Boundary update:

```text
Same-sign residual collision contributes to the red gate, but it is not the
single cause. Business/symbols stay green under residual pressure, while
network/ru_words fail across buckets. The next mechanism should target the
energy/readout mismatch with a generic cleanup/readout or learned
role-specificity candidate, not a standalone collision/sign patch.

Training epochs are compiler/proof-stage only. Runtime inference must remain a
compiled-table execution path, not an iterative train loop.
```

Guardrail:

```text
Do not count success if lookup, target_id, proof_rule_id authority,
concrete_x_lookup, manual local_out_t, or hidden hardcode is used.
```

## Frozen Regression

The previous green rung is locked separately:

```text
data/rule_logic_position_sequence_v3/REGRESSION_LOCK.md
```

Do not mutate that regression when iterating on v4.

## Battery Scope

Artifact:

```text
data/rule_logic_operator_battery_v4/accepted_operator_tasks_v4.jsonl
```

Rows:

```text
rows: 10624
train_rows: 5312
heldout_rows: 5312
lengths: 9..16
surface_families: symbols, ru_words, business, network
noise_types: clean, prefix_suffix, distractor, instruction_noise
```

Class sizes:

```text
order:       4096 rows, 2048 train, 2048 heldout
edit:        3072 rows, 1536 train, 1536 heldout
conditional: 1536 rows,  768 train,  768 heldout
composed:    1920 rows,  960 train,  960 heldout
```

## Shortcut Gate

Command:

```bash
python3 data/rule_logic_operator_battery_v4/run_shortcut_gates.py
```

Report:

```text
data/rule_logic_operator_battery_v4/shortcut_gate_report.json
data/rule_logic_operator_battery_v4/shortcut_gate_report.log
```

Overall:

```text
verdict: VALID_OPERATOR_BATTERY_V4_CANDIDATE
exact_lookup_accuracy_milli: 0
proof_rule_id_majority_accuracy_milli: 0
proof_rule_family_majority_accuracy_milli: 0
surface_family_majority_accuracy_milli: 0
length_only_accuracy_milli: 0
output_position_prior_accuracy_milli: 24
markov_bigram_pairwise_accuracy_milli: 512
bayesian_cooccurrence_pairwise_accuracy_milli: 512
l2_neighbor_target_copy_accuracy_milli: 0
operator_slots_non_order_count: 0
```

Per class:

```text
order:       VALID, bag 500, same-bag derangement 1000, Markov 500, Bayesian 500
edit:        VALID, edit overlap 870, Markov 542, Bayesian 542, output-position prior 83
conditional: VALID, bag 500, same-bag derangement 1000, Markov 500, Bayesian 500
composed:    VALID, bag 500, same-bag derangement 1000, Markov 500, Bayesian 500
```

Interpretation:

```text
The v4 corpora are not solved by exact lookup, proof-rule authority, class
majority, length-only prior, Markov/bigram, Bayesian co-occurrence, or
L2-neighbor copy.

For permutation-style classes, correct/wrong remain same-bag hard pairs.
For edit, correct/wrong are not required to be same-bag because edit changes
the state, but near-edit overlap is high enough to avoid alien negatives.
```

## Runtime Gates

### Order

Artifacts:

```text
data/rule_logic_operator_battery_v4/order/ORDER_RUNTIME_GATE_REPORT.md
data/rule_logic_operator_battery_v4/order/order_runtime_gate_release_l1_short_token_identity.log
```

Green metrics:

```text
order_slot_ordered_sequence_accuracy_milli: 1000
order_flat_slot_ordered_sequence_accuracy_milli: 1000
order_sequence_energy_accuracy_milli: 1000
order_sequence_energy_median_gap: 4765082
order_sequence_energy_p10_gap: 2646022
order_energy_pass_slot_fail: 0
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
ablations_without_binding/action/role/active_fringe: 0
state_delta_edges: 0
role_binding_edges: 185112
forbidden shortcut flags: false
```

Repair:

```text
SURFACE_WAVE_SHORT_TOKEN_IDENTITY_ATOMS = 4
generic salted identity atoms for normalized tokens shorter than 4-gram size
not target-specific
not proof_rule_id authority
not concrete_x_lookup
not manual local_out_t
not an L3 operator hack
```

### Edit

Artifacts:

```text
data/rule_logic_operator_battery_v4/edit/EDIT_RUNTIME_BOUNDARY_REPORT.md
data/rule_logic_operator_battery_v4/edit/edit_runtime_boundary_gate.log
data/rule_logic_operator_battery_v4/edit/edit_marker_length_runtime_gate_release.log
```

Historical red boundary:

```text
rows_not_representable_by_current_role_transfer: 1632 / 3072
rows_with_non_source_output_tokens: 1280
rows_with_marker_output_tokens: 1280
rows_output_len_over_slots: 192
rows_correct_wrong_len_mismatch: 256
```

Green mechanism:

```text
source role filler channel
+ action-derived edit demo slot channel
+ marker/end role slot
+ bounded 17 output slots
```

Green metrics:

```text
train_rows: 1536
heldout_rows: 1536
train_discriminative_slot_tasks: 13504
heldout_discriminative_slot_tasks: 13504
rows_with_full_demo_slot_map: 3072

edit_slot_ordered_sequence_accuracy_milli: 1000
edit_flat_slot_ordered_sequence_accuracy_milli: 1000
edit_sequence_energy_accuracy_milli: 1000
edit_sequence_energy_median_gap: 3457370
edit_sequence_energy_p10_gap: 551498
edit_energy_pass_slot_fail: 0

flat_sequence_energy_parity_mismatches: 0
flat_sequence_energy_parity_max_abs_gap_delta: 0
flat_gap_parity_mismatches: 0
state_delta_edges: 0
role_binding_edges: 130599
```

Edit ablations:

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_edit_demo_accuracy_milli: 260
ablation_without_edit_demo_energy_accuracy_milli: 715
ablation_without_marker_role_accuracy_milli: 984
ablation_without_marker_role_energy_accuracy_milli: 1000
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

Boundary:

```text
The edit proof is bounded to the current 17-slot battery. It does not claim an
unbounded text decoder. Non-discriminative slots where correct and wrong are
identical are not trained or scored as delta tasks.
```

### Conditional

Artifacts:

```text
data/rule_logic_operator_battery_v4/conditional/CONDITIONAL_RUNTIME_BOUNDARY_REPORT.md
data/rule_logic_operator_battery_v4/conditional/conditional_condition_action_runtime_gate_release.log
data/rule_logic_operator_battery_v4/conditional/conditional_no_action_surface_default_runtime_gate_release.log
```

Green mechanism:

```text
state condition channel read from state_before
+ learned condition/action conjunction
+ branch selection inside L3
+ generic conditional action-surface suppressed by default
```

Why generic conditional action-surface is suppressed:

```text
The conditional action text contains both then/else branches. Keeping its
surface centers active created a noisy shortcut/conflict channel. The selected
condition/action conjunction already carries the branch operator extracted from
rule_action_example + state_before condition, not from target/proof_rule_id.
```

Green metrics:

```text
conditional_slot_ordered_sequence_accuracy_milli: 1000
conditional_flat_slot_ordered_sequence_accuracy_milli: 1000
conditional_sequence_energy_accuracy_milli: 1000
conditional_sequence_energy_median_gap: 1868422
conditional_sequence_energy_p10_gap: 1081834
conditional_energy_pass_slot_fail: 0
flat_sequence_energy_parity_mismatches: 0
flat_sequence_energy_parity_max_abs_gap_delta: 0
flat_gap_parity_mismatches: 0
state_delta_edges: 0
role_binding_edges: 114388
```

Conditional ablations:

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_condition_accuracy_milli: 0
ablation_without_condition_energy_accuracy_milli: 0
ablation_without_condition_action_accuracy_milli: 30
ablation_without_condition_action_energy_accuracy_milli: 879
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

Boundary:

```text
Condition/action ablations collapse below the same-bag chance line of 500, not
all sequence-energy ablations collapse to zero because some branch pairs share
partial global sequence structure. Strict ordered readout is the hard gate.
```

### Composed

Artifacts:

```text
data/rule_logic_operator_battery_v4/composed/COMPOSED_RUNTIME_GATE_REPORT.md
data/rule_logic_operator_battery_v4/composed/composed_demo_channel_runtime_gate_release.log
data/rule_logic_operator_battery_v4/composed/composed_no_action_surface_default_runtime_gate_release.log
```

Green mechanism:

```text
full-length neutral action demo
+ parsed final demo state
+ composed demo slot page 20
+ generic composed action-surface suppressed by default
```

Why generic composed action-surface is suppressed:

```text
The composed action text includes an explicit demo. If its raw surface centers
remain active, ablation without page 20 can still solve above chance. The proof
channel must be the parsed neutral demo slot page, not a fuzzy action-text
surface shortcut.
```

Green metrics:

```text
composed_slot_ordered_sequence_accuracy_milli: 1000
composed_flat_slot_ordered_sequence_accuracy_milli: 1000
composed_sequence_energy_accuracy_milli: 1000
composed_sequence_energy_median_gap: 1838464
composed_sequence_energy_p10_gap: 999168
composed_energy_pass_slot_fail: 0
flat_sequence_energy_parity_mismatches: 0
flat_sequence_energy_parity_max_abs_gap_delta: 0
flat_gap_parity_mismatches: 0
state_delta_edges: 0
role_binding_edges: 65848
```

Composed ablations:

```text
ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_action_energy_accuracy_milli: 0
ablation_without_composed_demo_accuracy_milli: 0
ablation_without_composed_demo_energy_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_role_energy_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0
```

Boundary:

```text
The composed demo channel repairs strict slot readout and is now required by
ablation. Generic action-surface is suppressed so page 20 cannot be bypassed by
a raw action-text shortcut.
```

## Current Claim Boundary

Allowed current-rung claim:

```text
The seed0/current v4 rung and seed1 robustness rung prove compact operator
transfer for four separated operator classes -- order, edit, conditional,
composed -- on the current proof-gated 9..16 length battery, with shortcut
gates, ablations, field/flat parity, sequence energy, strict slot readout, and
forbidden flags checked.

The seeds 1,2,3 multi-seed run is not green under the strict gate. Seed2/order
has one strict slot failure even though rounded accuracy_milli remains 1000 and
sequence energy remains 1000.
```

Red-to-green lineage:

```text
seed1 baseline red:
  conditional strict-slot readout = 999
  sequence energy = 1000
  flat parity = 0
  failure: noisy generic conditional action-surface conflicted with selected
  condition/action branch motif.

candidate short-token atoms 8:
  rejected; fixed conditional but weakened edit/composed ablation proof.

candidate role lanes 48:
  rejected; no effect on conditional failure.

candidate all-token cleanup:
  rejected; worsened conditional strict readout to 988.

accepted repair:
  suppress generic action-surface for conditional/composed when an explicit
  operator motif page is present. This removes noisy/raw action-text shortcut
  pressure without adding target_id, proof_rule_id authority, concrete_x_lookup,
  manual local_out_t, or a new answer table.

multi-seed strict red:
  seed2/order/block_reverse_4_len13/ru_words/clean fails out12->src12 once.
  correct token: priem
  wrong token: oblast
  gap: -11426
  sequence_energy_gap: 4393850
  interpretation: global sequence energy selected the correct transform, but
  strict slot readout failed on one high-slot self-transfer case.

static diagnosis:
  target/wrong surface cosine = 0
  correct/wrong role-lane overlap = 0
  priem positive target impulses with other-role hits = 8 / 16
  same comparable row seed1 = 0 / 22
  same comparable row seed3 = 2 / 13
  likely issue: local role/filler collision pressure in strict readout.

dynamic weight audit:
  direct lane score = 0
  self-transfer score = 0
  true slot12 contribution = +15680
  other role-slot contribution = -51050
  worst suppressors: slot10 = -28740, slot11 = -11682
  cleanup on train repaired_rows = 0 for all 4 cleanup epochs
  interpretation: the learned true binding exists, but heldout multi-role
  lane collision is over-suppressed by other learned role bindings.

density sweep:
  factor1 baseline target_rule_train_rows = 16 -> RED, slot_failure_total = 1
  factor2 target_rule_train_rows = 32 -> GREEN, slot_failure_total = 0
  factor4 target_rule_train_rows = 64 -> GREEN, slot_failure_total = 0
  factor16 target_rule_train_rows = 256 -> GREEN, slot_failure_total = 0
  classification: data/weight sparsity under local multi-role collision pressure.
  boundary: targeted reweighting is diagnostic only, not the final v4 proof corpus.
```

Candidate repairs tested and rejected:

```text
short-token identity atoms 8:
  fixes conditional seed1, but weakens edit/composed ablation proof.

role lanes 48:
  no effect on the conditional seed1 failure.
```

Forbidden overclaim:

```text
Do not claim unbounded decoding.
Do not claim future operator families are solved.
Do not claim seeds 1,2,3 strict robustness as green.
Do not claim edit works without the bounded marker/end channel.
Do not claim the conditional energy ablation without condition/action goes to
zero; strict ordered readout is the hard conditional ablation gate.
```

## Next Work

The next rung should be a separately versioned battery, not a mutation of v4:

```text
1. replace targeted density duplication with a principled corpus policy,
   candidate: OPERATOR_BATTERY_TRAIN_PER_CELL=2 for the full v4 battery;
2. rebuild seeds 1,2,3 under that policy;
3. rerun shortcut gates and runtime gates for order/edit/conditional/composed;
4. accept only if strict_runtime_issues is empty, flat parity remains 0,
   ablations remain 0, and forbidden flags remain false;
5. runtime bytes/latency report for green paths only after strict issues are zero;
6. capacity curve: operators vs memory/edges/false positives/collapse;
7. 32-slot paged-u32 scaling rung;
8. wider operator families beyond current v4.
```
