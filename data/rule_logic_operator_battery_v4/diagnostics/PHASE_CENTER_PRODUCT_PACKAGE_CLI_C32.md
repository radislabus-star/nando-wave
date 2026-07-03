# Phase-Center Product Package CLI C32

Date: 2026-07-02

## Verdict

`PHASE_PACKAGE_V4_PASS`

## Scope

This is the first product-facing package harness for the v4 phase-center scorer.
It is not a Python demo and not an ignored cargo test. The CLI builds a package
from the fixed v4 corpus, saves runtime bytes, loads them back, inspects the
header/fingerprint, and scores the heldout batch through
`nando_core::PhaseCenterFlatRuntime`.

It also writes a sidecar manifest and validates that an already-built package
matches that manifest without reading the corpus.

## Command

```bash
cargo run -p nando-cli --release -- phase-package-v4
cargo run -p nando-cli --release -- phase-package-inspect
cargo run -p nando-cli --release -- phase-package-score-v4 \
  target/nando-wave/phase-center-v4-c32.nwpc \
  target/nando-wave/phase-center-v4-c32.nwpc.manifest.json \
  data/rule_logic_operator_battery_v4/accepted_operator_tasks_v4.jsonl \
  target/nando-wave/phase-center-v4-c32.score-report.json
cargo run -p nando-cli --release -- phase-eval-pack-v4 \
  target/nando-wave/phase-center-v4-c32.nwpc \
  target/nando-wave/phase-center-v4-c32.nwpc.manifest.json \
  data/rule_logic_operator_battery_v4/accepted_operator_tasks_v4.jsonl \
  target/nando-wave/phase-center-v4-c32.eval-pack
cargo run -p nando-cli --release -- phase-package-score-pack-v4 \
  target/nando-wave/phase-center-v4-c32.nwpc \
  target/nando-wave/phase-center-v4-c32.nwpc.manifest.json \
  target/nando-wave/phase-center-v4-c32.eval-pack \
  target/nando-wave/phase-center-v4-c32.score-pack-report.json
cargo run -p nando-cli --release -- phase-package-verify \
  target/nando-wave/phase-center-v4-c32.nwpc \
  target/nando-wave/phase-center-v4-c32.nwpc.manifest.json \
  target/nando-wave/phase-center-v4-c32.score-pack-report.json
cargo run -p nando-cli --release -- phase-action-boundary-v4
```

Default paths:

```text
corpus_path: data/rule_logic_operator_battery_v4/accepted_operator_tasks_v4.jsonl
package_path: target/nando-wave/phase-center-v4-c32.nwpc
manifest_path: target/nando-wave/phase-center-v4-c32.nwpc.manifest.json
eval_pack_path: target/nando-wave/phase-center-v4-c32.eval-pack
score_report_path: target/nando-wave/phase-center-v4-c32.score-report.json
score_pack_report_path: target/nando-wave/phase-center-v4-c32.score-pack-report.json
```

## Release Metrics

```text
rows: 10624
train_rows: 5312
heldout_rows: 5312
cells: 32
flat_records: 380
operator_key_count: 380
skipped_train_rows: 0
missing_centers: 0
skipped_rows: 0
action_ablation_eval_rows: 5312
action_ablation_missing_centers: 0
heldout_surface_groups: 4
heldout_noise_groups: 4
package_magic: [78, 87, 80, 67, 70, 48, 48, 49]
inspected_cells: 32
inspected_records: 380
inspected_payload_bytes: 389120
package_fingerprint64: 14549306353473335964
package_bytes: 389136
manifest_bytes_sample: 134182
serialized_len: 389136
runtime_bytes_estimate: 401280
accuracy_milli: 1000
wrong_wins: 0
median_margin: 0.767109
p10_margin: 0.312965
p50_latency_ns: 173
p99_latency_ns: 614
total_eval_us: 1295
rows_per_second: 3603718.51
action_ablation_accuracy_milli: 443
action_ablation_wrong_wins: 2958
action_ablation_median_margin: -0.017064
action_ablation_p10_margin: -0.735397
compiler_path: nando_core::PhaseCenterCompiler
package_path_api: nando_core::PhaseCenterFlatRuntime::to_bytes/from_bytes
runtime_path: nando_core::PhaseCenterFlatRuntime
```

Inspect result:

```text
verdict: PHASE_PACKAGE_INSPECT_PASS
manifest_schema_version: nando_phase_package_manifest_v1
manifest_verdict: PHASE_PACKAGE_V4_PASS
manifest_operator_keys: 380
manifest_matches_package: true
claim_boundary: phase-center scorer package; not strict ordered decoder, text generation, multi-step reasoning, or multi-seed strict readout robustness
```

Score-from-package result:

```text
verdict: PHASE_PACKAGE_SCORE_V4_PASS
compiler_used: false
manifest_operator_keys: 380
heldout_eval_rows: 5312
missing_centers: 0
skipped_rows: 0
action_ablation_eval_rows: 5312
action_ablation_missing_centers: 0
accuracy_milli: 1000
wrong_wins: 0
p50_latency_ns: 165
p99_latency_ns: 520
total_eval_us: 1106
rows_per_second: 4148032.93
action_ablation_accuracy_milli: 443
action_ablation_wrong_wins: 2958
action_ablation_median_margin: -0.017064
action_ablation_p10_margin: -0.735397
score_report_path: target/nando-wave/phase-center-v4-c32.score-report.json
score_report_bytes_sample: 1832
```

Prepared eval-pack result:

```text
verdict: PHASE_EVAL_PACK_V4_PASS
eval_pack_magic: [78, 87, 80, 67, 84, 48, 48, 49]
eval_pack_package_fingerprint64: 14549306353473335964
cells: 32
rows: 10624
heldout_eval_rows: 5312
action_ablation_eval_rows: 5312
missing_centers: 0
skipped_rows: 0
action_ablation_missing_centers: 0
eval_pack_bytes: 10921516
compiler_used: false
jsonl_used_after_pack_build: false
```

Score-from-eval-pack result:

```text
verdict: PHASE_PACKAGE_SCORE_PACK_V4_PASS
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used_in_score_loop: false
heldout_eval_rows: 5312
missing_centers: 0
skipped_rows: 0
action_ablation_eval_rows: 5312
action_ablation_missing_centers: 0
accuracy_milli: 1000
wrong_wins: 0
p50_latency_ns: 69
p99_latency_ns: 402
total_eval_us: 686
rows_per_second: 5614990.50
action_ablation_accuracy_milli: 443
action_ablation_wrong_wins: 2958
score_report_path: target/nando-wave/phase-center-v4-c32.score-pack-report.json
```

Verify result:

```text
verdict: PHASE_PACKAGE_VERIFY_PASS
manifest_matches_package: true
score_report_matches_package: true
score_report_verdict: PHASE_PACKAGE_SCORE_PACK_V4_PASS
manifest_forbidden_used: false
score_report_forbidden_used: false
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used_in_score_loop: false
accuracy_milli: 1000
wrong_wins: 0
action_ablation_accuracy_milli: 443
action_ablation_wrong_wins: 2958
```

Forbidden substitutions:

```text
epoch_repair_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

## Boundary

This proves a product-facing CLI harness for the compact C32 phase-center scorer
package:

```text
fixed v4 corpus -> Rust compiler -> binary package -> load/inspect -> heldout score
existing binary package -> inspect fingerprint -> verify manifest/operator keys
existing binary package + manifest operator keys -> score heldout without compiler
existing binary package + manifest operator keys -> write machine-readable score report
existing binary package + manifest + prepared eval-pack -> score without JSONL rebuild or compiler
existing binary package + manifest + score-pack report -> verify proof artifact without corpus/eval-pack
existing binary package + wrong action/operator center -> ablation drops to 443/1000
tampered manifest forbidden flag -> WATCH verdict + non-zero exit
tampered score report forbidden flag -> WATCH verdict + non-zero exit
tampered eval-pack fingerprint -> score-pack rejects before scoring + non-zero exit
tampered score-pack eval_task_package_used=false -> verify WATCH + non-zero exit
tampered score-pack corpus_jsonl_used_in_score_loop=true -> verify WATCH + non-zero exit
tampered score-pack missing corpus_jsonl_used_in_score_loop -> verify WATCH + non-zero exit
current v4 action-router boundary -> WATCH because action labels/slot maps are present
```

It does not prove:

```text
full strict ordered decoder
text generation
multi-step reasoning
multi-seed strict readout robustness
```

The current artifact is a scorer/energy package, not the final generator.
Latency and throughput values are single-run release samples; package bytes,
fingerprint, accuracy, wrong-wins, skipped rows, and forbidden flags are the
proof invariants for this gate.

Stdout audit check:

```text
tampered_manifest:
  command: phase-package-score-v4 target/nando-wave/phase-center-v4-c32.nwpc /tmp/nando-wave-forbidden-manifest.json data/rule_logic_operator_battery_v4/accepted_operator_tasks_v4.jsonl /tmp/nando-wave-forbidden-score.json
  verdict: PHASE_PACKAGE_SCORE_V4_WATCH
  exit_code: 1
  target_center_id_training_used: true
  score_report_verdict: PHASE_PACKAGE_SCORE_V4_WATCH

tampered_inspect:
  command: phase-package-inspect target/nando-wave/phase-center-v4-c32.nwpc /tmp/nando-wave-forbidden-manifest.json
  verdict: PHASE_PACKAGE_INSPECT_WATCH
  exit_code: 1
  target_center_id_training_used: true

tampered_score_report:
  command: phase-package-verify target/nando-wave/phase-center-v4-c32.nwpc target/nando-wave/phase-center-v4-c32.nwpc.manifest.json /tmp/nando-wave-forbidden-score-report.json
  verdict: PHASE_PACKAGE_VERIFY_WATCH
  exit_code: 1
  manifest_forbidden_used: false
  score_report_forbidden_used: true
  score_report_target_center_id_training_used: true

tampered_manifest_verify:
  command: phase-package-verify target/nando-wave/phase-center-v4-c32.nwpc /tmp/nando-wave-forbidden-manifest.json target/nando-wave/phase-center-v4-c32.score-report.json
  verdict: PHASE_PACKAGE_VERIFY_WATCH
  exit_code: 1
  manifest_forbidden_used: true
  score_report_forbidden_used: false
  manifest_target_center_id_training_used: true

tampered_eval_pack:
  command: phase-package-score-pack-v4 target/nando-wave/phase-center-v4-c32.nwpc target/nando-wave/phase-center-v4-c32.nwpc.manifest.json /tmp/nando-wave-bad.eval-pack /tmp/nando-wave-bad-score-pack-report.json
  verdict: WATCH before scoring
  exit_code: 1
  reason: phase eval task package does not match package/manifest

tampered_score_pack_report:
  command: phase-package-verify target/nando-wave/phase-center-v4-c32.nwpc target/nando-wave/phase-center-v4-c32.nwpc.manifest.json /tmp/nando-wave-bad-score-pack-report.json
  verdict: PHASE_PACKAGE_VERIFY_WATCH
  exit_code: 1
  manifest_forbidden_used: false
  score_report_forbidden_used: true
  score_report_local_out_t_runtime_extension_used: true

tampered_score_pack_no_eval_flag:
  command: phase-package-verify target/nando-wave/phase-center-v4-c32.nwpc target/nando-wave/phase-center-v4-c32.nwpc.manifest.json /tmp/nando-wave-score-pack-no-eval-flag.json
  verdict: PHASE_PACKAGE_VERIFY_WATCH
  exit_code: 1
  eval_task_package_used: false

tampered_score_pack_jsonl_loop_true:
  command: phase-package-verify target/nando-wave/phase-center-v4-c32.nwpc target/nando-wave/phase-center-v4-c32.nwpc.manifest.json /tmp/nando-wave-score-pack-jsonl-loop-true.json
  verdict: PHASE_PACKAGE_VERIFY_WATCH
  exit_code: 1
  corpus_jsonl_used_in_score_loop: true

tampered_score_pack_jsonl_missing:
  command: phase-package-verify target/nando-wave/phase-center-v4-c32.nwpc target/nando-wave/phase-center-v4-c32.nwpc.manifest.json /tmp/nando-wave-score-pack-jsonl-missing.json
  verdict: PHASE_PACKAGE_VERIFY_WATCH
  exit_code: 1
  corpus_jsonl_used_in_score_loop: null

action_boundary_audit:
  command: phase-action-boundary-v4
  verdict: PHASE_ACTION_BOUNDARY_V4_WATCH
  exit_code: 1
  explicit_operator_class_label_rows: 10624
  explicit_operator_family_label_rows: 10624
  explicit_order_slot_map_rows: 4096
  explicit_branch_slot_map_rows: 1536
  autonomous_action_router_claim_allowed: false
```

The sidecar manifest records the current license boundary:

```text
workspace Cargo metadata currently declares MIT;
final product/commercial license package is not closed by this scorer gate
```
