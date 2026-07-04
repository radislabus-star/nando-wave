# Executor Review Notes

## 2026-07-04 - Executor Integration: Test Output Parse Tool-Output State V1

Verdict:

```text
TEST_OUTPUT_PARSE_TOOL_OUTPUT_STATE_V1_REVIEW_TOOL_STATE_ATTACHED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-test-output-parse-tool-output-state-v1
```

Why:

```text
The previous test_output_parse dry-run found 104 real candidates, but only 3
scoreable request-side payloads. This stage measures whether those candidates
can be connected to request-time agent-loop tool-output state, using only the
previous command-output fingerprint/status visible before the user request.
It writes no raw prompt/tool-output/response text and does not enable local
accepts.
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-test-output-parse-tool-output-state-v1 \
  /home/ubu/.codex/history.jsonl \
  target/nando-wave/real-traffic-shadow/broad-route-split-discovery-v1.report.json \
  /home/ubu/.codex/sessions \
  target/nando-wave/real-traffic-shadow/test-output-parse-tool-output-state-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/test-output-parse-tool-output-state-v1.report.json \
  5000
```

Measured result:

```text
test_output_parse_candidate_events: 104
non_exact_candidate_events: 102
exact_cache_overlap_events: 2
session_ids_requested: 9
session_files_scanned: 9
codex_turns_indexed: 104
tool_outputs_indexed: 145578
tool_output_state_matched_events: 104
command_status_detected_events: 97
pass_status_events: 90
fail_status_events: 7
warning_status_events: 0
unknown_status_events: 7
raw_prompt_text_written: false
raw_tool_output_text_written: false
raw_response_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Decision:

```text
The bottleneck is no longer "no agent-loop state": all 104 candidates have a
previous tool-output fingerprint, and 97/104 have deterministic command status.
This is still not a savings claim. The next debt is to turn this previous
tool-output state into scoreable test_output_parse payloads without using final
answers, then train/compile a disabled profile and run admission/shadow.
```

Operational note:

```text
The session scan is offline and can be slow on large rollout JSONL files. The
route now prints per-session progress. Do not confuse this offline scan cost
with product hot-path latency.
```

Structural gate:

```text
docs/structural_gates/test-output-parse-tool-output-state-v1.md
verdict: PASS
complexity_score: 66
```

## 2026-07-04 - Executor Integration: Test Output Parse Output Evidence V1

Verdict:

```text
TEST_OUTPUT_PARSE_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-test-output-parse-output-evidence-v1
```

Why:

```text
The test_output_parse payload dry-run found only 3 scoreable rows. This stage
joins those scoreable rows back to local Codex session final-answer evidence
and attaches conservative test-status verifier labels, without writing raw
prompt/response text and without enabling local accepts.
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-test-output-parse-output-evidence-v1 \
  target/nando-wave/real-traffic-shadow/test-output-parse-payload-dry-run-v1.trace.jsonl \
  /home/ubu/.codex/sessions \
  target/nando-wave/real-traffic-shadow/test-output-parse-output-evidence-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/test-output-parse-output-evidence-v1.report.json
```

Measured result:

```text
operator_candidate_calls: 3
scoreable_candidate_calls: 3
session_ids_requested: 2
session_files_scanned: 2
codex_turns_indexed: 3
output_evidence_matched_events: 3
no_session_output_match_events: 0
deterministic_verification_events: 3
verifier_not_applicable_events: 0
verified_true_events: 3
verified_false_events: 0
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_verification: true
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Decision:

```text
The verifier works on the current scoreable subset: 3/3 rows are true and
false=0. This is still not a CPU savings claim. The limiting factor is support:
only 3 scoreable rows out of 104 candidates. Next step is either a
disabled-threshold test_output_parse profile plus shadow/audit for these rows,
or, preferably for CPU80, agent-loop tool-output state capture so many more
test_output_parse candidates become scoreable from real command-output context.
```

Structural gate:

```text
docs/structural_gates/test-output-parse-output-evidence-v1.md
verdict: PASS
complexity_score: 44
```

## 2026-07-04 - Executor Integration: Test Output Parse Payload Dry-Run V1

Verdict:

```text
TEST_OUTPUT_PARSE_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs
  docs/CPU_CALL_CATALOG.md

Added CLI route:
  role-binding-real-traffic-test-output-parse-payload-dry-run-v1
```

Why:

```text
The broad-route split report shows `test_output_parse` as a real narrow
candidate inside blocked broad routes. This stage builds request-side
active_fringe/slots for that split only, without reading raw responses, target
labels, proof labels, or enabling local accepts.
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-test-output-parse-payload-dry-run-v1 \
  /home/ubu/.codex/history.jsonl \
  target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json \
  target/nando-wave/real-traffic-shadow/broad-route-split-discovery-v1.report.json \
  target/nando-wave/real-traffic-shadow/test-output-parse-payload-dry-run-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/test-output-parse-payload-dry-run-v1.report.json \
  5000
```

Measured result:

```text
test_output_parse_candidate_events: 104
non_exact_candidate_events: 102
exact_cache_overlap_events: 2
payload_ready_events: 3
payload_built_events: 3
scoreable_payload_events: 3
profile_registered: false
shadow_score_ready: false
expected_unique_cpu_accepts_over_exact_cache: 0
expected_savings_milli: 0
false_accepts: 0
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Decision:

```text
`test_output_parse` has real traffic density, but current request-side payload
readiness is too narrow: only 3 scoreable rows out of 104 candidates. Do not
compile/promote accepts yet. Next debt is tool-output evidence and a deterministic
test_status_and_error_excerpt verifier; only after verifier labels exist should
we build the disabled-threshold profile and calibrate admission.
```

Structural gate:

```text
docs/structural_gates/test-output-parse-payload-dry-run-v1.md
verdict: PASS
complexity_score: 60
```

## 2026-07-04 - Executor Integration: File Path Evidence Admission Calibration V1

Verdict:

```text
FILE_PATH_EVIDENCE_ADMISSION_CALIBRATION_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs
  docs/CPU_CALL_CATALOG.md

Added CLI route:
  role-binding-real-traffic-file-path-evidence-admission-calibration-v1
```

Why:

```text
The file_path_evidence_answer split now has verifier labels, but enabling local
accepts would still be unsafe unless request-side features separate verifier-true
rows from verifier-false rows without reading the answer. This stage calibrates
that admission boundary and remains review-only.
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-file-path-evidence-admission-calibration-v1 \
  target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.trace.jsonl \
  /home/ubu/.codex/history.jsonl \
  target/nando-wave/real-traffic-shadow/file-path-evidence-admission-calibration-v1.report.json
```

Measured admission result:

```text
hook_ready_rows: 39
rows_with_prompt_features: 39
history_prompt_missing_rows: 0
label_true_rows: 15
label_false_rows: 24
minimum_true_support: 3
robust_safe_policy_found: false
singleton_safe_policy_found: true
best_robust_true_accepts: 0
best_singleton_true_accepts: 1
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_features: false
target_labels_used_for_runtime: false
proof_labels_used_for_runtime: false
local_accepts_enabled: false
market_claim_allowed: false
```

Decision:

```text
file_path_evidence_answer is not ready for promotion. The best zero-false policy
has only singleton support, so it is not a robust admission rule. Keep local
accepts disabled and move this split to WATCH until more verifier-true
non-synthetic rows or a narrower artifact-backed split exists.
```

Structural gate:

```text
docs/structural_gates/file-path-evidence-admission-calibration-v1.md
verdict: PASS
complexity_score: 73
```

## 2026-07-04 - Executor Integration: File Path Evidence Output Evidence V1

Verdict:

```text
FILE_PATH_EVIDENCE_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs
  docs/CPU_CALL_CATALOG.md

Added CLI route:
  role-binding-real-traffic-file-path-evidence-output-evidence-v1
```

Why:

```text
The file_path_evidence profile could score request-side payloads, but it had
no deterministic verifier labels. This stage attaches final-answer fingerprints
and source_path_or_url_presence verifier status from local Codex session
evidence, without writing raw prompts or raw responses.
```

Commands:

```text
cargo run -p nando-cli -- role-binding-real-traffic-file-path-evidence-output-evidence-v1 \
  target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.trace.jsonl \
  /home/ubu/.codex/sessions \
  target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.report.json

cargo run -p nando-cli -- role-binding-real-traffic-shadow-v1 \
  target/nando-wave/real-traffic-shadow/profile-registry-file-path-evidence-v1.json \
  target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-shadow-v1.report.json

cargo run -p nando-cli -- role-binding-real-traffic-verification-hook-audit-v1 \
  target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-shadow-v1.report.json \
  target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.verification-hook-audit.report.json
```

Measured output evidence:

```text
operator_candidate_calls: 44
scoreable_candidate_calls: 44
session_ids_requested: 9
session_files_scanned: 9
codex_turns_indexed: 39
output_evidence_matched_events: 39
no_session_output_match_events: 5
deterministic_verification_events: 39
verifier_not_applicable_events: 0
verified_true_events: 15
verified_false_events: 24
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_verification: true
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Measured shadow/audit:

```text
operator_candidate_calls: 44
nando_shadow_accepts: 0
nando_shadow_fallbacks: 44
verified_safe_accepts: 0
unverified_shadow_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 320643

verification_hook_ready_events: 39
verified_cpu_accept_eligible_events: 0
candidates_missing_explicit_verification: 5
candidates_missing_output_evidence: 5
provider_cost_events: 0
```

Decision:

```text
file_path_evidence_answer now has verifier labels: 15 true, 24 false, 5 missing.
It still adds zero CPU80 savings because the disabled profile accepts nothing.
The next step is request-side admission calibration against these labels.
```

Structural gate:

```text
docs/structural_gates/file-path-evidence-output-evidence-v1.md
verdict: PASS
```

## 2026-07-04 - Executor Integration: File Path Evidence Disabled Profile V1

Verdict:

```text
FILE_PATH_EVIDENCE_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs
  docs/CPU_CALL_CATALOG.md

Added CLI route:
  role-binding-real-traffic-file-path-evidence-profile-v1
```

Why:

```text
The previous stage produced 44 scoreable request-side payloads for the narrow
file_path_evidence_answer split, but shadow could not score them because no
profile existed. This stage compiles a dedicated disabled-threshold .nwrb
package and registry overlay for that split only.

This is not a local accept promotion and not a market claim.
```

Commands:

```text
cargo run -p nando-cli -- role-binding-real-traffic-file-path-evidence-profile-v1 \
  target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json \
  target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/file-path-evidence-seed0.nwrb \
  target/nando-wave/real-traffic-shadow/profile-registry-file-path-evidence-v1.json \
  target/nando-wave/real-traffic-shadow/file-path-evidence-profile-v1.report.json

cargo run -p nando-cli -- role-binding-real-traffic-shadow-v1 \
  target/nando-wave/real-traffic-shadow/profile-registry-file-path-evidence-v1.json \
  target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/file-path-evidence-profile-shadow-v1.report.json

cargo run -p nando-cli -- role-binding-real-traffic-verification-hook-audit-v1 \
  target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/file-path-evidence-profile-shadow-v1.report.json \
  target/nando-wave/real-traffic-shadow/file-path-evidence-profile-v1.verification-hook-audit.report.json
```

Measured profile result:

```text
profile_id: split_file_path_evidence_answer_profile_v1
package_bytes: 128
edge_count: 7
runtime_bytes_estimate: 32972
threshold: 2147483647
trace_rows_read: 5000
scoreable_payload_events: 44
package_training_requests: 44
positive_updates: 3096
negative_updates: 3168
changed_edges: 277
positive_margin_rows: 44
strict_ordered_pass_rows: 44
unexpected_local_accepts_under_disabled_threshold: 0
median_energy_margin: 1123328
p10_energy_margin: 1011712
min_energy_margin: 992256
local_accepts_enabled_on_real_traffic: false
market_claim_allowed: false
```

Measured shadow/audit result:

```text
total_requests: 5000
total_llm_calls: 5000
operator_candidate_calls: 44
exact_cache_hits: 456
nando_shadow_accepts: 0
nando_shadow_fallbacks: 44
verified_safe_accepts: 0
unverified_shadow_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 259065

verification_hook_ready_events: 0
verified_cpu_accept_eligible_events: 0
candidates_missing_explicit_verification: 44
candidates_missing_output_evidence: 44
provider_cost_events: 0
```

Decision:

```text
file_path_evidence_answer now has a scoreable disabled profile and shadow path.
It still does not count toward CPU80 savings because all rows fallback and no
source_path_or_url_presence verifier evidence exists.
```

Next route debt:

```text
attach source_path_or_url_presence_verifier_v1 output evidence
calibrate request-side admission on verifier labels
only then consider a safe-policy promote
feed verified unique accepts back into CPU_CALL_CATALOG
```

Structural gate:

```text
docs/structural_gates/file-path-evidence-profile-v1.md
verdict: PASS
```

## 2026-07-04 - Executor Integration: File Path Evidence Payload Dry-Run V1

Verdict:

```text
FILE_PATH_EVIDENCE_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs
  docs/CPU_CALL_CATALOG.md

Added CLI route:
  role-binding-real-traffic-file-path-evidence-payload-dry-run-v1
```

Why:

```text
Broad answer/project/continue routes are blocked as whole routes. The previous
broad-route split discovery found file_path_evidence_answer as the densest
artifact-backed narrow class, so this stage builds request-side payloads for
that split only.

This is not a local accept profile and not a market claim.
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-file-path-evidence-payload-dry-run-v1 \
  /home/ubu/.codex/history.jsonl \
  target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json \
  target/nando-wave/real-traffic-shadow/broad-route-split-discovery-v1.report.json \
  target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.report.json \
  5000
```

Measured result:

```text
sampled_history_rows: 5000
file_path_evidence_candidate_events: 146
non_exact_candidate_events: 144
exact_cache_overlap_events: 2
payload_ready_events: 122
payload_built_events: 44
scoreable_payload_events: 44
builder_rejected_events: 78
readiness_rejected_events: 24
profile_registered: false
shadow_score_ready: false
expected_unique_cpu_accepts_over_exact_cache: 0
expected_savings_milli: 0
false_accepts: 0
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Parent-route provenance:

```text
answer_or_explain: 79
project_context_dialogue: 65
agent_continue_execute: 2
```

Decision:

```text
file_path_evidence_answer now has 44 scoreable request-side payloads from
non-synthetic real Codex history and broad-route split discovery.

It does not count toward CPU80 savings. It stays CANDIDATE until a disabled
profile, source_path_or_url_presence verifier, admission audit, shadow, feedback,
and CPU catalog prove false_accepts=0 and unique value over exact cache.
```

Next route debt:

```text
compile disabled-threshold file_path_evidence .nwrb profile
attach source_path_or_url_presence_verifier_v1 output evidence
calibrate request-side admission
run shadow/audit and feed CPU_CALL_CATALOG
only then consider expected_unique_cpu_accepts_over_exact_cache > 0
```

Structural gate:

```text
docs/structural_gates/file-path-evidence-payload-dry-run-v1.md
verdict: PASS
```

## 2026-07-04 - Executor Integration: Broad Route Split Discovery V1

Verdict:

```text
BROAD_ROUTE_SPLIT_DISCOVERY_V1_REVIEW_SPLITS_FOUND
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs
  docs/CPU_CALL_CATALOG.md

Generated:
  target/nando-wave/real-traffic-shadow/broad-route-split-discovery-v1.report.json
```

Why:

```text
CPU80 cannot be reached by improving broad answer/project/continue routes as a
whole. They need to be split into narrow artifact-backed CPU call classes with
traffic count, exact-cache overlap, verifier shape, and expected unique value
before any new profile work.
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-broad-route-split-discovery-v1 \
  /home/ubu/.codex/history.jsonl \
  target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json \
  target/nando-wave/real-traffic-shadow/broad-route-split-discovery-v1.report.json \
  5000
```

Measured result:

```text
sampled_llm_calls: 5000
broad_candidate_events: 3330
non_exact_broad_candidate_events: 3089
candidate_split_rows: 11
watch_split_rows: 6
rejected_split_rows: 3
business_value_gate_passed_rows: 0
raw_text_written: false
local_accepts_enabled: false
```

Top candidate splits:

```text
answer_or_explain / file_path_evidence_answer: candidates=79, non_exact=79, payload=70, verifier=70
project_context_dialogue / file_path_evidence_answer: candidates=65, non_exact=63, payload=51, verifier=52
answer_or_explain / test_output_parse: candidates=57, non_exact=57, payload=3, verifier=35
answer_or_explain / metric_from_report: candidates=27, non_exact=27, payload=14, verifier=14
answer_or_explain / git_status_summary: candidates=10, non_exact=10, payload=5, verifier=10
agent_continue_execute / git_status_summary: candidates=8, non_exact=8, payload=4, verifier=8
```

Still blocked:

```text
project_context_dialogue / broad_reasoning_requires_llm: non_exact=1216
answer_or_explain / broad_reasoning_requires_llm: non_exact=1029
agent_continue_execute / artifact_progress: WATCH, high stateful singleton risk
```

Decision:

```text
Broad routes remain REJECT_FOR_NOW as whole routes.

Next build should start from a narrow deterministic split, not from the broad
route:
  file_path_evidence_answer
  test_output_parse
  metric_from_report
  git_status_summary

Discovery rows are not savings. They keep expected_unique_cpu_accepts_over_exact_cache=0
until payload builder, verifier, shadow/audit, feedback, and CPU catalog prove
false_accepts=0 and unique value over exact cache.
```

Structural gate:

```text
docs/structural_gates/broad-route-split-discovery-v1.md
verdict: PASS
```

## 2026-07-04 - Executor Integration: CPU Call Catalog Business Value Gate V1

Verdict:

```text
CPU_OPERATOR_CATALOG_V1_BUSINESS_VALUE_GATE_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

Added:
  docs/CPU_CALL_CATALOG.md

Generated:
  target/nando-wave/real-traffic-shadow/cpu-call-catalog-business-value-v1.report.json
  target/nando-wave/real-traffic-shadow/cpu-call-catalog-business-value-v1.nanda.txt
```

Why:

```text
CPU80 cannot be reached by building many attractive but low-value profiles.
Every operator profile now has to pass BUSINESS_VALUE_GATE before more work:
real trace count, non-exact candidate calls, deterministic verifier evidence,
expected unique CPU accepts over exact cache, expected savings, and
false_accepts=0.
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-cpu-operator-catalog-v1 \
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v5.post-ime-current.report.json \
  target/nando-wave/real-traffic-shadow/route-gap-catalog-v1-5k.report.json \
  target/nando-wave/real-traffic-shadow/cpu-call-catalog-business-value-v1.report.json \
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1-5k.report.json
```

Measured result:

```text
total_llm_calls: 1000
exact_cache_hits: 53
current_verified_cpu_accepts: 26
current_incremental_unique_cpu_accepts_over_exact_cache: 25
business_value_gate_passed_rows: 7
proven_profile_rows: 7
candidate_profile_rows: 4
watch_profile_rows: 12
rejected_profile_rows: 6
```

PROVEN rows:

```text
role_binding_mixed_map_seed0: expected_unique=7
role_binding_agent_control_seed0: expected_unique=5
git_control: expected_unique=4
role_binding_conditional_branch_seed0: expected_unique=3
metrics_report_readout: expected_unique=3
serving_ops: expected_unique=3
role_binding_edit_marker_length_seed0: expected_unique=1
```

CANDIDATE rows:

```text
uncatalogued / resource_pressure_budget
read_inspect
style_brevity
resource_pressure_budget
```

REJECT_FOR_NOW rows:

```text
answer_or_explain
project_context_dialogue
agent_continue_execute
```

Decision:

```text
Do not build the next profile because it is interesting.
Build only when a row can show expected unique CPU accepts over exact cache,
or when a deterministic verifier/split can plausibly raise that number.

Broad routes stay blocked as full routes. Work only narrow artifact-backed
subfamilies.

The next safe directions are:
  metrics_report_readout split
  git_control split
  serving_ops split
  read_inspect verifier/evidence
  test_output_parse if found in real trace
```

Structural gate:

```text
docs/structural_gates/cpu-call-catalog-business-value-v1.md
verdict: PASS
```

## 2026-07-04 - Executor Integration: Document Stamp Layout Payload Dry-Run V1

Verdict:

```text
DOCUMENT_STAMP_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-document-stamp-payload-dry-run-v1
```

Why:

```text
IME admission did not produce a robust safe policy, and resource_pressure has
verifier false evidence. The next narrow real-traffic subfamily from 5k manual
discovery is document_stamp_layout_edit.

This stage builds request-side active_fringe/slot payloads only. It does not
compile a profile, attach output evidence, enable local accepts, or claim
market savings.
```

Run artifact:

```text
target/nando-wave/real-traffic-shadow/document-stamp-payload-dry-run-v1.trace.jsonl
target/nando-wave/real-traffic-shadow/document-stamp-payload-dry-run-v1.report.json
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-document-stamp-payload-dry-run-v1 \
  /home/ubu/.codex/history.jsonl \
  target/nando-wave/real-traffic-shadow/profile-registry-read-inspect-v1.json \
  target/nando-wave/real-traffic-shadow/manual-route-discovery-v1-5k.report.json \
  target/nando-wave/real-traffic-shadow/document-stamp-payload-dry-run-v1.trace.jsonl \
  target/nando-wave/real-traffic-shadow/document-stamp-payload-dry-run-v1.report.json \
  5000
```

Measured result:

```text
document_stamp_candidate_events: 5
payload_ready_events: 4
payload_built_events: 3
scoreable_payload_events: 3
builder_rejected_events: 1
readiness_rejected_events: 1
profile_registered: false
shadow_score_ready: false
active_fringe_centers_total: 385
slots_total: 9
positive_impulses_total: 216
negative_impulses_total: 216
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Decision:

```text
document_stamp_layout_edit now has request-side scoreable payloads.
It is not CPU savings and must not be counted toward CPU80.

Next route debt:
  attach document_position_and_attachment_verifier_v1
  compile disabled-threshold document_stamp .nwrb profile only if verifier-true
  rows exist
  keep local accepts disabled until shadow/audit proves false_accepts=0
```

## 2026-07-04 - Executor Integration: Post-IME Current Feedback Snapshot

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

Run artifact:

```text
target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v5.post-ime-current.report.json
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-feedback-loop-v1 \
  target/nando-wave/real-traffic-shadow/cpu-route-forecast-v1.report.json \
  target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.report.json \
  target/nando-wave/real-traffic-shadow/verification-hook-audit-v1.report.json \
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v5.post-ime-current.report.json
```

Measured result:

```text
total_llm_calls: 1000
operator_candidate_calls: 1000
operator_candidate_route_sum_events: 1019
scoreable_candidate_calls: 174
verification_hook_ready_events: 143
verified_cpu_routability_milli: 32
verified_gap_to_80_calls: 768
unique_verified_cpu_accepts: 26
unique_verified_gap_to_80_calls: 774
incremental_unique_cpu_accepts: 25
incremental_unique_gap_to_80_calls: 775
```

Decision:

```text
IME admission audit added no verified CPU accepts, as intended.
The current consistent 1000-call window remains at 26 unique verified CPU
accepts and 25 incremental unique accepts over exact cache.

Do not count scoreable/profile rows as savings.
Do not count IME singleton policies as savings.
Next growth must come from either promoted existing narrow profiles with
non-synthetic soak or new narrow profiles with deterministic verifiers.
```

## 2026-07-04 - Executor Integration: IME Input-State Admission Audit V1

Verdict:

```text
IME_INPUT_STATE_ADMISSION_AUDIT_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-ime-input-state-admission-audit-v1
```

Why:

```text
The disabled IME/input-state profile can score real request-side payloads, but
profile scoring is not permission to local-accept. This stage audits whether
request-side features can safely separate verifier-true IME diagnostic rows
from verifier-false rows before any safe-policy promotion.

This is a quick profile audit, not a new sandbox. If no robust policy exists,
the route stays disabled and the CPU80 work moves to higher-yield narrow
agent-loop profiles.
```

Run artifact:

```text
target/nando-wave/real-traffic-shadow/ime-input-state-admission-audit-v1.report.json
```

Command:

```text
cargo run -p nando-cli -- role-binding-real-traffic-ime-input-state-admission-audit-v1 \
  target/nando-wave/real-traffic-shadow/ime-input-state-output-evidence-v1.trace.jsonl \
  /home/ubu/.codex/history.jsonl \
  target/nando-wave/real-traffic-shadow/ime-input-state-admission-audit-v1.report.json
```

Measured result:

```text
hook_ready_rows: 5
rows_with_prompt_features: 5
label_true_rows: 3
label_false_rows: 2
robust_safe_policy_found: false
singleton_safe_policy_found: true
best_robust_true_accepts: 0
best_singleton_true_accepts: 1

policy examples:
  all_hook_ready_rows: true=3 false=2
  ime_terms: true=3 false=1
  ime_and_artifact: true=3 false=1
  input_state_terms: true=1 false=0
```

Decision:

```text
IME input-state does not have a robust request-side admission policy.
The only safe-looking policies are singleton support, so they are not enough
to enable local accept.

Do not promote this route now.
Do not count IME toward CPU80 savings.
Do not spend another long loop on IME until more non-synthetic verifier-true
rows exist or the route is split into a narrower debug-artifact subfamily.
```

Claim boundary:

```text
No verified CPU accepts were added.
current_verified_cpu_accepts remains 26.
verified_gap_to_80_calls remains 774.
market_claim_allowed remains false.

No raw prompt text is written.
No raw response text is written.
Verifier labels are used only for offline policy evaluation.
No target labels are used.
No proof labels are used.
Local accepts remain disabled.
Broad answer_or_explain remains fallback-only.
```

Next CPU80 priority:

```text
metric_from_report / report_sync
git_status_summary
test_output_parse
resource_pressure_budget
edit_patch_small
document_stamp_layout_edit
```

## 2026-07-04 - Executor Integration: IME Input-State Disabled Profile V1

Verdict:

```text
IME_INPUT_STATE_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-ime-input-state-profile-v1
```

Why:

```text
The previous IME stage produced real request-side payloads and verifier
evidence, but shadow still saw profile_registered=false. This stage compiles a
dedicated ime_input_state_debug .nwrb package and overlay registry with
threshold=i32::MAX, so the profile can be scored in shadow while local accepts
remain disabled.

This is not a broad answer route and not a market claim. It is one narrow
deterministic profile rung for agent-loop coverage.
```

Run artifacts:

```text
Profile:
  target/nando-wave/real-traffic-shadow/ime-input-state-seed0.nwrb
  target/nando-wave/real-traffic-shadow/profile-registry-ime-input-state-v1.json
  target/nando-wave/real-traffic-shadow/ime-input-state-profile-v1.report.json

Shadow/audit:
  target/nando-wave/real-traffic-shadow/ime-input-state-profile-shadow-v1.report.json
  target/nando-wave/real-traffic-shadow/ime-input-state-profile-v1.verification-hook-audit.report.json
```

Measured result:

```text
profile:
  edge_count: 8
  scoreable_payload_events: 6
  package_training_requests: 6
  positive_updates: 419
  negative_updates: 432
  changed_edges: 154
  package_bytes: 140
  runtime_bytes_estimate: 33000
  threshold: 2147483647
  median_energy_margin: 1447936
  p10_energy_margin: 1120256
  min_energy_margin: 1120256
  median_min_slot_margin: 365568
  unexpected_local_accepts_under_disabled_threshold: 0
  local_accepts_enabled_on_real_traffic: false

shadow:
  total_llm_calls: 5000
  exact_cache_hits: 459
  operator_candidate_calls: 6
  nando_shadow_accepts: 0
  nando_shadow_fallbacks: 6
  verified_safe_accepts: 0
  false_accepts: 0
  incremental_reduction_vs_exact_cache_milli: 0
  p99_shadow_score_latency_ns: 311500

verification_audit:
  operator_candidate_calls: 6
  scoreable_candidate_calls: 6
  verification_hook_ready_events: 5
  verified_cpu_accept_eligible_events: 0
  market_claim_allowed: false
```

Decision:

```text
IME input-state now has a real profile package and registry overlay:
  profile_registered path is available
  scoring telemetry is available
  false_accepts remain 0

But this is still not CPU savings:
  nando_shadow_accepts=0
  verified_cpu_accept_eligible_events=0
  threshold remains i32::MAX

The next step for this route is admission audit / safe policy only if verifier
support can add verified unique CPU accepts over exact cache with false_accepts=0.
Otherwise move to the next narrow profile, not a broad answer route.
```

Claim boundary:

```text
No verified CPU accepts were added.
current_verified_cpu_accepts remains 26.
verified_gap_to_80_calls remains 774.
market_claim_allowed remains false.

No raw prompt text is written.
No raw response text is written.
No target labels are used.
No proof labels are used.
Local accepts remain disabled.
```

## 2026-07-04 - Executor Integration: IME Input-State Payload + Evidence V1

Verdict:

```text
IME_INPUT_STATE_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI routes:
  role-binding-real-traffic-ime-input-state-payload-dry-run-v1
  role-binding-real-traffic-ime-input-state-output-evidence-v1
```

Why:

```text
The 5k manual route discovery split identified ime_input_state_debug as the top
narrow, privacy-safe next profile:
  ime_input_state_debug: 16 events, 6 payload-ready

This stage turns that subfamily into request-side scoreable payloads and joins
Codex final-answer fingerprints through a conservative deterministic verifier.
It does not compile a .nwrb profile and does not enable local accepts.
```

Run artifacts:

```text
Payload dry-run:
  target/nando-wave/real-traffic-shadow/ime-input-state-payload-dry-run-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/ime-input-state-payload-dry-run-v1.report.json

Output evidence:
  target/nando-wave/real-traffic-shadow/ime-input-state-output-evidence-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/ime-input-state-output-evidence-v1.report.json

Shadow/audit:
  target/nando-wave/real-traffic-shadow/ime-input-state-shadow-v1.report.json
  target/nando-wave/real-traffic-shadow/ime-input-state-output-evidence-v1.verification-hook-audit.report.json
```

Measured result:

```text
payload:
  ime_input_state_candidate_events: 16
  payload_ready_events: 6
  payload_built_events: 6
  scoreable_payload_events: 6
  profile_registered: false

output_evidence:
  session_ids_requested: 2
  session_files_scanned: 2
  codex_turns_indexed: 5
  output_evidence_matched_events: 5
  deterministic_verification_events: 5
  verified_true_events: 3
  verified_false_events: 2
  no_session_output_match_events: 1

shadow:
  total_llm_calls: 5000
  exact_cache_hits: 459
  operator_candidate_calls: 6
  nando_shadow_accepts: 0
  nando_shadow_fallbacks: 6
  false_accepts: 0
  p99_shadow_score_latency_ns: 3042

verification_audit:
  verification_hook_ready_events: 5
  verified_cpu_accept_eligible_events: 0
  market_claim_allowed: false
```

Decision:

```text
IME now has real request-side payload support and real verifier evidence:
  6 scoreable payloads
  5 verification-hook-ready rows
  3 verifier-true rows

But this is not CPU savings yet:
  profile_registered=false
  nando_shadow_accepts=0
  verified_cpu_accept_eligible_events=0

The next step for this route is a disabled-threshold IME .nwrb profile and
admission audit. Do not promote local accepts from payload/evidence alone.
```

Claim boundary:

```text
No verified CPU accepts were added.
current_verified_cpu_accepts remains 26.
verified_gap_to_80_calls remains 774.
market_claim_allowed remains false.

No raw prompt text is written.
No raw response text is written.
No target labels are used.
No proof labels are used.
Local accepts remain disabled.
```

## 2026-07-04 - Executor Integration: Manual Route Discovery 5k Taxonomy Split

Verdict:

```text
MANUAL_ROUTE_DISCOVERY_V1_REVIEW_SUBFAMILIES_FOUND
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

Added privacy-safe manual route discovery subfamilies:
  ime_input_state_debug
  document_stamp_layout_edit
  product_price_certification_table
  business_party_identity_address
  gravity_material_physics_question
  nando_capacity_benchmark_state
```

Why:

```text
After the 1000-call registry was exhausted, a 5000-call real Codex history
window was run into separate diagnostic artifacts:
  target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1-5k.report.json
  target/nando-wave/real-traffic-shadow/route-gap-catalog-v1-5k.report.json
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1-5k.report.json
  target/nando-wave/real-traffic-shadow/manual-route-discovery-v1-5k.report.json

The first 5k manual discovery pass left too much uncatalogued traffic in
manual_review_required:
  manual_review_required: 78 events, 18 payload-ready

Local frequency inspection showed repeated privacy-safe route families rather
than one-off raw prompts.
```

5k result after taxonomy split:

```text
sampled_llm_calls: 5000
existing_route_candidate_events: 1457
no_candidate_events: 3543
route_gap_payload_ready_events: 803

manual_discovery_uncatalogued_events: 116
manual_discovery_payload_ready_events: 27
top_subfamily: ime_input_state_debug

Top discovered subfamilies:
  ime_input_state_debug: 16 events, 6 payload-ready
  document_stamp_layout_edit: 5 events, 4 payload-ready
  product_price_certification_table: 7 events, 3 payload-ready
  resource_pressure_budget: 7 events, 3 payload-ready
  business_party_identity_address: 6 events, 3 payload-ready
  manual_review_required: 39 events, 2 payload-ready
  wave_architecture_layout_decision: 7 events, 2 payload-ready
  gravity_material_physics_question: 6 events, 2 payload-ready
```

Decision:

```text
This is route visibility only.
No local accepts are enabled.
No raw prompt text is written.
No CPU savings are claimed.

The useful next profile candidate from 5k is not a broad answer route. It is
ime_input_state_debug, followed by document_stamp_layout_edit and
product_price_certification_table, but each still needs request-side payload,
deterministic verifier, shadow/audit, provider-cost evidence where relevant,
and false_accepts=0 before any promotion.
```

Claim boundary:

```text
No verified CPU accepts were added.
current_verified_cpu_accepts remains 26.
verified_gap_to_80_calls remains 774.
market_claim_allowed remains false.
```

## 2026-07-04 - Executor Integration: Agent Loop Registry Audit Rerank

Verdict:

```text
AGENT_LOOP_PROFILE_REGISTRY_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

The agent-loop registry now reads, when present:
  target/nando-wave/real-traffic-shadow/read-inspect-admission-audit-v1.report.json
  target/nando-wave/real-traffic-shadow/resource-pressure-output-evidence-v1.report.json
  target/nando-wave/real-traffic-shadow/resource-pressure-output-evidence-v1.verification-hook-audit.report.json

and applies it before profile ranking.
```

Why:

```text
The previous registry still selected read_context_path as top_next_profile_key
after the read-inspect admission audit had already proven:
  robust_safe_policy_found: false
  singleton_safe_policy_found: false
  best_robust_true_accepts: 0
  best_singleton_true_accepts: 0

That made the worklist loop back into a route whose current request-side
features cannot safely separate 1 verifier-true row from 8 verifier-false rows.

After that fix, resource_budget_extract became top_next_profile_key, but its
only hook-ready row was already verified false:
  verified_true_events: 0
  verified_false_events: 1
  verified_cpu_accept_eligible_events: 0
  provider_cost_events: 0
```

Reranked result:

```text
admission_blocked_profiles: 2
exhausted_current_support_profiles: 13
top_next_profile_key: null

read_context_path:
  readiness_state: admission_audit_no_safe_policy
  admission_blocked_by_audit: true
  priority_rank: 11

resource_budget_extract:
  readiness_state: verification_audit_no_true_resource_budget_rows
  admission_blocked_by_audit: true
  priority_rank: 12

response_shape_brevity:
  readiness_state: support_exhausted_or_singleton_only
  current_support_exhausted: true
```

Decision:

```text
Do not return to read_context_path until a split path/excerpt subfamily or
richer request-side serving state/evidence exists.

Do not return to resource_budget_extract from the current evidence: its only
hook-ready row is verifier-false and has no provider-cost evidence.

Do not promote response_shape_brevity as CPU80 growth from current evidence:
the catalog marks style-brevity verifier true support as zero.

The current 1000-call trace has no safe top_next_profile_key left after audit
blocks and support-exhaustion guards.

Next real work:
  1. collect more real trace traffic, especially test/tool/report/git paths, or
  2. split exhausted profiles into narrower sibling subprofiles with new
     verifier-true rows.
```

Claim boundary:

```text
No verified CPU accepts were added.
current_verified_cpu_accepts remains 26.
verified_gap_to_80_calls remains 774.
market_claim_allowed remains false.

The rerank changes the worklist only. It writes no raw prompt text, no raw
response text, no target labels, no proof labels, and enables no local accepts.
```

## 2026-07-04 - Executor Integration: Read Inspect Admission Audit V1

Verdict:

```text
READ_INSPECT_ADMISSION_AUDIT_V1_REVIEW_NO_SAFE_POLICY
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-read-inspect-admission-audit-v1

The command reads:
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.trace.jsonl
  /home/ubu/.codex/history.jsonl

and writes:
  target/nando-wave/real-traffic-shadow/read-inspect-admission-audit-v1.report.json
```

Why:

```text
The agent-loop registry selected read_context_path as the next narrow profile.
The current read_inspect score geometry had 9 verifier-hook-ready rows, but
only 1 verifier-true row and 8 verifier-false rows.

This audit checks whether request-side prompt features can separate the one
true read-only path/excerpt row from the false rows before any local accept
promotion.
```

Audit result:

```text
hook_ready_rows: 9
rows_with_prompt_features: 9
label_true_rows: 1
label_false_rows: 8
minimum_true_support: 3
robust_safe_policy_found: false
singleton_safe_policy_found: false
best_robust_true_accepts: 0
best_singleton_true_accepts: 0
local_accepts_enabled: false
market_claim_allowed: false
```

Decision:

```text
Do not lower the read_inspect threshold.
Do not promote read_context_path from current request-side features.
Do not count read_inspect savings.

The next safe work is not broad answer_or_explain and not a generic read route.
It must be either:
  1. split read_context_path into a narrower path/excerpt subfamily, or
  2. capture richer request-side serving state/evidence before scoring.
```

Claim boundary:

```text
No verified CPU accepts were added.
current_verified_cpu_accepts remains 26.
verified_gap_to_80_calls remains 774.
market_claim_allowed remains false.

The audit writes fingerprints, boolean features, counts, and policy scores
only. It writes no raw prompt text, no raw response text, no target labels, no
proof labels, and enables no local accepts.
```

## 2026-07-04 - Executor Integration: Agent Loop Profile Registry V1

Verdict:

```text
AGENT_LOOP_PROFILE_REGISTRY_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-agent-loop-profile-registry-v1

The command reads:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

and writes:
  target/nando-wave/real-traffic-shadow/agent-loop-profile-registry-v1.report.json
```

Why:

```text
CPU Routability 80 will not be reached by promoting one broad route.
The agent loop has to be split into narrow deterministic CPU profiles:
  metric_from_report
  git_status_summary
  test_output_parse
  report_sync
  edit_patch_small
  artifact_progress
  failure_triage-style branch profiles

This report converts the current feedback/catalog evidence into that
microprofile worklist without enabling accepts or re-counting old support.
```

Registry result:

```text
total_llm_calls: 1000
current_verified_cpu_accepts: 26
verified_gap_to_80_calls: 774
microprofile_count: 16
microprofiles_observed: 15
verified_current_profiles: 7
hook_ready_profiles: 15
scoreable_profiles: 15
blocked_wide_route_profiles: 3
exhausted_current_support_profiles: 12
top_next_profile_key: read_context_path
market_claim_allowed: false
```

Decision:

```text
Do not improve answer_or_explain, project_context_dialogue, or
agent_continue_execute as broad routes.

The next narrow route is read_context_path:
  source_route_key: read_inspect
  candidate_events: 27
  scoreable_payload_events: 12
  verification_hook_ready_events: 9
  unique_verified_request_fingerprints: 0
  readiness_state: verifier_hook_ready_needs_safe_admission

Next safe work:
  improve read_context_path admission/verifier geometry,
  require false_accepts=0,
  provider-cost evidence,
  and unique attribution before any local accept.
```

Claim boundary:

```text
No verified CPU accepts were added.
current_verified_cpu_accepts remains 26.
verified_gap_to_80_calls remains 774.
market_claim_allowed remains false.

The registry writes no raw prompt text, no raw response text, no target labels,
no proof labels, and enables no local accepts. It is a worklist, not a market
savings claim.
```

## 2026-07-04 - Executor Integration: Agent Continue State Admission Audit V1

Verdict:

```text
AGENT_CONTINUE_EXECUTE_STATE_ADMISSION_AUDIT_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-agent-continue-execute-state-admission-audit-v1

The command reads:
  target/nando-wave/real-traffic-shadow/agent-continue-execute-artifact-progress-v1.trace.jsonl
  /home/ubu/.codex/sessions

and writes:
  target/nando-wave/real-traffic-shadow/agent-continue-execute-state-admission-audit-v1.report.json
```

Why:

```text
agent_continue_execute prompt-only admission was inseparable:
  25 hook-ready rows
  6 verifier-true rows
  19 verifier-false rows
  robust_safe_policy_found=false

This audit checks whether the previous assistant state before the user prompt
can safely separate real artifact-progress continuation from false continuation
rows. It writes only fingerprints/features/counts.
```

State admission result:

```text
hook_ready_rows: 25
rows_with_state_features: 25
previous_state_missing_rows: 0
label_true_rows: 6
label_false_rows: 19
minimum_true_support: 3
robust_safe_policy_found: false
singleton_safe_policy_found: true
best_robust_true_accepts: 0
best_singleton_true_accepts: 1
```

Feedback/catalog result:

```text
feedback agent_continue_execute:
  stage: local_accept_calibration_failed
  verification_hook_ready_events: 25
  verified_cpu_accept_eligible_events: 0
  next_action: state admission singleton-only; collect more verifier-true rows or split

catalog existing_profile_route agent_continue_execute:
  agent_continue_state_admission_best_robust_true_accepts: 0
  agent_continue_state_admission_best_singleton_true_accepts: 1
  next_action: do not promote; singleton-only support
```

Decision:

```text
Do not enable local accepts for agent_continue_execute.
Do not count the singleton as CPU savings.
Do not lower thresholds.

The previous-state signal is useful as diagnostics, but still not robust.
The next safe work is either:
  1. collect more verifier-true continuation rows; or
  2. split agent_continue_execute into a narrower stateful subroute; or
  3. capture richer structured serving state instead of mining free-form text.
```

Claim boundary:

```text
No verified CPU accepts were added.
current_verified_cpu_accepts remains 26.
verified_gap_to_80_calls remains 774.
market_claim_allowed remains false.

The command uses previous assistant text only to derive privacy-safe boolean
features at analysis time. It writes no raw prompt text, no raw response text,
no target labels, no proof labels, and enables no local accepts.
```

## 2026-07-04 - Executor Integration: Answer Evidence Admission Calibration V1

Verdict:

```text
ANSWER_EVIDENCE_ADMISSION_CALIBRATION_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-answer-evidence-admission-calibration-v1

The command reads:
  target/nando-wave/real-traffic-shadow/answer-evidence-output-evidence-v1.trace.jsonl
  /home/ubu/.codex/history.jsonl

and writes:
  target/nando-wave/real-traffic-shadow/answer-evidence-admission-calibration-v1.report.json
```

Why:

```text
answer_or_explain has 216 route candidates, 9 scoreable grounded-evidence
payloads, and 9 verifier-hook-ready rows. The score/readout calibration did
not find a safe local accept policy, so this step checks whether request-side
prompt features can safely split verifier-true grounded evidence from
verifier-false broad answer rows.
```

Admission calibration result:

```text
hook_ready_rows: 9
rows_with_prompt_features: 9
label_true_rows: 3
label_false_rows: 6
minimum_true_support: 3
robust_safe_policy_found: false
singleton_safe_policy_found: true
best_robust_true_accepts: 0
best_singleton_true_accepts: 1
```

Policy finding:

```text
concise_grounded_question:
  true_accepts: 1
  false_accepts: 0
  robust_safe: false
```

Catalog result:

```text
answer_or_explain existing_profile_route:
  answer_evidence_admission_singleton_only: true
  answer_evidence_admission_best_singleton_true_accepts: 1
  next_action: collect more verifier-true grounded evidence rows or split the subfamily

answer_or_explain route_gap_family:
  answer_evidence_admission_singleton_only: true
  next_action: collect more verifier-true grounded rows or split the route
```

Decision:

```text
Do not promote answer_or_explain from this report.
Do not lower answer-evidence thresholds.
The broad route remains fallback-first. The next safe work is collecting more
verifier-true grounded evidence rows or splitting a narrower grounded-answer
subfamily with deterministic evidence.
```

Claim boundary:

```text
No verified CPU accepts were added.
current_verified_cpu_accepts remains 26.
verified_gap_to_80_calls remains 774.
market_claim_allowed remains false.
The command writes fingerprints/features/counts only; no raw prompt text,
raw response text, target labels, proof labels, or local accepts.
```

## 2026-07-04 - Executor Integration: Agent Continue Admission Feedback/Catalog V1

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

The feedback-loop and CPU-operator catalog now auto-load:
  target/nando-wave/real-traffic-shadow/agent-continue-execute-admission-calibration-v1.report.json

The admission result is now authoritative for agent_continue_execute route
promotion:
  robust_safe_policy_found: false
  singleton_safe_policy_found: false
  best_robust_true_accepts: 0
  best_singleton_true_accepts: 0
```

Result:

```text
feedback agent_continue_execute:
  stage: local_accept_calibration_failed
  local_accept_safe_policy_found: false
  local_accept_best_safe_true_accepts: 0
  verification_hook_ready_events: 25
  verified_cpu_accept_eligible_events: 0

catalog existing_profile_route agent_continue_execute:
  agent_continue_admission_no_safe_policy: true
  next_action: split agent_continue_execute or capture richer request-side state

catalog route_gap_family agent_continue_execute:
  agent_continue_admission_no_safe_policy: true
  next_action: do not promote this route-gap row
```

Claim boundary:

```text
No verified CPU accepts were added.
unique_verified_cpu_accepts remains 26.
incremental_unique_cpu_accepts remains 25.
market_claim_allowed remains false.
```

Decision:

```text
Do not keep chasing the old agent_continue_execute singleton/local-margin hint.
The current prompt-side features are inseparable. The next safe work is route
split or richer request-side state capture before any promotion attempt.
```

## 2026-07-04 - Executor Integration: Agent Continue Admission Calibration V1

Verdict:

```text
AGENT_CONTINUE_EXECUTE_ADMISSION_CALIBRATION_V1_REVIEW_NO_SAFE_POLICY
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-agent-continue-execute-admission-calibration-v1

The command reads:
  target/nando-wave/real-traffic-shadow/agent-continue-execute-artifact-progress-v1.trace.jsonl
  /home/ubu/.codex/history.jsonl

and writes:
  target/nando-wave/real-traffic-shadow/agent-continue-execute-admission-calibration-v1.report.json
```

Why:

```text
agent_continue_execute had 25 verifier-hook-ready rows and a weak margin-only
local accept candidate, but local score geometry could not safely separate true
artifact-progress rows from verifier-false continuation rows. This step checks
whether request-side prompt features can supply a safer admission gate before
any accept path is promoted.
```

Admission calibration result:

```text
hook_ready_rows: 25
rows_with_prompt_features: 25
history_prompt_missing_rows: 0
label_true_rows: 6
label_false_rows: 19
minimum_true_support: 3
robust_safe_policy_found: false
singleton_safe_policy_found: false
best_robust_true_accepts: 0
best_singleton_true_accepts: 0
```

Policy findings:

```text
all_hook_ready_rows:
  true_accepts: 6
  false_accepts: 19

direct_action_words:
  true_accepts: 6
  false_accepts: 19

nando_wave_terms:
  true_accepts: 0
  false_accepts: 1

direct_action_project_or_nando_no_failure:
  true_accepts: 0
  false_accepts: 1
```

Decision:

```text
Do not enable local accepts for agent_continue_execute.
Do not lower the disabled threshold.
Do not promote a prompt-feature policy from this report.

The route remains valuable as a candidate zone, but current prompt-side
features are inseparable: the same direct-action shape covers both real
artifact progress and false continuation rows. The next route work must either
split agent_continue_execute into narrower subroutes or capture richer
request-side state before verification.
```

Claim boundary:

```text
No verified CPU accepts were added.
No market savings claim is allowed.
The command writes fingerprints/features/counts only; no raw prompt text,
raw response text, target labels, proof labels, or local accepts.
```

## 2026-07-04 - Executor Integration: Resource Pressure Feedback Loop V1

Verdict:

```text
RESOURCE_PRESSURE_FEEDBACK_LOOP_V1_REVIEW_ROUTE_VISIBLE_NO_ACCEPTS
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

The feedback-loop report now auto-loads:
  target/nando-wave/real-traffic-shadow/resource-pressure-payload-dry-run-v1.report.json
  target/nando-wave/real-traffic-shadow/resource-pressure-output-evidence-v1.verification-hook-audit.report.json

and emits a separate route row:
  route_key: resource_pressure_budget
  profile_id: route_gap_resource_pressure_budget_profile_v1
```

Why:

```text
Resource Pressure Output Evidence V1 made one scoreable resource-pressure row
verification-hook-ready, but the CPU feedback loop did not expose it as a
route. That hid the stage transition from scoreable-only to hook-ready.
```

Feedback-loop result:

```text
route_count: 15
operator_candidate_route_sum_events: 1019
operator_candidate_calls: 1000
scoreable_candidate_calls: 174
verification_hook_ready_events: 143
verified_cpu_accept_eligible_events: 32
verified_gap_to_80_calls: 768
unique_verified_cpu_accepts: 26
unique_verified_gap_to_80_calls: 774
market_claim_allowed: false

resource_pressure_budget row:
  candidate_events: 3
  payload_ready_events: 1
  payload_built_events: 1
  scoreable_payload_events: 1
  verification_hook_ready_events: 1
  verified_cpu_accept_eligible_events: 0
  false_accepts: 0
  stage: verification_hook_ready_waiting_local_accept
```

Catalog after regeneration:

```text
existing_operator_candidate_calls: 1000
existing_operator_candidate_route_sum_events: 1019
no_candidate_calls: 0
current_verified_cpu_accepts: 26
verified_gap_to_80_calls: 774
```

Decision:

```text
Do not count resource_pressure_budget as CPU savings.
Do not enable local accepts.
Do not promote a normal accepting profile from verifier-false / provider-cost
missing evidence.

This step is report integration only: resource_pressure_budget is now visible
in the feedback loop, but it contributes 0 verified CPU accepts.
```

Claim boundary:

```text
No verified CPU accepts were added.
CPU Routability 80 remains open:

  current unique verified CPU accepts: 26
  unique verified gap to 80: 774

market_claim_allowed=false.
```

## 2026-07-04 - Executor Integration: Resource Pressure Output Evidence V1

Verdict:

```text
RESOURCE_PRESSURE_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-resource-pressure-output-evidence-v1

The command reads:
  target/nando-wave/real-traffic-shadow/resource-pressure-payload-dry-run-v1.trace.jsonl
  /home/ubu/.codex/sessions

and writes:
  target/nando-wave/real-traffic-shadow/resource-pressure-output-evidence-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/resource-pressure-output-evidence-v1.report.json
```

Why:

```text
Resource Pressure Payload Dry Run V1 proved one request-side scoreable payload,
but it still had no response/tool evidence and no deterministic verifier hook.
This step attaches conservative final-answer evidence through
write_rate_or_resource_budget_verifier_v1 without enabling local accepts.
```

Output evidence result:

```text
operator_candidate_calls: 1
scoreable_candidate_calls: 1
output_evidence_matched_events: 1
deterministic_verification_events: 1
verified_true_events: 0
verified_false_events: 1

raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_verification: true
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Shadow/audit result:

```text
shadow report:
  total_llm_calls: 1000
  exact_cache_hits: 53
  nando_shadow_accepts: 0
  verified_safe_accepts: 0
  false_accepts: 0
  p99_shadow_score_latency_ns: 6893

verification-hook audit:
  operator_candidate_calls: 1
  scoreable_candidate_calls: 1
  verification_hook_ready_events: 1
  verified_cpu_accept_eligible_events: 0
  candidates_missing_provider_cost: 1
  market_claim_allowed: false
```

Catalog after regeneration:

```text
top row:
  route_or_family_key: uncatalogued
  manual_route_discovery_top_subfamily: resource_pressure_budget
  candidate_events: 13
  payload_ready_events: 3
  scoreable_payload_events: 1
  verification_hook_ready_events: 1
  verified_cpu_accept_eligible_events: 0

next_action:
  resource_pressure_budget has 1 verifier-hook-ready rows and 0 verifier-true
  rows, but 0 verified CPU accepts. Compile only a disabled-threshold
  resource_pressure .nwrb profile and rerun shadow/audit with provider cost;
  keep local accepts disabled until false_accepts=0 and calibrated safe policy
  exist.
```

Decision:

```text
Do not count this route as CPU savings.
Do not enable local accepts.
Do not compile a normal accepting profile from one verifier-false row.

The hook is valuable because it closes the previous "no verifier hook" gap:
resource_pressure_budget moved from scoreable-only to hook-ready-but-not-safe.
The next concrete debt is disabled-threshold resource_pressure .nwrb scoring
profile only if more verifier-true evidence appears, plus provider-cost
evidence before any market claim.
```

Claim boundary:

```text
No verified CPU accepts were added.
CPU Routability 80 remains open:

  current_verified_cpu_accepts: 26
  verified_gap_to_80_calls: 774

market_claim_allowed=false.
```

## 2026-07-04 - Executor Integration: Resource Pressure Payload Dry Run V1

Verdict:

```text
RESOURCE_PRESSURE_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-resource-pressure-payload-dry-run-v1

The command reads:
  /home/ubu/.codex/history.jsonl
  target/nando-wave/real-traffic-shadow/manual-route-discovery-v1.report.json

and writes:
  target/nando-wave/real-traffic-shadow/resource-pressure-payload-dry-run-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/resource-pressure-payload-dry-run-v1.report.json

The CPU operator catalog now auto-loads that dry-run report when present and
carries resource_pressure scoreable payload progress into the uncatalogued row.
```

Why:

```text
Manual Route Discovery V1 found the top uncatalogued subfamily:

  resource_pressure_budget

but that was only a backlog label. We needed to prove whether the route can
actually build request-side active_fringe/slots from prompt text without using
the response, target labels, proof_rule_id, or raw prompt output.
```

Dry-run result:

```text
resource_pressure_candidate_events: 3
payload_ready_events: 1
payload_built_events: 1
scoreable_payload_events: 1
active_fringe_centers: 124
slots: 3
positive_impulses: 72
negative_impulses: 72

profile_registered: false
shadow_score_ready: false
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Catalog after regeneration:

```text
top row:
  route_or_family_key: uncatalogued
  manual_route_discovery_top_subfamily: resource_pressure_budget
  candidate_events: 13
  payload_ready_events: 3
  scoreable_payload_events: 1
  verified_cpu_accept_eligible_events: 0

next_action:
  Attach write_rate_or_resource_budget_verifier_v1 and keep local accepts
  disabled until deterministic verifier evidence, shadow/audit, provider cost,
  and false_accepts=0.
```

Decision:

```text
Do not compile/promote a resource_pressure .nwrb profile yet.
Do not count the one scoreable payload as CPU savings.
Do not treat resource/budget prompts as locally safe without measured
write-rate/resource-budget evidence.

The next concrete debt is:
  real response/tool-call evidence
  + write_rate_or_resource_budget_verifier_v1
  + only then disabled-threshold profile work if verifier-true support exists.
```

Claim boundary:

```text
No verified CPU accepts were added.
CPU Routability 80 remains open:

  current_verified_cpu_accepts: 26
  verified_gap_to_80_calls: 774

market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/resource-pressure-payload-dry-run-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/resource-pressure-payload-dry-run-v1.nanda.txt

verdict: PASS
complexity_score: 32
note: narrow claim-boundary check only: resource_pressure_budget has one
      scoreable request-side payload, but zero verified CPU accepts and no
      market claim. The gate packet intentionally keeps the non-claim core
      instead of duplicating every JSON guard field.
```

## 2026-07-04 - Executor Integration: Manual Route Discovery V1

Verdict:

```text
MANUAL_ROUTE_DISCOVERY_V1_REVIEW_SUBFAMILIES_FOUND
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-manual-route-discovery-v1

The command reads:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1.report.json

and writes:
  target/nando-wave/real-traffic-shadow/manual-route-discovery-v1.report.json

The CPU operator catalog now auto-loads that manual discovery report when
present and carries the top uncatalogued subfamily into the uncatalogued row.
```

Why:

```text
After style_brevity was correctly blocked, the top catalog row became:

  route_or_family_key: uncatalogued
  candidate_events: 13
  payload_ready_events: 3
  scoreable_payload_events: 0

That was too vague to guide CPU80 work. We needed a privacy-safe route backlog
that says which concrete route family should be built next, without enabling
local accepts or writing raw prompt text.
```

Discovery run:

```text
uncatalogued_events: 13
payload_ready_events: 3
missing_history_rows: 0
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false

top_subfamily:
  resource_pressure_budget
```

Discovered subfamilies:

```text
1. resource_pressure_budget
   events: 3
   payload_ready: 1
   builder: resource_pressure_payload_builder_v1
   verifier: write_rate_or_resource_budget_verifier_v1

2. wave_architecture_layout_decision
   events: 2
   payload_ready: 1
   builder: architecture_layout_delta_payload_builder_v1
   verifier: typed_layout_budget_and_invariant_verifier_v1

3. business_logistics_route_constraint
   events: 1
   payload_ready: 1
   builder: business_logistics_route_payload_builder_v1
   verifier: named_party_route_constraint_verifier_v1

Remaining one-off review families:
  claim_boundary_definition
  affective_rejection_control
  implementation_cost_objection
  project_synthesis_summary
  research_theory_gap
  skill_route_split_advice
```

Catalog after regeneration:

```text
top row:
  route_or_family_key: uncatalogued
  manual_route_discovery_top_subfamily: resource_pressure_budget
  manual_route_discovery_top_subfamily_events: 3
  manual_route_discovery_payload_ready_events: 1
  priority_score: 1065

next_action:
  Build resource_pressure_payload_builder_v1
  + write_rate_or_resource_budget_verifier_v1;
  keep local accepts disabled until false_accepts=0 is proven.
```

Decision:

```text
Do not treat uncatalogued as a CPU accept route.
Do not merge these rows into broad answer_or_explain or project_context_dialogue
just to inflate routability.

The next concrete route candidate is resource_pressure_budget, because it is
the top discovered uncatalogued subfamily and has at least one payload-ready
row.
```

Claim boundary:

```text
No verified CPU accepts were added.
CPU Routability 80 remains open:

  current_verified_cpu_accepts: 26
  verified_gap_to_80_calls: 774

market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/manual-route-discovery-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/manual-route-discovery-v1.nanda.txt

verdict: PASS
complexity_score: 31
note: narrow claim-boundary check only: manual discovery identifies
      resource_pressure_budget as the top uncatalogued subfamily, but enables
      no local accepts and no CPU-savings claim.
```

## 2026-07-04 - Executor Integration: Style Brevity Evidence Hook V1

Verdict:

```text
STYLE_BREVITY_OUTPUT_EVIDENCE_V1_REVIEW_VERIFIER_FALSE_NO_PROMOTION
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-style-brevity-output-evidence-v1

The style_brevity route now has a response-shape evidence join using:
  response_length_and_format_verifier_v1

The feedback-loop also auto-loads:
  target/nando-wave/real-traffic-shadow/style-brevity-output-evidence-v1.verification-hook-audit.report.json
```

Why:

```text
Before this step, style_brevity was scoreable but had no verification hook:

  candidate_events: 3
  scoreable_payload_events: 3
  verification_hook_ready_events: 0

The CPU operator catalog therefore ranked it as the top next row even though
no deterministic evidence existed.
```

Evidence run:

```text
style-brevity output evidence:
  session_ids_requested: 1
  session_files_scanned: 1
  codex_turns_indexed: 1
  output_evidence_matched_events: 1
  verified_true_events: 0
  verified_false_events: 1
  raw_prompt_text_written: false
  raw_response_text_written: false

style-brevity shadow:
  total_llm_calls: 1000
  nando_shadow_accepts: 0
  verified_safe_accepts: 0
  false_accepts: 0
  p99_shadow_score_latency_ns: 222178

style-brevity verification audit:
  operator_candidate_calls: 3
  scoreable_candidate_calls: 3
  verification_hook_ready_events: 1
  verified_cpu_accept_eligible_events: 0
  verified_true_events: 0
  verified_false_events: 1
  market_claim_allowed: false
```

Catalog correction:

```text
Before:
  top_catalog_row: style_brevity (existing_profile_route)

After:
  top_catalog_row: uncatalogued (route_gap_family)

style_brevity existing profile:
  priority_rank: 23
  style_brevity_verifier_true_events: 0
  style_brevity_verifier_false_events: 1
  style_brevity_verifier_true_support_zero: true
  priority_score: -158090
```

Decision:

```text
Do not run local-accept calibration for style_brevity yet.
Do not count style_brevity as CPU savings.
Do not treat a response-style policy as a semantic task solver.

The route needs better matching style-only samples or tighter admission before
promotion. Current verifier evidence is negative.
```

Claim boundary:

```text
No verified CPU accepts were added.
CPU Routability 80 remains open:

  verified_cpu_routability_milli: 32
  verified_gap_to_80_calls: 768
  unique_verified_cpu_accepts: 26
  incremental_unique_cpu_accepts: 25

market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/style-brevity-evidence-hook-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/style-brevity-evidence-hook-v1.nanda.txt

verdict: PASS
complexity_score: 37
note: narrow claim-boundary check only: style_brevity has verifier evidence,
      but the verifier found 0 true / 1 false rows, so the route remains
      blocked from local-accept calibration and CPU-savings claims.
```

## 2026-07-04 - Executor Integration: Short Decision Ack Prior Blocker V1

Verdict:

```text
SHORT_DECISION_ACK_PRIOR_BLOCKER_V1_REVIEW_STANDALONE_ACK_ROUTE_BLOCKED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

The CPU operator catalog now reads:
  target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v2.report.json

and carries prior short_ack false-accept evidence into the
`short_decision_ack` route-gap row.
```

Why:

```text
The catalog had promoted short_decision_ack to the top actionable row because
it had 19 route-gap candidates and no explicit profile route.

That was misleading. Prior agent-control admission evidence already showed:

  short_ack_intent:
    true_accepts: 1
    false_accepts: 48

So a standalone "ok/yes/да" route is unsafe without explicit previous-turn
decision-state evidence.
```

Catalog after regeneration:

```text
short_decision_ack:
  priority_rank: 23
  short_decision_ack_prior_blocked: true
  short_decision_ack_prior_true_accepts: 1
  short_decision_ack_prior_false_accepts: 48

top_catalog_row:
  style_brevity (existing_profile_route)
```

Decision:

```text
Do not build a standalone short_ack route.

The only acceptable future version is:
  explicit previous-turn decision-state evidence
  + active_turn_state_transition_verifier_v1
  + false_accepts=0
  + no task-content inference
```

Claim boundary:

```text
No new verified CPU accepts were created.
CPU Routability 80 remains open.

This change improves route selection quality only: it prevents the catalog from
spending engineering effort on a known false-accept-prone short-ack route.
```

Structural gate:

```text
packet:
  docs/structural_gates/short-decision-ack-prior-blocker-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/short-decision-ack-prior-blocker-v1.nanda.txt

verdict: PASS
complexity_score: 41
note: narrow catalog-routing check only: short_decision_ack is blocked by
      prior false-accept evidence and must not be treated as a standalone
      safe CPU promotion route.
```

## 2026-07-04 - Executor Integration: Answer Evidence Local-Accept Calibration Feedback V1

Verdict:

```text
ANSWER_EVIDENCE_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

The feedback-loop now reads:
  target/nando-wave/real-traffic-shadow/answer-evidence-local-accept-calibration-v1.report.json

and applies it to the `answer_or_explain` route row even though that row is
currently injected from the answer-evidence side path rather than from the
primary forecast route list.
```

Calibration result:

```text
hook_ready_rows:          9
scored_rows:              9
label_true_rows:          3
label_false_rows:         6
safe_policy_found:    false
best_safe_true_accepts:   0
local_accepts_enabled: false
market_claim_allowed:  false
```

Policy audit:

```text
energy_positive_no_slot_order:
  accepts: 9
  true_accepts: 3
  false_accepts: 6

strict_positive_slots_and_energy_positive:
  accepts: 9
  true_accepts: 3
  false_accepts: 6

evidence_slot_positive_only:
  accepts: 9
  true_accepts: 3
  false_accepts: 6

boundary_slot_positive_only:
  accepts: 9
  true_accepts: 3
  false_accepts: 6

best threshold policies:
  best_safe_true_accepts: 0
```

Margin collision:

```text
energy_margin:
  min_true_margin: 1123328
  false_rows_at_or_above_min_true_margin: 5

min_slot_margin:
  min_true_margin: 361472
  false_rows_at_or_above_min_true_margin: 6

marker_slot_margin:
  min_true_margin: 393216
  false_rows_at_or_above_min_true_margin: 5

end_slot_margin:
  min_true_margin: 368640
  false_rows_at_or_above_min_true_margin: 6
```

Feedback/catalog after regeneration:

```text
feedback answer_or_explain:
  stage: local_accept_calibration_failed
  local_accept_calibration_ran: true
  local_accept_safe_policy_found: false
  local_accept_best_safe_true_accepts: 0
  verified_cpu_accept_eligible_events: 0
  false_accepts: 0

catalog:
  answer_or_explain existing-profile row is no longer treated as
  waiting-for-calibration. It is marked local_accept_calibration_failed and
  deprioritized until request-side admission, verifier evidence, or payload
  geometry improves.
```

Decision:

```text
Do not promote answer_or_explain.
Do not lower thresholds.
Do not count scoreable/hook-ready answer rows as savings.

The current grounded answer-evidence readout does not separate true grounded
answers from verifier-false rows. This is a payload/readout geometry failure,
not a missing calibration run.
```

Claim boundary:

```text
No new verified CPU accepts were created.
CPU Routability 80 remains open:

  verified_cpu_accept_eligible_events: 32 / 1000
  verified_gap_to_80_calls: 768

market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/answer-evidence-local-accept-feedback-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/answer-evidence-local-accept-feedback-v1.nanda.txt

verdict: PASS
complexity_score: 42
note: narrow claim-boundary check only: answer_or_explain local-accept
      calibration failed, feedback marks the route failed, local accepts remain
      disabled, and verified CPU savings stay unchanged.
```

## 2026-07-04 - Executor Integration: Agent Continue Execute Local-Accept Calibration V1

Verdict:

```text
AGENT_CONTINUE_EXECUTE_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SINGLETON_OR_WEAK_SAFE_POLICY_CANDIDATE
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI route:
  role-binding-real-traffic-agent-continue-execute-local-accept-calibration-v1

The agent_continue_execute route now has a review-only local-accept calibration
report against the tool-backed artifact-progress/no-drift verifier labels.
The feedback-loop report also records the calibration artifact path.
```

Calibration result:

```text
report:
  target/nando-wave/real-traffic-shadow/agent-continue-execute-local-accept-calibration-v1.report.json

hook_ready_rows:              25
scored_rows:                  25
label_true_rows:               6
label_false_rows:             19
safe_policy_found:          true
best_safe_true_accepts:        1
local_accepts_enabled:     false
market_claim_allowed:      false
```

Policy audit:

```text
current_disabled_profile_policy:
  accepts: 0
  false_accepts: 0

energy_positive_no_slot_order:
  accepts: 25
  true_accepts: 6
  false_accepts: 19

strict_positive_slots_and_energy_positive:
  accepts: 25
  true_accepts: 6
  false_accepts: 19

next_action_slot_positive_only:
  accepts: 25
  true_accepts: 6
  false_accepts: 19

evidence_slot_positive_only:
  accepts: 25
  true_accepts: 6
  false_accepts: 19

best_energy_margin_threshold_request_side_only:
  threshold: 1114112
  accepts: 1
  true_accepts: 1
  false_accepts: 0

best_evidence_slot_margin_threshold_request_side_only:
  threshold: 720896
  accepts: 1
  true_accepts: 1
  false_accepts: 0
```

Feedback/catalog after regeneration:

```text
feedback:
  total_llm_calls:                       1000
  scoreable_candidate_calls:              173
  verification_hook_ready_events:         142
  verified_cpu_accept_eligible_events:     32
  verified_cpu_routability_milli:          32
  verified_gap_to_80_calls:               768

agent_continue_execute route row:
  stage: local_accept_calibration_support_insufficient
  local_accept_calibration_ran: true
  local_accept_safe_policy_found: true
  local_accept_support_qualified: false
  local_accept_best_safe_true_accepts: 1
  verified_cpu_accept_eligible_events: 0
  false_accepts: 0

catalog:
  agent_continue_execute existing-profile row is now deprioritized as
  local_accept_support_insufficient, not treated as a promotion candidate.
```

Decision:

```text
Do not promote this route.
Do not lower the disabled threshold.
Do not count the singleton safe policy as savings.

The weak safe candidate proves the scoring path is measurable, but support is
below the minimum true-support gate. The route needs more verifier-true rows or
better request-side admission/payload geometry before promotion.
```

Claim boundary:

```text
No new verified CPU accepts were created.
CPU Routability 80 remains open:

  verified_cpu_accept_eligible_events: 32 / 1000
  verified_gap_to_80_calls: 768

market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/agent-continue-execute-local-accept-calibration-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/agent-continue-execute-local-accept-calibration-v1.nanda.txt

verdict: PASS
complexity_score: 43
note: narrow claim-boundary check only: agent_continue_execute has weak
      request-side local-accept calibration, but support is insufficient,
      local accepts remain disabled, and verified CPU savings stay unchanged.
```

## 2026-07-04 - Executor Integration: Agent Continue Execute Payload/Profile V1

Verdict:

```text
AGENT_CONTINUE_EXECUTE_ROUTE_V1_REVIEW_SCOREABLE_AND_HOOK_READY_ACCEPTS_DISABLED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added CLI routes:
  role-binding-real-traffic-agent-continue-execute-payload-dry-run-v1
  role-binding-real-traffic-agent-continue-execute-profile-v1
  role-binding-real-traffic-agent-continue-execute-output-evidence-v1
  role-binding-real-traffic-agent-continue-execute-artifact-progress-v1

The agent_continue_execute route now has a request-side payload builder,
a disabled-threshold .nwrb profile, final-answer fingerprint attachment, and
tool-backed artifact-progress/no-drift verification surface.
```

Real Codex window:

```text
total_llm_calls:                         1000
exact_cache_hits:                          53
route-only operator candidates:           422
agent_continue_execute candidates:         32
agent_continue_execute payload_ready:      28
agent_continue_execute scoreable:          28
agent_continue_execute hook_ready:         25
artifact verifier true rows:                6
artifact verifier false rows:              19
tool_call_fingerprint_events:              20
```

Profile/shadow:

```text
profile_id: route_gap_agent_continue_execute_profile_v1
package: target/nando-wave/real-traffic-shadow/agent-continue-execute-seed0.nwrb
edge_count: 8
threshold: 2147483647
median_energy_margin: 835584
unexpected_local_accepts_under_disabled_threshold: 0

agent route shadow:
  nando_shadow_accepts: 0
  verified_safe_accepts: 0
  false_accepts: 0
  incremental_reduction_vs_exact_cache_milli: 0
  p99_shadow_score_latency_ns: 240540
```

Feedback-loop after regeneration:

```text
scoreable_candidate_calls:              173
verification_hook_ready_events:         142
verified_cpu_accept_eligible_events:     32
verified_cpu_routability_milli:          32
verified_gap_to_80_calls:               768

agent_continue_execute route row:
  candidate_events:                       32
  non_exact_candidate_calls:              22
  scoreable_payload_events:               28
  verification_hook_ready_events:         25
  verified_cpu_accept_eligible_events:     0
  false_accepts:                           0
```

Decision:

```text
This route is now measurable and verifier-hook-ready, but it is not a CPU
savings route yet. The profile intentionally uses threshold=i32::MAX and keeps
local accepts disabled.

Do not count:
  agent_continue_execute candidates,
  scoreable payloads,
  hook-ready rows,
  artifact verifier true rows,
  or disabled-profile margins

as verified CPU savings.
```

Claim boundary:

```text
No new verified CPU accepts were created. CPU Routability 80 remains open:

  verified_cpu_accept_eligible_events: 32 / 1000
  verified_gap_to_80_calls: 768

market_claim_allowed=false.
```

Next debt:

```text
Run local-accept calibration only after there is enough verifier-true support.
If no safe policy exists, improve request-side admission or payload features.
Never lower the disabled threshold just to create savings.
```

Structural gate:

```text
packet:
  docs/structural_gates/agent-continue-execute-payload-profile-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/agent-continue-execute-payload-profile-v1.nanda.txt

verdict: PASS
complexity_score: 38
note: narrow claim-boundary check only: agent_continue_execute is scoreable
      and hook-ready, but local accepts remain disabled and verified CPU
      accept eligible count stays 0 for this route.
```

## 2026-07-04 - Executor Integration: CPU Route Feedback Loop Integration Audit

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_INTEGRATION_AUDIT_V1_REVIEW_NEXT_ROUTE_HITLIST_READY
```

What was checked:

```text
The earlier review note that the feedback loop was unfinished is stale in the
current tree.

Confirmed current code:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
    feedback_route_stage(...)
    feedback_route_next_action(...)

  crates/nando-cli/src/main.rs
    role-binding-real-traffic-feedback-loop-v1 dispatch is present

  crates/nando-cli/src/help.rs
    role-binding-real-traffic-feedback-loop-v1 help text is present

Confirmed current artifact:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json
```

Current feedback loop:

```text
total_llm_calls:                       1000
operator_candidate_calls:              1000
scoreable_candidate_calls:              145
verification_hook_ready_events:         117
verified_cpu_accept_eligible_events:     32
verified_cpu_routability_milli:          32
verified_gap_to_80_calls:               768
false_accepts:                            0
```

CPU route hitlist:

```text
answer_or_explain:
  candidates: 216
  scoreable: 9
  hook_ready: 9
  verified_cpu_accept_eligible: 0
  status: profile + hook exist, but local-accept calibration found no safe
          readout policy. Do not lower threshold.
  next: improve request-side admission or payload geometry.

project_context_dialogue:
  candidates: 211
  scoreable: 2
  hook_ready: 2
  verified_cpu_accept_eligible: 0
  status: only the artifact-backed subset is real; 180/211 are short context
          chatter and must not be promoted as project-state operators.
  next: keep broad dialogue fallback-only; expand only when an active
        goal/artifact state channel exists.

role_binding_agent_control_seed0:
  candidates: 143
  scoreable: 11
  hook_ready: 11
  verified_cpu_accept_eligible: 11
  status: current robust stop/control support is exhausted.
  next: split broader agent-state/tool-state subfamilies; do not repeat the
        same stop promotion.

role_binding_conditional_branch_seed0:
  candidates: 166
  scoreable: 53
  hook_ready: 40
  verified_cpu_accept_eligible: 3
  status: payload path exists but support is exhausted by current admission.
  next: split a stronger conditional subfamily or improve payload geometry.

metrics_report_readout:
  candidates: 55
  scoreable: 3
  hook_ready: 3
  verified_cpu_accept_eligible: 3
  status: numeric report path has safe support but it is already exhausted.
  next: split a stronger numeric-evidence report subfamily.

agent_continue_execute:
  candidates: 31
  scoreable: 0
  hook_ready: 0
  verified_cpu_accept_eligible: 0
  status: next new route-gap family worth building because it is a real
          state-transition operator, not broad chat.
  next: build active_goal_next_step_payload_builder_v1 plus
        artifact_progress_and_no_drift_verifier_v1; accepts disabled until
        deterministic verification exists.
```

Decision:

```text
The feedback-loop integration debt is closed in the current source tree. The
remaining CPU Routability 80 debt is not command wiring; it is route quality:
request-side payload builders, deterministic verifier hooks, and safe local
accept calibration on non-synthetic traffic.

Do not count:
  route candidates,
  payload-ready rows,
  scoreable rows,
  verification-hook-ready rows,
  disabled-profile scores,
  or final-answer labels

as verified CPU savings.

Only hook-backed local accepts with false_accepts=0, provider cost evidence,
and non-synthetic trace evidence can count toward market savings.
```

Claim boundary:

```text
No new verified CPU accepts were created by this audit. CPU Routability 80
remains open:

  verified_cpu_accept_eligible_events: 32 / 1000
  verified_gap_to_80_calls: 768

market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/cpu-route-feedback-loop-integration-audit-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-integration-audit-v1.nanda.txt

verdict: PASS
complexity_score: 35
note: narrow claim-boundary check only: command wiring present and current
      32/1000 verified CPU eligible telemetry is not market savings or CPU
      Routability 80 completion.
```

## 2026-07-04 - Executor Integration: Answer-Evidence Local Accept Calibration V1

Verdict:

```text
ANSWER_EVIDENCE_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY
ANSWER_EVIDENCE_SAFE_POLICY_PROMOTE_V1_REJECTED_NO_SUPPORTED_POLICY
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-answer-evidence-local-accept-calibration-v1
  role-binding-real-traffic-answer-evidence-safe-policy-promote-v1
  docs/structural_gates/answer-evidence-local-accept-calibration-v1.md

The answer_or_explain route now has a review-only local-accept calibration
surface and a guarded safe-policy promotion command. The promotion command
requires robust support and refuses to write a promoted accept path when the
calibration is not separable.
```

Calibration:

```text
report:
  target/nando-wave/real-traffic-shadow/answer-evidence-local-accept-calibration-v1.report.json

hook_ready_rows: 9
label_true_rows: 3
label_false_rows: 6
safe_policy_found: false
best_safe_true_accepts: 0
local_accepts_enabled: false
market_claim_allowed: false
```

Promotion guard:

```text
command:
  role-binding-real-traffic-answer-evidence-safe-policy-promote-v1

result:
  rejected: answer-evidence calibration report has no supported robust
  energy-threshold policy
```

Decision:

```text
Do not enable answer_or_explain local accepts from the current grounded-answer
profile. The verifier hook is present, but the score geometry accepts true and
false rows together. This route stays at verification-hook-ready only until
request-side admission or payload geometry creates false_accept=0 separation.
```

Structural gate:

```text
packet:
  docs/structural_gates/answer-evidence-local-accept-calibration-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/answer-evidence-local-accept-calibration-v1.nanda.txt

verdict: PASS
complexity_score: 30
note: narrow route-swap check only: calibration/promotion review artifacts are
      not local accept, verified CPU saving, or market claim.
```

## 2026-07-04 - Executor Integration: Answer-Evidence Output Evidence Hook V1

Verdict:

```text
ANSWER_EVIDENCE_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
REAL_TRAFFIC_SHADOW_V1_REVIEW
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-answer-evidence-output-evidence-v1

The answer_or_explain route can now join offline Codex final-answer evidence
to the disabled-profile shadow trace. The command writes response fingerprints
and deterministic verifier status only. It writes no raw prompt or response
text and does not enable local accepts.
```

Output evidence:

```text
trace:
  target/nando-wave/real-traffic-shadow/answer-evidence-output-evidence-v1.trace.jsonl

report:
  target/nando-wave/real-traffic-shadow/answer-evidence-output-evidence-v1.report.json

output_evidence_matched_events: 9
verified_true_events: 3
verified_false_events: 6
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_verification: true
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Shadow and verification audit:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/answer-evidence-output-evidence-v1.shadow-report.json

nando_shadow_accepts: 0
verified_safe_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 214912

verification_audit:
  target/nando-wave/real-traffic-shadow/answer-evidence-output-evidence-v1.verification-hook-audit.report.json

operator_candidate_calls: 9
scoreable_candidate_calls: 9
verification_hook_ready_events: 9
verified_cpu_accept_eligible_events: 0
candidates_missing_provider_cost: 9
market_claim_allowed: false
```

Feedback loop:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

total_llm_calls: 1000
operator_candidate_calls: 1000
operator_candidate_route_sum_events: 1025
scoreable_candidate_calls: 145
verification_hook_ready_events: 117
verified_cpu_routability_milli: 32
unique_verified_cpu_accepts: 26
incremental_unique_cpu_accepts: 25

answer_or_explain:
  candidate_events: 216
  payload_ready_events: 17
  scoreable_payload_events: 9
  verification_hook_ready_events: 9
  verified_cpu_accept_eligible_events: 0
  false_accepts: 0
  stage: verification_hook_ready_waiting_local_accept
```

Catalog:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

top_catalog_row: answer_or_explain (existing_profile_route)
existing_operator_candidate_calls: 1000
existing_operator_candidate_route_sum_events: 1025
current_verified_cpu_accepts: 26
verified_gap_to_80_calls: 774
recommended_payload_builder: answer_evidence_payload_builder_v1
recommended_verifier: grounded_answer_evidence_verifier_v1
```

Decision:

```text
This closes the "verification hook missing" gap for the nine scoreable
answer_or_explain rows. It does not create verified CPU accepts or market
savings. The next engineering step is request-side local-accept calibration
and provider-cost evidence, with false_accepts=0 still required.
```

Claim boundary:

```text
candidate coverage is capped at total_llm_calls. Route overlap is reported
separately as operator_candidate_route_sum_events. No market claim is allowed:
answer_evidence has 0 shadow accepts, 0 verified CPU eligible accepts, and
missing provider cost on all 9 hook-ready rows.
```

Structural gate:

```text
packet:
  docs/structural_gates/answer-evidence-output-evidence-hook-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/answer-evidence-output-evidence-hook-v1.nanda.txt

verdict: PASS
complexity_score: 22
note: narrow route-swap check only: verification-hook-ready telemetry is not
      local accept, verified CPU saving, or market claim.
```

## 2026-07-04 - Executor Integration: Answer-Evidence Disabled Profile V1

Verdict:

```text
ANSWER_EVIDENCE_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
ANSWER_EVIDENCE_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED
REAL_TRAFFIC_SHADOW_V1_REVIEW
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-answer-evidence-profile-v1

The answer_or_explain grounded-evidence subset now has a disabled-threshold
.nwrb profile and registry overlay. The profile is generated from request-side
dry-run payloads only. It writes no raw prompt text, uses no response text, no
target labels, and no proof labels. The profile threshold is i32::MAX, so it
can produce score/margin telemetry but cannot locally accept.
```

Payload dry-run:

```text
trace:
  target/nando-wave/real-traffic-shadow/answer-evidence-payload-dry-run-v1.trace.jsonl

report:
  target/nando-wave/real-traffic-shadow/answer-evidence-payload-dry-run-v1.report.json

answer_evidence_candidate_events: 216
payload_ready_events: 17
payload_built_events: 9
scoreable_payload_events: 9
profile_registered: false
shadow_score_ready: false
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Disabled profile:

```text
package:
  target/nando-wave/real-traffic-shadow/answer-evidence-seed0.nwrb

registry:
  target/nando-wave/real-traffic-shadow/profile-registry-answer-evidence-v1.json

report:
  target/nando-wave/real-traffic-shadow/answer-evidence-profile-v1.report.json

profile_id: route_gap_answer_evidence_profile_v1
package_bytes: 116
edge_count: 6
runtime_bytes_estimate: 32944
threshold: 2147483647
trace_rows_read: 1000
scoreable_payload_events: 9
package_training_requests: 9
positive_updates: 648
negative_updates: 648
changed_edges: 130
positive_margin_rows: 9
strict_ordered_pass_rows: 9
unexpected_local_accepts_under_disabled_threshold: 0
median_energy_margin: 1123328
p10_energy_margin: 1106944
min_energy_margin: 1106944
median_min_slot_margin: 361472
p10_min_slot_margin: 361472
min_slot_margin: 361472
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled_on_real_traffic: false
market_claim_allowed: false
```

Shadow:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/answer-evidence-profile-v1.shadow-report.json

total_requests: 1000
total_llm_calls: 1000
exact_cache_hits: 53
nando_shadow_accepts: 0
verified_safe_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 233725
```

Catalog:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

answer_or_explain:
  priority_rank: 1
  candidate_events: 216
  payload_ready_events: 17
  scoreable_payload_events: 9
  recommended_payload_builder: answer_evidence_payload_builder_v1
  recommended_verifier: grounded_answer_evidence_verifier_v1
  next_action: run shadow/audit next, but keep local accepts disabled until
               grounded_answer_evidence_verifier_v1 proves false_accepts=0.
```

Decision:

```text
This closes the "profile missing" gap for the narrow grounded-evidence subset
of answer_or_explain. It does not create local accepts, verified CPU accepts,
or market savings. The next engineering step is grounded_answer_evidence_verifier_v1
over the disabled-profile shadow trace.
```

Claim boundary:

```text
No new verified CPU accepts were created. CPU Routability 80 remains open:
26/1000 unique verified accepts, gap 774.
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/answer-evidence-disabled-profile-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/answer-evidence-disabled-profile-v1.nanda.txt

verdict: PASS
complexity_score: 31
note: narrow role-swap check only: disabled answer-evidence profile ready is not
      a verifier, local accept, verified CPU saving, or market claim.
```

## 2026-07-04 - Executor Integration: Answer-Evidence Payload Dry-Run V1

Verdict:

```text
ROUTE_GAP_PAYLOAD_READINESS_V1_REVIEW_READY_FAMILIES_FOUND
ANSWER_EVIDENCE_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-answer-evidence-payload-dry-run-v1

The answer_or_explain route-gap family now has a request-side grounded-evidence
payload dry-run. It builds active_fringe/slot/impulse payloads only from local
Codex prompt-side signals and writes fingerprints/counters only. It does not
use response text, target labels, proof labels, or raw prompt text as runtime
authority. Local accepts remain disabled.
```

Route-gap readiness:

```text
report:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1.report.json

sampled_llm_calls: 1000
existing_route_candidate_events: 334
no_candidate_events: 666
payload_ready_events: 122
top_payload_ready_family: metrics_report_readout

answer_or_explain:
  candidate_events: 216
  payload_ready_events: 17
  payload_ready_rate_milli: 78
  request_signal_events: 198
  context_signal_events: 19
  evidence_signal_events: 22
  verifier_signal_events: 22
  readiness: low_needs_knowledge_evidence
```

Answer-evidence dry-run:

```text
trace:
  target/nando-wave/real-traffic-shadow/answer-evidence-payload-dry-run-v1.trace.jsonl

report:
  target/nando-wave/real-traffic-shadow/answer-evidence-payload-dry-run-v1.report.json

answer_evidence_candidate_events: 216
payload_ready_events: 17
payload_built_events: 9
scoreable_payload_events: 9
builder_rejected_events: 8
readiness_rejected_events: 199
profile_registered: false
shadow_score_ready: false
active_fringe_centers_total: 1130
slots_total: 27
positive_impulses_total: 648
negative_impulses_total: 648
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Feedback/catalog:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

total_llm_calls: 1000
operator_candidate_calls: 809
scoreable_candidate_calls: 136
verification_hook_ready_events: 108
verified_cpu_routability_milli: 32
unique_verified_cpu_accepts: 26
unique_verified_gap_to_80_calls: 774
incremental_unique_cpu_accepts: 25
incremental_unique_gap_to_80_calls: 775

catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

answer_or_explain catalog row:
  priority_rank: 1
  candidate_events: 216
  payload_ready_events: 17
  scoreable_payload_events: 9
  recommended_payload_builder: answer_evidence_payload_builder_v1
  recommended_verifier: grounded_answer_evidence_verifier_v1
  next_action: compile disabled-threshold answer-evidence .nwrb profile,
               then attach grounded verifier; broad answer_or_explain stays
               fallback-only.
```

Decision:

```text
This closes the "payload missing" gap for a narrow grounded-evidence subset of
answer_or_explain. It does not create local accepts, verified CPU accepts, or
market savings. The next engineering step is an answer-evidence disabled
profile plus grounded_answer_evidence_verifier_v1.
```

Claim boundary:

```text
No new verified CPU accepts were created. CPU Routability 80 remains open:
26/1000 unique verified accepts, gap 774.
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/answer-evidence-payload-dry-run-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/answer-evidence-payload-dry-run-v1.nanda.txt

verdict: PASS
complexity_score: 28
note: narrow role-swap check only: scoreable answer-evidence payloads are not a
      registered profile, verification hook, local accept, verified CPU saving,
      or market claim.
```

## 2026-07-04 - Executor Integration: Project-Context Workspace Evidence Hook V1

Verdict:

```text
PROJECT_CONTEXT_OUTPUT_EVIDENCE_V1_REVIEW_WORKSPACE_EVIDENCE_TRUE_LABELS_FOUND
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
PROJECT_CONTEXT_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SAFE_POLICY_CANDIDATE_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-project-context-output-evidence-v1
  role-binding-real-traffic-project-context-local-accept-calibration-v1

The project_context scoreable rows now have a deterministic workspace artifact
or goal-state evidence hook. The hook reads Codex session tool-call evidence and
writes only fingerprints/counters. It does not write raw prompt text, raw
response text, or raw tool outputs. Local accepts stay disabled unless a later
request-side admission path has enough verifier-true support.
```

Output evidence:

```text
report:
  target/nando-wave/real-traffic-shadow/project-context-output-evidence-v1.report.json

trace:
  target/nando-wave/real-traffic-shadow/project-context-output-evidence-v1.trace.jsonl

artifact_evidence_matched_events: 2
verified_true_events: 1
verified_false_events: 1
tool_call_fingerprint_events: 1
raw_prompt_text_written: false
raw_response_text_written: false
tool_outputs_written: false
market_claim_allowed: false
```

Shadow/audit:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/project-context-output-evidence-v1.shadow-report.json

verification_audit:
  target/nando-wave/real-traffic-shadow/project-context-output-evidence-v1.verification-hook-audit.report.json

total_llm_calls: 1000
operator_candidate_calls: 2
scoreable_candidate_calls: 2
nando_shadow_accepts: 0
verified_safe_accepts: 0
false_accepts: 0
p99_shadow_score_latency_ns: 204714
verification_hook_ready_events: 2
verified_cpu_accept_eligible_events: 0
candidates_missing_output_evidence: 0
candidates_missing_provider_cost: 2
market_claim_allowed: false
```

Local-accept calibration:

```text
report:
  target/nando-wave/real-traffic-shadow/project-context-local-accept-calibration-v1.report.json

hook_ready_rows: 2
label_true_rows: 1
label_false_rows: 1
no_score_rows: 0
safe_policy_found: true
best_safe_true_accepts: 1
minimum_true_support_required_by_feedback: 3
support_qualified: false
market_claim_allowed: false
```

Feedback/catalog:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

overall:
  total_llm_calls:                 1000
  operator_candidate_calls:         809
  scoreable_candidate_calls:        136
  verification_hook_ready_events:   108
  verified_cpu_accept_eligible_events: 32
  verified_cpu_routability_milli:    32
  unique_verified_cpu_accepts:       26
  unique_verified_gap_to_80_calls:  774
  incremental_unique_cpu_accepts:    25
  incremental_unique_gap_to_80_calls: 775

project_context_dialogue:
  candidate_events:                    211
  payload_ready_events:                  2
  scoreable_payload_events:              2
  verification_hook_ready_events:        2
  local_accept_calibration_ran:       true
  local_accept_safe_policy_found:     true
  local_accept_minimum_true_support:     3
  local_accept_support_qualified:    false
  local_accept_best_safe_true_accepts:   1
  verified_cpu_accept_eligible_events:   0
  false_accepts:                         0
  stage: local_accept_calibration_support_insufficient
  next_action: Collect more verifier-true rows or raise admission quality
               before promotion; low-support safe policies stay review-only.
```

Catalog:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

top_catalog_row:
  answer_or_explain (route_gap_family)

project_context_dialogue existing_profile_route:
  priority_rank:                 9
  verification_hook_ready:       2
  local_accept_support_insufficient: true
  verified_cpu_accepts:          0
  recommended_verifier:          workspace_artifact_or_goal_state_verifier_v1
  market_claim_allowed:          false
```

Decision:

```text
This closes the verification-hook-missing gap for the narrow project_context
scoreable subset. It does not create verified CPU accepts because shadow accepts
remain zero and the calibration has only one verifier-true support row.

project_context now moves from:
  scoreable_payload_missing_verification_hook

to:
  local_accept_calibration_support_insufficient
```

Claim boundary:

```text
No new verified CPU accepts were created. CPU Routability 80 remains open:
26/1000 unique verified accepts, gap 774.
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/project-context-workspace-evidence-hook-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/project-context-workspace-evidence-hook-v1.nanda.txt

verdict: PASS
complexity_score: 36
note: narrow role-swap check only: project-context workspace evidence hook
      ready != local accept, verified CPU savings, or market claim.
```

## 2026-07-04 - Executor Integration: Project-Context Disabled Profile V1

Verdict:

```text
PROJECT_CONTEXT_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED
PROJECT_CONTEXT_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PROFILE_READY
VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-project-context-profile-v1

The command compiles the artifact-backed project_context dry-run payloads into
a disabled-threshold .nwrb profile and writes a profile-registry overlay. The
threshold is i32::MAX, so shadow scoring is measurable but local accepts remain
impossible until workspace_artifact_or_goal_state_verifier_v1 exists.
```

Fresh profile:

```text
report:
  target/nando-wave/real-traffic-shadow/project-context-profile-v1.report.json

package:
  target/nando-wave/real-traffic-shadow/project-context-seed0.nwrb

registry:
  target/nando-wave/real-traffic-shadow/profile-registry-project-context-v1.json

scoreable_payload_events: 2
package_training_requests: 2
edge_count: 7
package_bytes: 128
runtime_bytes_estimate: 32972
threshold: 2147483647
positive_margin_rows: 2
strict_ordered_pass_rows: 2
median_energy_margin: 1155072
unexpected_local_accepts_under_disabled_threshold: 0
local_accepts_enabled_on_real_traffic: false
market_claim_allowed: false
```

Dry-run after registry overlay:

```text
report:
  target/nando-wave/real-traffic-shadow/project-context-payload-dry-run-v1.report.json

project_context_candidate_events: 211
payload_ready_events: 2
payload_built_events: 2
scoreable_payload_events: 2
profile_registered: true
shadow_score_ready: true
local_accepts_enabled: false
market_claim_allowed: false
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
```

Shadow/audit:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/project-context-profile-v1.shadow-report.json

verification_audit:
  target/nando-wave/real-traffic-shadow/project-context-profile-v1.verification-hook-audit.report.json

operator_candidate_calls: 2
scoreable_candidate_calls: 2
nando_shadow_accepts: 0
verified_safe_accepts: 0
false_accepts: 0
p99_shadow_score_latency_ns: 195244
verification_hook_ready_events: 0
market_claim_allowed: false
```

Feedback/catalog:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

project_context_dialogue:
  priority_rank:                       7
  stage:                               scoreable_payload_missing_verification_hook
  candidate_events:                    211
  payload_ready_events:                2
  scoreable_payload_events:            2
  verification_hook_ready_events:      0
  verified_cpu_accept_eligible_events: 0
  false_accepts:                       0
  next_action:                         Attach response/tool-call evidence and
                                       deterministic output verification.

overall:
  operator_candidate_calls:         809
  scoreable_candidate_calls:        136
  unique_verified_cpu_accepts:       26
  unique_verified_gap_to_80_calls:  774
```

Catalog:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

project_context_dialogue existing_profile_route:
  priority_rank:              1
  candidate_events:           211
  payload_ready_events:       2
  scoreable_payload_events:   2
  verification_hook_ready:    0
  verified_cpu_accepts:       0
  recommended_verifier:       workspace_artifact_or_goal_state_verifier_v1

project_context_dialogue route_gap_family:
  priority_rank: 12
  next_action:   project_context_dialogue already has 2 scoreable dry-run
                 payloads in feedback-loop; continue with
                 workspace_artifact_or_goal_state_verifier_v1 and keep local
                 accepts disabled until deterministic verification exists.
```

Decision:

```text
This closes the profile-missing gap for the narrow artifact-backed subset of
project_context. It deliberately does not promote broad project dialogue:
209 of 211 candidates remain fallback-only until explicit workspace artifact
or goal-state evidence exists. The current result is a route-prioritization
instrument, not a market savings claim.
```

Claim boundary:

```text
No new verified CPU accepts were created. CPU Routability 80 remains open:
26/1000 unique verified accepts, gap 774.
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/project-context-disabled-profile-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/project-context-disabled-profile-v1.nanda.txt

verdict: PASS
complexity_score: 42
note: narrow role-swap check only: disabled project-context profile /
      artifact-backed scoreable path != verified CPU accept.
```

## 2026-07-04 - Executor Integration: Style-Brevity Disabled Profile V1

Verdict:

```text
STYLE_BREVITY_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED
STYLE_BREVITY_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PROFILE_READY
VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-style-brevity-profile-v1

The command compiles the existing request-side style_brevity dry-run payloads
into a disabled-threshold .nwrb profile and writes a profile-registry overlay.
The threshold is i32::MAX, so serving/shadow can measure score/margins but
cannot local-accept real traffic.
```

Fresh profile:

```text
report:
  target/nando-wave/real-traffic-shadow/style-brevity-profile-v1.report.json

package:
  target/nando-wave/real-traffic-shadow/style-brevity-seed0.nwrb

registry:
  target/nando-wave/real-traffic-shadow/profile-registry-style-brevity-v1.json

scoreable_payload_events: 3
package_training_requests: 3
edge_count: 6
package_bytes: 116
runtime_bytes_estimate: 32944
threshold: 2147483647
positive_margin_rows: 3
strict_ordered_pass_rows: 3
median_energy_margin: 1196032
unexpected_local_accepts_under_disabled_threshold: 0
local_accepts_enabled_on_real_traffic: false
market_claim_allowed: false
```

Dry-run after registry overlay:

```text
report:
  target/nando-wave/real-traffic-shadow/style-brevity-payload-dry-run-v1.report.json

style_brevity_candidate_events: 3
payload_ready_events: 3
payload_built_events: 3
scoreable_payload_events: 3
profile_registered: true
shadow_score_ready: true
local_accepts_enabled: false
market_claim_allowed: false
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
```

Shadow/audit:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/style-brevity-profile-v1.shadow-report.json

verification_audit:
  target/nando-wave/real-traffic-shadow/style-brevity-profile-v1.verification-hook-audit.report.json

operator_candidate_calls: 3
nando_shadow_accepts: 0
verified_safe_accepts: 0
false_accepts: 0
p99_shadow_score_latency_ns: 319154
verification_hook_ready_events: 0
market_claim_allowed: false
```

Feedback/catalog:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

style_brevity:
  priority_rank:                       7
  stage:                               scoreable_payload_missing_verification_hook
  candidate_events:                    3
  scoreable_payload_events:            3
  verification_hook_ready_events:      0
  verified_cpu_accept_eligible_events: 0
  false_accepts:                       0
  next_action:                         Attach response/tool-call evidence and
                                       deterministic output verification.

overall:
  unique_verified_cpu_accepts:      26
  unique_verified_gap_to_80_calls: 774
```

Catalog:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

style_brevity existing_profile_route:
  priority_rank:              5
  candidate_events:           3
  scoreable_payload_events:   3
  verification_hook_ready:    0
  verified_cpu_accepts:       0
  recommended_verifier:       route_specific_deterministic_verifier_required

style_brevity route_gap_family:
  priority_rank: 15
  next_action:   style_brevity already has 3 scoreable dry-run payloads in
                 feedback-loop; continue with
                 response_length_and_format_verifier_v1.
```

Decision:

```text
This closes the profile-missing gap for style_brevity without creating fake
savings. The route now has scoreable payloads, a package, registry overlay,
shadow-score readiness, and a missing-hook audit. It still cannot count toward
CPU Routability until response_length_and_format_verifier_v1, calibration,
provider-cost evidence, and false_accepts=0 are all present.
```

Claim boundary:

```text
No new verified CPU accepts were created. CPU Routability 80 remains open:
26/1000 unique verified accepts, gap 774.
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/style-brevity-disabled-profile-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/style-brevity-disabled-profile-v1.nanda.txt

verdict: PASS
complexity_score: 31
note: first broader packet was VETO because it mixed profile, shadow, audit,
      catalog, and CPU80 summary too widely; repaired to the narrow role-swap
      check: disabled profile / scoreable path != verified CPU accept.
```

## 2026-07-04 - Executor Integration: Style-Brevity Payload Dry-Run V1

Verdict:

```text
STYLE_BREVITY_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW_STYLE_BREVITY_SCOREABLE_NO_ACCEPTS
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added a narrow request-side style_brevity payload dry-run:
  role-binding-real-traffic-style-brevity-payload-dry-run-v1

The route builds active_fringe/slots only for pure brevity/style-control prompts.
It writes fingerprints/counts only, no raw prompt text, no response text, no
target labels, no proof labels, and local accepts stay disabled.

Feedback-loop now auto-loads the default style-brevity dry-run artifact and
adds a route row when the route is scoreable but lacks deterministic output
verification.

CPU operator catalog now downranks the duplicate raw route-gap builder row when
a matching scoreable feedback row already exists. This prevents looping on
"build builder" after the builder is already present.
```

Fresh style-brevity dry-run:

```text
report:
  target/nando-wave/real-traffic-shadow/style-brevity-payload-dry-run-v1.report.json

candidate_events:         3
payload_ready_events:     3
payload_built_events:     3
scoreable_payload_events: 3
profile_registered:       false
local_accepts_enabled:    false
market_claim_allowed:     false
raw_text_written:         false
response_text_used:       false
target_labels_used:       false
proof_labels_used:        false
```

Feedback-loop row:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

style_brevity:
  stage:                               scoreable_payload_missing_verification_hook
  candidate_events:                    3
  scoreable_payload_events:            3
  verification_hook_ready_events:      0
  verified_cpu_accept_eligible_events: 0
  false_accepts:                       0
  next_action:                         Attach response/tool-call evidence and
                                       deterministic output verification.

overall:
  operator_candidate_calls:             598
  scoreable_candidate_calls:            134
  verified_cpu_accept_eligible_events:   32
  unique_verified_cpu_accepts:           26
  unique_verified_gap_to_80_calls:      774
```

Catalog:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

style_brevity scoreable row:
  priority_rank:             5
  source_kind:               existing_profile_route
  candidate_events:          3
  scoreable_payload_events:  3
  next_action:               Attach response/tool-call evidence and deterministic output verification.

style_brevity route-gap family row:
  priority_rank: 15
  next_action:   style_brevity already has 3 scoreable dry-run payloads in
                 feedback-loop; continue with response_length_and_format_verifier_v1.
```

Decision:

```text
This is useful progress but not savings. The style-brevity route has crossed
the request-side payload barrier and is now visible in the CPU80 feedback
ladder, but it cannot count as CPU savings until a disabled-threshold profile,
response_length_and_format_verifier_v1, shadow/audit, safe-policy calibration,
provider-cost evidence, and false_accepts=0 are all present.
```

Claim boundary:

```text
No new verified CPU accepts were created. CPU Routability 80 remains open:
26/1000 unique verified accepts, gap 774.
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/style-brevity-payload-dry-run-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/style-brevity-payload-dry-run-v1.nanda.txt

verdict: PASS
complexity_score: 32
note: first broader packet was VETO because candidate triads were too abstract
      and mixed route/report roles; repaired to the narrow role-swap check:
      scoreable_payload_events != verified_cpu_accept_eligible_events.
```

## 2026-07-04 - Executor Integration: Local-Accept Calibration Anti-Loop V1

Verdict:

```text
CPU_OPERATOR_CATALOG_V1_REVIEW_LOCAL_ACCEPT_FAILURES_AND_LOW_SUPPORT_DOWNRANKED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

The CPU operator catalog now distinguishes two non-promotable local-accept
states:

  local_accept_calibration_failed:
    calibration ran, but no zero-false safe policy exists.

  local_accept_support_insufficient:
    a zero-false policy exists, but true support is below the minimum support
    gate and must stay review-only.

Both states now receive priority penalties. This prevents the catalog from
looping on routes whose current score geometry cannot produce robust verified
CPU savings.
```

Fresh read-inspect calibration:

```text
calibration_report:
  target/nando-wave/real-traffic-shadow/read-inspect-local-accept-calibration-v1.report.json

hook_ready_rows:       9
label_true_rows:      1
label_false_rows:     8
safe_policy_found: false
best_safe_true_accepts: 0

margin collision:
  energy_margin false rows at/above true row:      7
  min_slot_margin false rows at/above true row:    7
  path_slot_margin false rows at/above true row:   7
  request_slot_margin false rows at/above true row: 8
```

Fresh catalog:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

current_verified_cpu_accepts: 26
verified_gap_to_80_calls:    774

penalized rows:
  read_inspect:
    rank: 15
    local_accept_calibration_failed: true
    next_action: improve request-side admission, verifier evidence, or payload
                 geometry before another promote.

  planning_next_step:
    rank: 11
    local_accept_support_insufficient: true
    best safe support: 1 true row

  retrieval_lookup:
    rank: 13
    local_accept_support_insufficient: true
```

Decision:

```text
Do not lower thresholds for read_inspect. Current margins do not separate
verifier-true from verifier-false rows.

Do not promote singleton/low-support planning_next_step or retrieval_lookup
policies as product savings. They need more verifier-true support or better
request-side admission before promotion.

The next actionable work shifts to route-gap families that need real builders
and deterministic verifiers instead of more threshold passes:
  agent_continue_execute
  retrieval_lookup route-gap
  short_decision_ack
  style_brevity
```

Claim boundary:

```text
This is route-selection instrumentation only. It adds no verified CPU accepts.
CPU Routability 80 remains at 26/1000 unique verified accepts, with
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/local-accept-calibration-anti-loop-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/local-accept-calibration-anti-loop-v1.nanda.txt

verdict: PASS
complexity_score: 39
note: first packet was VETO because a generic catalog evidence label was reused
      across unrelated metric fillers; repaired with route-specific evidence
      labels.
```

## 2026-07-04 - Executor Integration: Route-Gap Exhaustion Alignment V1

Verdict:

```text
CPU_OPERATOR_CATALOG_V1_REVIEW_EXHAUSTED_GAP_FAMILIES_RERANKED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

Route-gap payload readiness now uses the same base profile registry as the
route-gap catalog by default. This prevents the readiness pass from hiding
base-registry no-candidate families after later overlay profiles are generated.

The readiness analyzer now treats clean agent_control_stop prompts as
request-side payload-ready control surfaces, but the CPU operator catalog
also carries exhaustion signals from existing safe-policy audits into matching
route-gap families. Exhausted stop, metrics, git, and serving gap rows are
downranked instead of being rediscovered as fresh work.
```

Fresh route-gap readiness:

```text
readiness_report:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1.report.json

sampled_llm_calls:              1000
existing_route_candidate_events:  334
no_candidate_events:              666
payload_ready_events:             105
top_payload_ready_family:         metrics_report_readout

agent_control_stop:
  candidate_events:     36
  payload_ready_events: 31
  payload_ready_rate:   861 milli
```

Fresh catalog:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

current_verified_cpu_accepts: 26
verified_gap_to_80_calls:    774
top_catalog_row:             read_inspect (existing_profile_route)

exhausted gap families now downranked:
  metrics_report_readout route_gap_family: rank 21
  agent_control_stop     route_gap_family: rank 22
  serving_ops            route_gap_family: rank 23
  git_control            route_gap_family: rank 24
```

Decision:

```text
Do not repeat threshold-only promotes for agent_control_stop, metrics-report,
git-control, or serving-ops from the current request geometry. Their safe
support is already covered by current incremental unique accepts.

The next highest-priority row is now:
  read_inspect existing_profile_route

Recommended next action:
  improve read_inspect request-side admission or payload geometry, then attach
  a deterministic read-only path/excerpt verifier before any local accept.
```

Claim boundary:

```text
This is route-selection instrumentation only. It adds no verified CPU accepts.
CPU Routability 80 remains at 26/1000 unique verified accepts, with
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/route-gap-exhaustion-alignment-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/route-gap-exhaustion-alignment-v1.nanda.txt

verdict: PASS
complexity_score: 28
note: narrow packet checks the risky role swap only:
      agent_control_stop payload-ready does not imply a fresh savings route
      while existing strict stop support is exhausted.
```

## 2026-07-04 - Executor Integration: Catalog Exhaustion Rerank V1

Verdict:

```text
CPU_OPERATOR_CATALOG_V1_REVIEW_EXHAUSTED_ROUTES_RERANKED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

The CPU operator catalog now treats exhausted safe-policy support as a first
class routing signal. Metrics-report, serving-ops, and edit existing-profile
routes are downranked when their best safe calibration support is already
covered by current incremental unique verified accepts. The exhaustion penalty
was centralized as CPU_OPERATOR_EXHAUSTED_SUPPORT_PRIORITY_PENALTY.
```

Fresh catalog:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

current_verified_cpu_accepts: 26
verified_gap_to_80_calls:    774
top_catalog_row:             planning_next_step (route_gap_family)

metrics_report_readout existing route:
  rank:                                  26
  incremental_unique_accepts:             3
  metrics_report_best_robust_true_accepts: 3
  metrics_report_current_support_exhausted: true

serving_ops existing route:
  serving_ops_best_safe_true_accepts: 3
  serving_ops_current_support_exhausted: true

edit existing route:
  edit_best_safe_true_accepts: 1
  edit_current_support_exhausted: true
```

Decision:

```text
Do not spend the next CPU80 step on another threshold-only pass for
metrics-report, serving-ops, or edit. Their current request/score geometry has
already captured its safe support in the 1000-call window.

The next highest-priority route is now:
  planning_next_step route_gap_family

Recommended next action:
  build or improve goal_state_transition_payload_builder_v1
  plus plan_step_artifact_progress_verifier_v1
  while keeping local accepts disabled until deterministic verification exists.
```

Claim boundary:

```text
This is route-selection instrumentation only. It adds no verified CPU accepts.
CPU Routability 80 remains at 26/1000 unique verified accepts, with
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/catalog-exhaustion-rerank-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/catalog-exhaustion-rerank-v1.nanda.txt

verdict: PASS
complexity_score: 34
note: route-specific evidence labels are required; the first packet reused
      generic incremental-unique evidence labels across routes and NANDA
      correctly rejected that as a role-filler conflict
```

## 2026-07-04 - Executor Integration: Mixed Safe Policy V3

Verdict:

```text
MIXED_ADMISSION_AUDIT_V1_REVIEW_SAFE_REQUEST_SUBFAMILY_FOUND
MIXED_SAFE_POLICY_PROMOTE_V3_REVIEW_PROMOTED_TRACE_READY
REAL_TRAFFIC_SHADOW_V1_PASS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-mixed-admission-audit-v1
  role-binding-real-traffic-mixed-safe-policy-promote-v3

The admission audit scores mixed-map request-side feature conjunctions plus
energy thresholds against real verification labels. The v3 promoter reads the
best safe audit candidate, rewrites a promoted trace, and keeps runtime
authority limited to prompt-side features plus .nwrb score threshold. It writes
no raw prompt/response text and does not use target/proof labels as serving
authority.
```

Fresh mixed admission audit:

```text
audit_report:
  target/nando-wave/real-traffic-shadow/mixed-admission-audit-v1.report.json

scoreable_candidate_rows: 14
hook_ready_rows:         13
label_true_rows:         10
label_false_rows:         3
unverified_rows:          1
safe_policy_found:     true
best_safe_true_accepts:   7

best safe candidate:
  no_question_mark AND no_goal_terms AND energy >= 204800
  accepts:            7
  true_accepts:       7
  false_accepts:      0
  unverified_accepts: 0
```

Fresh v3 promotion + shadow:

```text
promote_report:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v3.report.json

promoted_registry:
  target/nando-wave/real-traffic-shadow/profile-registry-mixed-safe-policy-v3.json

promoted_trace:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v3.trace.jsonl

request_side_policy_accept_rows:      9
policy_accept_rows:                   7
policy_accept_verified_true_rows:     7
policy_accept_verified_false_rows:    0
policy_accept_unverified_rows:        0

shadow_report:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v3.shadow-report.json

total_llm_calls:                       1000
exact_cache_hits:                        53
nando_shadow_accepts:                     7
verified_safe_accepts:                    7
false_accepts:                            0
incremental_reduction_vs_exact_cache:     7 milli
p99_shadow_score_latency_ns:         220096

verification_hook_audit:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v3.verification-hook-audit.report.json

operator_candidate_calls:        9
scoreable_candidate_calls:       9
verification_hook_ready_events:  9
verified_cpu_accept_eligible:    7
market_claim_allowed:         true
```

Fresh global CPU80 feedback:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

total_llm_calls:                                      1000
operator_candidate_calls:                              595
scoreable_candidate_calls:                             131
verification_hook_ready_events:                        106
verified_cpu_accept_unique_request_fingerprints:        26
incremental_cpu_accept_unique_request_fingerprints:     25
unique_verified_gap_to_80_calls:                       774
market_claim_allowed:                                false
```

Fresh catalog after mixed v3:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

current_verified_cpu_accepts: 26
verified_gap_to_80_calls:    774
top_catalog_row:             metrics_report_readout

mixed row:
  rank:                                  15
  verified_cpu_accept_eligible_events:    7
  incremental_unique_accepts:             7
  false_accepts:                          0
  mixed_admission_best_safe_true_accepts: 7
  mixed_current_support_exhausted:     true
```

Decision:

```text
Mixed v3 is a real improvement over v2 for the mixed-map route:
  previous promoted mixed accepts: 3
  v3 promoted mixed accepts:       7

The current request-side mixed support is now exhausted. Another mixed-map
threshold/policy pass is not the next move; the next mixed work must improve
payload/evidence geometry or split a new subfamily.

Global CPU Routability 80 is still not achieved:
  unique verified accepts: 26 / 1000
  unique gap to 80%:      774 calls
  market_claim_allowed:   false
```

Structural gate:

```text
packet:
  docs/structural_gates/mixed-safe-policy-v3.md

gate_report:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v3.nanda.txt

verdict: PASS
complexity_score: 25
note: route-local metric-value packet used; the broader global market claim
      boundary stays in the feedback/catalog reports instead of being merged
      into this structural route
```

## 2026-07-04 - Executor Integration: Git-Control Admission Audit V1

Verdict:

```text
GIT_CONTROL_ADMISSION_AUDIT_V1_REVIEW_SAFE_REQUEST_SUBFAMILY_FOUND
CPU_OPERATOR_CATALOG_V1_REVIEW_GIT_CONTROL_SUPPORT_EXHAUSTED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-git-control-admission-audit-v1

The audit scores git-control request-side feature conjunctions plus energy
thresholds against real verification labels. Unverified rows are treated as
unsafe for promotion. It writes no raw prompt/response text, enables no local
accepts, and cannot count as savings.
```

Fresh git-control admission audit:

```text
audit_report:
  target/nando-wave/real-traffic-shadow/git-control-admission-audit-v1.report.json

scoreable_candidate_rows: 12
hook_ready_rows:         10
label_true_rows:          6
label_false_rows:         4
unverified_rows:          2
safe_policy_found:     true
best_safe_true_accepts:   4

best safe candidate:
  has_digit AND energy >= 1190912
  accepts:            4
  true_accepts:       4
  false_accepts:      0
  unverified_accepts: 0
```

Decision:

```text
Do not promote another git-control policy from the current request-side
features. The audit found only 4 market-safe accepts, already covered by the
current git-control v2 incremental unique support.

The apparent calibration support of 5 is not market-safe once request-side
admission and unverified rows are included.
```

Fresh catalog after git-control exhaustion awareness:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

top_catalog_row: role_binding_mixed_map_seed0

git_control row:
  rank: 26
  incremental_unique: 4
  git_control_admission_best_safe_true_accepts: 4
  git_control_current_support_exhausted: true
```

Claim boundary:

```text
This is route-selection instrumentation only. It adds no verified CPU accepts.
CPU Routability 80 remains at 22/1000 unique verified accepts, with
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/git-control-admission-audit-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/git-control-admission-audit-v1.nanda.json

verdict: PASS
complexity_score: 27
note: minimal metric-value packet used to check the rerank claim only
```

## 2026-07-04 - Executor Integration: Agent-Control Admission Audit V1

Verdict:

```text
AGENT_CONTROL_ADMISSION_AUDIT_V1_REVIEW_CURRENT_POLICY_SUPPORT_EXHAUSTED
CPU_OPERATOR_CATALOG_V1_REVIEW_AGENT_CONTROL_SUPPORT_EXHAUSTED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-agent-control-admission-audit-v1

The audit joins the existing agent-control admission calibration summary with
feedback-loop unique attribution. It writes no raw prompt/response text, enables
no local accepts, and cannot count as savings.
```

Fresh agent-control admission audit:

```text
audit_report:
  target/nando-wave/real-traffic-shadow/agent-control-admission-audit-v1.report.json

candidate_events:                         143
scoreable_payload_events:                  11
verification_hook_ready_events:            11
label_true_rows:                           12
label_false_rows:                         112
best_robust_true_accepts:                  11
robust_remaining_true_rows:                 1
verified_cpu_accept_eligible_events:       11
unique_verified_request_fingerprints:       6
incremental_verified_request_fingerprints:  5
exact_cache_overlap:                        1
duplicate_verified_route_hits:              5

current_policy_event_support_exhausted: true
unique_contribution_constrained:         true
```

Decision:

```text
Do not keep ranking agent_control first from the current stop/control policy.
The robust request-side policy has already captured its current event support,
and unique contribution is constrained by duplicates and exact-cache overlap.

Next agent-control work must split broader agent-state/tool-state subfamilies
or add stronger request-side evidence before another promote.
```

Fresh catalog after agent-control exhaustion awareness:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

top_catalog_row: git_control

agent_control row:
  current_policy_event_support_exhausted: true
  unique_contribution_constrained: true
  best_robust_true_accepts: 11
```

Claim boundary:

```text
This is route-selection instrumentation only. It adds no verified CPU accepts.
CPU Routability 80 remains at 22/1000 unique verified accepts, with
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/agent-control-admission-audit-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/agent-control-admission-audit-v1.nanda.json

verdict: PASS
complexity_score: 36
note: metric-value packet form used; evidence labels were made field-specific
      after NANDA rejected generic attribution labels as incompatible fillers
```

## 2026-07-04 - Executor Integration: Conditional Admission Audit V1

Verdict:

```text
CONDITIONAL_ADMISSION_AUDIT_V1_REVIEW_SAFE_SUBFAMILY_CANDIDATE_FOUND
CPU_OPERATOR_CATALOG_V1_REVIEW_CONDITIONAL_SUPPORT_EXHAUSTED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-conditional-admission-audit-v1

The audit joins request fingerprints back to local Codex prompt text at
analysis time, writes no raw prompt/response text, and evaluates only:
  request-side feature conjunctions
  .nwrb energy thresholds
  deterministic verifier labels

It enables no local accepts and cannot be counted as savings.
```

Fresh conditional admission audit:

```text
audit_report:
  target/nando-wave/real-traffic-shadow/conditional-admission-audit-v1.report.json

hook_ready_rows:        63
label_true_rows:        17
label_false_rows:       46
safe_policy_found:    true
best_safe_true_accepts:  3

best safe candidate:
  has_digit AND has_gate_terms AND energy >= 0
  accepts:       3
  true_accepts:  3
  false_accepts: 0
  missed_true:  14
```

Decision:

```text
Do not promote another broad conditional policy from the current geometry.
The only safe request-side subfamily found by the audit has support=3, which
is already covered by the current conditional-safe-policy-v2 incremental
unique accepts.

The CPU operator catalog now auto-loads this audit and deprioritizes the
conditional row when:
  conditional_admission_best_safe_true_accepts
    <= incremental unique accepts already credited to the route.
```

Fresh catalog after conditional exhaustion awareness:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

top_catalog_row: role_binding_agent_control_seed0

conditional row:
  rank: 7
  route_sum: 3
  unique: 3
  incremental_unique: 3
  conditional_admission_best_safe_true_accepts: 3
  conditional_admission_current_support_exhausted: true
  next_action:
    improve conditional payload geometry or split a stronger subfamily
    before another promote
```

Claim boundary:

```text
This is route-selection instrumentation only. It adds no verified CPU accepts.
CPU Routability 80 remains at 22/1000 unique verified accepts, with
market_claim_allowed=false.
```

Structural gate:

```text
packet:
  docs/structural_gates/conditional-admission-audit-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/conditional-admission-audit-v1.nanda.json

verdict: PASS
complexity_score: 36
note: compact metric-value packet form used after the first larger packet VETOed
      the catalog-top-row binding as a weak composite
```

## 2026-07-04 - Executor Integration: CPU Operator Catalog Unique Priority V1

Verdict:

```text
CPU_OPERATOR_CATALOG_V1_REVIEW_UNIQUE_PRIORITY_INSTRUMENTED
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

The CPU operator catalog row now exposes:
  verified_cpu_accept_eligible_events          (route-sum diagnostic)
  verified_cpu_accept_unique_request_fingerprints
  incremental_cpu_accept_unique_request_fingerprints
  exact_cache_overlap_verified_cpu_accepts
  duplicate_verified_route_hits
  cross_route_overlap_verified_request_fingerprints
  priority_verified_accept_events

Existing profile rows are prioritized by incremental unique verified accepts,
not by route-sum duplicate accepts.
```

Fresh catalog after unique-priority rerank:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

total_llm_calls:                         1000
current_verified_cpu_accepts:              22
incremental_unique_cpu_accepts:            21
verified_gap_to_80_calls:                 778
top_catalog_row: role_binding_conditional_branch_seed0

top rows:
  role_binding_conditional_branch_seed0:
    route_sum: 3
    unique:    3
    incremental_unique: 3
    priority_verified_accept_events: 3

  role_binding_agent_control_seed0:
    route_sum: 11
    unique:     6
    incremental_unique: 5
    exact_cache_overlap: 1
    duplicate_route_hits: 5
    priority_verified_accept_events: 5

  git_control:
    route_sum: 4
    unique:    4
    incremental_unique: 4
    cross_route_overlap: 1
    priority_verified_accept_events: 4
```

Decision:

```text
Do not rank the next CPU80 route by route-sum accepts. Route-sum is diagnostic
only. Operator ranking must prefer unique/incremental contribution and must
surface exact-cache overlap and duplicate route hits.
```

Claim boundary:

```text
This improves route accounting and next-route selection only. It does not add
new verified CPU accepts and does not change market_claim_allowed=false.
CPU Routability 80 is still not achieved.
```

Structural gate:

```text
packet:
  docs/structural_gates/cpu-operator-catalog-unique-priority-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-unique-priority-v1.nanda.json

verdict: PASS
complexity_score: 35
note: compact metric-value packet form used after the first larger packet VETOed
      weak composite numeric bindings
```

## 2026-07-04 - Executor Integration: CPU Route Feedback Unique Contribution V1

Verdict:

```text
CPU_ROUTE_FEEDBACK_UNIQUE_CONTRIBUTION_V1_REVIEW_INSTRUMENTED
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
RETRIEVAL_LOOKUP_WINDOW5000_REVIEW_NO_SAFE_POLICY
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

Feedback-loop report now separates:
  route-sum verified accepts
  route-local unique request fingerprints
  global unique request fingerprints
  conservative incremental unique request fingerprints
  duplicate hits inside one route
  cross-route overlaps
```

Fresh feedback after uniqueness instrumentation:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

total_llm_calls:                                      1000
operator_candidate_calls:                              595
scoreable_candidate_calls:                             133
verification_hook_ready_events:                        108
route_sum_verified_cpu_eligible_hits:                   28
route_unique_sum_request_fingerprints:                  23
global_unique_verified_cpu_accepts:                     22
conservative_incremental_unique_cpu_accepts:            21
exact_cache_overlap_verified_cpu_accepts:                1
duplicate_within_route_verified_hits:                    5
cross_route_overlap_verified_request_fingerprints:       1
duplicate_route_hits_total:                              6
unique_verified_gap_to_80_calls:                       778
incremental_unique_gap_to_80_calls:                    779
```

Route-level unique contribution:

```text
conditional:
  route_sum: 3
  unique:    3
  cross_route_overlap: 1
  exclusive: 2

edit:
  route_sum: 1
  unique:    1
  exclusive: 1

mixed:
  route_sum: 3
  unique:    3
  exclusive: 3

metrics_report:
  route_sum: 3
  unique:    3
  exclusive: 3

agent_control:
  route_sum: 11
  unique:     6
  conservative_incremental_unique: 5
  exact_cache_overlap_unique:      1
  duplicate_hits_inside_route:     5

git_control:
  route_sum: 4
  unique:    4
  cross_route_overlap: 1
  exclusive: 3

serving_ops:
  route_sum: 3
  unique:    3
  exclusive: 3
```

Retrieval wider-window diagnostic:

```text
artifacts:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-window5000-payload-dry-run.report.json
  target/nando-wave/real-traffic-shadow/retrieval-lookup-window5000-output-evidence.report.json
  target/nando-wave/real-traffic-shadow/retrieval-lookup-window5000-local-accept-calibration.report.json

window:                         5000 real Codex events
retrieval_lookup_candidates:       91
scoreable_payload_events:           7
output_evidence_matched:            6
verified_true_events:               5
verified_false_events:              1
safe_policy_found:              false
```

Decision:

```text
Do not promote retrieval_lookup. The wider window found a false row at the
same energy scale as true rows, so retrieval remains red. This is useful
negative evidence, not a failure of the feedback loop.
```

Claim boundary:

```text
The route-sum number is diagnostic only. The current honest CPU80 state is
22/1000 global unique verified CPU accepts, or 21/1000 conservative
incremental unique accepts over exact cache. CPU Routability 80 is not
achieved and market_claim_allowed remains false.
```

Structural gate:

```text
packet:
  docs/structural_gates/cpu-route-feedback-unique-contribution-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-unique-contribution-v1.nanda.json

verdict: PASS
complexity_score: 39
note: metric-value packet form used for numeric report bindings
```

## 2026-07-04 - Executor Integration: Conditional Safe-Policy Promote V2

Verdict:

```text
CONDITIONAL_SAFE_POLICY_PROMOTE_V2_REVIEW_PROMOTED_TRACE_READY
REAL_TRAFFIC_SHADOW_V1_PASS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

Completed existing command path:
  role-binding-real-traffic-conditional-safe-policy-promote-v2
```

Fresh promotion artifact:

```text
report:
  target/nando-wave/real-traffic-shadow/conditional-safe-policy-v2.report.json

promoted_registry:
  target/nando-wave/real-traffic-shadow/profile-registry-conditional-safe-policy-v2.json

promoted_trace:
  target/nando-wave/real-traffic-shadow/conditional-safe-policy-v2.trace.jsonl

request_side_policy_name:
  conditional_gate_digit_terms

selected_acceptance_policy:          energy_nonnegative
selected_policy_threshold:           0
registry_runtime_threshold:          1
request_side_policy_accept_rows:     53
runtime_policy_accept_rows:           3
runtime_policy_verified_true_rows:    3
runtime_policy_verified_false_rows:   0
runtime_policy_unverified_rows:       0
provider_cost_events_written:        53
market_claim_allowed:             false
```

Fresh conditional v2 shadow:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/conditional-safe-policy-v2.shadow-report.json

total_llm_calls:                        1000
exact_cache_hits:                         53
nando_shadow_accepts:                      3
verified_safe_accepts:                     3
false_accepts:                             0
unverified_shadow_accepts:                 0
incremental_savings_over_exact_cache:      3
p99_shadow_score_latency_ns:          255062
synthetic_trace_used:                  false
```

Fresh conditional v2 verification audit:

```text
audit_report:
  target/nando-wave/real-traffic-shadow/conditional-safe-policy-v2.verification-hook-audit.report.json

operator_candidate_calls:                 53
scoreable_candidate_calls:                53
verification_hook_ready_events:           40
verified_cpu_accept_eligible_events:       3
shadow_false_accepts:                      0
market_claim_allowed:                   true
```

Fresh default feedback after conditional v2 audit wiring:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

route_sum_verified_cpu_eligible_hits:      28
unique_verified_cpu_accepts:               22 / 1000
verified_cpu_accept_unique_routability_milli: 22
unique_verified_gap_to_80_calls:          778
incremental_cpu_accept_unique_request_fingerprints: 22

conditional route row:
  stage: verified_cpu_accept_eligible
  candidate_events:                        166
  scoreable_payload_events:                 53
  verification_hook_ready_events:           40
  verified_cpu_accept_eligible_events:       3
  false_accepts:                             0
```

Claim boundary:

```text
This is a narrow route-local conditional v2 promoted trace PASS: 3 verified
safe accepts, 0 false accepts, and 0 unverified accepts. It completes the
previously unfinished conditional-safe-policy-promote-v2 integration.

This is not CPU Routability 80 and does not improve the unique global feedback
count: unique verified CPU accepts remain 22/1000. The route-sum count is 28,
but route-sum can contain overlapping request fingerprints and must not be used
as the market claim.

The policy is intentionally low-margin: serving admission is request-side
gate/digit terms plus energy_nonnegative. The .nwrb registry still keeps a
positive runtime threshold field for package loading, but the acceptance policy
uses the explicit nonnegative energy rule. Treat this as a small safe pocket,
not as a broad conditional operator proof.
```

## 2026-07-04 - Executor Integration: Git-Control Safe-Policy Promote V2

Verdict:

```text
GIT_CONTROL_SAFE_POLICY_PROMOTE_V2_REVIEW_PROMOTED_TRACE_READY
REAL_TRAFFIC_SHADOW_V1_PASS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-git-control-safe-policy-promote-v2

Added feedback wiring:
  git_control_safe_policy_v2_verification_audit_report_path
```

Implementation note:

```text
The current code diff also contains a conditional-safe-policy-promote-v2 command
and optional feedback wiring from the open working set. It is not counted in
this promoted result because no conditional v2 default audit artifact exists in
the feedback window. The current PASS claim is git_control v2 only.
```

Fresh promotion artifact:

```text
report:
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v2.report.json

promoted_registry:
  target/nando-wave/real-traffic-shadow/profile-registry-git-control-safe-policy-v2.json

promoted_trace:
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v2.trace.jsonl

request_side_policy_name:
  git_control_digit_count_ge_1

selected_acceptance_policy:          energy_threshold_only
selected_policy_threshold:           1190912
request_side_policy_accept_rows:          9
policy_accept_rows:                       4
policy_accept_verified_true_rows:         4
policy_accept_verified_false_rows:        0
policy_accept_unverified_rows:            0
provider_cost_events_written:             9
market_claim_allowed:                 false
```

Fresh git_control shadow:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v2.shadow-report.json

total_llm_calls:                        1000
exact_cache_hits:                         53
nando_shadow_accepts:                      4
verified_safe_accepts:                     4
false_accepts:                             0
unverified_shadow_accepts:                 0
incremental_savings_over_exact_cache:      4
p99_shadow_score_latency_ns:          207390
synthetic_trace_used:                  false
```

Fresh git_control verification audit:

```text
audit_report:
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v2.verification-hook-audit.report.json

operator_candidate_calls:                  9
scoreable_candidate_calls:                 9
verification_hook_ready_events:            9
verified_cpu_accept_eligible_events:       4
shadow_false_accepts:                      0
market_claim_allowed:                   true
```

Fresh default feedback after safe-policy v2 audit wiring:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

route_sum_verified_cpu_eligible_hits:      27
unique_verified_cpu_accepts:               22 / 1000
verified_cpu_accept_unique_routability_milli: 22
unique_verified_gap_to_80_calls:          778
incremental_cpu_accept_unique_request_fingerprints: 22

git_control route row:
  stage: verified_cpu_accept_eligible
  candidate_events:                         18
  payload_ready_events:                     12
  payload_built_events:                     12
  scoreable_payload_events:                  9
  verification_hook_ready_events:            9
  verified_cpu_accept_eligible_events:       4
  false_accepts:                             0
```

Structural gate:

```text
packet:
  docs/structural_gates/git-control-safe-policy-promote-v2.md

gate_report:
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-promote-v2.nanda.json

verdict: PASS
complexity_score: 39
agent_action: SAFE_TO_EDIT
```

Claim boundary:

```text
This is a narrow non-synthetic git_control promoted trace PASS: 4 verified
safe accepts, 0 false accepts, 0 unverified accepts, and 4 incremental accepts
over exact cache inside the current 1000-call trace window.

This is not CPU Routability 80. The full feedback loop is now 22/1000 unique
verified CPU accepts, with 778 unique calls still missing to reach the 80%
target. Workspace mutation execution remains disabled; this route only scores
and audits shadow local accepts.
```

## 2026-07-04 - Executor Integration: Metrics-Report Admission Safe-Policy Promote V1

Verdict:

```text
METRICS_REPORT_ADMISSION_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY
REAL_TRAFFIC_SHADOW_V1_PASS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-metrics-report-admission-safe-policy-promote-v1

Added feedback wiring:
  metrics_report_safe_policy_verification_audit_report_path
```

Fresh promotion artifact:

```text
report:
  target/nando-wave/real-traffic-shadow/metrics-report-safe-policy-v1.report.json

promoted_registry:
  target/nando-wave/real-traffic-shadow/profile-registry-metrics-report-safe-policy-v1.json

promoted_trace:
  target/nando-wave/real-traffic-shadow/metrics-report-safe-policy-v1.trace.jsonl

request_side_policy_name:
  false_accept_terms_no_failure_active_fringe_min_99

selected_acceptance_policy:          first_slot_threshold
selected_policy_threshold:           278528
policy_accept_rows:                       3
policy_accept_verified_true_rows:         3
policy_accept_verified_false_rows:        0
policy_accept_unverified_rows:            0
provider_cost_events_written:             3
market_claim_allowed:                 false
```

Fresh metrics_report shadow:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/metrics-report-safe-policy-v1.shadow-report.json

total_llm_calls:                        1000
nando_shadow_accepts:                      3
verified_safe_accepts:                     3
unverified_shadow_accepts:                 0
false_accepts:                             0
incremental_savings_over_exact_cache:      3
p99_shadow_score_latency_ns:          260899
synthetic_trace_used:                  false
```

Fresh metrics_report verification audit:

```text
audit_report:
  target/nando-wave/real-traffic-shadow/metrics-report-safe-policy-v1.verification-hook-audit.report.json

operator_candidate_calls:                  3
scoreable_candidate_calls:                 3
verification_hook_ready_events:            3
verified_cpu_accept_eligible_events:       3
shadow_false_accepts:                      0
market_claim_allowed:                   true
```

Fresh default feedback after safe-policy audit wiring:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

verified_cpu_accept_unique_request_fingerprints: 19
verified_cpu_accept_unique_routability_milli:    19
unique_verified_gap_to_80_calls:                781
incremental_cpu_accept_unique_request_fingerprints: 19

metrics_report route row:
  stage: verified_cpu_accept_eligible
  candidate_events:                         55
  payload_ready_events:                     42
  payload_built_events:                     42
  scoreable_payload_events:                  3
  verification_hook_ready_events:            3
  verified_cpu_accept_eligible_events:       3
  false_accepts:                             0
```

Fresh operator catalog:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

existing_operator_candidate_calls:        595
no_candidate_calls:                       405
current_verified_cpu_accepts:              19
verified_gap_to_80_calls:                 781
```

Structural gate:

```text
packet:
  docs/structural_gates/metrics-report-admission-safe-policy-promote-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/metrics-report-admission-safe-policy-promote-v1.nanda.json

verdict: PASS
complexity_score: 33
agent_action: SAFE_TO_EDIT
```

Claim boundary:

```text
This is a narrow non-synthetic metrics_report promoted trace PASS: 3 verified
safe accepts, 0 false accepts, 0 unverified accepts, and 3 incremental accepts
over exact cache inside the current 1000-call trace window.

This is not CPU Routability 80. The full feedback loop is now 19/1000 unique
verified CPU accepts, with 781 calls still missing to reach the 80% target.
```

## 2026-07-04 - Executor Integration: Metrics-Report Admission Calibration + Feedback Wiring V1

Verdict:

```text
METRICS_REPORT_ADMISSION_CALIBRATION_V1_REVIEW_ROBUST_POLICY_CANDIDATE_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-metrics-report-admission-calibration-v1
```

Fresh metrics_report admission calibration:

```text
report:
  target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1.report.json

hook_ready_rows:                         32
rows_with_prompt_features:               32
history_prompt_missing_rows:              0
label_true_rows:                         18
label_false_rows:                        14
minimum true-support gate:                3
robust_safe_policy_found:              true
best_robust_true_accepts:                 3

best robust policy:
  false_accept_terms_no_failure
  true_accepts:                            3
  false_accepts:                           0

raw_prompt_text_written:              false
raw_response_text_written:            false
response_text_used_for_features:      false
target_labels_used_for_runtime:       false
proof_labels_used_for_runtime:        false
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Fresh default feedback after wiring metrics_report admission:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:                 595 / 1000
scoreable_candidate_calls:                177 / 1000
verification_hook_ready_events:           138
verified_cpu_accept_eligible_events:       21 route-sum
unique_verified_cpu_accepts:               16
unique_verified_gap_to_80_calls:          784

metrics_report route row:
  candidate_events:                        55
  payload_ready_events:                    42
  payload_built_events:                    42
  scoreable_payload_events:                42
  verification_hook_ready_events:          32
  local_accept_calibration_ran:          true
  local_accept_safe_policy_found:        true
  local_accept_support_qualified:        true
  local_accept_best_safe_true_accepts:      3
  verified_cpu_accept_eligible_events:      0
  false_accepts:                            0
  stage: local_accept_calibration_safe_policy_candidate
```

Structural gate:

```text
packet:
  docs/structural_gates/metrics-report-admission-calibration-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1.nanda.json

verdict: PASS
complexity_score: 36
agent_action: SAFE_TO_EDIT
```

Claim boundary:

```text
metrics_report now has a request-side admission candidate with enough support
for the minimum 3-row safe-policy gate, but it adds zero verified CPU accepts.
This is not promoted runtime savings and not a market claim. Current CPU
Routability remains 16/1000 unique requests, not 80%.

Next step: promote only through a separate shadow trace rewrite with provider
cost, rollback, unverified_shadow_accepts=0, false_accepts=0, and no answer or
label leak.
```

## 2026-07-04 - Executor Integration: Retrieval-Lookup Calibration + Feedback Wiring V1

Verdict:

```text
RETRIEVAL_LOOKUP_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SUPPORT_INSUFFICIENT
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-retrieval-lookup-local-accept-calibration-v1
```

Fresh retrieval_lookup calibration:

```text
report:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-local-accept-calibration-v1.report.json

hook_ready_rows:                         2
scored_rows:                             2
label_true_rows:                         2
label_false_rows:                        0
safe_policy_found:                    true
best_safe_true_accepts:                  2
minimum true-support gate:               3
local_accepts_enabled:               false
market_claim_allowed:                false
```

Decision:

```text
A safe-looking retrieval_lookup margin policy exists on the current two
verifier-true rows, but support is below the minimum true-support gate. It must
remain review-only. Do not lower the disabled threshold or count it as savings.
```

Fresh default feedback after wiring retrieval_lookup:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:                 595 / 1000
scoreable_candidate_calls:                177 / 1000
verification_hook_ready_events:           138
verified_cpu_accept_eligible_events:       21 route-sum
unique_verified_cpu_accepts:               16
unique_verified_gap_to_80_calls:          784

retrieval_lookup route row:
  candidate_events:                        25
  payload_ready_events:                     2
  payload_built_events:                     2
  scoreable_payload_events:                 2
  verification_hook_ready_events:           2
  local_accept_calibration_ran:          true
  local_accept_safe_policy_found:        true
  local_accept_support_qualified:       false
  verified_cpu_accept_eligible_events:      0
  false_accepts:                            0
  stage: local_accept_calibration_support_insufficient
```

Fresh operator catalog:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

existing_operator_candidate_calls:        595
no_candidate_calls:                       405
current_verified_cpu_accepts:             16
verified_gap_to_80_calls:                 784

retrieval_lookup existing-profile row:
  priority_rank:                           15
  candidate_events:                        25
  scoreable_payload_events:                 2
  verification_hook_ready_events:           2
  verified_cpu_accept_eligible_events:      0
```

Structural gate:

```text
packet:
  docs/structural_gates/retrieval-lookup-feedback-wiring-v1.md

gate_report:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-feedback-wiring-v1.nanda.json

verdict: PASS
complexity_score: 37
route_coherence retrieval_lookup: 0.9214
```

Claim boundary:

```text
retrieval_lookup is now visible in the main CPU80 feedback ladder and catalog,
but it adds zero verified CPU accepts. Current CPU Routability remains
16/1000 unique requests, not 80%. The next retrieval step is broader
request-side admission/evidence coverage, not promotion.
```

## 2026-07-04 - Executor Integration: Retrieval-Lookup Profile + Evidence V1

Verdict:

```text
RETRIEVAL_LOOKUP_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED
RETRIEVAL_LOOKUP_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added commands:
  role-binding-real-traffic-retrieval-lookup-profile-v1
  role-binding-real-traffic-retrieval-lookup-output-evidence-v1
```

Fresh profile:

```text
package:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-seed0.nwrb

registry:
  target/nando-wave/real-traffic-shadow/profile-registry-retrieval-lookup-v1.json

report:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-profile-v1.report.json

scoreable_payload_events:                       2
package_training_requests:                      2
edge_count:                                     7
runtime_bytes_estimate:                     32972
threshold:                             2147483647
positive_margin_rows:                           2
strict_ordered_pass_rows:                       2
unexpected_local_accepts_under_disabled_threshold: 0
median_energy_margin:                     1181696
local_accepts_enabled_on_real_traffic:      false
market_claim_allowed:                       false
```

Fresh output evidence:

```text
trace:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-output-evidence-v1.trace.jsonl

report:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-output-evidence-v1.report.json

total_trace_rows:                         1000
operator_candidate_calls:                    2
scoreable_candidate_calls:                   2
output_evidence_matched_events:              2
deterministic_verification_events:           2
verifier_not_applicable_events:              0
verified_true_events:                        2
verified_false_events:                       0
raw_response_text_written:               false
response_text_used_for_verification:      true
local_accepts_enabled:                   false
market_claim_allowed:                    false
```

Fresh shadow + audit:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-output-evidence-v1.shadow-report.json

audit_report:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-output-evidence-v1.verification-hook-audit.report.json

shadow total_llm_calls:                    1000
shadow exact_cache_hits:                     53
shadow nando_shadow_accepts:                  0
shadow verified_safe_accepts:                 0
shadow false_accepts:                         0
shadow incremental_reduction_vs_exact_cache_milli: 0
shadow p99_shadow_score_latency_ns:      250185

audit operator_candidate_calls:               2
audit scoreable_candidate_calls:              2
audit verification_hook_ready_events:         2
audit verified_cpu_accept_eligible_events:    0
audit market_claim_allowed:               false
```

Decision:

```text
retrieval_lookup now has a complete disabled route rung:
request-side payload -> .nwrb profile -> source/path/URL evidence hook -> shadow/audit.
It still does not affect the current CPU scoreboard because the profile threshold is
i32::MAX and local accepts remain disabled. This is the correct side of the line:
verified evidence exists for 2 rows, but verified CPU savings are still 0.
```

Next engineering debt:

```text
Do not promote retrieval_lookup to local accept yet. First run calibration with
enough verifier-true and verifier-false support, or broaden the request-side
payload builder to more real retrieval rows without answer leaks. External
freshness/free-form lookup still requires concrete source/path/URL verification.
```

## 2026-07-04 - Executor Integration: Retrieval-Lookup Payload Dry-Run V1

Verdict:

```text
RETRIEVAL_LOOKUP_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-retrieval-lookup-payload-dry-run-v1
```

Fresh dry-run:

```text
report:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-payload-dry-run-v1.report.json

trace:
  target/nando-wave/real-traffic-shadow/retrieval-lookup-payload-dry-run-v1.trace.jsonl

retrieval_lookup_candidate_events: 25 / 1000
payload_ready_events:              2
payload_built_events:              2
scoreable_payload_events:          2
active_fringe_centers_total:     246
slots_total:                       6
positive_impulses_total:         144
negative_impulses_total:         144
profile_registered:            false
shadow_score_ready:            false
raw_text_written:              false
response_text_used:            false
target_labels_used:            false
proof_labels_used:             false
local_accepts_enabled:         false
market_claim_allowed:          false
```

Decision:

```text
retrieval_lookup now has a request-side payload path for its strict-ready
subset, but it is still profile-missing and verifier-missing. It must not be
merged with read_inspect and must not affect the 16/1000 CPU scoreboard.
```

Next engineering debt:

```text
Compile a disabled-threshold retrieval_lookup .nwrb profile, attach
source_path_or_url_presence_verifier_v1 from Codex final-answer/tool evidence,
then run shadow/audit/calibration. Do not local-accept external freshness or
free-form answer lookup without a concrete source/path/URL verifier.
```

## 2026-07-04 - Executor Integration: Project-Context Subfamily Audit V1

Verdict:

```text
PROJECT_CONTEXT_SUBFAMILY_AUDIT_V1_REVIEW_ACTIONABLE_SUBSET_FOUND
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-project-context-subfamily-audit-v1
```

Fresh audit:

```text
report:
  target/nando-wave/real-traffic-shadow/project-context-subfamily-audit-v1.report.json

project_context_candidate_events: 211 / 1000
payload_ready_events:             2
payload_ready_rate_milli:         9
request_signal_events:            20
context_signal_events:            11
evidence_signal_events:           10
verifier_signal_events:           12
raw_text_written:                 false
response_text_used:               false
target_labels_used:               false
proof_labels_used:                false
local_accepts_enabled:            false
market_claim_allowed:             false
```

Subfamily split:

```text
artifact_backed_project_state:                 2 candidates, 2 ready
short_context_chatter:                       180 candidates, 0 ready
request_only_project_dialogue:                17 candidates, 0 ready
context_evidence_missing_request_or_verifier:  8 candidates, 0 ready
broad_project_context_dialogue:                3 candidates, 0 ready
request_context_missing_evidence_or_verifier:  1 candidate,  0 ready
```

Decision:

```text
Do not treat all project_context_dialogue rows as CPU routability surface.
Most of this family is short context/chatter without request/context/evidence
structure. The only safe project-state branch right now is the 2-row
artifact_backed_project_state subset.
```

Next engineering debt:

```text
Either:
  1. build the small artifact_backed_project_state branch into a disabled
     project_context profile + workspace_artifact_or_goal_state_verifier; or
  2. move to the next route-gap family with clearer verifier economics
     (retrieval_lookup / answer_or_explain with grounded evidence).

Do not lower evidence requirements to inflate project_context coverage.
```

## 2026-07-04 - Executor Integration: Project-Context Payload Dry-Run V1

Verdict:

```text
PROJECT_CONTEXT_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-project-context-payload-dry-run-v1
```

Fresh dry-run:

```text
report:
  target/nando-wave/real-traffic-shadow/project-context-payload-dry-run-v1.report.json

trace:
  target/nando-wave/real-traffic-shadow/project-context-payload-dry-run-v1.trace.jsonl

project_context_candidate_events:  211 / 1000
payload_ready_events:                2
payload_built_events:                2
scoreable_payload_events:            2
active_fringe_centers_total:       231
slots_total:                         6
positive_impulses_total:           133
negative_impulses_total:           144
profile_registered:              false
shadow_score_ready:              false
raw_text_written:                false
response_text_used:              false
target_labels_used:              false
proof_labels_used:               false
local_accepts_enabled:           false
market_claim_allowed:            false
```

Claim boundary:

```text
This is a request-side dry-run only. It proves that the top route-gap family can
emit scoreable active_fringe/slot payloads for the strict artifact-backed subset,
but it does not prove savings and must not change the current 16/1000 CPU
Routability scoreboard.
```

Next engineering debt:

```text
Build a disabled-threshold project_context .nwrb profile and workspace
artifact / goal-state verifier before any calibration. If the route remains
2/211 under strict evidence, split project_context into safer subfamilies
instead of lowering verifier requirements.
```

## 2026-07-04 - Executor Integration: Post-16 Route Triage

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW_POST_16_TRIAGE_NO_THRESHOLD_SHORTCUT
```

Current scoreboard:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

verified_cpu_accept_route_sum_events:             21 / 1000
verified_cpu_accept_unique_request_fingerprints:  16 / 1000
verified_cpu_accept_duplicate_route_hits:         5
exact_cache_overlap_verified_cpu_accepts:         1
unique_verified_gap_to_80_calls:                  784
false_accepts:                                    0
```

Route triage:

```text
conditional_branch:
  candidate_events:                 166
  verification_hook_ready_events:    40
  verified_cpu_accept_eligible:       2
  blocker: calibration says score/readout geometry does not separate 17 true from 46 false.
  decision: do not lower threshold; improve branch extraction/admission first.

metrics_report_readout:
  default_1000_window:
    hook_ready_rows:                  32
    label_true_rows:                  18
    label_false_rows:                 14
    best_safe_true_accepts:            2
    minimum_true_support:              3
    decision: keep review-only in default feedback.
  separate_5000_soak:
    audit: metrics-report-soak-v1/metrics-report-safe-policy-v1.verification-hook-audit.report.json
    total_llm_calls:                5000
    verified_cpu_accept_eligible:      3
    shadow_false_accepts:              0
    market_claim_allowed:           true
    decision: valid narrow soak, but excluded from the 1000-row feedback denominator.

agent_control:
  candidate_events:                 143
  verified_true_rows:                12
  current_safe_policy_true_accepts:  11
  false_accepts:                      0
  finding: the remaining true row is short_ack-shaped; no clean request-side conjunction
           over current features captures 12/12 without false rows.
  decision: keep strict_control_stop_forms at 11/0.

git_control:
  calibration_best_safe_true_accepts: 5
  promoted_safe_policy_accepts:       1
  reason: lower threshold would admit unverified rows in the promoted trace.
  decision: need more tool-output evidence, not a lower threshold.
```

Next engineering cut:

```text
Build a new route-gap operator instead of squeezing thresholds:
  1. project_context_dialogue -> active_project_state_payload_builder_v1
  2. answer_or_explain       -> grounded_answer_evidence_verifier_v1
  3. retrieval_lookup        -> local_path_or_link_lookup_payload_builder_v1

Keep local accepts disabled until deterministic verifier + shadow audit pass.
```

## 2026-07-04 - Executor Integration: Mixed V2 Default Promotion

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW_MIXED_V2_DEFAULT_ACTIVE
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

Default mixed-map feedback audit now points to:
  mixed-safe-policy-v2.verification-hook-audit.report.json
```

Why this is allowed:

```text
mixed-safe-policy-v2.verification-hook-audit.report.json:
  total_llm_calls:                      1000
  operator_candidate_calls:             11
  scoreable_candidate_calls:            11
  verification_hook_ready_events:       11
  verified_cpu_accept_eligible_events:  3
  shadow_false_accepts:                 0
  shadow_incremental_savings_over_exact_cache: 3
  market_claim_allowed:                 true
```

Fresh default feedback:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:                         570 / 1000
no_candidate_calls:                               430 / 1000
scoreable_candidate_calls:                        175 / 1000
verification_hook_ready_events:                   136 / 1000
verified_cpu_accept_eligible_events:              21
verified_cpu_accept_route_sum_events:             21
verified_cpu_accept_unique_request_fingerprints:  16
incremental_cpu_accept_unique_request_fingerprints: 16
verified_cpu_accept_duplicate_route_hits:         5
exact_cache_overlap_verified_cpu_accepts:         1
unique_verified_gap_to_80_calls:                  784
false_accepts:                                    0
```

Fresh catalog:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-current-feedback-v1.report.json

current_verified_cpu_accepts:                     16
verified_cpu_accept_route_sum_events:             21
verified_cpu_accept_duplicate_route_hits:         5
route_gap_feedback_no_candidate_mismatch:         true
```

Claim boundary:

```text
Default feedback now reaches the previous v3 diagnostic level: 16 unique
request-fingerprint accepts on the same 1000-row non-synthetic Codex trace.
This is still not CPU Routability 80.
```

## 2026-07-04 - Executor Integration: Agent-Control V2 Default Promotion

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW_AGENT_CONTROL_V2_DEFAULT_ACTIVE
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

Default agent-control artifacts now point to the existing v2 safe-policy set:
  agent-control-admission-calibration-v2.report.json
  agent-control-safe-policy-v2.trace.jsonl
  agent-control-safe-policy-v2.report.json
  agent-control-safe-policy-v2.verification-hook-audit.report.json

Catalog summary now treats unique request-fingerprint accepts as the CPU
Routability scoreboard. Route-sum accepts are exposed separately.
```

Why this is allowed:

```text
agent-control-safe-policy-v2.verification-hook-audit.report.json:
  total_llm_calls:                      1000
  operator_candidate_calls:             11
  scoreable_candidate_calls:            11
  verification_hook_ready_events:       11
  verified_cpu_accept_eligible_events:  11
  shadow_false_accepts:                 0
  shadow_incremental_savings_over_exact_cache: 6
  market_claim_allowed:                 true
```

Fresh default feedback:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:                         570 / 1000
no_candidate_calls:                               430 / 1000
scoreable_candidate_calls:                        178 / 1000
verification_hook_ready_events:                   138 / 1000
verified_cpu_accept_eligible_events:              20
verified_cpu_accept_route_sum_events:             20
verified_cpu_accept_unique_request_fingerprints:  15
incremental_cpu_accept_unique_request_fingerprints: 15
verified_cpu_accept_duplicate_route_hits:         5
exact_cache_overlap_verified_cpu_accepts:         1
unique_verified_gap_to_80_calls:                  785
false_accepts:                                    0
```

Fresh catalog:

```text
catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-current-feedback-v1.report.json

current_verified_cpu_accepts:                     15
verified_cpu_accept_route_sum_events:             20
verified_cpu_accept_duplicate_route_hits:         5
route_gap_feedback_no_candidate_mismatch:         true

interpretation:
  CPU Routability scoreboard is now unique accepts, not route-sum. The route-gap
  side can be stale relative to feedback and is explicitly marked when its
  no-candidate count differs from feedback.no_candidate_calls.
```

Claim boundary:

```text
This is real progress from 12 unique accepts to 15 unique accepts on the same
1000-row non-synthetic Codex trace. It is still not CPU Routability 80.
```

## 2026-07-04 - Executor Integration: Feedback-Loop Unique Accept Dedupe

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW_UNIQUE_ACCEPT_DEDUPE_ACTIVE
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

Also fixed:
  agent-control route is now exposed as its own feedback-loop route row when
  the forecast did not already contain it. Previously agent-control accepts
  could contribute to totals while the route row/candidate coverage stayed
  hidden.

Added feedback-loop report fields:
  verified_cpu_accept_route_sum_events
  verified_cpu_accept_unique_request_fingerprints
  verified_cpu_accept_unique_routability_milli
  incremental_cpu_accept_unique_request_fingerprints
  incremental_cpu_accept_unique_reduction_milli
  unique_verified_gap_to_80_calls
  incremental_unique_gap_to_80_calls
  exact_cache_overlap_verified_cpu_accepts
  verified_cpu_accept_duplicate_route_hits
  verified_cpu_accept_missing_request_fingerprint_rows
  unique_accept_shadow_reports
```

Default 1000-row feedback after dedupe:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:                         570 / 1000
no_candidate_calls:                               430 / 1000
scoreable_candidate_calls:                        170 / 1000
verification_hook_ready_events:                   130 / 1000
verified_cpu_accept_eligible_events:              12
verified_cpu_accept_route_sum_events:             12
verified_cpu_accept_unique_request_fingerprints:  12
incremental_cpu_accept_unique_request_fingerprints: 12
unique_verified_gap_to_80_calls:                  788
incremental_unique_gap_to_80_calls:               788
verified_cpu_accept_duplicate_route_hits:         0
audit_window_mismatches:                          []

newly visible route row:
  route_key:                                      role_binding_agent_control_seed0
  candidate_events:                              143
  scoreable_payload_events:                       3
  verification_hook_ready_events:                 3
  verified_cpu_accept_eligible_events:            3
  false_accepts:                                  0
```

Historical v3 same-window dedupe check:

```text
diagnostic_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v3.dedup-check.report.json

route_sum_verified_accepts:                       21
unique_verified_request_fingerprints:             16
incremental_unique_request_fingerprints:          16
exact_cache_overlap_verified_cpu_accepts:         1
duplicate_verified_route_hits:                    5
unique_verified_gap_to_80_calls:                  784
incremental_unique_gap_to_80_calls:               784

excluded_mismatch:
  planning-next-step-artifact-progress-v3 audit has 12000 total_llm_calls
  while the forecast window has 1000 total_llm_calls.
```

Claim boundary:

```text
Feedback-loop totals now expose route-sum and unique request-fingerprint counts
separately. CPU Routability progress must be read from unique accepted real
requests, not from a raw route-sum. The current default 12/1000 is not
duplicated; the historical v3 21 route-sum collapses to 16 unique accepted
requests. CPU Routability 80 is still not achieved.
```

Next engineering debt:

```text
Use unique request-fingerprint accepts as the scoreboard for all future route
promotions. The next route work should increase incremental unique accepts,
not merely add overlapping route hits.
```

## 2026-07-04 - Executor Integration: Feedback-Loop Window Guard

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW_WINDOW_GUARD_ACTIVE
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added report field:
  audit_window_mismatches
```

Window-guard proof:

```text
negative_test_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-metrics-soak-window-guard-v1.report.json

forecast_total_llm_calls:             1000
metrics_soak_audit_total_llm_calls:   5000
excluded_route_keys:                  [metrics_report_readout]
excluded_from_feedback:               true
verified_cpu_accept_eligible_events:  12
verified_cpu_routability_milli:       12
verified_gap_to_80_calls:             788
```

Default feedback after guard:

```text
feedback_report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

total_llm_calls:                      1000
verified_cpu_accept_eligible_events:  12
audit_window_mismatches:              []
```

Claim boundary:

```text
Feedback-loop aggregation now excludes route-specific verification audits whose
total_llm_calls differ from the forecast window. The 5000-row metrics-report
soak PASS can no longer be accidentally counted into the default 1000-row CPU
Routability report. CPU Routability 80 is still not achieved.
```

## 2026-07-04 - Executor Integration: Metrics-Report 5000-Row Safe-Policy Soak

Verdict:

```text
REAL_TRAFFIC_SHADOW_V1_PASS
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-metrics-report-safe-policy-promote-v1

Added soak artifacts under ignored target path:
  target/nando-wave/real-traffic-shadow/metrics-report-soak-v1/
```

5000-row metrics-report soak:

```text
payload_report:
  target/nando-wave/real-traffic-shadow/metrics-report-soak-v1/metrics-report-payload-dry-run-v1.report.json

trace_rows_written:                   5000
metrics_report_candidate_events:      98
payload_ready_events:                 63
scoreable_payload_events:             63
```

Evidence and calibration:

```text
evidence_report:
  target/nando-wave/real-traffic-shadow/metrics-report-soak-v1/metrics-report-output-evidence-v1.report.json

output_evidence_matched_events:       51
verified_true_events:                 31
verified_false_events:                20
raw_prompt_text_written:              false
raw_response_text_written:            false

calibration_report:
  target/nando-wave/real-traffic-shadow/metrics-report-soak-v1/metrics-report-local-accept-calibration-v1.report.json

safe_policy_found:                    true
best_safe_true_accepts:               3
best policy:                          best_metric_slot_margin_threshold
best threshold:                       393216
```

Promoted shadow attempt:

```text
promote_report:
  target/nando-wave/real-traffic-shadow/metrics-report-soak-v1/metrics-report-safe-policy-v1.report.json

selected_policy_name:                 market_safe_metric_slot_margin_threshold_with_active_fringe_min
selected_policy_source:               evidence_trace_metric_slot_safe_threshold_plus_active_fringe_min
selected_acceptance_policy:           first_slot_threshold_active_fringe_min_114
request_side_policy_name:             metrics_report_active_fringe_min_114
selected_policy_threshold:            393216
request_side_policy_evaluated_rows:   63
request_side_policy_accept_rows:      11
request_side_policy_reject_rows:      52
policy_accept_rows:                   3
policy_accept_verified_true_rows:     3
policy_accept_verified_false_rows:    0
policy_accept_unverified_rows:        0
runtime_acceptance_mismatches:        0
provider_cost_events_written:         63
```

Shadow/audit:

```text
shadow_report:
  target/nando-wave/real-traffic-shadow/metrics-report-soak-v1/metrics-report-safe-policy-v1.shadow-report.json

verdict:                              REAL_TRAFFIC_SHADOW_V1_PASS
total_llm_calls:                      5000
operator_candidate_calls:             63
nando_shadow_accepts:                 3
verified_safe_accepts:                3
unverified_shadow_accepts:            0
false_accepts:                        0
p99_shadow_score_latency_ns:          272859
synthetic_trace_used:                 false

audit_report:
  target/nando-wave/real-traffic-shadow/metrics-report-soak-v1/metrics-report-safe-policy-v1.verification-hook-audit.report.json

verified_cpu_accept_eligible_events:  3
market_claim_allowed:                 true
```

Claim boundary:

```text
This is a narrow promoted route PASS for the separate 5000-row non-synthetic
metrics-report soak. The unverified accept was removed by a request-side
active-fringe admission gate, not by reading the answer: runtime now requires
first_slot_margin >= 393216 and active_fringe_len >= 114 for this promoted
metrics_report profile.

Do not add these 3 accepts to the current 1000-row feedback total until the
overall feedback window is regenerated with the same denominator. CPU
Routability 80 is still not achieved.
```

Next engineering debt:

```text
Regenerate the overall real-traffic feedback loop on a single consistent
window if metrics_report should be counted beside git_control, serving_ops,
mixed, edit, and conditional routes.
```

## 2026-07-04 - Executor Integration: Git-Control Tool-Output Safe-Policy Promotion

Verdict:

```text
REAL_TRAFFIC_SHADOW_V1_PASS
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-git-control-safe-policy-promote-v1

Added promoted artifacts:
  target/nando-wave/real-traffic-shadow/profile-registry-git-control-safe-policy-v1.json
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v1.report.json
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v1.shadow-report.json
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v1.verification-hook-audit.report.json
```

Tool-output evidence:

```text
report:
  target/nando-wave/real-traffic-shadow/git-control-output-evidence-v1.report.json

output_evidence_matched_events:       10
deterministic_verification_events:    10
verified_true_events:                 6
verified_false_events:                4
tool_call_fingerprint_events:         5
verification_source:
  codex_session_tool_output_fingerprint_plus_deterministic_git_command_outcome_verifier_v1
raw_prompt_text_written:              false
raw_response_text_written:            false
raw_tool_output_written:              false
workspace_mutation_enabled:           false
```

Git-control safe-policy promotion:

```text
selected_policy_name:                 market_safe_energy_margin_threshold
selected_policy_source:               evidence_trace_market_safe_threshold
selected_policy_threshold:            1505280
policy_accept_rows:                   1
policy_accept_verified_true_rows:     1
policy_accept_verified_false_rows:    0
policy_accept_unverified_rows:        0
provider_cost_events_written:         12
market_claim_allowed:                 false
```

Promoted shadow:

```text
report:
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v1.shadow-report.json

verdict:                              REAL_TRAFFIC_SHADOW_V1_PASS
total_llm_calls:                      1000
operator_candidate_calls:             12
nando_shadow_accepts:                 1
verified_safe_accepts:                1
unverified_shadow_accepts:            0
false_accepts:                        0
incremental_savings_over_exact_cache: 1
incremental_reduction_vs_exact_cache_milli: 1
p99_shadow_score_latency_ns:          273719
synthetic_trace_used:                 false
```

Verification audit:

```text
report:
  target/nando-wave/real-traffic-shadow/git-control-safe-policy-v1.verification-hook-audit.report.json

operator_candidate_calls:             12
scoreable_candidate_calls:            12
verification_hook_ready_events:       10
tool_call_fingerprint_events:         5
verified_cpu_accept_eligible_events:  1
market_claim_allowed:                 true
false_accepts:                        0
```

Feedback loop after promoted git-control audit:

```text
feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:             427 / 1000
scoreable_candidate_calls:            167 / 1000
verification_hook_ready_events:       130 / 1000
verified_cpu_accept_eligible_events:  12 / 1000
verified_cpu_routability_milli:       12
verified_gap_to_80_calls:             788

git_control route row:
  stage:                              verified_cpu_accept_eligible
  candidate_events:                   18
  scoreable_payload_events:           12
  verification_hook_ready_events:     10
  local_accept_best_safe_true_accepts: 5
  verified_cpu_accept_eligible_events: 1
  false_accepts:                      0
```

Claim boundary:

```text
This is the first promoted git_control shadow artifact backed by tool-output
fingerprints and deterministic git command outcome verification. It does not
run git, does not mutate the workspace, does not write raw tool output, and
does not enable a live daemon mutation path.

It counts as 1 verified CPU accept in the current 1000-call non-synthetic Codex
trace window. It still does not prove CPU Routability 80. Current default
routability is 12/1000, gap to 80% is 788 calls.
```

Next engineering debt:

```text
Grow verified routes by adding real verifier-backed accepts, not by relaxing
the safe policy. The next highest-value blockers remain:
  metrics_report_readout: support insufficient
  read_inspect: calibration failed
  planning_next_step: support insufficient
  serving_ops/git_control: need larger non-synthetic soak before external
    market claim
```

## 2026-07-04 - Executor Integration: Serving-Ops Safe-Policy Promotion

Verdict:

```text
REAL_TRAFFIC_SHADOW_V1_PASS
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-serving-ops-safe-policy-promote-v1

Added promoted artifacts:
  target/nando-wave/real-traffic-shadow/profile-registry-serving-ops-safe-policy-v1.json
  target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1.report.json
  target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1.shadow-report.json
  target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1.verification-hook-audit.report.json
```

Serving-ops safe-policy promotion:

```text
selected_policy_name:                 market_safe_energy_margin_threshold
selected_policy_source:               evidence_trace_market_safe_threshold
selected_policy_threshold:            1392640
policy_accept_rows:                   3
policy_accept_verified_true_rows:     3
policy_accept_verified_false_rows:    0
policy_accept_unverified_rows:        0
provider_cost_events_written:         8
market_claim_allowed:                 false
```

Promoted shadow:

```text
report:
  target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1.shadow-report.json

verdict:                              REAL_TRAFFIC_SHADOW_V1_PASS
total_llm_calls:                      1000
operator_candidate_calls:             8
nando_shadow_accepts:                 3
verified_safe_accepts:                3
unverified_shadow_accepts:            0
false_accepts:                        0
incremental_savings_over_exact_cache: 3
incremental_reduction_vs_exact_cache_milli: 3
estimated_cost_saved_microusd:        300
p99_shadow_score_latency_ns:          156653
synthetic_trace_used:                 false
```

Verification audit:

```text
report:
  target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1.verification-hook-audit.report.json

operator_candidate_calls:             8
scoreable_candidate_calls:            8
verification_hook_ready_events:       7
verified_cpu_accept_eligible_events:  3
market_claim_allowed:                 true
```

Feedback loop after promoted serving-ops audit:

```text
feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:             427 / 1000
scoreable_candidate_calls:            167 / 1000
verification_hook_ready_events:       130 / 1000
verified_cpu_accept_eligible_events:  11 / 1000
verified_cpu_routability_milli:       11
verified_gap_to_80_calls:             789

serving_ops route row:
  stage:                              verified_cpu_accept_eligible
  candidate_events:                   25
  scoreable_payload_events:           8
  verification_hook_ready_events:     7
  verified_cpu_accept_eligible_events: 3
  false_accepts:                      0
```

Claim boundary:

```text
This is the first promoted serving_ops shadow artifact that passes shadow and
verification audit with provider cost, false_accepts=0, and
unverified_shadow_accepts=0. It counts as 3 verified CPU accepts in the current
1000-call non-synthetic Codex trace window.

It still does not prove CPU Routability 80, does not touch a server, does not
restart daemons, and does not enable any real daemon/server mutation path.
Routability remains 11/1000, gap to 80% is 789 calls.
```

Next engineering debt:

```text
Grow verified routes from isolated safe-policy pockets to broader real-traffic
coverage. Highest current blockers:
  planning_next_step: support insufficient
  read_inspect: calibration failed
  metrics_report_readout: support insufficient
  git_control: needs real tool-output/status verifier
  serving_ops: needs larger non-synthetic soak before external market claim
```

## 2026-07-04 - Executor Integration: Serving-Ops Route Payload/Profile/Evidence

Verdict:

```text
SERVING_OPS_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SAFE_POLICY_CANDIDATE_FOUND
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added commands:
  role-binding-real-traffic-serving-ops-payload-dry-run-v1
  role-binding-real-traffic-serving-ops-profile-v1
  role-binding-real-traffic-serving-ops-output-evidence-v1
  role-binding-real-traffic-serving-ops-local-accept-calibration-v1

Added route/profile:
  route_key: serving_ops
  profile_id: route_gap_serving_ops_profile_v1

Added verifier:
  deterministic_serving_ops_output_verification
  verification_source:
    codex_session_final_answer_fingerprint_plus_deterministic_service_health_metric_verifier_v1
```

Serving-ops request-side payload dry-run:

```text
report:
  target/nando-wave/real-traffic-shadow/serving-ops-payload-dry-run-v1.report.json

trace:
  target/nando-wave/real-traffic-shadow/serving-ops-payload-dry-run-v1.trace.jsonl

serving_ops_candidate_events:         25
payload_ready_events:                 8
payload_built_events:                 8
scoreable_payload_events:             8
profile_registered:                   false
raw_text_written:                     false
response_text_used:                   false
target_labels_used:                   false
proof_labels_used:                    false
server_mutation_enabled:              false
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Serving-ops profile:

```text
package:
  target/nando-wave/real-traffic-shadow/serving-ops-seed0.nwrb

registry:
  target/nando-wave/real-traffic-shadow/profile-registry-serving-ops-v1.json

report:
  target/nando-wave/real-traffic-shadow/serving-ops-profile-v1.report.json

scoreable_payload_events:             8
package_training_requests:            8
edge_count:                           8
package_bytes:                        140
runtime_bytes_estimate:               33000
median_energy_margin:                 1359872
p10_energy_margin:                    720896
unexpected_local_accepts_under_disabled_threshold: 0
server_mutation_enabled:              false
local_accepts_enabled_on_real_traffic: false
market_claim_allowed:                 false
```

Serving-ops output evidence:

```text
report:
  target/nando-wave/real-traffic-shadow/serving-ops-output-evidence-v1.report.json

verification audit:
  target/nando-wave/real-traffic-shadow/serving-ops-output-evidence-v1.verification-hook-audit.report.json

output_evidence_matched_events:       7
deterministic_verification_events:    7
verifier_not_applicable_events:       0
verified_true_events:                 5
verified_false_events:                2
verification_hook_ready_events:       7
verified_cpu_accept_eligible_events:  0
raw_prompt_text_written:              false
raw_response_text_written:            false
response_text_used_for_verification:  true
server_mutation_enabled:              false
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Serving-ops local accept calibration:

```text
report:
  target/nando-wave/real-traffic-shadow/serving-ops-local-accept-calibration-v1.report.json

hook_ready_rows:                      7
scored_rows:                          7
label_true_rows:                      5
label_false_rows:                     2
safe_policy_found:                    true
best_safe_true_accepts:               3
minimum_true_support:                 3
local_accept_support_qualified:       true
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Feedback loop after serving-ops integration:

```text
feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:             427 / 1000
scoreable_candidate_calls:            167 / 1000
verification_hook_ready_events:       130 / 1000
verified_cpu_accept_eligible_events:  8 / 1000
verified_cpu_routability_milli:       8
verified_gap_to_80_calls:             792
market_claim_allowed:                 false

serving_ops route row:
  candidate_events:                   25
  scoreable_payload_events:           8
  verification_hook_ready_events:     7
  local_accept_calibration_ran:       true
  local_accept_safe_policy_found:     true
  local_accept_support_qualified:     true
  local_accept_best_safe_true_accepts: 3
  verified_cpu_accept_eligible_events: 0
  false_accepts:                      0
  stage:                              local_accept_calibration_safe_policy_candidate
  next_action:                        Promote the safe policy only through a separate shadow trace rewrite with provider cost, rollback, and false_accepts=0.
```

Route-gap after registering serving-ops:

```text
catalog:
  target/nando-wave/real-traffic-shadow/route-gap-catalog-serving-ops-v1.report.json

readiness:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-serving-ops-v1.report.json

existing_route_candidate_events:      520 / 1000
no_candidate_events:                  480 / 1000
payload_ready_events:                 10
top_payload_ready_family:             uncatalogued
```

Claim boundary:

```text
This moves serving_ops from route-gap into a scoreable registered service /
daemon / HTTP metric readout profile with deterministic final-answer evidence
labels. It does not touch a server, does not restart daemons, does not enable
local accepts, and does not prove savings.

The route has a safe readout candidate with enough support for a separate
promotion artifact, but verified CPU accepts remain 0 for serving_ops until a
promoted shadow trace carries provider cost and passes false_accepts=0 /
unverified_shadow_accepts=0. Red gates stay red.
```

Next engineering debt:

```text
Build serving_ops safe-policy promotion artifact:
  registry threshold/policy must come from calibration;
  trace rewrite must stay request-side only;
  provider_cost_microusd must be attached before savings;
  run shadow + verification-hook audit;
  require false_accepts=0 and unverified_shadow_accepts=0 before counting
  serving_ops verified CPU accepts.
```

## 2026-07-04 - Executor Integration: Git-Control Local Accept Calibration

Verdict:

```text
GIT_CONTROL_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_FINAL_ANSWER_SAFE_POLICY_CANDIDATE
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-git-control-local-accept-calibration-v1
```

Calibration:

```text
input_trace:
  target/nando-wave/real-traffic-shadow/git-control-output-evidence-v1.trace.jsonl

registry_config:
  target/nando-wave/real-traffic-shadow/profile-registry-git-control-v1.json

report:
  target/nando-wave/real-traffic-shadow/git-control-local-accept-calibration-v1.report.json

hook_ready_rows:       10
scored_rows:           10
label_true_rows:       4
label_false_rows:      6
no_score_rows:         0
safe_policy_found:     true
best_safe_true_accepts: 3
local_accepts_enabled: false
market_claim_allowed:  false
```

Feedback loop after calibration:

```text
feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:            402 / 1000
scoreable_candidate_calls:           159 / 1000
verification_hook_ready_events:      123
verified_cpu_accept_eligible_events: 8
verified_cpu_routability_milli:      8
verified_gap_to_80_calls:            792
market_claim_allowed:                false

git_control route row:
  candidate_events:                   18
  scoreable_payload_events:           12
  verification_hook_ready_events:     10
  local_accept_calibration_ran:       true
  local_accept_safe_policy_found:     true
  local_accept_support_qualified:     true
  local_accept_best_safe_true_accepts: 3
  verified_cpu_accept_eligible_events: 0
  false_accepts:                      0
  stage:                              local_accept_calibration_needs_stronger_verifier
  next_action:                        Attach a real tool-output/status verifier before safe-policy promotion; final-answer labels are review-only.
```

Claim boundary:

```text
The calibration is useful as a score/readout probe, but its labels come from
conservative final-answer command-outcome evidence. It does not execute git,
does not inspect real command status/output, does not mutate the workspace,
does not enable local accepts, and does not prove savings.

Git-control remains REVIEW with 0 verified CPU accepts until a real
tool-output/status verifier proves the policy on non-synthetic trace rows.
```

## 2026-07-04 - Executor Integration: Git-Control Output Evidence Verifier

Verdict:

```text
GIT_CONTROL_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-git-control-output-evidence-v1
```

Git-control output evidence:

```text
input_trace:
  target/nando-wave/real-traffic-shadow/git-control-payload-dry-run-v1.trace.jsonl

output_trace:
  target/nando-wave/real-traffic-shadow/git-control-output-evidence-v1.trace.jsonl

report:
  target/nando-wave/real-traffic-shadow/git-control-output-evidence-v1.report.json

total_trace_rows:                    1000
operator_candidate_calls:            12
scoreable_candidate_calls:           12
session_ids_requested:               4
session_files_scanned:               4
output_evidence_matched_events:      10
no_session_output_match_events:      2
deterministic_verification_events:   10
verifier_not_applicable_events:      0
verified_true_events:                4
verified_false_events:               6
raw_prompt_text_written:             false
raw_response_text_written:           false
response_text_used_for_verification: true
target_labels_used:                  false
proof_labels_used:                   false
workspace_mutation_enabled:          false
local_accepts_enabled:               false
market_claim_allowed:                false
```

Git-control shadow/audit:

```text
shadow:
  target/nando-wave/real-traffic-shadow/git-control-output-evidence-v1.shadow-report.json

audit:
  target/nando-wave/real-traffic-shadow/git-control-output-evidence-v1.verification-hook-audit.report.json

shadow_total_llm_calls:              1000
shadow_exact_cache_hits:             53
nando_shadow_accepts:                0
verified_safe_accepts:               0
false_accepts:                       0
incremental_savings_over_exact_cache: 0
p99_shadow_score_latency_ns:         217673
synthetic_trace_used:                false

audit_operator_candidate_calls:       12
audit_scoreable_candidate_calls:      12
verification_hook_ready_events:       10
verified_cpu_accept_eligible_events:  0
audit_market_claim_allowed:           false
```

Feedback loop after git-control output evidence:

```text
feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:             402 / 1000
scoreable_candidate_calls:            159 / 1000
verification_hook_ready_events:       123
verified_cpu_accept_eligible_events:  8
verified_cpu_routability_milli:       8
verified_gap_to_80_calls:             792
market_claim_allowed:                 false

git_control route row:
  candidate_events:                   18
  scoreable_payload_events:           12
  verification_hook_ready_events:     10
  verified_cpu_accept_eligible_events: 0
  false_accepts:                      0
  stage:                              verification_hook_ready_waiting_local_accept
  next_action:                        Run local-accept calibration; if no safe policy exists, improve request-side admission or payload features.
```

Claim boundary:

```text
This attaches conservative final-answer command-outcome evidence to the
git_control route and moves it out of missing_verification_hook. It does not
execute git, does not read raw prompt/response into reports, does not mutate
the workspace, does not enable local accepts, and does not prove savings.

The next required debt is a real tool-output/status verifier plus local-accept
calibration. Until then git_control remains REVIEW with 0 verified CPU accepts.
```

## 2026-07-04 - Executor Integration: Git-Control Route Payload/Profile

Verdict:

```text
GIT_CONTROL_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added commands:
  role-binding-real-traffic-git-control-payload-dry-run-v1
  role-binding-real-traffic-git-control-profile-v1

Added route/profile:
  route_key: git_control
  profile_id: route_gap_git_control_profile_v1
```

Git-control request-side payload dry-run:

```text
report:
  target/nando-wave/real-traffic-shadow/git-control-payload-dry-run-v1.report.json

trace:
  target/nando-wave/real-traffic-shadow/git-control-payload-dry-run-v1.trace.jsonl

git_control_candidate_events:         18
payload_ready_events:                 12
payload_built_events:                 12
scoreable_payload_events:             12
profile_registered:                   false
raw_text_written:                     false
response_text_used:                   false
target_labels_used:                   false
proof_labels_used:                    false
workspace_mutation_enabled:           false
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Git-control profile:

```text
package:
  target/nando-wave/real-traffic-shadow/git-control-seed0.nwrb

registry:
  target/nando-wave/real-traffic-shadow/profile-registry-git-control-v1.json

report:
  target/nando-wave/real-traffic-shadow/git-control-profile-v1.report.json

scoreable_payload_events:             12
package_training_requests:            12
edge_count:                           8
package_bytes:                        140
runtime_bytes_estimate:               33000
median_energy_margin:                 1240064
p10_energy_margin:                    906240
unexpected_local_accepts_under_disabled_threshold: 0
workspace_mutation_enabled:           false
local_accepts_enabled_on_real_traffic: false
market_claim_allowed:                 false
```

Feedback loop after git-control profile shadow:

```text
feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:             402 / 1000
scoreable_candidate_calls:            159 / 1000
verification_hook_ready_events:       113
verified_cpu_accept_eligible_events:  8
verified_cpu_routability_milli:       8
verified_gap_to_80_calls:             792
market_claim_allowed:                 false

git_control route row:
  candidate_events:                   18
  scoreable_payload_events:           12
  verification_hook_ready_events:     0
  verified_cpu_accept_eligible_events: 0
  stage:                              scoreable_payload_missing_verification_hook
  next_action:                        Attach response/tool-call evidence and deterministic output verification.
```

Route-gap after registering git-control profile:

```text
catalog:
  target/nando-wave/real-traffic-shadow/route-gap-catalog-git-control-v1.report.json

readiness:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-git-control-v1.report.json

existing_route_candidate_events:      507 / 1000
no_candidate_events:                  493 / 1000
payload_ready_events:                 13
top_payload_ready_family:             serving_ops
```

Claim boundary:

```text
This moves git_control from route-gap into a scoreable registered workspace
command profile. It does not run git, does not mutate the workspace, does not
enable local accepts, and does not prove savings.

The next required debt is git_status_and_command_outcome_verifier_v1. Until that
exists, git_control must remain scoreable_payload_missing_verification_hook.
```

## 2026-07-04 - Executor Integration: Read-Inspect Local Accept Calibration

Verdict:

```text
READ_INSPECT_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-read-inspect-local-accept-calibration-v1
```

Read-inspect local accept calibration:

```text
report:
  target/nando-wave/real-traffic-shadow/read-inspect-local-accept-calibration-v1.report.json

registry:
  target/nando-wave/real-traffic-shadow/profile-registry-read-inspect-v1.json

trace:
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.trace.jsonl

hook_ready_rows:                      9
scored_rows:                          9
label_true_rows:                      1
label_false_rows:                     8
no_score_rows:                        0
safe_policy_found:                    false
best_safe_true_accepts:               0
minimum_true_support:                 3
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Feedback loop after read-inspect calibration:

```text
feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:             384 / 1000
scoreable_candidate_calls:            147 / 1000
verification_hook_ready_events:       113
verified_cpu_accept_eligible_events:  8
verified_cpu_routability_milli:       8
verified_gap_to_80_calls:             792
market_claim_allowed:                 false

read_inspect route row:
  candidate_events:                   27
  scoreable_payload_events:           12
  verification_hook_ready_events:     9
  local_accept_calibration_ran:       true
  local_accept_safe_policy_found:     false
  local_accept_minimum_true_support:  3
  local_accept_support_qualified:     false
  local_accept_best_safe_true_accepts: 0
  verified_cpu_accept_eligible_events: 0
  stage:                              local_accept_calibration_failed
  next_action:                        Improve request-side admission or payload geometry before enabling local accepts.
```

Claim boundary:

```text
This closes the read_inspect calibration integration debt, but it is a red
route gate. The current read_inspect score/readout geometry does not separate
the single verifier-true row from eight verifier-false rows. Do not enable local
accepts for this route and do not count read_inspect as savings.

Verified CPU accepts remain 8/1000 on the fresh default artifact base. Red gate
stays red.
```

## 2026-07-04 - Executor Integration: Metrics-Report Route Payload/Profile/Evidence

Verdict:

```text
METRICS_REPORT_ROUTE_V1_REVIEW_HOOKS_READY_ACCEPTS_DISABLED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added commands:
  role-binding-real-traffic-metrics-report-payload-dry-run-v1
  role-binding-real-traffic-metrics-report-profile-v1
  role-binding-real-traffic-metrics-report-output-evidence-v1
  role-binding-real-traffic-metrics-report-local-accept-calibration-v1

Added route/profile:
  route_key: metrics_report_readout
  profile_id: route_gap_metrics_report_profile_v1

Added verifier:
  deterministic_metrics_report_output_verification
  verification_source:
    codex_session_final_answer_fingerprint_plus_deterministic_numeric_report_field_verifier_v1
```

Metrics-report request-side payload dry-run:

```text
report:
  target/nando-wave/real-traffic-shadow/metrics-report-payload-dry-run-v1.report.json

trace:
  target/nando-wave/real-traffic-shadow/metrics-report-payload-dry-run-v1.trace.jsonl

metrics_report_candidate_events:      55
payload_ready_events:                 42
payload_built_events:                 42
scoreable_payload_events:             42
profile_registered:                   false
raw_text_written:                     false
response_text_used:                   false
target_labels_used:                   false
proof_labels_used:                    false
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Metrics-report profile:

```text
package:
  target/nando-wave/real-traffic-shadow/metrics-report-seed0.nwrb

registry:
  target/nando-wave/real-traffic-shadow/profile-registry-metrics-report-v1.json

report:
  target/nando-wave/real-traffic-shadow/metrics-report-profile-v1.report.json

scoreable_payload_events:             42
package_training_requests:            42
edge_count:                           8
package_bytes:                        140
runtime_bytes_estimate:               33000
median_energy_margin:                 1105920
p10_energy_margin:                    835584
unexpected_local_accepts_under_disabled_threshold: 0
local_accepts_enabled_on_real_traffic: false
```

Metrics-report output evidence:

```text
report:
  target/nando-wave/real-traffic-shadow/metrics-report-output-evidence-v1.report.json

verification audit:
  target/nando-wave/real-traffic-shadow/metrics-report-output-evidence-v1.verification-hook-audit.report.json

output_evidence_matched_events:       32
deterministic_verification_events:    32
verifier_not_applicable_events:       0
verified_true_events:                 18
verified_false_events:                14
candidates_missing_output_evidence:   10
candidates_missing_explicit_verification: 10
verified_cpu_accept_eligible_events:  0
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Metrics-report local accept calibration:

```text
report:
  target/nando-wave/real-traffic-shadow/metrics-report-local-accept-calibration-v1.report.json

hook_ready_rows:                      32
scored_rows:                          32
label_true_rows:                      18
label_false_rows:                     14
safe_policy_found:                    true
best_safe_true_accepts:               2
minimum_true_support:                 3
local_accept_support_qualified:       false

best policy:
  policy_name:                        best_metric_slot_margin_threshold
  threshold:                          393216
  accepts:                            2
  true_accepts:                       2
  false_accepts:                      0

interpretation:
  Real safe score separation exists, but support is too small for promotion.
  Do not enable local accepts from this route yet.
```

Feedback loop after metrics-report evidence:

```text
feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:             384 / 1000
scoreable_candidate_calls:            147 / 1000
verification_hook_ready_events:       113
verified_cpu_accept_eligible_events:  8
verified_cpu_routability_milli:       8
verified_gap_to_80_calls:             792
market_claim_allowed:                 false

metrics_report_readout route row:
  candidate_events:                   55
  scoreable_payload_events:           42
  verification_hook_ready_events:     32
  local_accept_calibration_ran:       true
  local_accept_safe_policy_found:     true
  local_accept_minimum_true_support:  3
  local_accept_support_qualified:     false
  local_accept_best_safe_true_accepts: 2
  verified_cpu_accept_eligible_events: 0
  stage:                              local_accept_calibration_support_insufficient
```

Catalog default refresh:

```text
role-binding-real-traffic-cpu-operator-catalog-v1 now defaults to:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json
  target/nando-wave/real-traffic-shadow/route-gap-catalog-v1.report.json

It no longer silently prefers older conditional/agent-control feedback or
route-gap reports when no explicit paths are passed. For a specific current
route base, pass all report paths explicitly.
```

Route-gap after registering metrics-report:

```text
readiness report:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-metrics-report-v1.report.json

existing_route_candidate_events:      499
no_candidate_events:                  501
payload_ready_events:                 17
top_payload_ready_family:             git_control

next deterministic route candidates:
  git_control
  serving_ops
  retrieval_lookup
```

Claim boundary:

```text
This moves metrics_report_readout from route-gap into a scoreable registered
profile with deterministic output evidence labels. It does not enable local
accepts and does not prove savings.

The route has 18 verifier-true rows and 14 verifier-false rows. Calibration found
a 2-row safe score pocket, but the minimum promotion support is 3 true rows.
All 42 scoreable rows still run under disabled/REVIEW policy. Verified CPU
accepts remain 8/1000 on the fresh default artifact base. Red gate stays red.
```

## 2026-07-04 - Executor Integration: Read-Inspect Output Evidence Verifier

Verdict:

```text
READ_INSPECT_OUTPUT_EVIDENCE_V1_REVIEW_HOOKS_READY_ACCEPTS_DISABLED
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added command:
  role-binding-real-traffic-read-inspect-output-evidence-v1

Added verifier:
  deterministic_read_inspect_output_verification
  verification_source:
    codex_session_final_answer_fingerprint_plus_deterministic_read_only_path_and_excerpt_verifier_v1

Feedback-loop default read_inspect audit path now points to:
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.verification-hook-audit.report.json
```

Read-inspect output evidence:

```text
report:
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.report.json

trace:
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.trace.jsonl

input trace:
  target/nando-wave/real-traffic-shadow/read-inspect-payload-dry-run-v1.trace.jsonl

output_evidence_matched_events:       9
deterministic_verification_events:    8
verifier_not_applicable_events:       1
verified_true_events:                 1
verified_false_events:                8
raw_prompt_text_written:              false
raw_response_text_written:            false
response_text_used_for_verification:  true
target_labels_used:                   false
proof_labels_used:                    false
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Read-inspect shadow and audit after output evidence:

```text
shadow report:
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.shadow-report.json

verification audit:
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.verification-hook-audit.report.json

total_llm_calls:                      1000
exact_cache_hits:                     53
operator_candidate_calls:             12
scoreable_candidate_calls:            12
verification_hook_ready_events:       9
verified_cpu_accept_eligible_events:  0
candidates_missing_output_evidence:   3
candidates_missing_explicit_verification: 3
nando_shadow_accepts:                 0
verified_safe_accepts:                0
false_accepts:                        0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns:          249929
market_claim_allowed:                 false
```

Feedback loop after read_inspect output evidence:

```text
feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json

operator_candidate_calls:             314
scoreable_candidate_calls:            105
verification_hook_ready_events:       81
verified_cpu_accept_eligible_events:  8
verified_cpu_routability_milli:       8
verified_gap_to_80_calls:             792
market_claim_allowed:                 false
```

Claim boundary:

```text
This closes the missing read_only_path_and_excerpt_verifier_v1 integration debt
for read_inspect output evidence. It does not enable read_inspect local accepts
and does not prove savings.

The read_inspect route now has verifier labels:
  true:  1
  false: 8

But all 12 read_inspect scoreable rows still have local accepts disabled, so
verified_cpu_accept_eligible_events remains 0 for this route.

The current default feedback bundle reports 8/1000 verified CPU accepts on its
own artifact base. The historical mixed-v2 bundle remains the stronger 17/1000
snapshot. Do not mix these numbers as a single claim.
```

Next engineering debt:

```text
Calibrate read_inspect local accept only after verifier-true support is large
enough. Singleton true support is not enough to promote a policy.

Next route-gap families remain:
  metrics_report_readout
  retrieval_lookup

Also refresh stale extended feedback/catalog defaults that still point at older
schema artifacts.
```

NANDA structural gate:

```text
task_id:
  read-inspect-output-evidence-v1

packet:
  docs/structural_gates/read-inspect-output-evidence-v1.md

report:
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.nanda.json

verdict:
  VETO

complexity_score:
  89

explanation:
  weak route coherence / weak composite-mode support / task should be split

interpretation:
  proof_shape_debt_not_runtime_failure

Do not use this packet as structural PASS. JSON reports are the numeric
authority for route/profile/evidence measurements.
```

## 2026-07-04 - Executor Integration: Read-Inspect Route Payload/Profile

Verdict:

```text
READ_INSPECT_ROUTE_V1_REVIEW_SCOREABLE_NO_VERIFIER
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added commands:
  role-binding-real-traffic-read-inspect-payload-dry-run-v1
  role-binding-real-traffic-read-inspect-profile-v1

CodexHistoryRouteCatalog now recognizes read_inspect profiles whose
operator_classes contain:
  read_only_inspection
  path_evidence
```

Read-inspect dry-run:

```text
report:
  target/nando-wave/real-traffic-shadow/read-inspect-payload-dry-run-v1.report.json

trace:
  target/nando-wave/real-traffic-shadow/read-inspect-payload-dry-run-v1.trace.jsonl

history window:                       1000 local Codex rows
read_inspect_candidate_events:        27
payload_ready_events:                 12
payload_built_events:                 12
scoreable_payload_events:             12
raw_text_written:                     false
response_text_used:                   false
target_labels_used:                   false
proof_labels_used:                    false
local_accepts_enabled:                false
market_claim_allowed:                 false
```

Read-inspect profile:

```text
package:
  target/nando-wave/real-traffic-shadow/read-inspect-seed0.nwrb

registry:
  target/nando-wave/real-traffic-shadow/profile-registry-read-inspect-v1.json

report:
  target/nando-wave/real-traffic-shadow/read-inspect-profile-v1.report.json

scoreable_payload_events:             12
package_training_requests:            12
edge_count:                           8
package_bytes:                        140
runtime_bytes_estimate:               33000
median_energy_margin:                 1456128
p10_energy_margin:                    1345536
unexpected_local_accepts_under_disabled_threshold: 0
local_accepts_enabled_on_real_traffic: false
market_claim_allowed:                 false
```

Read-inspect shadow and audit:

```text
shadow report:
  target/nando-wave/real-traffic-shadow/read-inspect-payload-dry-run-v1.shadow-report.json

verification audit:
  target/nando-wave/real-traffic-shadow/read-inspect-payload-dry-run-v1.verification-hook-audit.report.json

operator_candidate_calls:             12
scoreable_candidate_calls:            12
verification_hook_ready_events:       0
verified_cpu_accept_eligible_events:  0
candidates_missing_output_evidence:   12
candidates_missing_explicit_verification: 12
shadow_accepts:                       0
false_accepts:                        0
market_claim_allowed:                 false
```

Current route map after registering read_inspect:

```text
route candidate report:
  target/nando-wave/real-traffic-shadow/codex-history-route-candidates-read-inspect-v1.report.json

route-gap catalog:
  target/nando-wave/real-traffic-shadow/route-gap-catalog-read-inspect-v1.report.json

route-gap readiness:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-read-inspect-v1.report.json

forecast:
  target/nando-wave/real-traffic-shadow/cpu-route-forecast-read-inspect-v1.report.json

feedback:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v4.read-inspect-current.report.json

operator catalog:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v5.read-inspect.report.json

existing_route_candidate_events:      489
no_candidate_events:                  511
route_gap_payload_ready_events:       23
top_payload_ready_family:             metrics_report_readout
read_inspect route candidates:        33
read_inspect scoreable payloads:      12
read_inspect stage:                   scoreable_payload_missing_verification_hook
```

Claim boundary:

```text
This is route/payload/profile progress, not verified CPU savings.

The read_inspect profile is scoreable and registered, but its local accept
threshold remains disabled and all 12 scoreable rows are missing output
evidence / explicit verification.

The rebuilt current feedback bundle reports 9 verified CPU accepts because it
uses a fresh route-only forecast artifact set. The previous mixed-v2
feedback bundle remains the historical 17/1000 verified-accept snapshot until
all promoted/audit artifacts are regenerated on one unified route base.
Do not mix these two numbers as a single claim.
```

Next engineering debt:

```text
Build read_only_path_and_excerpt_verifier_v1:
  request path must match a read-only local file/path intent;
  response/tool evidence must include path/excerpt fingerprint;
  no raw prompt/response text is written to reports;
  local accept may be calibrated only after verifier true/false labels exist;
  false_accepts must remain 0.

Next route-gap families after read_inspect moved into existing routes:
  metrics_report_readout: 10 candidates / 6 payload-ready
  retrieval_lookup:      25 candidates / 2 payload-ready
```

NANDA structural gate:

```text
task_id:
  read-inspect-route-v1

packet:
  docs/structural_gates/read-inspect-route-v1.md

report:
  target/nando-wave/real-traffic-shadow/read-inspect-route-v1.nanda.json

verdict:
  VETO

interpretation:
  proof_shape_debt_not_runtime_failure

details:
  complexity_score: 74
  reason: candidate group weak under composite-mode support

Do not use this packet as structural PASS. JSON reports are the numeric
authority for route/profile measurements; the packet is retained as review
evidence until the NANDA proof shape is improved.
```

## 2026-07-04 - Executor Integration: Current Route-Gap Mining After Planning Profile

Verdict:

```text
CURRENT_ROUTE_GAP_MINING_V1_REVIEW_NEXT_ROUTE_READ_INSPECT
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

CodexHistoryRouteCatalog now recognizes profiles whose operator_classes contain:
  project_planning
  state_transition
  route_gap

Those profiles are classified through the planning_next_step route family
instead of being left in the no-candidate zone.
```

Why this matters:

```text
The old route-gap catalog was stale after planning_next_step was added.
It counted planning-shaped calls as route gaps even though the current feedback
loop already accounts for the planning profile separately.

The new current route-gap run uses:
  target/nando-wave/real-traffic-shadow/profile-registry-planning-next-step-v3.json
```

Current route-gap catalog:

```text
report:
  target/nando-wave/real-traffic-shadow/route-gap-catalog-current-v1.report.json

sampled_llm_calls:                   1000
existing_route_candidate_events:     462
no_candidate_events:                 538
top_gap_family:                      answer_or_explain
raw_text_written:                    false
local_accepts_enabled:               false
```

Current route-gap payload readiness:

```text
report:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-current-v1.report.json

existing_route_candidate_events:     462
no_candidate_events:                 538
payload_ready_events:                35
top_payload_ready_family:            read_inspect

read_inspect:
  candidate_events:                  27
  payload_ready_events:              12
  recommended_payload_builder:       read_inspect_request_payload_builder_v1
  recommended_verifier:              read_only_path_and_excerpt_verifier_v1

metrics_report_readout:
  candidate_events:                  10
  payload_ready_events:              6

retrieval_lookup:
  candidate_events:                  25
  payload_ready_events:              2
```

Current CPU operator catalog:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v4.current-route-gap.report.json

current_verified_cpu_accepts:        17
verified_gap_to_80_calls:            783
existing_operator_candidate_calls:   457
no_candidate_calls:                  538
route_gap_payload_ready_events:      35
market_claim_allowed:                false
```

Claim boundary:

```text
This is measurement progress, not verified CPU savings.
No local accepts were enabled by the route-gap mining run.

The next deterministic route should be read_inspect because it is the top
payload-ready no-candidate family after planning is removed from the gap.
Do not rebuild planning as a duplicate route.
```

NANDA structural gate:

```text
task_id:
  route-gap-current-v1

packet:
  docs/structural_gates/route-gap-current-v1.md

report:
  target/nando-wave/real-traffic-shadow/route-gap-current-v1.nanda.json

verdict:
  VETO

interpretation:
  proof_shape_debt_not_runtime_failure

details:
  route_coherence: 1.0
  conflicts: []
  evidence_gaps: []
  reason: candidate rows still weak under composite-mode support

Do not use this packet as structural PASS. JSON reports are the numeric
authority for the route-gap measurement. The packet is retained as review
evidence until the NANDA proof shape is improved.
```

## 2026-07-04 - Executor Integration: Mixed-Map Safe Policy V2

Verdict:

```text
MIXED_SAFE_POLICY_V2_REVIEW_VERIFIED_GAIN
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-mixed-safe-policy-promote-v2

The v2 promotion keeps the same mixed-map `.nwrb` score path and adds a
request-side admission gate:
  mixed_no_goal_control_prompt

Goal/control prompts are forced to fallback before energy threshold selection.
This prevents an unverified meta-goal row from raising the mixed-map runtime
threshold and blocking verified safe rows.
```

V2 promotion:

```text
report:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v2.report.json

request_side_policy_name:             mixed_no_goal_control_prompt
selected_policy_threshold:            393216
selected_policy_accepts:              3
selected_policy_true_accepts:         3
selected_policy_false_accepts:        0
selected_policy_unverified_accepts:   0
request_side_policy_accept_rows:      11
request_side_policy_reject_rows:      3
policy_accept_rows:                   3
policy_accept_verified_true_rows:     3
policy_accept_verified_false_rows:    0
policy_accept_unverified_rows:        0
runtime_acceptance_mismatches:        0
```

V2 shadow:

```text
shadow report:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v2.shadow-report.json

nando_shadow_accepts:                 3
verified_safe_accepts:                3
unverified_shadow_accepts:            0
false_accepts:                        0
incremental_savings_over_exact_cache: 3
p99_shadow_score_latency_ns:          214688
synthetic_trace_used:                 false
```

V2 verification audit:

```text
audit report:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v2.verification-hook-audit.report.json

operator_candidate_calls:             11
scoreable_candidate_calls:            11
verification_hook_ready_events:       11
verified_cpu_accept_eligible_events:  3
false_accepts:                        0
market_claim_allowed:                 true for the route-specific audit only
```

Unified feedback with mixed v2 + agent-control v2 + planning v3:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v3.mixed-v2-agent-control-planning-v3.report.json

total_llm_calls:                      1000
exact_cache_hits:                     53
operator_candidate_calls:             457
scoreable_candidate_calls:            136
verification_hook_ready_events:       100
verified_cpu_accept_eligible_events:  17
verified_cpu_routability_milli:       17
verified_gap_to_80_calls:             783
market_claim_allowed:                 false

mixed-map row:
  candidate_events:                   37
  scoreable_payload_events:           11
  verification_hook_ready_events:     11
  local_accept_best_safe_true_accepts: 4
  local_accept_support_qualified:     true
  verified_cpu_accept_eligible_events: 3
  false_accepts:                      0
```

Operator catalog:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v3.mixed-v2-agent-control-planning-v3.report.json

current_verified_cpu_accepts:         17
verified_gap_to_80_calls:             783
top_catalog_row:                      role_binding_agent_control_seed0
```

Claim boundary:

```text
This is real verified progress, not CPU Routability 80.
The project moved from 16/1000 to 17/1000 verified CPU accepts on the current
1000-call Codex trace window.

The mixed route-specific audit is market-claim-eligible for that route packet,
but the unified market-wide CPU Routability claim remains closed while
verified_cpu_routability_milli is 17.

Conditional branch remains blocked by prompt/score collisions:
  verifier true rows:  17
  verifier false rows: 46
  safe prompt+energy accepts found: 2

No response text, target labels, proof labels, concrete lookup, or manual
local_out_t are used by the serving policy.
```

NANDA structural gate:

```text
task_id:
  mixed-safe-policy-v2

packet:
  docs/structural_gates/mixed-safe-policy-v2.md

report:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v2.nanda.json

verdict:
  VETO

interpretation:
  proof_shape_debt_not_runtime_failure

details:
  route_coherence: 1.0
  conflicts: []
  evidence_gaps: []
  exact_candidate_matches: 16
  reason: candidate rows still weak under composite-mode support

Do not use this packet as a structural PASS. The JSON runtime/shadow/audit
reports are the numeric authority for the route-specific mixed v2 gain.
The NANDA packet is retained as review evidence and must be improved before
any public structural claim.
```

## 2026-07-04 - Executor Integration: Agent-Control Strict Control Forms V2

Verdict:

```text
AGENT_CONTROL_STRICT_CONTROL_FORMS_V2_REVIEW_VERIFIED_GAIN
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added agent-control admission features:
  one_token_lowercase_stop
  stop_uppercase_goal_control

Added request-side policy:
  strict_control_stop_forms

Extended role-binding-real-traffic-feedback-loop-v1 with optional explicit
agent-control artifact paths:
  agent-control admission calibration report
  agent-control safe-policy verification audit report
```

Why this was needed:

```text
The broad agent-control route is unsafe as a local accept policy:
  verifier true rows:  12
  verifier false rows: 112

The previous hard-stop policy was safe but small:
  true_accepts:  3
  false_accepts: 0

The new policy keeps broad agent-control blocked and admits only narrow
control-plane forms: strict hard-stop forms, explicit pause request, uppercase
GOAL stop control, and one-token lowercase stop.
```

V2 calibration:

```text
report:
  target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v2.report.json

hook_ready_rows:          124
label_true_rows:          12
label_false_rows:         112
selected policy candidate:
  strict_control_stop_forms
  accepts:                11
  true_accepts:           11
  false_accepts:          0
  missed_true:            1
  robust_safe:            true
```

V2 promoted shadow:

```text
promoted trace:
  target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v2.trace.jsonl

shadow report:
  target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v2.shadow-report.json

nando_shadow_accepts:                  11
verified_safe_accepts:                 11
unverified_shadow_accepts:             0
false_accepts:                         0
incremental_savings_over_exact_cache:  6
incremental_reduction_vs_exact_cache:  6 milli
p99_shadow_score_latency_ns:           23783
synthetic_trace_used:                  false
```

V2 verification audit:

```text
audit report:
  target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v2.verification-hook-audit.report.json

verification_hook_ready_events:        11
verified_cpu_accept_eligible_events:   11
shadow_accepts:                        11
shadow_false_accepts:                  0
market_claim_allowed:                  true
```

Unified feedback with agent-control v2 + planning v3:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v2.agent-control-planning-v3.report.json

total_llm_calls:                       1000
exact_cache_hits:                      53
operator_candidate_calls:              457
scoreable_candidate_calls:             139
verification_hook_ready_events:        102
verified_cpu_accept_eligible_events:   16
verified_cpu_routability_milli:        16
verified_gap_to_80_calls:              784
market_claim_allowed:                  false

agent-control row:
  candidate_events:                    143
  scoreable_payload_events:            11
  verification_hook_ready_events:      11
  local_accept_best_safe_true_accepts: 11
  local_accept_support_qualified:      true
  verified_cpu_accept_eligible_events: 11
  false_accepts:                       0
```

Operator catalog v2:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v2.agent-control-planning-v3.report.json

current_verified_cpu_accepts:          16
verified_gap_to_80_calls:              784
top_catalog_row:                       role_binding_agent_control_seed0
```

Claim boundary:

```text
This is real verified progress, not CPU Routability 80.
The project moved from 8/1000 to 16/1000 verified CPU accepts on the current
1000-call Codex trace window.

Broad agent-control remains blocked. Only the strict request-side policy is
promoted into the v2 trace.

No market-wide savings claim is allowed from the unified feedback report while
verified_cpu_routability_milli is 16.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: agent-control-strict-control-safe-policy-v2
packet:
  docs/structural_gates/agent-control-strict-control-safe-policy-v2.md
report:
  target/nando-wave/real-traffic-shadow/agent-control-strict-control-safe-policy-v2.nanda.json

Treat VETO as the CPU Routability 80 promotion guard, not as a route failure.
```

## 2026-07-04 - Executor Integration: Planning Next-Step Admission Calibration V1/V3

Verdict:

```text
PLANNING_NEXT_STEP_ADMISSION_CALIBRATION_V1_REVIEW_ROBUST_POLICY_CANDIDATE_FOUND
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

Added:
  role-binding-real-traffic-planning-next-step-admission-calibration-v1

The command calibrates request-side prompt features against
planning_next_step artifact-progress verifier labels without writing raw
prompt text and without enabling local accepts.
```

V2 lesson:

```text
The first v2 admission split looked promising, but it was not enough. On the
full v3 artifact window the direct-action + project-artifact policy admitted:

  accepts:       4
  true_accepts:  2
  false_accepts: 2

The two false collisions were goal-control/meta-workflow prompts, not real
artifact-progress execution prompts.
```

V3 diagnostic result:

```text
hook_ready_rows:              29
rows_with_prompt_features:    29
label_true_rows:              2
label_false_rows:             27
minimum_true_support:         2
robust_safe_policy_found:     true
best_robust_true_accepts:     2
market_claim_allowed:         false
local_accepts_enabled:        false
response_text_used_for_features: false
target_labels_used_for_runtime:  false
proof_labels_used_for_runtime:   false

policy:
  direct_action_project_no_goal_control_no_failure
  accepts:       2
  true_accepts:  2
  false_accepts: 0
  missed_true:   0
  robust_safe:   true
```

Feedback dashboard v3 check:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.planning-v3.report.json

operator_candidate_calls:              337
scoreable_candidate_calls:             128
verification_hook_ready_events:        94
verified_cpu_accept_eligible_events:   8
verified_cpu_routability_milli:        8
verified_gap_to_80_calls:              792
market_claim_allowed:                  false

planning_next_step row:
  candidate_events:                    49
  payload_ready_events:                63
  payload_built_events:                49
  scoreable_payload_events:            49
  verification_hook_ready_events:      29
  local_accept_best_safe_true_accepts: 1
  local_accept_support_qualified:      false
  verified_cpu_accept_eligible_events: 0
```

Interpretation:

```text
The new admission split is a useful request-side diagnostic: it separates both
known artifact-progress true rows from the goal-control false collisions.

It is not a CPU savings improvement. It does not promote planning_next_step,
does not enable local accepts, and does not change verified CPU routability.

Promotion still needs a larger support window and a full shadow/audit path
where verified_safe_accepts increase while false_accepts remain zero.
```

Claim boundary:

```text
Current global verified CPU remains 8/1000.
The gap to the 80% goal remains 792 calls.
This is admission calibration, not market savings.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: planning-next-step-admission-calibration-v1
packet:
  docs/structural_gates/planning-next-step-admission-calibration-v1.md
report:
  target/nando-wave/real-traffic-shadow/planning-next-step-admission-calibration-v1.nanda.json

Treat VETO as a promotion guard: the split is useful but still REVIEW-only.
```

## 2026-07-04 - Executor Integration: Feedback Loop Planning Artifact Args V1

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_PLANNING_ARTIFACT_ARGS_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

role-binding-real-traffic-feedback-loop-v1 now accepts explicit planning
artifact paths after the existing 4 positional arguments:
  planning dry-run report
  planning local-accept calibration report
  planning verification audit report

If these arguments are omitted, the command keeps the old default v1 artifact
paths.
```

Default v1 dashboard check:

```text
operator_candidate_calls: 302
scoreable_candidate_calls: 93
verification_hook_ready_events: 72
verified_cpu_accept_eligible_events: 8
verified_cpu_routability_milli: 8
verified_gap_to_80_calls: 792

planning_next_step row:
  candidate_events: 14
  payload_ready_events: 19
  payload_built_events: 14
  scoreable_payload_events: 14
  verification_hook_ready_events: 7
  local_accept_best_safe_true_accepts: 1
  local_accept_support_qualified: false
  verified_cpu_accept_eligible_events: 0
```

Explicit planning v2 dashboard check:

```text
operator_candidate_calls: 319
scoreable_candidate_calls: 110
verification_hook_ready_events: 87
verified_cpu_accept_eligible_events: 8
verified_cpu_routability_milli: 8
verified_gap_to_80_calls: 792

planning_next_step row:
  candidate_events: 31
  payload_ready_events: 36
  payload_built_events: 31
  scoreable_payload_events: 31
  verification_hook_ready_events: 22
  local_accept_best_safe_true_accepts: 1
  local_accept_support_qualified: false
  verified_cpu_accept_eligible_events: 0
```

Interpretation:

```text
The feedback dashboard can now compare route-gap artifact windows without
overwriting the baseline. The v2 artifact set increases planning_next_step
candidate / scoreable / hook-ready coverage, but does not increase verified CPU
accepts because the support guard still blocks promotion.
```

Claim boundary:

```text
This is dashboard plumbing, not a savings claim.
The default path remains v1-compatible.
The explicit v2 path remains REVIEW because verified CPU stays 8/1000.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: feedback-loop-planning-artifact-args-v1
packet:
  docs/structural_gates/feedback-loop-planning-artifact-args-v1.md
report:
  target/nando-wave/real-traffic-shadow/feedback-loop-planning-artifact-args-v1.nanda.json

Scope:
  The packet checks that optional planning artifacts can change dashboard
  planning coverage while preserving v1 defaults and not promoting local
  accepts.

  Treat VETO as a promotion guard: v2 coverage is stronger, but still not
  support-qualified.
```

## 2026-07-04 - Executor Integration: Planning Next-Step Trace Window V2

Verdict:

```text
PLANNING_NEXT_STEP_TRACE_WINDOW_V2_REVIEW_SUPPORT_STILL_INSUFFICIENT
```

What changed:

```text
No runtime or scoring code changed.

The planning_next_step route was rerun over a wider real Codex history window:
  max_events: 1000 -> 5000

Artifacts were written under v2 names so the v1 baseline remains intact.
```

V1 -> V2 route evidence:

```text
planning candidates:           54 -> 170
payload ready events:          19 -> 36
payload built events:          14 -> 31
scoreable payload events:      14 -> 31

artifact evidence matches:      7 -> 22
verified true events:           1 -> 2
verified false events:          6 -> 20
tool-call fingerprint events:   1 -> 3

verification hook ready:        7 -> 22
verified CPU eligible:          0 -> 0
shadow accepts:                 0 -> 0
shadow false accepts:           0 -> 0
market claim allowed:           false -> false
```

Calibration result:

```text
safe_policy_found: true
best_safe_true_accepts: 1
minimum_true_support_for_promotion: 3
request_side_margin_only_accepts_all_true_without_false: false

The wider trace found a second verifier-true artifact-progress row, but the
current request-side boundary-slot margin policy still safely accepts only one
true row:
  true_accepts: 1
  false_accepts: 0
  missed_true: 1
```

Margin collision diagnostics:

```text
energy_margin:
  min_true_margin: 1213440
  false_rows_at_or_above_min_true_margin: 2
  safe_accepts_all_true_rows: false
  best_safe_true_accepts: 0

min_slot_margin:
  min_true_margin: 328704
  false_rows_at_or_above_min_true_margin: 18
  safe_accepts_all_true_rows: false
  best_safe_true_accepts: 0

marker_slot_margin:
  min_true_margin: 328704
  false_rows_at_or_above_min_true_margin: 18
  safe_accepts_all_true_rows: false
  best_safe_true_accepts: 0

end_slot_margin:
  min_true_margin: 786432
  false_rows_at_or_above_min_true_margin: 8
  safe_accepts_all_true_rows: false
  best_safe_true_accepts: 1
```

Interpretation:

```text
The route is real and growing: wider real traffic increased candidate,
scoreable, hook-ready, and tool-backed evidence counts.

The route is not promotion-ready: safe local accept support remains singleton,
so planning_next_step must stay review-only.

This is now proven not to be fixable by a single request-side margin threshold:
accepting both verifier-true rows would admit false rows under every current
margin family.
```

Claim boundary:

```text
This is not a CPU savings improvement.
It proves better evidence density for planning_next_step, not verified local
replacement of LLM calls.

Current global CPU Routability remains 8/1000 until the feedback dashboard is
fed with a support-qualified route row and verified accepts increase under
false_accepts=0.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: planning-next-step-trace-window-v2
packet:
  docs/structural_gates/planning-next-step-trace-window-v2.md
report:
  target/nando-wave/real-traffic-shadow/planning-next-step-trace-window-v2.nanda.json

Scope:
  The packet checks that v2 increased real evidence while keeping local accept
  promotion blocked by minimum support.

  Treat VETO as a promotion guard and proof-shape debt. It is not a runtime
  failure.
```

Next debt:

```text
planning_next_step_admission_split_v1:
  compare the two verifier-true rows and twenty verifier-false rows to find a
  request-side feature that separates both true rows without admitting false
  rows. Candidate direction: distinguish tool-backed progress intent shape
  before scoring, not by lowering score thresholds.

planning_next_step_feedback_loop_artifact_args_v1:
  avoid hard-wiring only v1 planning reports in the feedback loop; allow a
  named planning artifact set or route-gap source bundle so v2 windows can feed
  the dashboard without overwriting the baseline.
```

## 2026-07-04 - Executor Integration: Feedback Loop Planning Route-Gap Row V1

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs

The feedback loop now auto-loads planning_next_step artifacts:
  planning-next-step-payload-dry-run-v1.report.json
  planning-next-step-local-accept-calibration-v1.report.json
  planning-next-step-artifact-progress-v1.verification-hook-audit.report.json

It appends planning_next_step as an explicit route-gap row even though the base
cpu-route-forecast route list only contains conditional/edit/mixed profiles.
```

New dashboard metrics:

```text
total_llm_calls: 1000
exact_cache_hits: 53
operator_candidate_calls: 302
operator_candidate_coverage_milli: 302
scoreable_candidate_calls: 93
scoreable_candidate_coverage_milli: 93
verification_hook_ready_events: 72
verified_cpu_accept_eligible_events: 8
verified_cpu_routability_milli: 8
verified_gap_to_80_calls: 792
no_candidate_calls: 698
```

Planning route row:

```text
route_key: planning_next_step
candidate_events: 14
scoreable_payload_events: 14
verification_hook_ready_events: 7
local_accept_calibration_ran: true
local_accept_safe_policy_found: true
local_accept_minimum_true_support: 3
local_accept_support_qualified: false
local_accept_best_safe_true_accepts: 1
verified_cpu_accept_eligible_events: 0
false_accepts: 0
stage: local_accept_calibration_support_insufficient
next_action: Collect more verifier-true rows or raise admission quality before promotion; singleton safe policies stay review-only.
```

Interpretation:

```text
This fixes the global dashboard shape. Planning route-gap traffic is now counted
as candidate and scoreable traffic, so the feedback-loop coverage picture no
longer hides the 14 planning candidates.

It does not increase verified CPU accepts. The planning singleton safe policy is
visible, but it is blocked by minimum support:
  local_accept_minimum_true_support: 3
  local_accept_best_safe_true_accepts: 1
```

Claim boundary:

```text
Do not treat local_accept_safe_policy_found=true as promotion-ready. For route
planning_next_step, support-qualified means:
  safe_policy_found=true
  best_safe_true_accepts >= 3
  later promoted shadow/audit has false_accepts=0 and unverified accepts=0
  provider cost exists before savings claims

Current state remains CPU Routability 8/1000, not 80%.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: feedback-loop-planning-route-gap-row-v1
packet:
  docs/structural_gates/feedback-loop-planning-route-gap-row-v1.md
report:
  target/nando-wave/real-traffic-shadow/feedback-loop-planning-route-gap-row-v1.nanda.json

Scope:
  The packet checks that planning route-gap traffic is counted in the feedback
  dashboard while the singleton safe-policy candidate remains blocked.

  NANDA reports no conflicts or evidence gaps, but keeps VETO because the
  candidate-support triads are weak. Treat this as a promotion guard and proof-
  shape debt, not as a runtime failure.
```

Next debt:

```text
planning_next_step_true_label_collection_v1:
  collect more tool-backed planning true rows or improve request-side admission
  until local_accept_support_qualified=true without false accepts.

global_feedback_route_source_unification_v1:
  stop treating route-gap rows as a side-channel. Fold route-gap families into
  the same forecast/catalog layer without allowing them to bypass verifier and
  support gates.
```

## 2026-07-04 - Executor Integration: Planning Next-Step Local Accept Calibration V1

Verdict:

```text
PLANNING_NEXT_STEP_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SAFE_POLICY_CANDIDATE_FOUND
```

What changed:

```text
Added:
  role-binding-real-traffic-planning-next-step-local-accept-calibration-v1

Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

The command evaluates request-side score/readout policies for the
planning_next_step profile against artifact-progress labels. It writes only
fingerprints and margins, does not use target/proof labels, does not use raw
prompt/response/tool output as runtime features, and does not enable local
accepts.
```

Artifacts:

```text
registry:
  target/nando-wave/real-traffic-shadow/profile-registry-planning-next-step-v1.json

trace:
  target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.trace.jsonl

calibration_report:
  target/nando-wave/real-traffic-shadow/planning-next-step-local-accept-calibration-v1.report.json

hook_ready_rows: 7
scored_rows: 7
label_true_rows: 1
label_false_rows: 6
no_score_rows: 0
safe_policy_found: true
best_safe_true_accepts: 1
local_accepts_enabled: false
market_claim_allowed: false
```

Best request-side policy candidate:

```text
policy_name:
  best_boundary_slot_margin_threshold_request_side_only

threshold:
  884736

accepts:
  1

true_accepts:
  1

false_accepts:
  0
```

Interpretation:

```text
This is a useful calibration smoke, not CPU savings.

The planning profile can separate the single tool-backed true row from the six
artifact-verified false rows by the boundary-slot margin in the current 7-label
sample. That is enough to identify a possible request-side policy candidate.

It is not enough to promote local accept:
  true support is only one row;
  provider_cost_microusd is still missing;
  the profile registry still has threshold=i32::MAX;
  shadow accepts remain zero;
  verification-hook audit still reports verified_cpu_accept_eligible_events=0.
```

Claim boundary:

```text
Do not count this as market savings or CPU Routability progress. It is
calibration fuel only. A promoted planning trace would require more non-
synthetic true labels, provider cost, a separate registry/trace promotion,
shadow/audit with false_accepts=0 and unverified_shadow_accepts=0, and rollback
documentation.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: planning-next-step-local-accept-calibration-v1
packet:
  docs/structural_gates/planning-next-step-local-accept-calibration-v1.md
report:
  target/nando-wave/real-traffic-shadow/planning-next-step-local-accept-calibration-v1.nanda.json

Scope:
  The packet checks that a singleton safe-policy candidate is not promoted into
  CPU savings.

  NANDA reports no conflicts, but keeps VETO because candidate support is weak.
  Treat this as proof-shape debt and promotion guard, not as a runtime failure
  and not as structural PASS.
```

Next debt:

```text
planning_next_step_safe_policy_promote_guarded_v1:
  do not implement broad promotion from this singleton support alone. First
  collect more tool-backed planning true rows or add a minimum-support guard
  that refuses promotion below a configured true-label floor.

feedback loop integration:
  planning_next_step currently lives in route-gap artifacts and is not counted
  by the base cpu-route-forecast route list. Add explicit planning route-gap
  rows to the CPU feedback loop before using it as the global CPU Routability
  dashboard.
```

## 2026-07-04 - Executor Integration: Planning Next-Step Artifact Progress V1

Verdict:

```text
PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_V1_REVIEW_TOOL_BACKED_TRUE_LABELS_FOUND
REAL_TRAFFIC_SHADOW_V1_REVIEW
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
```

What changed:

```text
Added:
  role-binding-real-traffic-planning-next-step-artifact-progress-v1

Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

The command scans local Codex rollout JSONL for planning_next_step turns and
attaches tool-call fingerprints when the same turn contains successful
project-progress tools. It writes no raw prompt, raw response, or raw tool
output. It does not use target labels or proof labels.

True-label rule:
  verified_safe_accept=true is allowed only when the planning prompt is
  request-side ready, the turn has successful nando-wave project-progress tool
  evidence, and the final answer does not report failure.

  Current progress kinds:
    progress_apply_patch_nando_wave
    progress_git_commit
    progress_real_traffic_report_generation
    progress_structural_gate_artifact
    progress_generated_target_artifact

  Validation-only tools such as cargo check, cargo clippy, cargo fmt, and
  git diff --check are recorded as evidence, but do not by themselves become a
  true label.
```

Artifacts:

```text
input_trace:
  target/nando-wave/real-traffic-shadow/planning-next-step-output-evidence-v1.trace.jsonl

artifact_progress_trace:
  target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.trace.jsonl

artifact_progress_report:
  target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json

shadow_report:
  target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.shadow-report.json

verification_audit:
  target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.verification-hook-audit.report.json

total_trace_rows: 1000
operator_candidate_calls: 14
scoreable_candidate_calls: 14
session_ids_requested: 4
session_files_scanned: 4
codex_turns_indexed: 7
tool_events_indexed: 1
artifact_evidence_matched_events: 7
no_session_artifact_match_events: 7
verifier_not_applicable_events: 0
verified_true_events: 1
verified_false_events: 6
tool_call_fingerprint_events: 1
raw_prompt_text_written: false
raw_response_text_written: false
tool_outputs_written: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Shadow / audit:

```text
total_llm_calls: 1000
exact_cache_hits: 53
operator_candidate_calls: 14
nando_shadow_accepts: 0
verified_safe_accepts: 0
false_accepts: 0
unverified_shadow_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 217288
synthetic_trace_used: false

verification_hook_ready_events: 7
verified_cpu_accept_eligible_events: 0
candidates_missing_output_evidence: 7
candidates_missing_explicit_verification: 7
candidates_missing_provider_cost: 14
shadow_accepts: 0
shadow_false_accepts: 0
```

Interpretation:

```text
This is the first planning_next_step rung with a tool-backed true verification
label on real Codex traffic. It proves the verifier can distinguish a plain
final-answer claim from project-state progress evidence.

It is still not CPU savings: the planning profile threshold remains disabled,
shadow accepts are zero, provider cost is absent, and seven scoreable planning
rows still lack matching output/artifact evidence.

Current verified CPU remains 8/1000 and the gap to CPU Routability 80 remains
792 calls.
```

Claim boundary:

```text
Tool-backed true labels are calibration fuel, not product savings. A planning
row can count as verified CPU only after a calibrated safe policy enables a
local shadow accept, provider cost is attached, the trace is non-synthetic,
false_accepts=0, unverified accepts=0, and verification-hook audit reports
verified_cpu_accept_eligible_events > 0.
```

Next debt:

```text
planning_next_step_safe_policy_calibration_v1:
  use the artifact-progress trace to search thresholds/admission policy. Do not
  promote if false labels would be accepted, if provider cost is missing for
  market savings, or if the disabled threshold still masks runtime behavior.

missing-output-evidence gap:
  7 of 14 planning rows still lack matching Codex output/artifact evidence.
  Keep them false/unverified until session matching or trace capture improves.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: planning-next-step-artifact-progress-v1
packet:
  docs/structural_gates/planning-next-step-artifact-progress-v1.md
report:
  target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.nanda.json

Scope:
  The packet checks that a tool-backed planning true label is not promoted into
  CPU local accept or market savings.

  NANDA reports no conflicts, but keeps VETO because candidate route coherence
  / composite support is weak. Treat this as proof-shape debt. It is not a
  runtime failure and not a structural PASS.
```

## 2026-07-04 - Executor Integration: Planning Next-Step Output Evidence V1

Verdict:

```text
PLANNING_NEXT_STEP_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
REAL_TRAFFIC_SHADOW_V1_REVIEW
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
```

What changed:

```text
Added:
  role-binding-real-traffic-planning-next-step-output-evidence-v1

Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

The command joins planning_next_step scoreable trace rows with local Codex
final-answer evidence by request fingerprint. It writes only response
fingerprints and explicit deterministic verifier results. It writes no raw
prompt/response text and does not enable local accepts.

Important guard:
  deterministic_planning_next_step_output_verification intentionally refuses
  verified_safe_accept=true from final-answer text alone. True planning accepts
  require a later plan_step_artifact_progress_verifier_v1 that can prove
  project-state progress through artifacts such as reports, checks, diffs, or
  commits.
```

Artifacts:

```text
input_trace:
  target/nando-wave/real-traffic-shadow/planning-next-step-payload-dry-run-v1.trace.jsonl

evidence_trace:
  target/nando-wave/real-traffic-shadow/planning-next-step-output-evidence-v1.trace.jsonl

evidence_report:
  target/nando-wave/real-traffic-shadow/planning-next-step-output-evidence-v1.report.json

shadow_report:
  target/nando-wave/real-traffic-shadow/planning-next-step-output-evidence-v1.shadow-report.json

verification_audit:
  target/nando-wave/real-traffic-shadow/planning-next-step-output-evidence-v1.verification-hook-audit.report.json

total_trace_rows: 1000
operator_candidate_calls: 14
scoreable_candidate_calls: 14
session_ids_requested: 4
session_files_scanned: 4
codex_turns_indexed: 7
output_evidence_matched_events: 7
no_session_output_match_events: 7
deterministic_verification_events: 7
verifier_not_applicable_events: 0
verified_true_events: 0
verified_false_events: 7
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_verification: true
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Shadow / audit:

```text
total_llm_calls: 1000
exact_cache_hits: 53
operator_candidate_calls: 14
nando_shadow_accepts: 0
verified_safe_accepts: 0
false_accepts: 0
unverified_shadow_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 212160
synthetic_trace_used: false

verification_hook_ready_events: 7
verified_cpu_accept_eligible_events: 0
candidates_missing_output_evidence: 7
candidates_missing_explicit_verification: 7
candidates_missing_provider_cost: 14
shadow_accepts: 0
shadow_false_accepts: 0
```

Interpretation:

```text
This closes half of the planning_next_step verification-hook debt: the route now
has real response fingerprints and explicit false verifier labels for matched
Codex turns. The other half remains open: seven rows still lack matching output
evidence, no row has artifact-progress verification, provider costs are absent,
and the profile threshold remains disabled, so verified CPU accepts stay zero.

Current verified CPU remains 8/1000 and the gap to CPU Routability 80 remains
792 calls.
```

Claim boundary:

```text
Output evidence is not savings. A planning row can count as verified CPU only
after artifact-progress verification marks verified_safe_accept=true, local
shadow accept is enabled by a calibrated safe policy, provider cost is attached,
the trace is non-synthetic, and false_accepts/unverified accepts remain zero.
```

Next debt:

```text
plan_step_artifact_progress_verifier_v1:
  verify planning_next_step by real project-state progress, not by response
  prose. Candidate evidence sources: report JSON timestamps/fingerprints,
  generated structural-gate packets, git diff/commit facts, cargo/check/clippy
  artifacts, and explicit tool-call fingerprints when available.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: planning-next-step-output-evidence-v1
packet:
  docs/structural_gates/planning-next-step-output-evidence-v1.md
report:
  target/nando-wave/real-traffic-shadow/planning-next-step-output-evidence-v1.nanda.json

Scope:
  The packet checks that output evidence is attached without converting it into
  verified CPU savings.

  After switching to JSON-field evidence anchors, NANDA reports no conflicts,
  but keeps VETO because candidate route coherence / composite support is weak.
  Treat this as proof-shape debt. It is not a runtime failure and not a
  structural PASS.
```

## 2026-07-04 - Executor Integration: Planning Next-Step Profile V1

Verdict:

```text
PLANNING_NEXT_STEP_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED
REAL_TRAFFIC_SHADOW_V1_REVIEW
VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS
```

What changed:

```text
Added:
  role-binding-real-traffic-planning-next-step-profile-v1

Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

The command compiles the request-side planning_next_step dry-run payloads into
a real `.nwrb` package and writes a serving registry overlay. The profile is
registered, but its threshold is i32::MAX, so shadow can measure score/margins
while local accepts stay disabled until a deterministic verifier exists.
```

Artifacts:

```text
package:
  target/nando-wave/real-traffic-shadow/planning-next-step-seed0.nwrb

overlay_registry:
  target/nando-wave/real-traffic-shadow/profile-registry-planning-next-step-v1.json

profile_report:
  target/nando-wave/real-traffic-shadow/planning-next-step-profile-v1.report.json

shadow_report:
  target/nando-wave/real-traffic-shadow/planning-next-step-profile-v1.shadow-report.json

verification_audit:
  target/nando-wave/real-traffic-shadow/planning-next-step-profile-v1.verification-hook-audit.report.json

trace_rows_read: 1000
scoreable_payload_events: 14
package_training_requests: 14
edge_count: 8
package_bytes: 140
runtime_bytes_estimate: 33000
changed_edges: 155
positive_updates: 608
negative_updates: 672
threshold: 2147483647
positive_margin_rows: 14
strict_ordered_pass_rows: 14
unexpected_local_accepts_under_disabled_threshold: 0
median_energy_margin: 1106944
p10_energy_margin: 873472
min_energy_margin: 873472
median_min_slot_margin: 366592
p10_min_slot_margin: 316416
min_slot_margin: 287744
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled_on_real_traffic: false
market_claim_allowed: false
```

Shadow / audit:

```text
operator_candidate_calls: 14
nando_shadow_accepts: 0
verified_safe_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 213476

verification_hook_ready_events: 0
verified_cpu_accept_eligible_events: 0
candidates_missing_output_evidence: 14
candidates_missing_explicit_verification: 14
```

Interpretation:

```text
This closes the profile-missing gap for planning_next_step. The runtime now
loads and scores the planning payload path, producing strong positive margins,
but the disabled threshold keeps every row in fallback_to_llm. This is scoring
telemetry, not savings.

Current verified CPU remains 8/1000 and the gap to CPU Routability 80 remains
792 calls. The next engineering debt is plan_step_artifact_progress_verifier_v1
plus safe-policy calibration before any threshold can be lowered.
```

Claim boundary:

```text
Profile-ready is not verified CPU. Planning-next-step contributes to CPU
Routability only after deterministic output/artifact evidence verifies the
predicted transition, verified_safe_accepts > 0, provider cost is attached,
and false_accepts remain zero.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: planning-next-step-profile-v1
packet:
  docs/structural_gates/planning-next-step-profile-v1.md
report:
  target/nando-wave/real-traffic-shadow/planning-next-step-profile-v1.nanda.json

Scope:
  The packet checks that the planning_next_step profile is registered and
  scored while the disabled threshold prevents local accepts and market claims.

  NANDA found no role conflicts, but VETOs the packet because the exact
  candidate triads are weak under composite-mode support. Treat this as
  proof-shape debt, not as runtime failure and not as structural PASS.
```

## 2026-07-03 - Executor Integration: Planning Next-Step Payload Dry-Run V1

Verdict:

```text
PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING
REAL_TRAFFIC_SHADOW_V1_REVIEW
VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS
```

What changed:

```text
Added:
  role-binding-real-traffic-planning-next-step-payload-dry-run-v1

Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

The command takes no-candidate real Codex prompts whose route-gap family is
planning_next_step and builds request-side active_fringe/slots from prompt text
only. It writes fingerprints, lane/impulse payloads, and counts; it does not
write raw prompt text, response text, target labels, proof labels, or verified
accepts.
```

Artifacts:

```text
dry_run_trace:
  target/nando-wave/real-traffic-shadow/planning-next-step-payload-dry-run-v1.trace.jsonl

dry_run_report:
  target/nando-wave/real-traffic-shadow/planning-next-step-payload-dry-run-v1.report.json

shadow_report:
  target/nando-wave/real-traffic-shadow/planning-next-step-payload-dry-run-v1.shadow-report.json

verification_audit:
  target/nando-wave/real-traffic-shadow/planning-next-step-payload-dry-run-v1.verification-hook-audit.report.json

sampled_llm_calls: 1000
planning_next_step_candidate_events: 54
payload_ready_events: 19
payload_built_events: 14
scoreable_payload_events: 14
builder_rejected_events: 5
readiness_rejected_events: 35
profile_registered: false
shadow_score_ready: false
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Shadow / audit:

```text
operator_candidate_calls: 14
exact_cache_hits: 53
nando_shadow_accepts: 0
verified_safe_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 4151
synthetic_trace_used: false

verification_hook_ready_events: 0
verified_cpu_accept_eligible_events: 0
candidates_missing_output_evidence: 14
candidates_missing_explicit_verification: 14
```

Interpretation:

```text
This closes the next request-side bridge for the largest payload-ready
route-gap family. The 14 scoreable payloads are not CPU savings yet: the
planning profile is not registered and the deterministic plan/artifact
verifier is missing, so shadow correctly stays REVIEW with zero accepts.

Current verified CPU remains 8/1000 and the gap to CPU Routability 80 remains
792 calls. The next engineering debt is a planning-next-step .nwrb profile plus
plan_step_artifact_progress_verifier_v1.
```

Claim boundary:

```text
Payload is not savings. Planning-next-step contributes to CPU Routability only
after a registered profile scores the request, deterministic output/artifact
evidence verifies the predicted transition, verified_safe_accepts > 0, and
false_accepts remain zero.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: planning-next-step-payload-dry-run-v1
packet:
  docs/structural_gates/planning-next-step-payload-dry-run-v1.md
report:
  target/nando-wave/real-traffic-shadow/planning-next-step-payload-dry-run-v1.nanda.json

Scope:
  The packet checks that planning_next_step payloads are built from request-side
  signals while local accepts and market claims stay disabled.

  NANDA found no role conflicts, but VETOs the packet because the exact
  candidate triads are weak under composite-mode support. Treat this as
  proof-shape debt, not as runtime failure and not as structural PASS.
```

## 2026-07-03 - Executor Integration: Route-Gap Payload Readiness V1

Verdict:

```text
ROUTE_GAP_PAYLOAD_READINESS_V1_REVIEW_READY_FAMILIES_FOUND
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-route-gap-payload-readiness-v1

Updated:
  role-binding-real-traffic-cpu-operator-catalog-v1

The new readiness command inspects no-candidate real Codex prompts and counts
which route-gap families have enough request-side signals to attempt a future
payload builder. It writes fingerprints/features/counts only; no raw prompt
text, response text, target labels, proof labels, or local accepts are used.

The CPU operator catalog now consumes this readiness report when present, so
no-candidate families are ranked by real payload-builder readiness, not only by
traffic volume.
```

Artifacts:

```text
readiness_report:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1.report.json

catalog_report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

sampled_llm_calls: 1000
existing_route_candidate_events: 408
no_candidate_events: 592
route_gap_payload_ready_events: 54
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Top route-gap readiness:

```text
planning_next_step:
  candidates: 54
  payload_ready: 19
  readiness: medium_state_transition_candidate
  next builder: goal_state_transition_payload_builder_v1
  next verifier: plan_step_artifact_progress_verifier_v1

read_inspect:
  candidates: 27
  payload_ready: 12
  readiness: medium_read_only_tool_candidate
  next builder: read_inspect_request_payload_builder_v1
  next verifier: read_only_path_and_excerpt_verifier_v1

metrics_report_readout:
  candidates: 10
  payload_ready: 6
  readiness: medium_structured_readout_candidate
  next builder: metrics_report_payload_builder_v1
  next verifier: numeric_report_field_verifier_v1

answer_or_explain:
  candidates: 215
  payload_ready: 0
  blocker: missing_evidence_signal + missing_verifier_signal
```

Interpretation:

```text
This is the first measured bridge from no-candidate traffic into request-side
payload-builder work. It does not change verified CPU accepts: current verified
CPU remains 8/1000 and the gap to CPU Routability 80 remains 792 calls.

The next honest build target is planning_next_step because it has the largest
payload-ready no-candidate set. read_inspect is smaller but cleaner for a
strict deterministic verifier.
```

Claim boundary:

```text
Readiness is not savings. A route-gap family contributes to CPU Routability
only after a real payload builder emits active_fringe/slots, deterministic
verification attaches output/tool/artifact evidence, shadow accepts are
verified, and false_accepts remain zero.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: route-gap-payload-readiness-v1
packet:
  docs/structural_gates/route-gap-payload-readiness-v1.md
report:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1.nanda.json

Scope:
  The packet checks that route-gap payload readiness is measured from
  request-side signals while local accepts and market claims stay disabled.

  NANDA found no role conflicts, but VETOs the packet because the exact
  candidate triads are weak under composite-mode support. Treat this as
  proof-shape debt, not as runtime failure and not as structural PASS.
```

## 2026-07-03 - Executor Integration: CPU Operator Catalog V1

Verdict:

```text
CPU_OPERATOR_CATALOG_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-cpu-operator-catalog-v1

Updated:
  crates/nando-cli/src/role_binding_runtime_cmd.rs
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

The new command merges:
  feedback-loop route stages
  route-gap no-candidate families

into one machine-readable CPU operator worklist. It ranks existing routed
profiles and no-candidate route families by candidate volume, scoreable payload
readiness, verification-hook readiness, verified CPU accepts, false accepts,
and route readiness.
```

Catalog artifact:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json

total_llm_calls: 1000
exact_cache_hits: 53
existing_operator_candidate_calls: 408
no_candidate_calls: 592
route_gap_payload_ready_events: 54
current_verified_cpu_accepts: 8
target_verified_cpu_accepts: 800
verified_gap_to_80_calls: 792
market_claim_allowed: false
```

Top current routed profiles:

```text
role_binding_conditional_branch_seed0:
  candidates: 150
  scoreable: 55
  hook_ready: 40
  verified_cpu: 2

role_binding_agent_control_seed0:
  candidates: 143
  scoreable: 3
  hook_ready: 3
  verified_cpu: 3

role_binding_mixed_map_seed0:
  candidates: 37
  scoreable: 14
  hook_ready: 13
  verified_cpu: 2

role_binding_edit_marker_length_seed0:
  candidates: 78
  scoreable: 10
  hook_ready: 9
  verified_cpu: 1
```

Top no-candidate families:

```text
answer_or_explain:
  candidates: 215
  readiness: low_needs_knowledge_evidence
  builder: question_shape_payload_builder_v1
  verifier: grounded_answer_evidence_verifier_v1

project_context_dialogue:
  candidates: 211
  readiness: low_stateful_dialogue_candidate
  builder: active_project_state_payload_builder_v1
  verifier: workspace_artifact_or_goal_state_verifier_v1

planning_next_step:
  candidates: 54
  payload_ready: 19
  readiness: medium_state_transition_candidate
  builder: goal_state_transition_payload_builder_v1
  verifier: plan_step_artifact_progress_verifier_v1

read_inspect:
  candidates: 27
  payload_ready: 12
  readiness: medium_read_only_tool_candidate
  builder: read_inspect_request_payload_builder_v1
  verifier: read_only_path_and_excerpt_verifier_v1
```

Interpretation:

```text
The high-volume no-candidate zone is real, but answer_or_explain and
project_context_dialogue cannot become CPU accepts without grounded evidence.
The next honest route expansion should target a deterministic medium-readiness
family, likely planning_next_step or read_inspect, or improve safe admission on
existing conditional/agent-control profiles.
```

Claim boundary:

```text
The catalog writes no raw prompt/response text, enables no local accepts, and
does not change verified CPU routability. It is an operator worklist, not a
savings claim.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: cpu-operator-catalog-v1
packet:
  docs/structural_gates/cpu-operator-catalog-v1.md
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.nanda.json

Scope:
  The packet checks that the CPU operator catalog merges existing routed
  profiles with route-gap no-candidate families while keeping local accepts and
  market claims disabled.

  NANDA found no role conflicts after the source/report/output roles were
  split, but still VETOs the packet because most exact catalog triads are weak
  under composite-mode support. Treat this as proof-shape debt, not as a
  runtime safety failure and not as a structural PASS.
```

## 2026-07-03 - Executor Integration: Feedback Loop CLI Audit

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
CLI_INTEGRATION_AUDIT_PASS
```

What changed:

```text
Updated:
  crates/nando-cli/src/main.rs
  crates/nando-cli/src/help.rs

The feedback-loop command was already implemented and wired:
  feedback_route_stage(...)
  feedback_route_next_action(...)
  run_role_binding_real_traffic_feedback_loop_v1(...)
  main.rs dispatch
  help.rs command listing

The stale CLI usage text made this look incomplete because it showed only the
base 4 reports. The usage/help now states that conditional, agent-control, and
mixed route reports are auto-loaded from default artifact paths when present.
```

Current feedback-loop artifact:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-conditional-agent-control-v1.report.json

total_llm_calls: 1000
exact_cache_hits: 53
operator_candidate_calls: 408
scoreable_candidate_calls: 82
verification_hook_ready_events: 65
verified_cpu_accept_eligible_events: 8
verified_cpu_routability_milli: 8
verified_gap_to_80_calls: 792
false_accepts: 0
```

Route ladder:

```text
conditional:
  candidates: 150
  scoreable: 55
  hook_ready: 40
  verified_cpu: 2

agent_control:
  candidates: 143
  scoreable: 3
  hook_ready: 3
  verified_cpu: 3

edit:
  candidates: 78
  scoreable: 10
  hook_ready: 9
  verified_cpu: 1

mixed:
  candidates: 37
  scoreable: 14
  hook_ready: 13
  verified_cpu: 2
```

Claim boundary:

```text
No new accepts were created by this audit. CPU Routability 80 remains open:
8/1000 verified CPU accepts, 792 verified accepts short of the target.
The next real engineering work is still route expansion / verifier evidence,
not threshold relaxation.
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: feedback-loop-cli-audit-v1
packet:
  docs/structural_gates/feedback-loop-cli-audit-v1.md

Scope:
  The packet checks that the feedback-loop CLI command is wired, route-specific
  reports are auto-loaded from default artifact paths, and CPU Routability 80 is
  still not achieved.

  NANDA VETOs the current packet because exact numeric candidate triads remain
  weak under composite-mode support. Treat this as proof-shape debt, not as a
  runtime safety failure and not as a structural PASS.
```

## 2026-07-03 - Executor Integration: Mixed Evidence Index Hardening

Verdict:

```text
MIXED_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
MIXED_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY
REAL_TRAFFIC_SHADOW_V1_PASS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Updated:
  build_codex_session_output_evidence_index

The Codex session evidence index now has a conservative fallback for older or
alternate session JSONL rows: when a pending user_message is followed by
task_complete.last_agent_message, the index may attach that final answer as
output evidence, but only if the pending request fingerprint is in the wanted
real-traffic set.

It still writes no raw prompt/response text, does not enable local accepts, and
does not use target/proof labels for runtime.
```

Mixed route finding:

```text
calibration:
  hook_ready_rows: 13
  label_true_rows: 10
  label_false_rows: 3
  best_safe_true_accepts: 4
  best_energy_margin_threshold: 393216

promotion:
  selected_policy_name: market_safe_energy_margin_threshold
  selected_policy_threshold: 466944
  policy_accept_rows: 2
  policy_accept_verified_true_rows: 2
  policy_accept_verified_false_rows: 0
  policy_accept_unverified_rows: 0
```

Interpretation:

```text
The lower 393216 threshold is not promoted because it touches one scoreable
mixed row without explicit output evidence. That row remains unverified after
the session-index fallback, so the firewall correctly keeps the promoted
threshold at 466944.

This preserves the claim boundary: no fake accept, no inferred response, no
market savings from calibration labels.
```

Latest mixed shadow/audit:

```text
shadow report:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v1.shadow-report.json

audit report:
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v1.verification-hook-audit.report.json

total_llm_calls: 1000
exact_cache_hits: 53
operator_candidate_calls: 14
nando_shadow_accepts: 2
verified_safe_accepts: 2
false_accepts: 0
unverified_shadow_accepts: 0
incremental_reduction_vs_exact_cache_milli: 2
p99_shadow_score_latency_ns: 271120
verification_hook_ready_events: 13
verified_cpu_accept_eligible_events: 2
route-local market_claim_allowed: true
```

Feedback-loop status after rerun:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-conditional-agent-control-v1.report.json

total_llm_calls: 1000
operator_candidate_calls: 408
scoreable_candidate_calls: 82
verification_hook_ready_events: 65
verified_cpu_accept_eligible_events: 8
verified_cpu_routability_milli: 8
verified_gap_to_80_calls: 792
overall market_claim_allowed: false
```

Checks:

```text
cargo fmt --check
cargo check -p nando-cli
cargo clippy -p nando-cli -- -D warnings
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: mixed-evidence-index-hardening-v1
packet:
  docs/structural_gates/mixed-evidence-index-hardening-v1.md

Scope:
  The packet checks mixed evidence-index hardening, the mixed route-local
  promoted threshold, and the still-red overall CPU Routability 80 boundary.

  NANDA VETOs the current packet because the exact numeric candidate triads are
  weak under composite-mode support. Treat this as proof-shape debt, not as a
  runtime safety failure and not as a structural PASS.
```

Claim boundary:

```text
This hardens the real-traffic evidence join, but it does not improve total CPU
routability. The goal remains open at 8/1000 verified CPU accepts. The next
real route work must add deterministic output/tool-call evidence for currently
unverified scoreable rows or build new request-side payload/verifier pairs for
high-frequency route-gap families.
```

## 2026-07-03 - Executor Integration: Conditional Safe Policy Promotion

Verdict:

```text
CONDITIONAL_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY
REAL_TRAFFIC_SHADOW_V1_PASS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-conditional-safe-policy-promote-v1

Updated:
  role-binding-real-traffic-feedback-loop-v1

The new command builds a separate conditional safe-policy registry and trace.
It does not promote the broad conditional route. Rows rejected by the
request-side admission gate have nando_shadow_request removed, so broad
conditional false rows stay fallback-only.
```

Selected admission/readout:

```text
request-side policy:
  conditional_gate_terms_prompt_len_ge_300

runtime acceptance policy:
  energy_threshold_only

selected positive threshold:
  8192

selection rule:
  offline evidence may choose the threshold;
  serving sees only request-side prompt features plus score >= threshold.
```

Promotion artifact:

```text
registry:
  target/nando-wave/real-traffic-shadow/profile-registry-conditional-safe-policy-v1.json

trace:
  target/nando-wave/real-traffic-shadow/conditional-safe-policy-v1.trace.jsonl

report:
  target/nando-wave/real-traffic-shadow/conditional-safe-policy-v1.report.json

conditional_candidate_rows: 79
scoreable_candidate_calls: 79
request_side_policy_accept_rows: 55
runtime_policy_accept_rows: 2
runtime_policy_verified_true_rows: 2
runtime_policy_verified_false_rows: 0
runtime_policy_unverified_rows: 0
runtime_acceptance_mismatches: 0
raw_prompt_text_written: false
raw_response_text_written: false
target_labels_used_for_runtime: false
proof_labels_used_for_runtime: false
```

Shadow/audit result:

```text
shadow report:
  target/nando-wave/real-traffic-shadow/conditional-safe-policy-v1.shadow-report.json

audit report:
  target/nando-wave/real-traffic-shadow/conditional-safe-policy-v1.verification-hook-audit.report.json

total_llm_calls: 1000
exact_cache_hits: 53
operator_candidate_calls: 55
nando_shadow_accepts: 2
verified_safe_accepts: 2
unverified_shadow_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 2
p99_shadow_score_latency_ns: 282856
verification_hook_ready_events: 40
verified_cpu_accept_eligible_events: 2
route-local market_claim_allowed: true
```

Feedback-loop after conditional safe-policy audit:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-conditional-agent-control-v1.report.json

total_llm_calls: 1000
operator_candidate_calls: 408
scoreable_candidate_calls: 82
verification_hook_ready_events: 65
verified_cpu_accept_eligible_events: 8
verified_cpu_routability_milli: 8
verified_gap_to_80_calls: 792
overall market_claim_allowed: false

conditional route:
  stage: verified_cpu_accept_eligible
  candidate_events: 150
  scoreable_payload_events: 55
  verification_hook_ready_events: 40
  verified_cpu_accept_eligible_events: 2
  false_accepts: 0
```

Structural gate:

```text
nanda_structural_gate: VETO
task_id: conditional-safe-policy-v1
packet:
  docs/structural_gates/conditional-safe-policy-v1.md

Scope:
  The packet checks the route-local safety core for the conditional safe policy:
  request-side admitted gate rows, positive threshold 8192, 2 verified true
  accepts, zero false rows, zero unverified rows, zero runtime acceptance
  mismatches, and zero shadow false/unverified accepts.

  NANDA reports no conflicts, no evidence gaps, no foreign_pull, and
  route_coherence=1.0, but VETOs the packet because most exact-match numeric
  candidate triads are weak under composite-mode support. Treat this as a gate
  formatting/proof-shape debt, not as a route-local runtime safety PASS.
```

Claim boundary:

```text
This is a tiny conditional route-local savings proof, not CPU Routability 80.
The broad conditional route is still unsafe by calibration: 17 verifier-true
rows and 46 verifier-false rows under the broad hook-ready set. The overall
feedback loop remains REVIEW because verified CPU routability is 8/1000, not
800/1000.

Do not use route-local market_claim_allowed=true as a broad market claim. It
only applies to the 2 admitted conditional rows in this non-synthetic trace.
```

## 2026-07-03 - Executor Integration: Agent-Control Hard-Stop Safe Policy Promotion

Verdict:

```text
AGENT_CONTROL_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY
REAL_TRAFFIC_SHADOW_V1_PASS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-agent-control-safe-policy-promote-v1

Updated:
  role-binding-real-traffic-agent-control-admission-calibration-v1
  role-binding-real-traffic-feedback-loop-v1

The new command builds a separate request-side-admitted hard-stop trace. It
does not promote the broad agent-control .nwrb profile. Rows rejected by the
selected request-side policy have nando_shadow_request removed, so the shadow
runtime cannot accidentally accept the broad unsafe route.
```

Selected admission policy:

```text
policy:
  hard_stop_exclamation_caps_or_one_token

definition:
  intent_stop
  has_ostanov_word
  has_exclamation
  tokens <= 3
  no work/action words
  and (tokens <= 1 or all_capsish)

calibration report:
  target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v1.report.json

hook_ready_rows: 124
rows_with_prompt_features: 124
label_true_rows: 12
label_false_rows: 112
policy_accepts: 3
policy_true_accepts: 3
policy_false_accepts: 0
missed_true: 9
robust_safe: true
```

Promotion artifact:

```text
trace:
  target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v1.trace.jsonl

report:
  target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v1.report.json

policy_accept_rows: 3
policy_accept_verified_true_rows: 3
policy_accept_verified_false_rows: 0
policy_accept_unverified_rows: 0
runtime_acceptance_mismatches: 0
profile_acceptance_policy_changed: false
broad_agent_control_profile_promoted: false
raw_prompt_text_written: false
raw_response_text_written: false
target_labels_used_for_runtime: false
proof_labels_used_for_runtime: false
```

Shadow/audit result:

```text
shadow report:
  target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v1.shadow-report.json

audit report:
  target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v1.verification-hook-audit.report.json

total_llm_calls: 1000
exact_cache_hits: 53
operator_candidate_calls: 3
nando_shadow_accepts: 3
verified_safe_accepts: 3
unverified_shadow_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 3
p99_shadow_score_latency_ns: 25829
verified_cpu_accept_eligible_events: 3
route-local market_claim_allowed: true
```

Feedback-loop after safe-policy audit:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-agent-control-v1.report.json

total_llm_calls: 1000
operator_candidate_calls: 408
scoreable_candidate_calls: 106
verification_hook_ready_events: 88
verified_cpu_accept_eligible_events: 6
verified_cpu_routability_milli: 6
verified_gap_to_80_calls: 794
overall market_claim_allowed: false

agent-control route:
  stage: verified_cpu_accept_eligible
  payload_ready_events: 143
  scoreable_payload_events: 3
  verification_hook_ready_events: 3
  verified_cpu_accept_eligible_events: 3
  false_accepts: 0
```

Structural gate:

```text
nanda_structural_gate: PASS
task_id: agent-control-hard-stop-safe-policy-v1
packet:
  docs/structural_gates/agent-control-hard-stop-safe-policy-v1.md

Scope:
  The gate checks the route-local safety core for the hard-stop safe policy:
  selected policy, 3 verified true rows, zero false rows, zero unverified rows,
  zero runtime acceptance mismatches, and zero shadow false/unverified accepts.
  The wider CPU Routability 80 claim remains outside this PASS and stays red.
```

Claim boundary:

```text
This is a tiny hard-stop route-local savings proof, not CPU Routability 80.
The broad agent-control route is still unsafe unless split/admission proves
zero false accepts. The overall feedback loop remains REVIEW because verified
CPU routability is 6/1000, not 800/1000.

Do not use the route-local market_claim_allowed=true as a broad market claim.
It only applies to the 3 admitted hard-stop rows in this non-synthetic trace.
```

## 2026-07-03 - Executor Integration: Agent-Control Admission Calibration

Verdict:

```text
AGENT_CONTROL_ADMISSION_CALIBRATION_V1_REVIEW_ROBUST_POLICY_CANDIDATE_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-agent-control-admission-calibration-v1

The command searches request-side agent-control admission policies against the
evidence-enriched real Codex trace. It reads request text at analysis time, but
writes only fingerprints, feature booleans, policy counts, and labels. It
writes no raw prompt text and no raw response text. It enables no local accepts.
```

Calibration result:

```text
input_trace:
  target/nando-wave/real-traffic-shadow/agent-control-output-evidence-v1.trace.jsonl

report:
  target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v1.report.json

hook_ready_rows: 124
rows_with_prompt_features: 124
label_true_rows: 12
label_false_rows: 112
minimum_true_support: 3
robust_safe_policy_found: true
best_robust_true_accepts: 3
```

Safe candidate:

```text
hard_stop_exclamation_len_le_3:
  accepts: 3
  true_accepts: 3
  false_accepts: 0
  missed_true: 9
  robust_safe: true

hard_stop_exclamation_len_le_4:
  accepts: 3
  true_accepts: 3
  false_accepts: 0
  missed_true: 9
  robust_safe: true
```

Feedback-loop after calibration:

```text
agent-control route:
  local_accept_calibration_ran: true
  local_accept_safe_policy_found: true
  local_accept_best_safe_true_accepts: 3
  stage: false_accepts_block_local_policy
  false_accepts: 112
  verified_cpu_accept_eligible_events: 0
```

Structural gate:

```text
nanda_structural_gate: PASS
task_id: agent-control-admission-calibration-v1
packet:
  target/nando-wave/structural-gates/agent-control-admission-calibration-v1.md

Scope:
  The gate checks that the hard-stop safe candidate stays separate from the
  still-blocked broad agent-control profile.
```

Claim boundary:

```text
This is not a market savings result.
The calibration found a tiny request-side safe admission candidate for hard
stop/exclamation rows, but the broad agent-control profile is still unsafe and
must not be promoted.

The next step is a separate request-side-admitted shadow trace for the hard-stop
candidate, with provider cost, false_accepts=0, unverified_shadow_accepts=0,
and NANDA claim-boundary gate. Do not count the 3 rows until that promotion
trace and audit pass.
```

## 2026-07-03 - Executor Integration: Agent-Control Output Evidence + False-Accept Blocker

Verdicts:

```text
AGENT_CONTROL_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
REAL_TRAFFIC_SHADOW_V1_REVIEW
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-agent-control-output-evidence-v1

Updated:
  role-binding-real-traffic-feedback-loop-v1

The new evidence command joins agent-control dry-run trace rows with local
Codex session final-answer fingerprints and applies a deterministic
control-plane verifier. It writes no raw prompt text and no raw response text.

The feedback loop now reads the agent-control dry-run report and the
agent-control verification audit, so the control route participates in the
main CPU Routability ladder instead of living as a side report.
```

Agent-control output-evidence join:

```text
input_trace:
  target/nando-wave/real-traffic-shadow/agent-control-payload-dry-run-v1.trace.jsonl

output_trace:
  target/nando-wave/real-traffic-shadow/agent-control-output-evidence-v1.trace.jsonl

report:
  target/nando-wave/real-traffic-shadow/agent-control-output-evidence-v1.report.json

operator_candidate_calls: 143
scoreable_candidate_calls: 143
output_evidence_matched_events: 124
no_session_output_match_events: 19
deterministic_verification_events: 124
verified_true_events: 12
verified_false_events: 112
session_ids_requested: 9
session_files_scanned: 9
codex_turns_indexed: 395
raw_prompt_text_written: false
raw_response_text_written: false
```

Verifier status breakdown:

```text
verified_stop_ack_without_work_claim: 11
verified_short_ack_without_work_claim: 1
rejected_short_ack_response_too_long: 47
rejected_continue_requires_live_task_execution_or_tool_state_verifier: 31
rejected_stop_ack_absent: 19
rejected_stop_response_claims_work: 14
rejected_short_ack_absent: 1
missing matching Codex final answer: 19
```

Shadow over the evidence-enriched trace:

```text
total_llm_calls: 1000
exact_cache_hits: 53
nando_shadow_accepts: 143
verified_safe_accepts: 12
unverified_shadow_accepts: 19
false_accepts: 112
incremental_reduction_vs_exact_cache_milli: 7
p99_shadow_score_latency_ns: 4052
```

Audit:

```text
scoreable_candidate_calls: 143
verification_hook_ready_events: 124
verified_cpu_accept_eligible_events: 0
provider_cost_events: 0
candidates_missing_provider_cost: 143
shadow_false_accepts: 112
market_claim_allowed: false
```

Feedback-loop with the agent-control overlay:

```text
operator_candidate_calls: 408 / 1000
scoreable_candidate_calls: 246 / 1000
verification_hook_ready_events: 209 / 1000
verified_cpu_accept_eligible_events: 3 / 1000
verified_cpu_routability_milli: 3
verified_gap_to_80_calls: 797

agent-control route:
  candidate_events: 143
  scoreable_payload_events: 143
  verification_hook_ready_events: 124
  verified_cpu_accept_eligible_events: 0
  false_accepts: 112
  stage: false_accepts_block_local_policy
```

Structural gate:

```text
nanda_structural_gate: PASS
task_id: agent-control-output-evidence-v1
packet:
  target/nando-wave/structural-gates/agent-control-output-evidence-v1.md

Scope:
  The gate checks the coherent blocker route:
    shadow_accepts_143
    false_accepts_112
    verified_cpu_accept_eligible_zero
    false_accepts_block_local_policy
    market_claim_allowed_false
    must_not_promote

It does not upgrade the market claim. It only verifies that the blocker and
claim-boundary structure is not role-swapped.
```

Claim boundary:

```text
This is not a market savings result.
The agent-control runtime path is scoreable and has output-evidence hooks, but
the current profile is too broad: it local-accepts continue/ack/stop rows that
the deterministic verifier rejects.

Do not promote this profile.
Do not count the 12 verified_true rows as product savings.
Provider cost is absent, unverified accepts remain, and false_accepts are high.
```

Diagnostic read:

```text
The important finding is negative:
  agent-control has real routable pressure, but a one-edge shared control
  profile is not a safe local-accept policy.

A quick request-side admission probe found no simple zero-false subset:
  stop_tokens_le_2: 31 accepted -> 10 true / 14 false / 7 missing
  stop_exact: 12 accepted -> 2 true / 10 false / 0 missing
  short_ack_tokens_le_1: 28 accepted -> 0 true / 26 false / 2 missing

So the next engineering route is not threshold tuning. It is one of:
  split stop / ack / continue into separate profile routes;
  attach real agent-state or tool-state evidence;
  add a no-mutating-tool-after-stop verifier;
  keep continue as fallback until active-goal state can prove the next step.
```

## 2026-07-03 - Executor Integration: Agent-Control Profile + Payload Dry-Run

Verdicts:

```text
AGENT_CONTROL_PROFILE_V1_REVIEW_PROFILE_READY
AGENT_CONTROL_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT
REAL_TRAFFIC_SHADOW_V1_REVIEW
VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS
```

What changed:

```text
Added:
  role-binding-real-traffic-agent-control-profile-v1
  role-binding-real-traffic-agent-control-payload-dry-run-v1

The profile command builds a serving-only `.nwrb` control-plane overlay profile
for short agent-control/dialogue-state intents. The dry-run command builds
scoreable `active_fringe` + slot payloads from request text only. It writes no
raw prompt text, uses no response text, no target labels, no proof labels, and
does not enable verified local accepts.
```

Profile artifact:

```text
profile_id: role_binding_agent_control_seed0
package: target/nando-wave/real-traffic-shadow/agent-control-seed0.nwrb
registry: target/nando-wave/real-traffic-shadow/profile-registry-agent-control-v1.json
edge_count: 1
sample_margin: 65536
market_claim_allowed: false
```

Fresh route funnel over the same 1000 non-synthetic Codex calls with the
agent-control overlay:

```text
route_candidates: 408 / 1000
no_candidate_events: 592 / 1000

role_binding_agent_control_seed0: 143 candidates
role_binding_conditional_branch_seed0: 150 candidates
role_binding_edit_marker_length_seed0: 78 candidates
role_binding_mixed_map_seed0: 37 candidates
```

Remaining no-candidate head after the overlay:

```text
answer_or_explain: 215
project_context_dialogue: 211
planning_next_step: 54
read_inspect: 27
retrieval_lookup: 25
```

Agent-control payload dry-run:

```text
agent_control_candidate_events: 143
payload_built_events: 143
scoreable_payload_events: 143

intent_counts:
  stop: 54
  short_ack: 55
  continue: 34

active_fringe_centers_total: 286
slots_total: 143
positive_impulses_total: 143
negative_impulses_total: 143
raw_text_written: false
local_accepts_enabled: false
market_claim_allowed: false
```

Shadow + audit:

```text
total_llm_calls: 1000
exact_cache_hits: 53
nando_shadow_accepts: 143
verified_safe_accepts: 0
unverified_shadow_accepts: 143
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 10145

scoreable_candidate_calls: 143
verification_hook_ready_events: 0
candidates_missing_output_evidence: 143
candidates_missing_explicit_verification: 143
provider_cost_events: 0
verified_cpu_accept_eligible_events: 0
```

Claim boundary:

```text
This is route/payload pressure, not verified savings.
The 143 local accepts are deliberately unverified shadow accepts.
No market claim is allowed until a deterministic control verifier attaches
actual agent decision/tool/output evidence and the shadow/audit path reports:
  verified_safe_accepts > 0
  unverified_shadow_accepts = 0 for accepted rows
  false_accepts = 0
  incremental_reduction_vs_exact_cache_milli > 0
```

Structural gate:

```text
NANDA agent-control claim-boundary gate: PASS
checked:
  profile ready is review-only
  route candidate zone is 408/1000
  agent-control route candidates are 143/1000
  payload shadow accepts are unverified_shadow_accepts, not verified savings
  verified_safe_accepts: 0
  false_accepts: 0
  incremental_reduction_vs_exact_cache_milli: 0
  market_claim_allowed: false

Note:
  the gate packet is intentionally narrow. It proves the claim boundary is
  structurally coherent; it does not prove verified savings.
```

Diagnostic read:

```text
The route candidate zone moved from 288/1000 to 408/1000 real Codex calls.
That is useful coverage growth, but verified CPU routability did not increase
yet because the control route has no output/tool evidence verifier.

The next real engineering debt is not another classifier tweak. It is a
deterministic agent-control verifier that can prove stop/continue/ack outcomes
from actual agent state, tool calls, or command execution traces without
reading target/proof labels or inventing response evidence.
```

## 2026-07-03 - Executor Integration: Real-Traffic Route-Gap Catalog

Verdict:

```text
ROUTE_GAP_CATALOG_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-route-gap-catalog-v1

The command reads real local Codex prompt history and the serving profile
registry, separates already-routed calls from no-candidate calls, and writes a
privacy-safe operator-family backlog. It writes no raw prompt text and enables
no local accepts.

Report:
  target/nando-wave/real-traffic-shadow/route-gap-catalog-v1.report.json
```

Fresh route-gap catalog over the same 1000 non-synthetic Codex calls:

```text
sampled_llm_calls: 1000
existing_route_candidate_events: 288
existing_route_coverage_milli: 288
no_candidate_events: 712
no_candidate_coverage_milli: 712

top three no-candidate families:
  answer_or_explain: 215
  project_context_dialogue: 211
  planning_next_step: 82

top_three_no_candidate_events: 508
top_three_no_candidate_coverage_milli: 508
```

Full no-candidate family rank:

```text
answer_or_explain: 215
project_context_dialogue: 211
planning_next_step: 82
agent_continue_execute: 36
agent_control_stop: 36
read_inspect: 27
retrieval_lookup: 25
short_decision_ack: 19
serving_ops: 13
uncatalogued: 13
metrics_report_readout: 10
git_control: 8
research_literature: 7
dataset_corpus: 6
style_brevity: 3
social_affect: 1
```

Claim boundary:

```text
This is not savings and not a market claim.
raw_text_written: false
local_accepts_enabled: false
market_claim_allowed: false
```

Structural gate:

```text
NANDA route-gap catalog claim-boundary gate: PASS
checked:
  existing_route_candidate_events: 288
  no_candidate_events: 712
  top_three_no_candidate_events: 508
  raw_text_written: false
  local_accepts_enabled: false
  market_claim_allowed: false
  conditional safe_policy_found: false

Note:
  earlier broad packets VETOed because they mixed interpretation with source
  counts. The accepted gate is intentionally narrow: counts and claim-boundary
  only.
```

Diagnostic read:

```text
The current three runtime routes cover only 288/1000 real Codex calls, so CPU
Routability 80 cannot be reached by tuning edit/conditional/mixed alone.

The largest gap is not an edit route. It is conversational/project-state work:
answer_or_explain + project_context_dialogue + planning_next_step account for
508/1000 calls. Those families need separate request-side payload builders and
deterministic verifiers, not forced promotion through conditional/mixed.

Conditional remains unsafe to promote: its calibration still has no safe
readout policy. Route-gap catalog is the correct next map for deciding which
operator profile to build next.
```

## 2026-07-03 - Executor Integration: Route Priority + Edit Safe-Policy Promotion

Verdicts:

```text
CODEX_HISTORY_ROUTE_CANDIDATES_V1_REVIEW
EDIT_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY
REAL_TRAFFIC_SHADOW_V1_PASS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Route priority now treats explicit validation/gate/fallback/pass requests as
conditional unless there is a direct edit command such as fix/patch/commit.

Added:
  role-binding-real-traffic-edit-safe-policy-promote-v1

The command writes:
  target/nando-wave/real-traffic-shadow/profile-registry-edit-safe-policy-v1.json
  target/nando-wave/real-traffic-shadow/edit-safe-policy-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/edit-safe-policy-v1.report.json
```

Fresh route funnel over the same 1000 non-synthetic Codex calls:

```text
route_candidates: 288 / 1000
exact_cache_hits: 53 / 1000

role_binding_conditional_branch_seed0: 166 candidates
role_binding_edit_marker_length_seed0: 83 candidates
role_binding_mixed_map_seed0: 39 candidates

conditional scoreable payloads: 79
conditional verification hooks: 63
conditional safe local-accept policy: false

edit scoreable payloads: 10
edit verification hooks: 9
edit safe local-accept policy: true

mixed scoreable payloads: 14
mixed verification hooks: 13
mixed safe local-accept policy: true
```

Edit promoted safe policy:

```text
selected_policy_name: market_safe_energy_margin_threshold
selected_policy_source: evidence_trace_market_safe_threshold
selected_acceptance_policy: energy_threshold_only
selected_policy_threshold: 9472
policy_accept_rows: 1
policy_accept_verified_true_rows: 1
policy_accept_verified_false_rows: 0
policy_accept_unverified_rows: 0
runtime_acceptance_mismatches: 0
```

Edit shadow + audit:

```text
shadow_verdict: REAL_TRAFFIC_SHADOW_V1_PASS
shadow_nando_accepts: 1
shadow_verified_safe_accepts: 1
shadow_unverified_shadow_accepts: 0
shadow_false_accepts: 0
shadow_incremental_savings_over_exact_cache: 1
shadow_incremental_reduction_vs_exact_cache_milli: 1
shadow_p99_score_latency_ns: 439824

audit_verified_cpu_accept_eligible_events: 1
audit_market_claim_allowed: true
```

Updated feedback-loop:

```text
scoreable_candidate_calls: 103
verification_hook_ready_events: 85
verified_cpu_accept_eligible_events: 3
verified_cpu_routability_milli: 3
verified_gap_to_80_calls: 797

route stages:
  conditional: local_accept_calibration_failed
  edit: verified_cpu_accept_eligible
  mixed: verified_cpu_accept_eligible
```

Structural gate:

```text
NANDA route-priority gate: PASS
checked route:
  direct edit commands keep edit priority
  validation/gate/pass/fallback/margin requests route before ambient code surfaces
  evidence uses line anchors in role_binding_runtime_cmd.rs

Edit safe-policy savings counts are verified by CLI/JSON shadow/audit reports,
not by NANDA numeric count triads.
```

Diagnostic read:

```text
This is a real but small verified-routability increase: 2 -> 3 verified CPU
accepts per 1000 non-synthetic Codex calls. The claim boundary remains tight:
false_accepts=0 and unverified_shadow_accepts=0 on the promoted edit and mixed
traces.

The main next lever is conditional. Route priority fixed the funnel and exposed
79 scoreable conditional payloads with 63 verification hooks and 17 true labels,
but the current conditional payload/readout geometry still cannot separate true
from false rows without false accepts.
```

## 2026-07-03 - Executor Integration: Mixed Safe-Policy Promotion Shadow

Verdicts:

```text
MIXED_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY
REAL_TRAFFIC_SHADOW_V1_PASS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-mixed-safe-policy-promote-v1

Also added profile acceptance_policy:
  strict_ordered_energy_threshold  # default / backwards compatible
  energy_threshold_only            # used only in promoted mixed safe-policy registry

The command writes:
  target/nando-wave/real-traffic-shadow/profile-registry-mixed-safe-policy-v1.json
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/mixed-safe-policy-v1.report.json
```

Selected safe policy:

```text
calibration_policy_name: best_energy_margin_threshold
calibration_policy_threshold: 393216
selected_policy_name: market_safe_energy_margin_threshold
selected_policy_source: evidence_trace_market_safe_threshold
selected_acceptance_policy: energy_threshold_only
selected_policy_threshold: 466944
promoted_profile_ids:
  role_binding_mixed_map_seed0
provider_cost_microusd: 100
runtime_acceptance_mismatches: 0
```

Promoted trace result:

```text
policy_accept_rows: 2
policy_accept_verified_true_rows: 2
policy_accept_verified_false_rows: 0
policy_accept_unverified_rows: 0
```

Shadow + audit over promoted registry/trace:

```text
shadow_verdict: REAL_TRAFFIC_SHADOW_V1_PASS
shadow_nando_accepts: 2
shadow_verified_safe_accepts: 2
shadow_unverified_shadow_accepts: 0
shadow_false_accepts: 0
shadow_incremental_savings_over_exact_cache: 2
shadow_incremental_reduction_vs_exact_cache_milli: 2
shadow_estimated_cost_saved_microusd: 200
shadow_p99_score_latency_ns: 270771

audit_verified_cpu_accept_eligible_events: 2
audit_provider_cost_events: 14
audit_candidates_missing_provider_cost: 0
audit_market_claim_allowed: true
```

Updated feedback-loop:

```text
scoreable_candidate_calls: 61
verification_hook_ready_events: 34
verified_cpu_accept_eligible_events: 2
verified_cpu_routability_milli: 2
verified_gap_to_80_calls: 798

role_binding_mixed_map_seed0:
  stage: verified_cpu_accept_eligible
  verified_cpu_accept_eligible_events: 2
```

Structural gate:

```text
NANDA mixed-safe-policy route: PASS
checked route:
  promoted shadow verdict / unverified boundary
  selected threshold
  runtime branch = energy_margin >= threshold
  forbidden branch = verifier label at serving time

numeric accept counts are verified by CLI/JSON reports, not by NANDA, because
the sparse triad checker VETOed raw count triads as weak composite support.
```

Diagnostic read:

```text
The old calibration threshold 393216 accepted one unverified row. The promoter
now recomputes an evidence-trace market-safe threshold and selects 466944, which
keeps serving score-only while suppressing unverified local accepts.

This is the first non-zero verified CPU savings PASS on non-synthetic Codex
traffic for the promoted mixed trace: 2 verified local accepts, false_accepts=0,
unverified_shadow_accepts=0. The overall CPU Routability 80 goal remains red:
feedback-loop routability is 2 milli and gap_to_80 is 798 calls.

Next debt: grow verified route coverage, starting with edit/conditional safe
admission or stronger mixed payload/evidence coverage, without relaxing the
zero false/unverified accept boundary.
```

## 2026-07-03 - Executor Integration: Mixed Map Payload/Evidence/Calibration

Verdicts:

```text
MIXED_PAYLOAD_READINESS_V1_REVIEW_READY_CANDIDATES_FOUND
MIXED_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT
MIXED_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
MIXED_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SAFE_POLICY_CANDIDATE_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-mixed-payload-readiness-v1
  role-binding-real-traffic-mixed-payload-dry-run-v1
  role-binding-real-traffic-mixed-output-evidence-v1
  role-binding-real-traffic-mixed-local-accept-calibration-v1

The mixed commands write:
  target/nando-wave/real-traffic-shadow/mixed-payload-readiness-v1.report.json
  target/nando-wave/real-traffic-shadow/mixed-payload-dry-run-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/mixed-payload-dry-run-v1.report.json
  target/nando-wave/real-traffic-shadow/mixed-output-evidence-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/mixed-output-evidence-v1.report.json
  target/nando-wave/real-traffic-shadow/mixed-local-accept-calibration-v1.report.json
```

Privacy / anti-leak boundary:

```text
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_verification: true
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Mixed payload dry-run over the same 1000 real Codex calls:

```text
mixed_route_candidate_events: 39
payload_ready_events: 14
payload_built_events: 14
scoreable_payload_events: 14
shadow_nando_accepts: 0
shadow_false_accepts: 0
shadow_incremental_reduction_vs_exact_cache_milli: 0
shadow_p99_score_latency_ns: 241947
```

Mixed output evidence and calibration:

```text
output_evidence_matched_events: 13
deterministic_verification_events: 13
verified_true_events: 10
verified_false_events: 3

hook_ready_rows: 13
scored_rows: 13
safe_policy_found: true
best_safe_true_accepts: 4
```

Updated feedback-loop:

```text
scoreable_candidate_calls: 61 = 23 edit + 24 conditional + 14 mixed
verification_hook_ready_events: 51 = 17 edit + 21 conditional + 13 mixed
verified_cpu_routability_milli: 0
verified_gap_to_80_calls: 800

role_binding_mixed_map_seed0:
  candidate_events: 39
  payload_ready_events: 14
  payload_built_events: 14
  scoreable_payload_events: 14
  verification_hook_ready_events: 13
  local_accept_calibration_ran: true
  local_accept_safe_policy_found: true
  local_accept_best_safe_true_accepts: 4
  stage: local_accept_calibration_safe_policy_candidate
```

Diagnostic read:

```text
The mixed route is no longer payload-builder missing. It now has request-side
active_fringe/slot construction, response-backed deterministic labels, and a
safe-policy candidate on the current 13-row hook-ready sample.

This is still not verified CPU savings. Local accepts are not promoted, provider
cost is absent, shadow accepts are 0, market_claim_allowed is false, and CPU
Routability 80 remains red.
```

## 2026-07-03 - Executor Integration: Conditional Output Evidence Hook

Verdicts:

```text
CONDITIONAL_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
REAL_TRAFFIC_SHADOW_V1_REVIEW
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-conditional-output-evidence-v1

The command reads:
  target/nando-wave/real-traffic-shadow/conditional-payload-dry-run-v1.trace.jsonl
  /home/ubu/.codex/sessions

and writes:
  target/nando-wave/real-traffic-shadow/conditional-output-evidence-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/conditional-output-evidence-v1.report.json
```

Privacy / anti-leak boundary:

```text
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_verification: true
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Conditional output evidence:

```text
operator_candidate_calls: 24
scoreable_candidate_calls: 24
session_ids_requested: 3
session_files_scanned: 3
codex_turns_indexed: 21
output_evidence_matched_events: 21
no_session_output_match_events: 3
deterministic_verification_events: 21
verified_true_events: 6
verified_false_events: 15
```

Shadow + audit over the evidence trace:

```text
shadow_nando_accepts: 0
shadow_verified_safe_accepts: 0
shadow_false_accepts: 0
shadow_incremental_reduction_vs_exact_cache_milli: 0
shadow_p99_score_latency_ns: 169208

audit_scoreable_candidate_calls: 24
audit_response_fingerprint_events: 21
audit_explicit_verified_safe_accept_events: 21
audit_verification_hook_ready_events: 21
audit_verified_cpu_accept_eligible_events: 0
audit_provider_cost_events: 0
audit_market_claim_allowed: false
```

Updated feedback-loop stage:

```text
role_binding_conditional_branch_seed0:
  candidate_events: 92
  payload_ready_events: 24
  payload_built_events: 24
  scoreable_payload_events: 24
  verification_hook_ready_events: 21
  stage: verification_hook_ready_waiting_local_accept

Total verification_hook_ready_events in feedback loop:
  38 = 17 edit + 21 conditional
```

Conditional local-accept calibration:

```text
verdict: CONDITIONAL_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY
hook_ready_rows: 21
scored_rows: 21
label_true_rows: 6
label_false_rows: 15
safe_policy_found: false
best_safe_true_accepts: 0

energy_only_no_slot_order:
  accepts: 6
  true_accepts: 1
  false_accepts: 5

branch_slot_only_ignore_evidence_slot:
  accepts: 6
  true_accepts: 1
  false_accepts: 5
```

Diagnostic read:

```text
The conditional route is no longer missing a verification hook. It now has
21 real response-backed deterministic verification rows, including 6 true and
15 false labels for later local-accept calibration.

The first readout calibration did not find a safe policy. Local accepts remain
disabled, provider cost is absent, shadow accepts are 0, and verified CPU
accepts remain 0. CPU Routability 80 is still red.
```

## 2026-07-03 - Executor Integration: Conditional Payload Dry-Run

Verdicts:

```text
CONDITIONAL_PAYLOAD_READINESS_V1_REVIEW_READY_CANDIDATES_FOUND
CONDITIONAL_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT
REAL_TRAFFIC_SHADOW_V1_REVIEW
VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added:
  role-binding-real-traffic-conditional-payload-readiness-v1
  role-binding-real-traffic-conditional-payload-dry-run-v1

The dry-run command writes:
  target/nando-wave/real-traffic-shadow/conditional-payload-dry-run-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/conditional-payload-dry-run-v1.report.json

Shadow/audit reports:
  target/nando-wave/real-traffic-shadow/conditional-payload-dry-run-v1.shadow-report.json
  target/nando-wave/real-traffic-shadow/conditional-payload-dry-run-v1.verification-hook-audit.report.json
```

Privacy / anti-leak boundary:

```text
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Conditional readiness over the same 1000 real Codex calls:

```text
candidate_events: 92
payload_ready_events: 24
payload_ready_rate_milli: 260
missing_condition_signal: 5
missing_branch_signal: 35
missing_evidence_signal: 58
missing_branch_tokens: 2
```

Conditional dry-run result:

```text
conditional_route_candidate_events: 92
payload_ready_events: 24
payload_built_events: 24
scoreable_payload_events: 24
active_fringe_centers_total: 2287
slots_total: 36
positive_impulses_total: 709
negative_impulses_total: 752
```

Shadow + audit:

```text
shadow_operator_candidate_calls: 24
shadow_nando_accepts: 0
shadow_verified_safe_accepts: 0
shadow_false_accepts: 0
shadow_incremental_reduction_vs_exact_cache_milli: 0
shadow_p99_score_latency_ns: 153389

audit_scoreable_candidate_calls: 24
audit_verification_hook_ready_events: 0
audit_verified_cpu_accept_eligible_events: 0
audit_market_claim_allowed: false
```

Updated feedback-loop stage:

```text
role_binding_conditional_branch_seed0:
  candidate_events: 92
  payload_ready_events: 24
  payload_built_events: 24
  scoreable_payload_events: 24
  stage: scoreable_payload_missing_verification_hook
  next_action: Attach response/tool-call evidence and deterministic output verification.

Total scoreable_candidate_calls in feedback loop:
  47 = 23 edit + 24 conditional
```

Diagnostic read:

```text
The conditional route is no longer payload_builder_missing. It now has a
request-side active_fringe/slot dry-run for 24 real prompt-side rows. This is
real funnel progress, not verified savings.

Superseded by the Conditional Output Evidence Hook section above: output
evidence is now attached for 21 conditional rows, but local accepts remain
disabled and verified CPU accepts remain 0, so CPU Routability 80 is still red.
```

## 2026-07-03 - Executor Integration: Edit Admission Calibration

Verdict:

```text
EDIT_ADMISSION_CALIBRATION_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY
```

What changed:

```text
Added role-binding-real-traffic-edit-admission-calibration-v1.

The command reads:
  target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.trace.jsonl
  /home/ubu/.codex/history.jsonl

and writes:
  target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1.report.json
```

Privacy / anti-leak boundary:

```text
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_features: false
target_labels_used_for_runtime: false
proof_labels_used_for_runtime: false
local_accepts_enabled: false
market_claim_allowed: false
```

Admission population:

```text
hook_ready_rows: 17
rows_with_prompt_features: 17
label_true_rows: 3
label_false_rows: 14
minimum_true_support: 2
robust_safe_policy_found: false
singleton_safe_policy_found: true
best_robust_true_accepts: 0
best_singleton_true_accepts: 1
```

Policy read:

```text
all_hook_ready_rows:
  accepts: 17
  true_accepts: 3
  false_accepts: 14

starts_what:
  accepts: 1
  true_accepts: 1
  false_accepts: 0
  singleton_safe: true
  robust_safe: false

starts_goal_or_starts_what_length_lt_1800:
  accepts: 1
  true_accepts: 1
  false_accepts: 0
  singleton_safe: true
  robust_safe: false

runtime_and_length_lt_1800:
  accepts: 2
  true_accepts: 1
  false_accepts: 1

not_report_marker_and_length_lt_1800:
  accepts: 3
  true_accepts: 1
  false_accepts: 2
```

Diagnostic read:

```text
No robust request-side admission policy exists on the current 17 hook-ready
real edit rows. Singleton-safe gates exist, but support only 1 true row, so they
are not product proof and must not enable local accepts.

Combined with local-accept calibration, this closes the cheap edit route:
  readout loosening is unsafe;
  request-side admission from current features is too weak;
  verified CPU accepts remain 0.

Next debt:
  either collect more real edit evidence and add richer edit payload/admission
  features, or move to the larger route pool:
    conditional_branch: 92 candidates;
    mixed_map: 39 candidates.
```

## 2026-07-03 - Executor Integration: Edit Local Accept Calibration

Verdict:

```text
EDIT_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY
```

What changed:

```text
Added role-binding-real-traffic-edit-local-accept-calibration-v1.

The command reads:
  target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json
  target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.trace.jsonl

and writes:
  target/nando-wave/real-traffic-shadow/edit-local-accept-calibration-v1.report.json
```

Calibration population:

```text
hook_ready_rows: 17
label_true_rows: 3
label_false_rows: 14
safe_policy_found: false
best_safe_true_accepts: 0
local_accepts_enabled: false
market_claim_allowed: false
```

Policy sweep:

```text
current_strict_all_slots:
  accepts: 0
  true_accepts: 0
  false_accepts: 0
  missed_true: 3

energy_only_no_slot_order:
  accepts: 17
  true_accepts: 3
  false_accepts: 14

marker_slot_only_ignore_end_slot:
  accepts: 17
  true_accepts: 3
  false_accepts: 14

strict_slots_but_ignore_zero_end_slot:
  accepts: 17
  true_accepts: 3
  false_accepts: 14

best_marker_slot_margin_threshold:
  accepts: 0
  true_accepts: 0
  false_accepts: 0

best_energy_margin_threshold:
  accepts: 0
  true_accepts: 0
  false_accepts: 0
```

Diagnostic read:

```text
The end slot explains the current 0 accepts:
  marker_slot_margin > 0 for all 17 hook-ready rows;
  end_slot_margin = 0 for all 17 hook-ready rows.

But dropping the end slot would be unsafe:
  it would accept all 17 rows;
  only 3 are deterministic-verifier true;
  14 would become false accepts.

Therefore the next step is not threshold/readout tuning. The current edit
payload geometry lacks a request-side admission signal that separates
verifier-true rows from verifier-false rows. Build a request-side admission
gate or improve edit payload features before enabling local accepts.
```

## 2026-07-03 - Executor Integration: Edit Output Evidence Join

Verdict:

```text
EDIT_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added role-binding-real-traffic-edit-output-evidence-v1.

The command reads:
  target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.trace.jsonl
  /home/ubu/.codex/sessions

and writes:
  target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.report.json
```

Privacy / anti-leak boundary:

```text
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_verification: true
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Real Codex output-evidence result:

```text
total_trace_rows: 1000
operator_candidate_calls: 23
scoreable_candidate_calls: 23
output_evidence_matched_events: 17
no_session_output_match_events: 6
deterministic_verification_events: 17
verified_true_events: 3
verified_false_events: 14
```

Shadow + audit over evidence-enriched trace:

```text
shadow_verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
shadow_operator_candidate_calls: 23
shadow_nando_accepts: 0
shadow_verified_safe_accepts: 0
shadow_false_accepts: 0
shadow_incremental_reduction_vs_exact_cache_milli: 0
shadow_p99_score_latency_ns: 555869

audit_verdict: VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
audit_verification_hook_ready_events: 17
audit_verified_cpu_accept_eligible_events: 0
audit_market_claim_allowed: false
```

Updated feedback-loop stage:

```text
role_binding_edit_marker_length_seed0:
  candidate_events: 154
  scoreable_payload_events: 23
  verification_hook_ready_events: 17
  stage: verification_hook_ready_waiting_local_accept
  next_action: run local-accept calibration; if no safe policy exists, improve request-side admission or payload features

verified_cpu_routability_milli: 0
verified_gap_to_80_calls: 800
```

Diagnostic read:

```text
The edit route is no longer blocked at "missing output evidence" for all
scoreable rows. 17/23 scoreable rows now have real Codex final-answer evidence
and explicit deterministic verification. The blocker moved forward:
  output evidence exists;
  local CPU runtime still accepts 0 real rows;
  verified CPU savings are still 0.

Next debt:
  inspect the 3 verifier-true edit rows by fingerprint/metrics only, then tune
  score/readout or the edit payload geometry so hook-backed true rows can become
  local accepts without creating false accepts on the 14 verifier-false rows.
```

## 2026-07-03 - Executor Integration: CPU Route Feedback Loop

Verdict:

```text
CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
```

What changed:

```text
Added role-binding-real-traffic-feedback-loop-v1.

The feedback loop reads:
  target/nando-wave/real-traffic-shadow/cpu-route-forecast-v1.report.json
  target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.report.json
  target/nando-wave/real-traffic-shadow/verification-hook-audit-v1.report.json

and writes:
  target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json
```

Current real Codex funnel:

```text
total_llm_calls: 1000
exact_cache_hits: 54
exact_cache_coverage_milli: 54
operator_candidate_calls: 285
operator_candidate_coverage_milli: 285
scoreable_candidate_calls: 23
scoreable_candidate_coverage_milli: 23
verification_hook_ready_events: 0
verification_hook_coverage_milli: 0
verified_cpu_accept_eligible_events: 0
verified_cpu_routability_milli: 0
target_routability_milli: 800
target_verified_cpu_calls: 800
routing_gap_to_80_calls: 515
verified_gap_to_80_calls: 800
market_claim_allowed: false
```

Route stages:

```text
1. role_binding_edit_marker_length_seed0
   candidate_events: 154
   scoreable_payload_events: 23
   stage: scoreable_payload_missing_verification_hook
   next_action: attach response/tool-call evidence and deterministic output verification

2. role_binding_conditional_branch_seed0
   candidate_events: 92
   stage: payload_builder_missing
   next_action: build conditional_branch_payload_builder_v1

3. role_binding_mixed_map_seed0
   candidate_events: 39
   stage: payload_builder_missing
   next_action: build mixed_map_payload_builder_v1
```

Diagnostic read:

```text
The current real-traffic bottleneck is now explicit:
  route coverage exists for 285/1000 calls;
  executable scoreable payload exists for 23/1000 calls;
  verified CPU eligibility is still 0/1000.

The next highest-value move is not tuning score thresholds. It is either:
  A) add edit response/tool evidence + deterministic edit verifier for the 23
     scoreable edit rows; or
  B) build the conditional payload builder to open the 92-call route.
```

## 2026-07-03 - Executor Integration: Verification Hook Audit

Verdict:

```text
VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS
VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND_ON_SYNTHETIC_CONTROL
```

What changed:

```text
Added role-binding-real-traffic-verification-hook-audit-v1.

The audit reads:
  target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.shadow-report.json

and writes:
  target/nando-wave/real-traffic-shadow/verification-hook-audit-v1.report.json
```

Validation contract tightened:

```text
If verified_safe_accept is present, the trace/event row must also carry:
  nando_shadow_request
  response_fingerprint or tool_call_fingerprints
  verification_source

This prevents a raw verified_safe_accept label from becoming evidence by
itself.
```

Real edit dry-run hook audit:

```text
total_requests: 1000
total_llm_calls: 1000
operator_candidate_calls: 23
scoreable_candidate_calls: 23
local_accepts_disabled_events: 23
local_accepts_enabled_events: 0
response_fingerprint_events: 0
tool_call_fingerprint_events: 0
verification_source_events: 1000
explicit_verified_safe_accept_events: 0
provider_cost_events: 0
candidates_missing_output_evidence: 23
candidates_missing_explicit_verification: 23
candidates_missing_provider_cost: 23
verification_hook_ready_events: 0
verified_cpu_accept_eligible_events: 0
shadow_accepts: 0
shadow_fallbacks: 23
shadow_false_accepts: 0
shadow_incremental_savings_over_exact_cache: 0
market_claim_allowed: false
```

Synthetic positive-control hook audit:

```text
total_requests: 14
operator_candidate_calls: 14
scoreable_candidate_calls: 14
response_fingerprint_events: 14
verification_source_events: 14
explicit_verified_safe_accept_events: 14
provider_cost_events: 14
verification_hook_ready_events: 14
shadow_accepts: 7
shadow_false_accepts: 0
shadow_incremental_savings_over_exact_cache: 7
verified_cpu_accept_eligible_events: 0
market_claim_allowed: false
```

Diagnostic read:

```text
The audit proves the hook logic works in both directions:
  real Codex dry-run: scoreable payloads exist, but output evidence is missing;
  synthetic control: output evidence exists, but synthetic source blocks market claim.

Next debt:
  attach real response/tool-call evidence and deterministic edit-output
  verification to scoreable edit payloads before enabling any local accept.
```

## 2026-07-03 - Executor Integration: Edit Payload Dry-Run Builder

Verdict:

```text
EDIT_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT
REAL_TRAFFIC_SHADOW_V1_REVIEW
```

What changed:

```text
Added role-binding-real-traffic-edit-payload-dry-run-v1.

The dry-run builder reads real local Codex history at analysis time, writes no
raw prompt text, and emits scoreable edit-route payloads only for rows that
already passed edit-payload readiness.

It writes:
  target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.report.json
  target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.shadow-report.json
```

Dry-run payload result:

```text
trace_rows_written: 1000
edit_route_candidate_events: 154
payload_ready_events: 23
payload_built_events: 23
scoreable_payload_events: 23
builder_rejected_events: 0
readiness_rejected_events: 131
active_fringe_centers_total: 11019
slots_total: 46
positive_impulses_total: 1077
negative_impulses_total: 1010
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Shadow result over the dry-run trace:

```text
total_requests: 1000
total_llm_calls: 1000
operator_candidate_calls: 23
exact_cache_hits: 54
nando_shadow_accepts: 0
nando_shadow_fallbacks: 23
verified_safe_accepts: 0
unverified_shadow_accepts: 0
false_accepts: 0
incremental_savings_over_exact_cache: 0
incremental_reduction_vs_exact_cache_milli: 0
p50_shadow_score_latency_ns: 422771
p90_shadow_score_latency_ns: 585353
p99_shadow_score_latency_ns: 683880
synthetic_trace_used: false
```

Diagnostic read:

```text
The request-side path is no longer empty for the first edit route slice:
  route/profile candidate -> active_fringe + slots works for 23 real rows.

The scorer still falls back:
  energy_margin: 6912..10752
  min_slot_margin: 0

So the next debt is not another route classifier. It is an edit verifier/output
hook that can prove a local edit candidate safe before enabling local accepts.
Until then, the dry-run correctly keeps verified_safe_accepts=0 and
market_claim_allowed=false.
```

Boundary:

```text
This is a scoreable request payload proof, not savings.
Any local accept from this command would be counted as unverified/false because
expect_local_operator=false and verified_safe_accept=None.
```

## 2026-07-03 - Executor Integration: Real Traffic CPU Route Forecast

Verdict:

```text
CPU_ROUTE_FORECAST_V1_REVIEW
EDIT_PAYLOAD_READINESS_V1_REVIEW_READY_CANDIDATES_FOUND
```

What changed:

```text
Added role-binding-real-traffic-cpu-route-forecast-v1.
Added role-binding-real-traffic-edit-payload-readiness-v1.

The forecast reads:
  target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.report.json
  target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.shadow-report.json

and writes:
  target/nando-wave/real-traffic-shadow/cpu-route-forecast-v1.report.json

The edit readiness audit reads real local Codex history at analysis time and
writes:
  target/nando-wave/real-traffic-shadow/edit-payload-readiness-v1.report.json
```

Current real Codex forecast:

```text
total_llm_calls: 1000
exact_cache_hits: 54
exact_cache_coverage_milli: 54
operator_candidate_calls: 285
operator_candidate_coverage_milli: 285
current_nando_accepts: 0
current_verified_safe_accepts: 0
current_false_accepts: 0
full_shadow_request_payload_built: false
market_claim_allowed: false

forecast_25_percent_additional_savings: 69
forecast_50_percent_additional_savings: 141
forecast_80_percent_additional_savings: 226
forecast_25_percent_total_calls_removed: 123
forecast_50_percent_total_calls_removed: 195
forecast_80_percent_total_calls_removed: 280
```

Priority CPU route backlog:

```text
1. role_binding_edit_marker_length_seed0
   candidate_events: 154
   payload_builder: edit_marker_length_payload_builder_v1

2. role_binding_conditional_branch_seed0
   candidate_events: 92
   exact_cache_hits_inside_route: 2
   payload_builder: conditional_branch_payload_builder_v1

3. role_binding_mixed_map_seed0
   candidate_events: 39
   payload_builder: mixed_map_payload_builder_v1
```

Edit payload readiness:

```text
candidate_events: 154
payload_ready_events: 23
payload_ready_rate_milli: 149
missing_scope_or_file: 14
missing_marker: 57
missing_length_or_shape: 90
missing_edit_intent: 119
raw_text_written: false
local_accepts_enabled: false
market_claim_allowed: false
```

Boundary:

```text
This is route-zone capacity, not verified savings.
Do not use it as market claim.
The next debt is request-side payload builders:
  route/profile candidate -> active_fringe + slots
without response text, target labels, proof labels, or expected answer.

The first concrete builder target is the 23 payload-ready edit-route rows, not
all 154 edit route candidates.
```

## 2026-07-03 - Executor Integration: Real Traffic Shadow Recorder And Operator Mining

Verdict:

```text
REAL_TRAFFIC_SHADOW_V1_READY
SYNTHETIC_SMOKE_FORCES_REVIEW
```

What changed:

```text
Added three CLI commands:
  role-binding-real-traffic-record-v1
  role-binding-real-traffic-record-serve-v1
  role-binding-real-traffic-ingest-events-v1
  role-binding-real-traffic-codex-history-ingest-v1
  role-binding-real-traffic-codex-history-route-candidates-v1
  role-binding-real-traffic-shadow-v1
  role-binding-real-traffic-shadow-smoke-v1

The new path records JSONL trace rows without changing the live LLM flow,
then analyzes the trace in shadow mode against serving-only `.nwrb` profiles.
It computes exact-cache baseline, Nando routability, verified local accepts,
fallbacks, false accepts, latency/RSS, and operator rankings.

The HTTP recorder exposes GET /health, POST /trace, and GET /metrics with an
optional request limit, so smoke/watchdog runs can stop without leaving a
hanging daemon.

The event ingester converts agent/API event JSONL into the trace contract and
keeps synthetic/no-candidate batches in REVIEW.

The Codex history ingester converts local Codex prompt history into
privacy-safe event fingerprints. It writes no raw text and does not infer
`nando_shadow_request`.

The Codex history route-candidate adapter selects route/profile candidates from
request-side prompt text only. It writes empty score payloads, so every candidate
must fallback until a real `active_fringe`/slot builder exists.

Bugfix: REAL_TRAFFIC_SHADOW_V1_PASS now requires `verified_safe_accepts > 0`
and `incremental_savings_over_exact_cache > 0`. Candidate-only/no-savings traces
stay REVIEW.
```

Key boundary:

```text
Synthetic savings are not market savings.
Synthetic traces force REAL_TRAFFIC_SHADOW_V1_REVIEW even if reduction is high.
Market claim requires non-synthetic real traffic with verified_safe_accept=true
and false_local_accepts=0.
```

Local smoke:

```text
record_serve_trace: target/nando-wave/real-traffic-shadow/real-traffic-record-server-smoke.trace.jsonl
record_serve_requests_handled: 3
record_serve_rows_written: 1
record_serve_bad_requests: 0
record_serve_exited_after_request_limit: true
ingest_events_verdict: REAL_TRAFFIC_INGEST_V1_REVIEW
ingest_events_total_events: 1
ingest_events_operator_candidate_events: 0
ingest_events_synthetic_events: 1
ingest_shadow_verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
ingest_shadow_nando_shadow_accepts: 0
codex_history_events_verdict: CODEX_HISTORY_EVENTS_V1_READY
codex_history_total_history_rows: 12187
codex_history_events_written: 1000
codex_history_raw_text_written: false
codex_history_shadow_verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
codex_history_total_llm_calls: 1000
codex_history_exact_cache_hits: 54
codex_history_operator_candidate_calls: 0
codex_history_incremental_reduction_vs_exact_cache_milli: 0
codex_history_route_candidate_verdict: CODEX_HISTORY_ROUTE_CANDIDATES_V1_REVIEW
codex_history_route_candidate_events: 285
codex_history_route_no_candidate_events: 715
codex_history_route_full_shadow_request_payload_built: false
codex_history_route_counts:
  role_binding_edit_marker_length_seed0: 154
  role_binding_conditional_branch_seed0: 92
  role_binding_mixed_map_seed0: 39
codex_history_route_shadow_verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
codex_history_route_shadow_operator_candidate_calls: 285
codex_history_route_shadow_nando_shadow_accepts: 0
codex_history_route_shadow_incremental_reduction_vs_exact_cache_milli: 0
trace: target/nando-wave/real-traffic-shadow/real-traffic-shadow-smoke-v1.trace.jsonl
report: target/nando-wave/real-traffic-shadow/real-traffic-shadow-smoke-v1.product-proof.json
rows: 28
verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
total_llm_calls: 28
operator_candidate_calls: 28
exact_cache_hits: 0
nando_shadow_accepts: 14
verified_safe_accepts: 14
unverified_shadow_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 500
estimated_cost_saved_microusd: 1400
p99_shadow_score_latency_ns: 144392
synthetic_trace_used: true
operator_rankings: 14
```

Operator ranking formula carried into report:

```text
operator_value =
  frequency_in_real_traces
  * local_accept_rate
  * saved_llm_cost
  * safety_score
  / runtime_cost
```

Interpretation:

```text
The proof/runtime operators are not automatically the best commercial
compression operators. This rung creates the measurement path that can discover
the money-ranked operators from real traces. The next proof is real agent/API
traffic in shadow mode, not another synthetic savings claim.

The first local non-synthetic baseline exists now, but it has no Nando route
candidates. That means the next engineering debt is a real route/candidate
adapter, not more synthetic replay.

The route-only adapter now finds 285/1000 real local Codex candidate events, but
builds no executable payload and therefore saves 0 calls. The next debt is the
request-side builder for active fringe and slot impulses.
```

No SSH was used for this rung.

## 2026-07-03 - Executor Integration: Bounded LB Throughput Proof

Verdict:

```text
LOCAL_ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_PASS
HOSTWORLD_ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
```

What changed:

```text
Added role-binding-profile-lb-throughput-v1. It runs a bounded concurrent
POST /score pressure proof through the local LB and serving-only `.nwrb`
workers. The proof has fixed request count, per-client progress output,
client socket read/write timeouts, and child-process cleanup through the
existing harness Drop guards.
```

Local result:

```text
command: cargo run --release -p nando-cli -- role-binding-profile-lb-throughput-v1
report: target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-v1.product-proof.json
verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_PASS
worker_count: 2
client_threads: 4
score_requests: 896
local_operator_calls: 448
fallback_to_llm_calls: 448
false_local_accepts: 0
missed_expected_local: 0
unexpected_local_accepts: 0
client_errors: 0
throughput_requests_per_second_milli: 1510939
client_p99_latency_ns: 3633433
load_balancer_p99_latency_ns: 743295
core_score_p99_latency_ns: 72666
worker_score_p99_latency_ns: 169460
lb_upstream_roundtrip_p99_latency_ns: 742942
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
max_worker_runtime_bytes_estimate: 492792
```

HostWorld capacity curve:

```text
hostworld 1 client:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  load_balancer_p99_latency_ns: 3507669
  lb_upstream_roundtrip_p99_latency_ns: 3507021
  worker_score_p99_latency_ns: 643303
  false_local_accepts: 0
  client_errors: 0

hostworld 2 clients:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  load_balancer_p99_latency_ns: 3250420
  lb_upstream_roundtrip_p99_latency_ns: 3249782
  worker_score_p99_latency_ns: 579524
  false_local_accepts: 0
  client_errors: 0

hostworld 4 clients:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  load_balancer_p99_latency_ns: 3611864
  lb_upstream_roundtrip_p99_latency_ns: 3610931
  worker_score_p99_latency_ns: 577626
  false_local_accepts: 0
  client_errors: 0
```

Interpretation:

```text
The green replay-batch HostWorld gate remains valid, but it does not close
individual POST /score pressure. The deployed per-score throughput gate is red
because the cheap-VPS LB/upstream roundtrip p99 is above 3 ms even at one
client. Safety and parity are green; the next speed debt is the per-score
serving envelope, not the Wave score loop.
```

## 2026-07-03 - Executor Integration: Packed Hot-Path Metrics

Verdict:

```text
LOCAL_PACKED_ROLE_BINDING_LB_REPLAY_PASS
HOSTWORLD_PACKED_ROLE_BINDING_LB_REPLAY_PASS
```

What changed:

```text
The `.nwrb` role-binding runtime now has a packed edge-group score path and a
reference score path. The CLI serving hot path prepares active centers from an
iterator instead of collecting a temporary Vec. The load-balancer replay also
checks packed-vs-reference score parity before accepting the product proof.

Transport cleanup after the first deployed WATCH/FAIL:
  TCP_NODELAY is enabled on accepted server sockets and client streams;
  HTTP response reading stops at Content-Length instead of waiting for close;
  LB -> worker uses the internal POST /score-compact response shape.

Rejected experiment:
  grouped LB -> worker POST /replay batching was tested and rejected because it
  changes per-row latency semantics into batch latency and made p99 worse.
```

Current local result after serving packed-only split and compact worker response:

```text
command: cargo run --release -p nando-cli -- role-binding-profile-lb-replay-v1
verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
worker_count: 2
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
load_balancer_p99_latency_ns: 736030
core_score_p99_latency_ns: 78902
worker_score_p99_latency_ns: 167663
lb_upstream_roundtrip_p99_latency_ns: 735692
replay_client_wall_p99_latency_ns: 5489536
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
max_worker_runtime_bytes_estimate: 492792
```

Packed serving-only score delta against the previous reference-path local baseline:

```text
previous_reference_core_score_p99_latency_ns: 88086
packed_serving_only_core_score_p99_latency_ns: 78902
core_p99_delta_ns: -9184
core_p99_reduction_milli: 104
previous_packed_dual_max_worker_runtime_bytes_estimate: 891248
packed_serving_only_max_worker_runtime_bytes_estimate: 492792
runtime_bytes_delta: -398456
runtime_bytes_reduction_milli: 447
```

Current clean HostWorld result after redeploying the serving packed-only static
binary plus transport cleanup:

```text
command: ssh hostworld-ee 'cd /opt/nando-wave-profile-runtime-v1 && ./target/release/nando-cli role-binding-profile-lb-replay-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1-clean2.product-proof.json 2 128 4'
verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
worker_count: 2
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
load_balancer_p99_latency_ns: 2744444
core_score_p99_latency_ns: 184328
worker_score_p99_latency_ns: 698145
lb_upstream_roundtrip_p99_latency_ns: 2743851
replay_client_wall_p99_latency_ns: 19478822
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
max_worker_runtime_bytes_estimate: 492792
```

Second HostWorld clean run evidence:

```text
profile-lb-replay-hostworld-v1-clean.product-proof.json:
  verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
  load_balancer_p99_latency_ns: 2993688
  core_score_p99_latency_ns: 187721
  worker_score_p99_latency_ns: 545095
  packed_score_parity_mismatches: 0
  false_local_accepts: 0
```

Interpretation:

```text
The packed score path is correct and still faster than the reference-path local
baseline at the score-loop layer. The serving packed-only split removes the
reference table and reference HashMap from workers and cuts max runtime bytes
from 891248 to 492792. The deployed cheap-VPS target is back inside the 3 ms
p99 envelope on two clean runs after transport cleanup and compact LB -> worker
responses. This is still not real Codex/API production traffic and not
concurrent throughput proof.
```

## 2026-07-03 - Executor Integration: Historical Deployed HostWorld Replay PASS Before Packed Hot-Path

Verdict:

```text
ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
```

Live remote command:

```text
ssh hostworld-ee 'cd /opt/nando-wave-profile-runtime-v1 && ./target/release/nando-cli role-binding-profile-lb-replay-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1.product-proof.json 2 128 4'
```

Historical remote result:

```text
host_alias: hostworld-ee
bundle_root: /opt/nando-wave-profile-runtime-v1
binary: x86_64-unknown-linux-musl static nando-cli
worker_count: 2
total_profile_count: 7
unique_sequences_replayed: 896
http_replay_batches: 224
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
local_operator_calls: 448
fallback_to_llm_calls: 448
false_local_accepts: 0
missed_expected_local: 0
load_balancer_p50_latency_ns: 1726306
load_balancer_p90_latency_ns: 2303526
load_balancer_p99_latency_ns: 2834992
load_balancer_rss_bytes: 2555904
max_worker_runtime_bytes_estimate: 398456
max_worker_p99_latency_ns: 587727
all_workers_serving_only: true
load_balancer_serving_only: true
```

Important implementation note:

```text
The first deployed attempt exposed a packaging issue: the local glibc binary
required GLIBC_2.43. Executor rebuilt `nando-cli` as
x86_64-unknown-linux-musl static binary.

The first remote static-binary run was WATCH on latency:
load_balancer_p99_latency_ns: 3340918.

Executor removed the artificial 2 ms sleep in the HTTP client helper and
reran. That historical deployed bundle passed at 2834992 ns p99. This section
is kept only as provenance; the current packed hot-path deployed result is the
WATCH/FAIL section above.
```

What this closed at that checkpoint:

```text
The historical product runtime had a deployed cheap-VPS replay proof for the
sampled release-suite profile runtime before the current packed hot-path
redeploy:
  client sends replay traffic to one proxy endpoint;
  proxy loads route-to-upstream metadata only;
  proxy dispatches to serving-only `.nwrb` worker shards;
  `.nwreb` is used only by the replay client;
  false_local_accepts = 0;
  p99 was inside the 1-3 ms cheap-VPS target envelope.
```

Boundary:

```text
This is still not real Codex/API production traffic, not concurrent throughput
proof, and not full OPERATOR_BLUEPRINT coverage.
```

## 2026-07-03 - Executor Integration: Profile Load-Balancer Replay PASS

Verdict:

```text
ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
```

Live command:

```text
cargo run --release -p nando-cli -- role-binding-profile-lb-replay-v1
```

Current result:

```text
worker_count: 2
total_profile_count: 7
unique_sequences_replayed: 896
http_replay_batches: 224
no_cache_llm_calls: 1792
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
local_operator_calls: 448
fallback_to_llm_calls: 448
false_local_accepts: 0
missed_expected_local: 0
load_balancer_p50_latency_ns: 474665
load_balancer_p90_latency_ns: 580300
load_balancer_p99_latency_ns: 736030
core_score_p99_latency_ns: 78902
worker_score_p99_latency_ns: 167663
lb_upstream_roundtrip_p99_latency_ns: 735692
replay_client_wall_p99_latency_ns: 5489536
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
load_balancer_rss_bytes: 6307840
max_worker_runtime_bytes_estimate: 492792
max_worker_p99_latency_ns: 167663
all_workers_serving_only: true
load_balancer_serving_only: true
```

What this closes:

```text
The product path now has a local external load-balancer/proxy proof:
  client sends replay traffic to one proxy endpoint;
  proxy loads route-to-upstream metadata only;
  proxy dispatches to serving-only `.nwrb` worker shards;
  `.nwreb` is used only by the replay client;
  false_local_accepts = 0.
```

Boundary:

```text
This is the current packed hot-path local proof. The deployed HostWorld packed
hot-path replay above is now back inside the 3 ms latency envelope. The
remaining product proof is real Codex/API traffic replay and, separately,
concurrent throughput under production routing.
```

## 2026-07-03 - Executor Integration: Profile Worker Replay PASS

Verdict:

```text
ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS
```

Live command:

```text
cargo run --release -p nando-cli -- role-binding-profile-worker-replay-v1
```

Current result:

```text
worker_count: 2
total_profile_count: 7
unique_sequences_replayed: 896
http_replay_batches: 224
no_cache_llm_calls: 1792
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
local_operator_calls: 448
fallback_to_llm_calls: 448
false_local_accepts: 0
missed_expected_local: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_rss_bytes: 7135232
max_worker_p99_latency_ns: 265277
all_workers_serving_only: true
```

What this closes:

```text
The product path now has local sharded HTTP replay:
  exact-cache competition across workers;
  route-to-profile-shard dispatch in the replay client;
  `.nwreb` used only outside serving workers;
  serving workers load `.nwrb` only;
  false_local_accepts = 0.
```

Boundary:

```text
This is still not real Codex/API traffic, not an external load-balancer proof,
not concurrent throughput scaling, and not cheap-VPS deployment.
```

## 2026-07-03 - Executor Integration: Profile Worker Scaling PASS

Verdict:

```text
ROLE_BINDING_PROFILE_WORKER_SCALING_V1_PASS
```

Live command:

```text
cargo run --release -p nando-cli -- role-binding-profile-worker-scaling-v1
```

Current result:

```text
worker_count: 2
total_profile_count: 7
total_local_operator_calls: 7
total_fallback_to_llm_calls: 2
wrong_worker_route_fallbacks: 2
false_local_accepts: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_rss_bytes: 6557696
max_worker_p99_latency_ns: 6286
all_workers_serving_only: true
all_profile_score_pass: true
all_wrong_worker_routes_fallback: true
```

What changed:

```text
Executor adopted and completed the previously unfinished worker-scaling branch.
The command is wired into the CLI, writes shard registry configs, starts two
serving-only `.nwrb` workers, checks local accept for every profile, checks
wrong-worker route fallback, and writes a product-proof JSON report.
```

Boundary:

```text
This closes local profile-shard worker acceptance. It is not real Codex
traffic, not an external load-balancer proof, and not throughput scaling proof.
This historical boundary is superseded by:
  ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS.
  ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS.
  historical deployed HostWorld pre-packed replay PASS.
  current deployed HostWorld packed replay PASS after transport cleanup.
Next product layer should focus on real Codex/API traffic replay and
concurrent production-style throughput.
```

## 2026-07-03 - HARD ROLE BOUNDARY: Reviewer-Only Mode

Verdict:

```text
REVIEWER_ONLY_MODE_ACTIVE
NO_CODE_WRITES_BY_REVIEWER
COMMUNICATION_THROUGH_THIS_FILE
```

User instruction:

```text
Reviewer must not write implementation code.
Reviewer communicates with executor through this file only.
Reviewer must coordinate all actions with executor before acting.
```

Important correction:

```text
The reviewer started a worker/profile-shard scaling implementation without
explicit executor/user handoff. Treat that as NOT approved product integration.

Do not treat reviewer-started worker-scaling code as authoritative.
Executor owns the implementation path.
Reviewer role is now:
  inspect,
  find gaps,
  write notes here,
  verify after executor integration.
```

Coordination rule:

```text
Before any non-read-only action, reviewer must first leave a proposed action in
this file and wait for executor/user direction.

This includes:
  code edits,
  report edits,
  doc rewrites,
  running heavy commands,
  changing default constants,
  changing product claims.

Allowed without prior executor agreement:
  read-only inspection,
  summarizing current evidence,
  writing a reviewer note in this file when asked by user.
```

Current known worker-scaling state:

```text
Resolved by executor-owned integration:
  role-binding-profile-worker-scaling-v1 now passes.
  The previous unfinished reviewer-started branch is no longer the current
  state of the product runtime.
```

Executor-owned next product target:

```text
Build the next external proof layer:
  real Codex/API traffic replay if a trace exists;
  concurrent production-style throughput;
  keep serving workers .nwrb-only;
  keep exact-cache baseline;
  false_local_accepts = 0;
  report p50/p90/p99, RSS, runtime_bytes_estimate, and LLM-call reduction.
```

Reviewer will check after executor integration:

```text
cargo fmt --check
cargo check -p nando-cli
cargo clippy -p nando-cli -- -D warnings
worker/profile-shard report JSON
docs/report consistency
no stale demo/research wording in product path
```

## 2026-07-03 - Reviewer Sync: Profile Fallback Smoke PASS

Verdict:

```text
ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_PASS
```

Live command:

```text
cargo run -p nando-cli -- role-binding-profile-fallback-smoke-v1
```

Current result:

```text
profile_count: 7
local_accept_pass: true
bad_route_fallback_pass: true
low_margin_fallback_pass: true
local_action: local_operator
bad_route_fallback_reason: profile_not_found
low_margin_fallback_reason: margin_below_threshold
local_energy_margin: 4194304
low_margin_energy_margin: 1024
low_margin_threshold: 1000000
local_operator_calls: 1
fallback_to_llm_calls: 2
false_local_accepts: 0
p99_latency_ns: 24312
runtime_bytes_estimate: 790020
compiler_used: false
eval_packs_loaded: false
corpus_jsonl_loaded: false
python_demo_used: false
```

What this closes:

```text
The serving-only `.nwrb` profile runtime now has a direct product guard for:
  local accept on a valid high-margin profile request;
  missing route fallback;
  low-margin fallback;
  zero false local accepts.
```

Boundary:

```text
This is not worker/profile-shard scaling and not real Codex traffic.
This historical boundary is superseded by:
  ROLE_BINDING_PROFILE_WORKER_SCALING_V1_PASS.
  ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS.
  ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS.
  historical deployed HostWorld pre-packed replay PASS.
  current deployed HostWorld packed replay PASS after transport cleanup.
Next product layer should focus on real Codex/API traffic replay and
concurrent production-style throughput.
```

## 2026-07-03 - Reviewer Sync: Profile Replay Suite PASS + Batch Boundary

Verdict:

```text
ROLE_BINDING_PROFILE_REPLAY_SUITE_V1_PASS
DEFAULT_REPLAY_CLI_PATH_PASS
```

Live rerun:

```text
cargo run --release -p nando-cli -- role-binding-profile-replay-suite-v1
```

Result:

```text
unique_sequences_replayed: 896
http_replay_batches: 224
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
missed_expected_local: 0
p50_latency_ns: 125821
p90_latency_ns: 148048
p99_latency_ns: 213509
runtime_bytes_estimate: 790020
rss_bytes: 8101888
eval_packs_loaded_in_serving_worker: false
eval_packs_used_by_replay_client: true
corpus_jsonl_loaded_in_serving_worker: false
python_demo_used: false
```

Important boundary:

```text
The replay-suite is now a real product-shaped step beyond smoke:
  serving worker loads only `.nwrb`;
  replay client uses `.nwreb` only to generate requests;
  exact-cache baseline is measured;
  reduction target >= 20% is exceeded at 50%;
  false_local_accepts stays 0.
```

Found issue:

```text
The first live check exposed two default-path problems:

1. batch=32 failed with:
   HTTP POST /replay failed: status=413
   body: HTTP request body too large

2. after lowering batch, max_unique=256 replayed 1792 unique sequences and
   failed the current p99 <= 3ms gate:
   p99_latency_ns: 6582625

Fixed in code:
  DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE = 128
  DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES = 4

Default CLI rerun now passes.
```

Next executor instruction:

```text
Continue from the now-green default replay path:
  realistic product replay mix,
  real Codex traffic replay,
  concurrent production-style throughput.

Do not widen max_unique or batch again without a source-verified report that
still passes p99 <= 3ms and false_local_accepts = 0.
```

## 2026-07-03 - Reviewer Sync: Product Runtime Direction Confirmed

Verdict:

```text
REVIEWER_SYNC_PASS
ROLE_BINDING_PROFILE_RUNTIME_SMOKE_LIVE_RERUN_PASS
```

I rechecked the current product-runtime direction after the handoff.

Live checks:

```text
cargo fmt --check: PASS
cargo check -p nando-cli: PASS
cargo run -p nando-cli -- role-binding-profile-runtime-smoke-v1: PASS
```

Current live smoke result:

```text
role-binding-profile-runtime-smoke-v1:
  ROLE_BINDING_PROFILE_RUNTIME_SMOKE_V1_PASS

profile_count: 7
runtime_bytes_estimate: 790020
exact_cache_llm_calls: 2
exact_cache_plus_nando_llm_calls: 1
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
p99_latency_ns: 21436

compiler_used: false
eval_packs_loaded: false
corpus_jsonl_loaded: false
python_demo_used: false
```

What is correct:

```text
The executor started in the right direction:
  serving-only .nwrb runtime
  profile registry
  /health /profiles /score /replay /metrics
  exact-cache comparison
  latency/RSS/runtime-bytes reporting
  no .nwreb eval packs in serving mode
```

Boundary:

```text
This closes a local product-shaped smoke, not real Codex traffic.
OPERATOR_BLUEPRINT is still WATCH:
  proven_classes: 0
  partial_classes: 7
  missing_classes: 2
  missing: FIELD, FILTER_GROUP
```

Next reviewer instruction:

```text
Do not go back to research-only gates.
Next product work should extend the serving runtime with:
  product replay with more realistic cache-miss flow,
  real Codex traffic replay,
  external load-balancer / cheap-VPS proof,
  report proving >=20% incremental reduction vs exact cache with false_local_accepts=0.
```

## 2026-07-03 - Role-Binding Product Profile Runtime PASS

Verdict:

```text
ROLE_BINDING_PROFILE_REGISTRY_FROM_RELEASE_V1_PASS
ROLE_BINDING_PROFILE_RUNTIME_SMOKE_V1_PASS
ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_V1_WATCH
```

Current evidence:

```text
registry_config: target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json
runtime_smoke_report: target/nando-wave/role-binding-profile-runtime/profile-runtime-smoke-v1.product-proof.json
runtime_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_RUNTIME_SMOKE.md
release_suite_report: target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json
gap_report: target/nando-wave/slot32-role-binding/role-binding-operator-blueprint-gap-v1.product-proof.json

profile_count: 7
runtime_bytes_estimate: 790020
package_bytes: 134912
edge_count: 11217
exact_cache_llm_calls: 2
exact_cache_plus_nando_llm_calls: 1
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
p99_latency_ns: 37468
rss_bytes: 10932224

compiler_used: false
eval_packs_loaded: false
corpus_jsonl_loaded: false
python_demo_used: false
```

Boundary:

```text
Serving runtime PASS is a product-shaped local smoke over `.nwrb` profiles. It
is not real Codex production traffic, not raw-language action parsing, not
`.nwpc` bridge proof, and not full OPERATOR_BLUEPRINT closure.

OPERATOR_BLUEPRINT remains WATCH:
  partial_classes: 7
  missing_classes: 2
  missing: FIELD, FILTER_GROUP
```

## 2026-07-03 - Role-Binding Release Suite With EDIT

Verdict:

```text
ROLE_BINDING_RELEASE_SUITE_V1_PASS
ROLE_BINDING_RELEASE_SUITE_VERIFY_V1_PASS
EDIT_RELEASE_SUITE_INTEGRATION_PASS
ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_V1_WATCH
```

Current evidence:

```text
release_suite_report: target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json
binary_suite_report: target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json
edit_package_log: data/rule_logic_operator_battery_v4/edit/edit_role_binding_public_sdk_package_release.log
edit_report: data/rule_logic_operator_battery_v4/edit/EDIT_RUNTIME_BOUNDARY_REPORT.md

package_count: 7
binary_eval_pack_count: 7
score_report_count: 7
total_sequence_count: 27648
total_sequence_false_local_accepts: 0
min_sequence_strict_ordered_accuracy_milli: 1000

sdk_edit_marker_length:
  margin_threshold: 1
  package_bytes: 1664
  package_edge_count: 135
  sequence_median_energy_margin: 6144
```

Boundary:

```text
EDIT is now source-verified in the `.nwrb/.nwreb` release suite, but only as
PARTIAL OPERATOR_BLUEPRINT coverage. Do not claim full EDIT, FIELD,
FILTER_GROUP, or full 32-slot operator battery closure.
```

## 2026-07-03 - Product Runtime Direction

Verdict:

```text
PRODUCT_RUNTIME_TASK_READY
```

Next work should start from:

```text
docs/NANDO_WAVE_PRODUCT_RUNTIME_TASK.md
docs/NANDO_WAVE_L2_PROFILE_SHARDING_TESTS.md
```

Core direction:

```text
route -> L2-sized profile shard -> local score -> fallback
```

Do not build more eval-heavy demo workers. Build a serving-only runtime:

```text
load .nwrb packages only
no .nwreb eval packs in serving mode
profile registry
/health /profiles /score /replay /metrics
exact-cache comparison
0 false local accepts
latency/RSS/runtime-bytes report
```

## 2026-07-03 - Operator Battery V4 EDIT Current-Source Runtime Gate

Verdict:

```text
EDIT_CURRENT_SOURCE_RUNTIME_GATE_PASS
EDIT_RELEASE_SUITE_INTEGRATION_PASS
```

What changed:

```text
Reran the EDIT marker/length runtime gate against the current Rust sources and
overwrote the stale release log. The fresh run is stronger than the old report:
the gate still passes, but with only 136 role_binding_edges in the current
runtime path.
```

Current evidence:

```text
report: data/rule_logic_operator_battery_v4/edit/EDIT_RUNTIME_BOUNDARY_REPORT.md
runtime_log: data/rule_logic_operator_battery_v4/edit/edit_marker_length_runtime_gate_release.log
boundary_log: data/rule_logic_operator_battery_v4/edit/edit_runtime_boundary_gate.log

runtime_command:
  cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_edit_marker_length_must_transfer_without_lookup_or_runtime_phase_hack --nocapture

boundary_command:
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_edit_current_role_binding_runtime_boundary_must_be_explicit --nocapture

runtime_result: ok, 1 passed, 40 filtered out, finished in 76.94s
boundary_result: ok, 1 passed, 40 filtered out, finished in 0.26s

train_rows: 1536
heldout_rows: 1536
edit_output_slot_count: 17
edit_role_slot_count: 17
edit_marker_role_slot: 16
edit_slot_ordered_sequence_accuracy_milli: 1000
edit_flat_slot_ordered_sequence_accuracy_milli: 1000
edit_sequence_energy_accuracy_milli: 1000
edit_sequence_energy_median_gap: 39424
edit_sequence_energy_p10_gap: 13056
edit_energy_pass_slot_fail: 0
edit_output_slot_cleanup_failed_slots: 0
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
state_delta_edges: 0
role_binding_edges: 136

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_edit_demo_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

Boundary:

```text
EDIT is now a fresh current-source runtime PASS and is integrated into the
`.nwrb/.nwreb` role-binding release suite as `sdk_edit_marker_length`. Do not
overclaim it: OPERATOR_BLUEPRINT marks EDIT as PARTIAL because the full EDIT
family is still not closed.
```

## 2026-07-03 - Slot32 Role-Binding Operator Blueprint Gap

Verdict:

```text
ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_V1_WATCH
ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_VERIFY_V1_PASS
```

What changed:

```text
Added a claim-boundary audit over the current strict 32-slot role-binding
release suite:
  role-binding-operator-blueprint-gap-v1
  role-binding-operator-blueprint-gap-verify-v1

The audit checks the green `.nwrb/.nwreb` release-suite evidence against
docs/OPERATOR_BLUEPRINT.md and refuses to treat the current role-binding bundle
as the full 32-slot operator battery.
```

Current evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_OPERATOR_BLUEPRINT_GAP.md
gap_report: target/nando-wave/slot32-role-binding/role-binding-operator-blueprint-gap-v1.product-proof.json
release_suite_report: target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json

release_suite_report_fingerprint64: 6657695271699713258
release_suite_gate_pass: true
release_suite_package_count: 7
release_suite_total_sequence_count: 27648
release_suite_min_sequence_strict_ordered_accuracy_milli: 1000
release_suite_min_sequence_median_energy_margin: 6144
all_forbidden_flags_false: true

blueprint_required_class_count: 9
proven_classes: 0
partial_classes: 7
missing_classes: 2
coverage_gate_pass: false
full_32_slot_operator_battery_closed: false
report_matches_sources: true
```

Coverage:

```text
PARTIAL: SELECT, MOVE_COPY, EDIT, ORDER, CONDITION_ROUTE, COMPOSE, VERIFY_REPAIR
MISSING: FIELD, FILTER_GROUP
PROVEN: none against the full OPERATOR_BLUEPRINT class contract
```

Boundary:

```text
This is a WATCH claim-boundary report, not a release failure. The existing
role-binding release suite remains green, but the full 32-slot operator battery
remains open.
```

## 2026-07-03 - Slot32 Role-Binding Release Suite

Verdict:

```text
ROLE_BINDING_RELEASE_SUITE_V1_PASS
ROLE_BINDING_RELEASE_SUITE_VERIFY_V1_PASS
```

What changed:

```text
Added a release-suite proof bundle for the current strict 32-slot role-binding
path:
  role-binding-release-suite-v1
  role-binding-release-suite-verify-v1

The suite ties `.nwrb` packages, all-seed `.nwreb` eval-packs, per-row
binary/score reports, and the aggregate binary suite into one source-verified
product-proof report.
```

Current evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_RELEASE_SUITE.md
release_suite_report: target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json
binary_suite_report: target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json

package_count: 7
binary_eval_pack_count: 7
score_report_count: 7
total_package_bytes: 134912
total_binary_eval_pack_bytes: 369676909
total_sequence_count: 27648
total_expected_local_sequences: 13824
total_expected_fallback_sequences: 13824
total_sequence_false_local_accepts: 0
total_sequence_missed_expected_local: 0
min_sequence_strict_ordered_accuracy_milli: 1000
min_sequence_median_energy_margin: 6144
all_packages_magic_match: true
all_packages_bytes_match_inspect: true
all_package_fingerprints_match_suite: true
all_eval_pack_magic_match: true
all_eval_pack_fingerprints_match_suite: true
all_binary_reports_match_suite_rows: true
all_score_reports_match_suite_rows: true
all_forbidden_flags_false: true
report_matches_sources: true
```

Boundary:

```text
This closes a product-proof release bundle for the current strict 32-slot
role-binding package/eval-pack set.

Do not claim this closes the full 32-slot operator battery, `.nwpc` bridge,
raw-language action parsing, broad workflow reasoning, text generation, or
commercial license. The serving-only profile runtime is tracked separately.
```

## 2026-07-03 - Slot32 Role-Binding Binary Eval-Pack Suite

Verdict:

```text
ROLE_BINDING_BINARY_EVAL_PACK_SUITE_V1_PASS
ROLE_BINDING_BINARY_EVAL_PACK_SUITE_VERIFY_V1_PASS
```

What changed:

```text
Added all-seed `.nwreb` suite commands for the current slot32 role-binding
package set:
  role-binding-binary-eval-pack-suite-v1
  role-binding-binary-eval-pack-suite-verify-v1

The suite converts and scores all current seed/label corpus eval-packs through the
serialized `.nwrb` role-binding runtime and verifies the aggregate report
against current sources.
```

Current evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_BINARY_EVAL_PACK_SUITE.md
suite_report: target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json

suite_items: 7
total_source_eval_pack_bytes: 2790622842
total_binary_eval_pack_bytes: 369676909
suite_size_reduction_milli: 867
total_sequence_count: 27648
total_expected_local_sequences: 13824
total_expected_fallback_sequences: 13824
total_sequence_false_local_accepts: 0
total_sequence_missed_expected_local: 0
min_sequence_strict_ordered_accuracy_milli: 1000
min_sequence_median_energy_margin: 6144
all_binary_gate_pass: true
all_binary_reports_match_sources: true
all_score_gate_pass: true
all_score_reports_match_sources: true
all_eval_pack_format_binary: true
all_package_fingerprints_match: true
```

Boundary:

```text
This closes compact binary `.nwreb` role-binding eval-pack packaging and
scoring for the current 32-slot role-binding package set with per-item margin
thresholds.

Do not claim this closes the full 32-slot operator battery, `.nwpc` bridge,
raw-language action parsing, broad workflow reasoning, or text generation.
```

## 2026-07-03 - Slot32 Role-Binding Binary Eval-Pack Rung

Verdict:

```text
ROLE_BINDING_EVAL_PACK_BINARY_V1_PASS
ROLE_BINDING_PACKAGE_SCORE_V1_PASS
ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS
```

What changed:

```text
Added compact binary `.nwreb` eval-pack support for role-binding sequence
scoring. The same `role-binding-package-score-v1` command now auto-detects JSON
or binary eval-packs by magic/header.
```

Current evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_BINARY_EVAL_PACK_RUNG.md
source_eval_pack_json: target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.json
binary_eval_pack: target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.nwreb
binary_pack_report: target/nando-wave/slot32-role-binding/role-binding-eval-pack-binary-corpus-v1.product-proof.json
binary_score_report: target/nando-wave/slot32-role-binding/role-binding-package-score-binary-corpus-v1.product-proof.json

binary_magic_text: NWRE0001
package_fingerprint64: 365065097387925697
sequence_count: 4096
source_eval_pack_bytes: 455828420
binary_eval_pack_bytes: 60587229
size_reduction_milli: 867
roundtrip_exact: true
eval_pack_format: binary
sequence_strict_ordered_accuracy_milli: 1000
sequence_false_local_accepts: 0
sequence_missed_expected_local: 0
report_matches_sources: true
```

Boundary:

```text
This closes compact binary role-binding eval-pack packaging and scoring for the
representative 32-slot conditional package.

The all-seed binary eval-pack boundary is superseded by the suite section
above. Do not claim this representative rung closes `.nwrb` daemon/registry
routing, `.nwpc` bridge, raw-language action parsing, broad workflow reasoning,
or text generation.
```

## 2026-07-03 - Slot32 Role-Binding CLI Corpus Score/Verify Rung

Verdict:

```text
ROLE_BINDING_PACKAGE_SCORE_V1_PASS
ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS
```

What changed:

```text
The `.nwrb` CLI score path now supports full sequence eval rows:
  active_fringe
  output slots
  positive/negative impulses
  strict ordered slot pass
  sequence energy margin

The 32-slot public SDK package gate now emits corpus eval-packs from heldout
corpus rows, not from package edges.
```

Current evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_CLI_CORPUS_SCORE_RUNG.md
package: target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb
corpus_eval_pack: target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.json
score_report: target/nando-wave/slot32-role-binding/role-binding-package-score-corpus-v1.product-proof.json

package_fingerprint64: 365065097387925697
eval_pack_fingerprint64: 14754950188000667967
margin_threshold: 1000000
sequence_count: 4096
expected_local_sequences: 2048
expected_fallback_sequences: 2048
sequence_local_operator_calls: 2048
sequence_fallback_to_llm_calls: 2048
sequence_false_local_accepts: 0
sequence_missed_expected_local: 0
sequence_strict_ordered_accuracy_milli: 1000
sequence_median_energy_margin: 2449664
report_matches_sources: true
```

Current-source package rerun:

```text
verdict: SLOT32_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG_PASS
seeds: 3
labels: {"sdk_conditional_branch", "sdk_mixed_map"}
min_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
total_sdk_gap_parity_mismatches: 0
total_sdk_sequence_energy_parity_mismatches: 0
total_false_local_accepts: 0
max_p99_latency_ns: 689788
```

Boundary:

```text
This closes independent corpus-emitted `.nwrb` CLI sequence scoring for one
representative 32-slot conditional package.

The JSON corpus eval-pack is huge (~456 MB for seed1 conditional; target
slot32 dir ~2.6 GB after all six seed/label exports). Treat compact binary
role-binding eval-pack as the next packaging debt.

The next rung closed compact binary eval-pack packaging for the representative
conditional package. Daemon registry routing, `.nwpc` bridge, raw-language
action parsing, broad workflow reasoning, and text generation remain open.
```

## 2026-07-03 - Slot32 Role-Binding CLI Score/Verify Rung

Verdict:

```text
ROLE_BINDING_PACKAGE_SCORE_V1_PASS
ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS
```

What changed:

```text
Added `.nwrb` CLI scoring commands over an explicit eval-pack interface:
  role-binding-eval-pack-from-package-v1
  role-binding-package-score-v1
  role-binding-package-score-verify-v1

The score path loads package bytes through the public role-binding SDK runtime,
scores local/fallback eval rows, writes a deterministic report, and verify
rebuilds the report from package + eval-pack.
```

Current evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_CLI_SCORE_RUNG.md
package: target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb
eval_pack: target/nando-wave/slot32-role-binding/role-binding-eval-pack-v1.json
score_report: target/nando-wave/slot32-role-binding/role-binding-package-score-v1.product-proof.json

package_fingerprint64: 365065097387925697
eval_pack_fingerprint64: 14619240648419331465
task_count: 128
local_operator_calls: 64
fallback_to_llm_calls: 64
false_local_accepts: 0
missed_expected_local: 0
report_matches_sources: true
```

Boundary:

```text
This closes `.nwrb` CLI scoring/verify plumbing over an explicit eval-pack.
The generated eval-pack in this rung is package-derived and is only a scoring
smoke, not an independent corpus proof.

Do not claim this closes independent corpus-emitted `.nwrb` eval-pack, daemon
registry routing, `.nwpc` bridge, raw-language action parsing, broad workflow
reasoning, or text generation.
```

## 2026-07-02 - Reviewer Direction: Build 32-Slot Product Rung

Verdict:

```text
BUILD_32_SLOT_PRODUCT_RUNG_NEXT
```

Direction:

```text
Keep the current 16-slot/C32 path as frozen baseline and demo fallback.
Do not replace or destabilize it.

Next product-facing scaling target is 32 slots.

Reason:
  16 slots prove the base transferable-action engine.
  32 slots are needed for realistic Codex-like local offload windows:
    file/function/symbol/import/callsite/type/error/test/condition/patch roles
    can exceed 16 active roles quickly.

Goal:
  make 32 slots a real proof rung, not just capacity smoke.
```

Required 32-slot gates:

```text
1. real 32-slot operator corpus, not only smoke maps
2. strict ordered decoder must pass
3. sequence/operator energy must pass
4. flat/runtime parity must pass
5. action/role/binding ablations must collapse
6. shortcut gates must stay clean
7. multi-seed robustness
8. cache/offload workflow benchmark
9. no false local accepts
10. explicit p99 runtime target and memory/cache report
```

Boundary:

```text
Do not count success from:
  Python demo
  lookup
  target_id
  proof_rule_id authority
  concrete_x_lookup
  manual local_out_t
  stale logs
  hidden hardcode

Do not change architecture until a red 32-slot gate is reproduced and diagnosed.
```

## 2026-07-02 - Reviewer Direction: Split Phase Package CLI

Verdict:

```text
PHASE_PACKAGE_CMD_NEEDS_MECHANICAL_MODULE_SPLIT
```

Why:

```text
crates/nando-cli/src/phase_package_cmd.rs is now 16415 lines.
It mixes too many concerns:
  package build/inspect/score;
  product proof/release/regression/freeze;
  workflow bench/replay;
  strict multiseed audit;
  corpus generation;
  coverage/shortcut gates;
  report structs;
  argument parsing;
  path defaults;
  JSON/report printing.

This size makes freshness/audit review harder and increases the chance of
touching proof-source files after a long runtime rerun.
```

Refactor gate first:

```text
Do not split `phase_package_cmd.rs` by intuition or by file size alone.

Use the NANDA structural-gate refactor workflow first:

  /home/ubu/.codex/skills/nanda-structural-gate/scripts/nanda-self-check
  /home/ubu/.codex/skills/nanda-structural-gate/scripts/nanda-boundary-economics . --find-refactors --format json
  /home/ubu/.codex/skills/nanda-structural-gate/scripts/nanda-dogfood . --refactor-plan --boundary-economics --format json
  /home/ubu/.codex/skills/nanda-structural-gate/scripts/nanda-map-code crates/nando-cli/src/phase_package_cmd.rs --format json

Policy from the skill:
  NO EVIDENCE => NO CUT
  WATCH => unresolved, not permission to split
  SPLIT_STRONG => mechanical split allowed after required tests
  SPLIT_WEAK => only a small preparatory step plus human review
  VETO => stop and repair route/owner conflict first

If the global packet is too large, split into route-local checks instead of
raising limits or pretending the global WATCH is PASS.
```

Direction after the refactor gate allows it:

```text
Do a mechanical split only. No semantic/runtime changes in the same step.

Suggested shape:
  crates/nando-cli/src/phase_package_cmd/mod.rs
  crates/nando-cli/src/phase_package_cmd/args.rs
  crates/nando-cli/src/phase_package_cmd/paths.rs
  crates/nando-cli/src/phase_package_cmd/package.rs
  crates/nando-cli/src/phase_package_cmd/action_corpus.rs
  crates/nando-cli/src/phase_package_cmd/action_runtime.rs
  crates/nando-cli/src/phase_package_cmd/product_proof.rs
  crates/nando-cli/src/phase_package_cmd/release.rs
  crates/nando-cli/src/phase_package_cmd/workflow.rs
  crates/nando-cli/src/phase_package_cmd/strict_multiseed.rs
  crates/nando-cli/src/phase_package_cmd/reports.rs

Keep public command functions re-exported from mod.rs so main.rs changes as
little as possible.
```

Required guard:

```text
After split:
  cargo fmt --check
  cargo check -p nando-cli
  cargo clippy -p nando-cli -- -D warnings
  phase-action-regression-verify-v1
  phase-action-regression-freeze-verify-v1

Do not rerun the 12 strict runtime logs until the split is done and source is
frozen. Otherwise the proof freshness loop repeats again.
```

## 2026-07-02 - Replay-Anchored Regression/Freeze And Strict Freshness Watch

Verdict:

```text
PHASE_ACTION_REGRESSION_V1_PASS_WITH_WORKFLOW_REPLAY_ANCHOR
PHASE_ACTION_REGRESSION_FREEZE_V1_PASS_WITH_WORKFLOW_REPLAY_ANCHOR
STRICT_MULTI_SEED_RUST_AUDIT_BEHAVIORAL_PASS_BUT_CURRENT_SOURCE_FRESHNESS_WATCH
```

What changed:

```text
`phase-action-regression-v1` and `phase-action-regression-freeze-v1` now read,
verify, and carry the stronger multi-package workflow replay report instead of
depending only on the small workflow-bench smoke.

This is proof-chain wiring only. It does not change runtime operator semantics.
```

Commands rerun:

```text
cargo run -p nando-cli --release -- phase-action-workflow-replay-v1
cargo run -p nando-cli --release -- phase-action-workflow-replay-verify-v1
cargo run -p nando-cli --release -- phase-action-regression-v1
cargo run -p nando-cli --release -- phase-action-regression-verify-v1
cargo run -p nando-cli --release -- phase-action-regression-freeze-v1
cargo run -p nando-cli --release -- phase-action-regression-freeze-verify-v1
```

Artifacts:

```text
workflow replay:
  target/nando-wave/action-runtime-v1-workflow-replay.product-proof.json

regression:
  target/nando-wave/action-runtime-v1-regression.product-proof.json

regression freeze:
  target/nando-wave/action-runtime-v1-regression-freeze.product-proof.json
```

Current evidence:

```text
workflow_replay_report_fingerprint64: 16637049491119000274
workflow_replay_report_bytes: 5274
workflow_replay_verify_pass: true
workflow_replay_report_matches_sources: true
workflow_replay_verdict: PHASE_ACTION_WORKFLOW_REPLAY_V1_PASS
workflow_replay_package_count: 3
workflow_replay_trace_calls: 3072
workflow_replay_total_unique_eval_rows: 308
workflow_replay_unique_rows: 308
workflow_replay_exact_cache_llm_calls: 308
workflow_replay_exact_cache_plus_nando_llm_calls: 36
workflow_replay_incremental_llm_calls_removed_vs_cache: 272
workflow_replay_incremental_llm_call_reduction_vs_cache_milli: 883
workflow_replay_local_accuracy_milli: 1000
workflow_replay_false_local_accepts: 0
workflow_replay_max_bench_p99_latency_ns: 117

regression_verdict: PHASE_ACTION_REGRESSION_V1_PASS
freeze_verdict: PHASE_ACTION_REGRESSION_FREEZE_V1_PASS
freeze_regression_report_fingerprint64: 2002304595771295125
freeze_regression_report_bytes: 6413
operator_blueprint_fingerprint64: 9874423192353457577
release_suite_report_fingerprint64: 9827723825761118426
```

Strict freshness boundary:

```text
After the replay-regression/freeze wiring, `crates/nando-cli/src/phase_package_cmd.rs`
was touched again at 2026-07-02 23:05:01 for CLI output cleanup.

The previous 12 runtime logs remain behaviorally green, but their current-source
freshness is no longer proven because the last canonical runtime log is
2026-07-02 22:45:41.

Do not claim STRICT_MULTI_SEED_RUST_AUDIT_PASS_CURRENT_SOURCE again until the
12 canonical logs and strict audit/verify reports are rerun after the latest
source timestamp.
```

Boundary:

```text
This closes replay anchoring inside regression/freeze for the packaged flat
action scorer. It does not close strict current-source freshness after the
23:05 CLI source edit, real pilot workflow, raw action parsing, text generation,
commercial license closure, or 32-slot full corpus proof.
```

## 2026-07-02 - Fresh Current-Source Rerun After Workflow Replay Code

Verdict:

```text
STRICT_MULTI_SEED_RUST_AUDIT_PASS_CURRENT_SOURCE_AFTER_RERUN
```

What changed:

```text
The reviewer override below was valid: the previous green 12-log audit was
stale against newer Rust source/test timestamps.

The canonical 12 release logs were rerun after freezing the relevant Rust
source/test files, then the strict audit and verify commands were rerun.

This rerun supersedes the earlier 18:23..18:56 freshness window because
`crates/nando-cli/src/phase_package_cmd.rs` changed again during the workflow
replay product-gate work at 2026-07-02 21:44:10.
```

Freshness check:

```text
relevant source/test files:
  crates/nando-core/tests/wavepredictor_binding_pressure_l3.rs
    2026-07-02 18:11:21
  crates/nando-core/src/wave/wavepredictor_hebbian.rs
    2026-07-02 17:28:01
  crates/nando-cli/src/phase_package_cmd.rs
    2026-07-02 21:44:10

fresh runtime logs:
  logs newer than source/test: 12/12
  first fresh runtime log: 2026-07-02 22:01:41
  last fresh runtime log:  2026-07-02 22:45:41
```

Current proof artifact:

```text
report: target/nando-wave/strict-multiseed-rust-audit-v1.product-proof.json
strict-multiseed-rust-audit-v1: STRICT_MULTI_SEED_RUST_AUDIT_PASS
strict-multiseed-rust-audit-verify-v1: STRICT_MULTI_SEED_RUST_AUDIT_VERIFY_PASS
report_matches_sources: true

observed_logs: 12
missing_logs: 0
strict_runtime_issues: 0
evidence_warnings: 0
logs_fingerprint64: 2824724535851559095
logs_total_bytes: 133299

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_logs_used: true
```

Boundary:

```text
This closes the current-source freshness debt for the v4 16-slot strict
multi-seed Rust runtime audit.

It does not close 32-slot full corpus proof, text generation, autonomous raw
action parsing, real pilot workflow, SDK/daemon packaging, or commercial
licensing.
```

Subchannel caveat:

```text
The required hard ablations still pass under the Rust audit.

Do not overclaim every diagnostic subchannel as independently sufficient:
  edit marker_role removal leaves energy high while strict accuracy falls to
  500 milli;
  conditional condition_action removal leaves energy partially high and seed 3
  has 3 milli strict accuracy.

The product claim remains the full channel proof plus required hard ablations,
not isolated marker_role / condition_action energy claims.
```

## 2026-07-02 - Reviewer Override

Verdict:

```text
WATCH_STALE_AFTER_FULL_12_LOG_RERUN
```

Reviewer finding:

```text
The executor's STRICT_MULTI_SEED_RUST_AUDIT_PASS_CURRENT_SOURCE note is not
currently proven by filesystem freshness.

Good news:
  all 12 canonical v4 multiseed release logs exist;
  all 12 logs ended with test result ok;
  forbidden flags are false in all 12 logs;
  no runtime failure/panic was found in the logs;
  slot/flat/energy/parity/ablation metrics in the logs are green.

But the current source/test timestamps are newer than the runtime logs:
  last runtime log:
    seed_003/composed/composed_runtime_gate_release.log
    2026-07-02 17:24:22

  newer source/test files:
    crates/nando-core/src/wave/wavepredictor_hebbian.rs
    2026-07-02 17:28:01

    crates/nando-cli/src/phase_package_cmd.rs
    2026-07-02 17:24:47

    crates/nando-core/tests/wavepredictor_binding_pressure_l3.rs
    2026-07-02 18:11:21

Freshness counts at review:
  logs newer than wavepredictor_hebbian.rs: 0/12
  logs newer than wavepredictor_binding_pressure_l3.rs: 0/12
  logs newer than phase_package_cmd.rs: 0/12
```

Boundary:

```text
This is not a model failure.
It is a proof freshness failure.

Do not call this current-source PASS until the 12 canonical logs are rerun
after the latest relevant source/test timestamps and the strict audit/verify
reports are regenerated after that rerun.
```

Direction for executor:

```text
Freeze source/test files first.
Then rerun the 12 canonical v4 multiseed release logs.
Then rerun:
  strict-multiseed-rust-audit-v1
  strict-multiseed-rust-audit-verify-v1

If any source/test file changes after the rerun, the current-source proof
becomes WATCH again.
```

## 2026-07-02 - Current Executor Result

Verdict:

```text
STRICT_MULTI_SEED_RUST_AUDIT_PASS_CURRENT_SOURCE_AFTER_RERUN
```

Current proof artifact:

```text
report: target/nando-wave/strict-multiseed-rust-audit-v1.product-proof.json
strict-multiseed-rust-audit-v1: STRICT_MULTI_SEED_RUST_AUDIT_PASS
strict-multiseed-rust-audit-verify-v1: STRICT_MULTI_SEED_RUST_AUDIT_VERIFY_PASS
report_matches_sources: true

observed_logs: 12
missing_logs: 0
strict_runtime_issues: 0
evidence_warnings: 0
logs_fingerprint64: 2847134219208477714
logs_total_bytes: 133299

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_logs_used: true
```

Fresh canonical release logs:

```text
latest_relevant_source_timestamp: 2026-07-02 23:05:01 +0300
fresh_log_window: 2026-07-02 23:24:45 +0300 .. 2026-07-03 00:08:10 +0300
stale_logs_vs_latest_source: 0

data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_001/order/order_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_002/order/order_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_003/order/order_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_001/edit/edit_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_002/edit/edit_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_003/edit/edit_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_001/conditional/conditional_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_002/conditional/conditional_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_003/conditional/conditional_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_001/composed/composed_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_002/composed/composed_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_003/composed/composed_runtime_gate_release.log
```

Runtime facts from fresh logs:

```text
order/edit/conditional/composed:
  strict ordered slot readout: 1000
  flat strict slot readout: 1000
  sequence energy: 1000
  flat gap parity mismatches: 0
  flat sequence-energy parity mismatches: 0
  energy_pass_slot_fail: 0
  output_slot_cleanup_failed_slots: 0
  slot_failure_total: 0

order role_binding_edges by seed: 87952 / 87867 / 88441
edit role_binding_edges by seed: 136 / 136 / 136
conditional role_binding_edges by seed: 40813 / 40813 / 40858
composed role_binding_edges by seed: 366 / 366 / 366
```

Diagnostic subchannel caveats from the same fresh logs:

```text
edit:
  ablation_without_marker_role_accuracy_milli: 500
  ablation_without_marker_role_energy_accuracy_milli: 1000

conditional:
  ablation_without_condition_action_accuracy_milli by seed: 0 / 0 / 3
  ablation_without_condition_action_energy_accuracy_milli by seed: 776 / 818 / 780

These are not blockers for the strict audit because the hard proof gates still
verify binding/action/role/active_fringe collapse, flat parity, strict slot
readout, and sequence energy. They are claim-boundary notes for future channel
cleanup work.
```

Engineering finding:

```text
The previous seed_002/order role/filler collision is fixed by principled
slot-scoped action filtering: scoped operator action pages must match both the
output slot and the source role slot before they vote.

This is not targeted duplication and not a manual local_out_t extension.
The edit path also removed the raw edit action surface; edit now relies on the
edit demo pair page plus role pages, which removes the marker/action surface
shortcut and shrinks edit role-binding to 136 edges.
```

Boundary:

```text
This closes the current v4 16-slot strict multi-seed Rust runtime audit over
canonical release logs.

It does not close:
  32-slot ordered decoder;
  64-slot capacity;
  broad product reasoning;
  autonomous raw action parsing;
  text generation;
  Python demo authority.
```

## 2026-07-02 - Slot32 Capacity Smoke

Verdict:

```text
SLOT32_PAGED_LAYOUT_CAPACITY_SMOKE_PASS
```

Artifact:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_PAGED_LAYOUT_CAPACITY_SMOKE.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_paged_layout_capacity_smoke_release.log
```

Current evidence:

```text
page_count: 64
total_center_count: 262144
output_slot_count: 32
role_slot_count: 32
role_top_l1_lanes: 64
operator_pair_source_bits: 5
lengths: 17..32

slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
energy_pass_slot_fail: 0
flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_mismatches: 0

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0

role_binding_edges: 892
flat_role_binding_edges: 892
hot_bytes_estimate: 600536

flat_eval_rows: 64
flat_eval_total_ns: 9434598
flat_eval_avg_ns_per_row: 147415

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
```

Multi-seed smoke evidence:

```text
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_paged_layout_multiseed_capacity_smoke_release.log

verdict: SLOT32_PAGED_LAYOUT_MULTI_SEED_CAPACITY_SMOKE_PASS
seeds: 3
min_slot_accuracy_milli: 1000
min_flat_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
min_sequence_energy_p10_gap: 593664
total_energy_pass_slot_fail: 0
total_flat_gap_parity_mismatches: 0
total_flat_sequence_energy_parity_mismatches: 0
max_hot_bytes_estimate: 600536
max_flat_eval_avg_ns_per_row: 150392

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
```

Flat runtime latency smoke:

```text
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_flat_runtime_latency_smoke_release.log

verdict: SLOT32_FLAT_RUNTIME_LATENCY_SMOKE_PASS
seed: 0
bench_repeats: 256
measured_rows: 16384
correct_rows: 16384
flat_accuracy_milli: 1000
p50_latency_ns: 135476
p99_latency_ns: 245822
max_latency_ns: 653733
avg_latency_ns: 144066
latency_gate_ns: 1000000
flat_role_binding_edges: 892
hot_bytes_estimate: 600536

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
```

Important red finding before the fix:

```text
role_top_l1_lanes=32:
  slot_accuracy_milli: 797
  sequence_energy_accuracy_milli: 1000
  energy_pass_slot_fail: 13

Interpretation:
  32-slot operator energy was present, but strict slot readout was
  under-resolved. The first fix is role-lane recall capacity, not local_out_t.
```

Engineering note:

```text
The flat readout path now prepares role strengths once per sequence row and
uses slot-scoped action grouping. Field/flat parity remains zero; the timing
above is a smoke-path measurement, not a p99 product latency claim.
```

Boundary:

```text
This is a 32-slot paged layout capacity smoke only.

It does not close:
  full 32-slot corpus battery;
  full 32-slot multi-seed corpus robustness;
  packed product runtime proof;
  product p99 latency proof;
  64-slot capacity.
```

## 2026-07-03 - Slot32 Order Corpus Rung

Verdict:

```text
SLOT32_ORDER_CORPUS_RUNG_PASS
```

Why this matters:

```text
This is the first real 32-slot order corpus rung beyond the synthetic capacity
smoke. It keeps the same paged u32 layout and flat runtime path, but raises the
proof pressure to a rule/surface/noise/length matrix.

Tokens are independent of rule_name and the same state key is reused under 8
different rules, so input/state alone is not enough. The action/operator-pair
channel is required.
```

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_order_corpus_must_transfer_without_lookup_or_runtime_phase_hack --nocapture
```

Artifacts:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ORDER_CORPUS_RUNG.md
log:    data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_order_corpus_rung_release.log
```

Corpus:

```text
seed: 0
train_rows: 1024
heldout_rows: 1024
unique_rules: 8
unique_surfaces: 4
unique_noise_types: 2
unique_lengths: 16
lengths: 17..32
same_bag_rows: 1024
max_train_state_reuse: 8
max_heldout_state_reuse: 8
train_tokens_overlap_heldout: 0
```

Runtime result:

```text
slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
sequence_energy_p10_gap: 3110016
energy_pass_slot_fail: 0

flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_mismatches: 0
flat_failed_rows: 0

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0
ablation_without_active_fringe_accuracy_milli: 0

role_binding_edges: 1354
hot_bytes_estimate: 606080
flat_eval_avg_ns_per_row: 185511
flat_eval_latency_gate_ns: 1000000

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Boundary:

```text
This closes the first 32-slot order corpus rung.

It does not close:
  full 32-slot operator battery;
  32-slot edit / conditional / composed gates;
  32-slot multi-seed corpus robustness;
  packed product runtime proof;
  product p99 latency proof.
```

## 2026-07-03 - Slot32 Order Corpus Multi-Seed Rung

Verdict:

```text
SLOT32_ORDER_CORPUS_MULTI_SEED_RUNG_PASS
```

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_order_corpus_multiseed_must_transfer_without_lookup_or_runtime_phase_hack --nocapture
```

Artifacts:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ORDER_CORPUS_MULTI_SEED_RUNG.md
log:    data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_order_corpus_multiseed_rung_release.log
```

Corpus:

```text
seeds: 3
rows_per_seed_train: 1024
rows_per_seed_heldout: 1024
unique_rules: 8
unique_surfaces: 4
unique_noise_types: 2
unique_lengths: 16
lengths: 17..32
same_bag_rows_per_seed: 1024
max_state_reuse_per_seed: 8
train_tokens_overlap_heldout_per_seed: 0
```

Runtime result:

```text
min_slot_accuracy_milli: 1000
min_flat_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
min_sequence_energy_p10_gap: 2976640
total_energy_pass_slot_fail: 0
total_flat_gap_parity_mismatches: 0
total_flat_sequence_energy_parity_mismatches: 0

max_role_binding_edges: 1354
max_hot_bytes_estimate: 606080
max_flat_eval_avg_ns_per_row: 187982
flat_eval_latency_gate_ns: 1000000

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Boundary:

```text
This closes 32-slot order corpus multi-seed robustness.

It does not close:
  full 32-slot operator battery;
  32-slot edit / conditional / composed gates;
  packed product runtime proof;
  product p99 latency proof.
```

## 2026-07-03 - Slot32 Mixed Map Corpus Rung

Verdict:

```text
SLOT32_MIXED_MAP_CORPUS_RUNG_PASS
```

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_mixed_map_corpus_must_transfer_without_lookup_or_runtime_phase_hack --nocapture
```

Artifacts:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_MIXED_MAP_CORPUS_RUNG.md
log:    data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_mixed_map_corpus_rung_release.log
```

Corpus:

```text
seed: 0
train_rows: 2048
heldout_rows: 2048
unique_operator_classes: 3
unique_rules: 16
unique_surfaces: 4
unique_noise_types: 2
unique_lengths: 16
lengths: 17..32
same_bag_rows: 1536
edit_rows: 512
edit_non_same_bag_rows: 512
max_train_state_reuse: 16
max_heldout_state_reuse: 16
train_tokens_overlap_heldout: 0
```

Runtime result:

```text
slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
sequence_energy_p10_gap: 3106560
energy_pass_slot_fail: 0
flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_mismatches: 0
flat_failed_rows: 0

ablation_without_binding/action/role/active: 0 / 0 / 0 / 0
state_delta_edges: 0
role_binding_edges: 1492
hot_bytes_estimate: 607736
flat_eval_avg_ns_per_row: 219009

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Boundary:

```text
This closes 32-slot mixed map transfer for order + edit-map + composed-map on
one seed.

It does not close:
  32-slot conditional branch selection;
  32-slot mixed-map multi-seed robustness;
  full 32-slot operator battery;
  insert-new-constant edit operators;
  packed product runtime proof;
  product p99 latency proof.
```

## 2026-07-03 - Slot32 Conditional Branch Corpus Rung

Verdict:

```text
SLOT32_CONDITIONAL_BRANCH_CORPUS_RUNG_PASS
```

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_conditional_branch_must_select_without_lookup_or_runtime_phase_hack --nocapture
```

Artifacts:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_CONDITIONAL_BRANCH_CORPUS_RUNG.md
log:    data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_conditional_branch_corpus_rung_release.log
```

Corpus:

```text
seed: 0
train_rows: 2048
heldout_rows: 2048
unique_operator_classes: 1
unique_rules: 8
unique_surfaces: 4
unique_noise_types: 2
unique_lengths: 16
lengths: 17..32
same_bag_rows: 2048
condition_true_rows: 1024
condition_false_rows: 1024
direct_operator_pair_active_centers: 0
condition_action_active_centers: 50176
state_condition_active_centers: 120832
max_train_state_reuse: 16
max_heldout_state_reuse: 16
train_tokens_overlap_heldout: 0
```

Runtime result:

```text
slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
sequence_energy_p10_gap: 3122560
energy_pass_slot_fail: 0
flat_gap_parity_mismatches: 0
flat_sequence_energy_parity_mismatches: 0
flat_failed_rows: 0

ablation_without_binding/action/condition-action/role/active: 0 / 0 / 0 / 0 / 0
state_delta_edges: 0
role_binding_edges: 2202
hot_bytes_estimate: 681792
flat_eval_avg_ns_per_row: 174654

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
direct_operator_pair_action_centers_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Boundary:

```text
This closes 32-slot conditional branch selection on symbolic branch-map action
inputs for one seed.

It does not close:
  full 32-slot operator battery multi-seed proof;
  raw-language action parsing;
  autonomous action_tree induction;
  packed product runtime proof;
  product p99 latency proof.
```

## 2026-07-02 - Manual Reviewer Update (Superseded)

Verdict:

```text
SUPERSEDED_BY_FRESH_CURRENT_SOURCE_RERUN
```

This note is historical. The rerun it requested is now complete; see the fresh
current-source rerun section at the top of this file.

Current evidence:

```text
seed_003/order was refreshed and is green:
  order_slot_ordered_sequence_accuracy_milli: 1000
  order_flat_slot_ordered_sequence_accuracy_milli: 1000
  order_sequence_energy_accuracy_milli: 1000
  order_energy_pass_slot_fail: 0
  order_output_slot_cleanup_failed_slots: 0
  flat_gap_parity_mismatches: 0
  flat_sequence_energy_parity_mismatches: 0
  ablations without binding/action/role/active_fringe: 0
  forbidden flags: false

But it is not enough for current-source multi-seed proof:
  wavepredictor_binding_pressure_l3.rs was updated at 2026-07-02 16:28.
  Existing order logs are older than that latest test source timestamp.

Freshness counts at this review:
  runtime logs newer than wavepredictor_hebbian.rs: 4/12
  runtime logs newer than wavepredictor_binding_pressure_l3.rs: 1/12
  runtime logs newer than phase_package_cmd.rs: 4/12

Live run observed:
  seed_001/edit release runtime gate is running.
  It reached eval_flat_gap_parity_start after train/cleanup showed 1000/1000.
  It is not a completed PASS until the test exits ok and final metrics print.
```

Boundary:

```text
Do not mark v4 strict multi-seed current-source proof green yet.

Required close condition:
  1. all 12 canonical runtime logs rerun after the latest relevant Rust
     source/test timestamps;
  2. each log exits ok and keeps parity/ablation/forbidden-flag checks clean;
  3. strict-multiseed-rust-audit-v1 rerun;
  4. strict-multiseed-rust-audit-verify-v1 rerun;
  5. report source matching stays true after the rerun.
```

Direction for executor:

```text
Continue the canonical current-source rerun.
Do not switch to new architecture or new operator claims while this rerun is open.
The useful result right now is boring but critical: fresh 12/12 logs or a real red.
```

## 2026-07-02 - Reviewer Note (Superseded)

Verdict:

```text
SUPERSEDED_BY_FRESH_CURRENT_SOURCE_RERUN
```

This note is historical. The stale-runtime-log warning below was valid at the
time, but the current 12-log rerun and audit/verify have now closed it.

Current verified chain:

```text
cargo fmt --check: PASS
cargo check -p nando-cli: PASS
cargo clippy -p nando-cli -p nando-core -- -D warnings: PASS
phase-action-release-suite-v1: PASS
phase-action-release-verify-v1: PASS, report_matches_sources = true
phase-action-license-verify-v1: PASS, report_matches_sources = true
phase-action-offload-verify-v1: PASS, report_matches_sources = true
phase-action-cache-offload-bench-verify-v1: PASS, report_matches_sources = true
phase-action-workflow-bench-v1: PASS
phase-action-workflow-bench-verify-v1: PASS, report_matches_sources = true
phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS, report_matches_sources = true
phase-action-regression-freeze-v1: PASS
phase-action-regression-freeze-verify-v1: PASS, report_matches_sources = true
strict-multiseed-rust-audit-v1: PASS as log audit
strict-multiseed-rust-audit-verify-v1: PASS, report_matches_sources = true
git diff --check: PASS
RUSTFLAGS=-Dwarnings cargo test -p nando-core phase_center_runtime --lib: PASS
```

Current WATCH:

```text
Canonical strict audit changed from RED to PASS after the seed=2/order runtime
log was regenerated:

  strict_multiseed_verdict: STRICT_MULTI_SEED_RUST_AUDIT_PASS
  strict_multiseed_observed_logs: 12
  strict_multiseed_missing_logs: 0
  strict_multiseed_logs_fingerprint64: 10852598576795674512
  strict_multiseed_logs_total_bytes: 73816
  strict_runtime_issues: 0
  strict_multiseed_python_demo_used: false
  strict_multiseed_corpus_jsonl_used: false
  strict_multiseed_evidence_warnings: 0

New seed=2/order runtime log:

  order_runtime_gate_release.log updated at 2026-07-02 16:04
  order_slot_ordered_sequence_accuracy_milli: 1000
  order_sequence_energy_accuracy_milli: 1000
  order_energy_pass_slot_fail: 0
  order_output_slot_cleanup_failed_slots: 0
  slot_failure_total: 0
  flat_gap_parity_mismatches: 0
  flat_sequence_energy_parity_mismatches: 0
  ablations without binding/action/role/active_fringe: 0
  forbidden flags: false

But this is still a WATCH, not a current-source multi-seed proof:

  report_matches_sources=true means the audit report matches the runtime log
  files. It does not prove all runtime logs were regenerated from the current
  Rust source.

  Current source/test files are newer than most canonical runtime logs:
    wavepredictor_hebbian.rs: 2026-07-02 15:53
    wavepredictor_binding_pressure_l3.rs: 2026-07-02 15:52
    phase_package_cmd.rs: 2026-07-02 16:04

  Examples of stale canonical logs still used by the PASS audit:
    seed_001/order:       2026-07-01 21:22
    seed_001/conditional: 2026-07-01 21:31
    seed_001/composed:    2026-07-01 21:32
    seed_002/conditional: 2026-07-01 21:53
    seed_002/composed:    2026-07-01 21:54
    seed_003/order:       2026-07-01 22:06
    seed_003/conditional: 2026-07-01 22:15
    seed_003/composed:    2026-07-01 22:16
    seed_001/edit:        2026-07-02 15:33
    seed_002/edit:        2026-07-02 15:39
    seed_003/edit:        2026-07-02 15:45

Diagnostic density sweep:
  factor1 baseline: RED
  factor2 targeted reweighting: GREEN
  factor4 targeted reweighting: GREEN
  factor16 targeted reweighting: GREEN

Boundary:
  targeted reweighting is diagnostic only, not a final corpus policy.

Boundary:

  Superseded. The current-source multi-seed strict robustness debt is now
  closed by the fresh 12-log rerun recorded at the top of this file.
```

Freshness note:

```text
Reviewer caught a stale regression/freeze state after OPERATOR_BLUEPRINT.md was
updated. The regression and freeze reports were regenerated and re-verified.

current release_suite_report_fingerprint64: 9827723825761118426
current regression_report_fingerprint64: 2002304595771295125
current workflow_bench_report_fingerprint64: 7479237649753576261
current operator_blueprint_fingerprint64: 9874423192353457577
regression_verify report_matches_sources: true
freeze_verify report_matches_sources: true
```

Release-suite status:

```text
artifact_count: 3
artifacts:
  generated_action
  domain_action
  coverage_action

total_runtime_bytes_estimate: 48576
total_bench_samples: 308000
max_bench_p99_latency_ns: 117
total_source_rebuild_action_tree_key_count: 46
all_package_report_parity_pass: true
all_shortcut_reports_pass: true
all_action_ablation_collapses: true
compiler_used: false
corpus_jsonl_used: false
forbidden_used: false
```

Operator coverage integration:

```text
all_operator_coverage_reports_match_sources: true
operator_dimension_coverage_artifact_count: 1
release_operator_dimension_coverage_pass: true
max_operator_coverage_min_dimension_value_count: 5
max_operator_coverage_wide_dimension_count: 5

generated_action:
  operator_coverage_report_verdict: PHASE_ACTION_OPERATOR_COVERAGE_V1_WATCH
  full_operator_dimension_coverage_pass: false
  min_dimension_value_count: 1

domain_action:
  operator_coverage_report_verdict: PHASE_ACTION_OPERATOR_COVERAGE_V1_WATCH
  full_operator_dimension_coverage_pass: false
  min_dimension_value_count: 1

coverage_action:
  operator_coverage_report_verdict: PHASE_ACTION_OPERATOR_COVERAGE_V1_PASS
  full_operator_dimension_coverage_pass: true
  min_dimension_value_count: 5
  wide_dimension_count: 5
```

Offload/cache product benchmark:

```text
offload_rate_milli: 880
local_accuracy_milli: 1000
false_local_accepts: 0

exact_cache_llm_calls: 308
exact_cache_plus_nando_llm_calls: 36
incremental_llm_calls_removed_vs_cache: 272
incremental_llm_call_reduction_vs_cache_milli: 883
```

Claim boundary:

```text
This closes the V5 coverage_action integration into the packaged flat action
scorer release/regression/freeze chain.

It does not close:
  strict ordered decoder beyond the known 16-slot rung;
  32-slot full corpus proof;
  autonomous raw action parser;
  text generation;
  broad workflow reasoning;
  commercial license package.
```

Direction for executor (superseded):

```text
This historical direction has been completed by the fresh 12-log rerun.

Closed in this note:
  1. source-matching strict multi-seed audit exists and verifies.
  2. strict audit helper wiring now passes cargo check and clippy -D warnings.
  3. edit-class cleanup counters are present in seed 1/2/3 logs.

Next honest targets:
  1. keep the current v4 16-slot strict audit frozen;
  2. public Rust SDK surface is closed by current offload audit + SDK test;
  3. loopback HTTP service smoke is closed by phase-action-daemon-smoke-v1;
  4. single-package HTTP service smoke is closed by phase-action-daemon-package-smoke-v1;
  5. first HTTP hardening smoke is closed by phase-action-daemon-hardening-smoke-v1;
  6. bearer-auth smoke is closed by phase-action-daemon-auth-smoke-v1;
  7. static multi-package registry smoke is closed by phase-action-daemon-registry-smoke-v1;
  8. registry config-file smoke is closed by phase-action-daemon-registry-config-smoke-v1;
  9. registry config validation smoke is closed by phase-action-daemon-config-validation-smoke-v1;
  10. score rate-limit smoke is closed by phase-action-daemon-rate-limit-smoke-v1;
  11. structured observability smoke is closed by phase-action-daemon-observability-smoke-v1;
  12. structured audit-log smoke is closed by phase-action-daemon-audit-log-smoke-v1;
  13. HTTP error-taxonomy smoke is closed by phase-action-daemon-error-taxonomy-smoke-v1;
  14. daemon proof suite is closed by phase-action-daemon-proof-suite-v1;
  15. live daemon proof suite is closed by phase-action-daemon-live-proof-suite-v1;
  16. systemd service packaging smoke is closed by phase-action-daemon-systemd-smoke-v1;
  17. production HTTP daemon hardening is still open beyond these smokes;
  18. real external pilot workflow beyond synthetic domain_action;
  19. full 32-slot corpus proof and cache/runtime benchmark.

HTTP service smoke evidence:

```text
command: cargo run -p nando-cli --release -- phase-action-daemon-smoke-v1
report: target/nando-wave/action-runtime-v1-daemon-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_SMOKE_V1_PASS
http_requests_handled: 2
http_bad_requests: 0
local_action: local_operator
fallback_action: fallback_to_llm
false_local_accepts: 0
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this is a loopback service-boundary smoke over PhaseCenterOffloadRuntime
  package bytes, not a production daemon, auth/TLS layer, service manager,
  multi-package registry, or real pilot workflow.

Existing package HTTP service smoke evidence:

```text
serve command: phase-action-daemon-serve-v1
proof command: cargo run -p nando-cli --release -- phase-action-daemon-package-smoke-v1
report: target/nando-wave/action-runtime-v1-daemon-package-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_PACKAGE_SMOKE_V1_PASS
package_path: target/nando-wave/action-runtime-v1-generated-coverage-c32.nwpc
package_fingerprint64: 11103824464258352074
package_record_count: 30
fixture_task_id: generated_coverage_contract_v1_heldout_len5_select_span_reverse_replace_always_bag_0
fixture_center_index: 9
http_requests_handled: 2
http_bad_requests: 0
local_action: local_operator
fallback_action: fallback_to_llm
local_margin_micro: 791009
fallback_margin_micro: -791009
false_local_accepts: 0
request_fixture_corpus_jsonl_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes a single-package HTTP surface over an existing .nwpc package.
  The proof command reads corpus JSONL only to construct one request fixture;
  the server runtime path reads package bytes only. It is not production
  hardening: no auth/TLS, service manager, multi-package registry, rate limits,
  observability, or real pilot traffic yet.

HTTP hardening smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-hardening-smoke-v1
report: target/nando-wave/action-runtime-v1-daemon-hardening-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_HARDENING_SMOKE_V1_PASS
package_path: target/nando-wave/action-runtime-v1-generated-coverage-c32.nwpc
package_fingerprint64: 11103824464258352074
package_record_count: 30
health_status_code: 200
stats_status_code: 200
bad_route_status_code: 404
http_max_request_bytes: 65536
max_score_atoms: 1024
max_score_atom_bytes: 256
http_requests_handled: 4
http_score_requests: 2
http_health_requests: 1
http_stats_requests: 1
http_bad_requests: 1
local_operator_calls: 1
fallback_to_llm_calls: 1
false_local_accepts: 0
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes only the first HTTP hardening smoke: health endpoint, stats
  endpoint, request-size/atom limits, route errors, and local/fallback counters.
  It is still not bearer auth, TLS, service-manager integration,
  multi-package registry, rate limits, structured observability, or real pilot
  traffic.

HTTP bearer-auth smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-auth-smoke-v1
report: target/nando-wave/action-runtime-v1-daemon-auth-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_AUTH_SMOKE_V1_PASS
package_path: target/nando-wave/action-runtime-v1-generated-coverage-c32.nwpc
package_fingerprint64: 11103824464258352074
package_record_count: 30
auth_enabled: true
health_public_status_code: 200
unauthorized_score_status_code: 401
authorized_score_status_code: 200
authorized_fallback_status_code: 200
authorized_stats_status_code: 200
http_requests_handled: 4
http_score_requests: 2
http_health_requests: 1
http_stats_requests: 1
http_bad_requests: 1
local_operator_calls: 1
fallback_to_llm_calls: 1
false_local_accepts: 0
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes only bearer auth for /score and /stats over an existing .nwpc
  package. /health remains public for liveness. It is still not TLS,
  service-manager integration, multi-package registry, rate limits, structured
  observability, or real pilot traffic.

HTTP multi-package registry smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-registry-smoke-v1
report: target/nando-wave/action-runtime-v1-daemon-registry-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_REGISTRY_SMOKE_V1_PASS
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
generated_package_fingerprint64: 14869999570221545448
domain_package_fingerprint64: 5367415087033800111
coverage_package_fingerprint64: 11103824464258352074
generated_status_code: 200
domain_status_code: 200
coverage_status_code: 200
missing_alias_status_code: 404
packages_status_code: 200
stats_status_code: 200
health_status_code: 200
generated_action: local_operator
domain_action: local_operator
coverage_action: local_operator
generated_margin_micro: 675249
domain_margin_micro: 1526347
coverage_margin_micro: 791009
http_score_requests: 3
http_packages_requests: 1
http_bad_requests: 1
local_operator_calls: 3
false_local_accepts: 0
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes only static registry routing over already built .nwpc packages.
  It is not dynamic package reload, registry config, rate limits, TLS,
  service-manager integration, structured observability, or real pilot traffic.

HTTP registry config smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-registry-config-smoke-v1
config: target/nando-wave/action-runtime-v1-daemon-registry.config.json
report: target/nando-wave/action-runtime-v1-daemon-registry-config-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_REGISTRY_CONFIG_SMOKE_V1_PASS
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
generated_status_code: 200
domain_status_code: 200
coverage_status_code: 200
missing_alias_status_code: 404
packages_status_code: 200
stats_status_code: 200
health_status_code: 200
generated_action: local_operator
domain_action: local_operator
coverage_action: local_operator
generated_margin_micro: 675249
domain_margin_micro: 1526347
coverage_margin_micro: 791009
http_score_requests: 3
http_packages_requests: 1
http_bad_requests: 1
local_operator_calls: 3
fallback_to_llm_calls: 0
false_local_accepts: 0
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes config-file loading for a multi-package HTTP registry over
  already built .nwpc packages with manifest parity validation. It is not
  dynamic package reload, rate limits, TLS, service-manager integration,
  structured observability, or real pilot traffic.

HTTP registry config validation smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-config-validation-smoke-v1
report: target/nando-wave/action-runtime-v1-daemon-config-validation-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_CONFIG_VALIDATION_SMOKE_V1_PASS
valid_registry_load_pass: true
valid_package_count: 3
invalid_case_count: 5
invalid_reject_count: 5
invalid_error_messages_pass: true
invalid cases:
  invalid_schema
  empty_alias
  duplicate_alias
  missing_manifest
  manifest_mismatch
server_started_for_invalid_configs: false
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes only startup-time registry config validation for valid load and
  five invalid reject-before-serve cases. It is not dynamic reload, TLS,
  service-manager integration, or real pilot traffic.

HTTP score rate-limit smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-rate-limit-smoke-v1
config: target/nando-wave/action-runtime-v1-daemon-registry.config.json
report: target/nando-wave/action-runtime-v1-daemon-rate-limit-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_RATE_LIMIT_SMOKE_V1_PASS
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
max_score_requests: 1
health_status_code: 200
packages_status_code: 200
allowed_score_status_code: 200
rate_limited_score_status_code: 429
stats_status_code: 200
allowed_action: local_operator
allowed_margin_micro: 791009
http_requests: 5
http_requests_handled: 4
http_score_requests: 1
http_health_requests: 1
http_packages_requests: 1
http_stats_requests: 1
http_bad_requests: 1
http_rate_limited_requests: 1
local_operator_calls: 1
fallback_to_llm_calls: 0
false_local_accepts: 0
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes only a deterministic /score max_score_requests guard over a
  registry-config loaded .nwpc service. It proves over-limit requests return
  429 and do not invoke the scorer. It is not time-window rate limiting,
  TLS, dynamic reload, service-manager integration, structured observability,
  or real pilot traffic.

HTTP structured observability smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-observability-smoke-v1
config: target/nando-wave/action-runtime-v1-daemon-registry.config.json
report: target/nando-wave/action-runtime-v1-daemon-observability-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_OBSERVABILITY_SMOKE_V1_PASS
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
max_score_requests: 1
health_status_code: 200
packages_status_code: 200
missing_alias_status_code: 404
allowed_score_status_code: 200
rate_limited_score_status_code: 429
stats_status_code: 200
requests_handled_observed_by_stats: 3
score_requests_observed_by_stats: 1
bad_requests_observed_by_stats: 2
rate_limited_requests_observed_by_stats: 1
local_operator_calls_observed_by_stats: 1
fallback_to_llm_calls_observed_by_stats: 0
false_local_accepts_observed_by_stats: 0
requests_handled_final: 4
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes only structured /stats observability for package aliases,
  request counters, rate-limit counters, and runtime provenance flags. It is
  not distributed tracing, persistent logs, TLS, dynamic reload,
  service-manager integration, or real pilot traffic.

HTTP structured audit-log smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-audit-log-smoke-v1
event log: target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.events.jsonl
report: target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_AUDIT_LOG_SMOKE_V1_PASS
audit_event_count: 6
audit_status_codes: 200, 200, 404, 200, 429, 200
audit_request_kinds: health, packages, error, score, error, stats
audit_sequences_are_dense: true
audit_missing_alias_event_found: true
audit_rate_limit_event_found: true
audit_local_operator_event_found: true
audit_flags_pass: true
local_operator_calls: 1
fallback_to_llm_calls: 0
false_local_accepts: 0
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes only server-side structured JSONL audit events for handled and
  rejected requests. It is not distributed tracing, log rotation, TLS, dynamic
  reload, service-manager integration, or real pilot traffic.

HTTP error-taxonomy smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-error-taxonomy-smoke-v1
report: target/nando-wave/action-runtime-v1-daemon-error-taxonomy-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_ERROR_TAXONOMY_SMOKE_V1_PASS
error_status_codes: 400, 404, 413, 413, 400, 405, 413
malformed_json_status_code: 400
missing_alias_status_code: 404
too_many_atoms_status_code: 413
too_long_atom_status_code: 413
out_of_bounds_status_code: 400
unsupported_method_status_code: 405
oversized_request_status_code: 413
error_messages_pass: true
score_requests: 0
bad_requests: 7
local_operator_calls: 0
fallback_to_llm_calls: 0
false_local_accepts: 0
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this closes only explicit HTTP rejection taxonomy and proves these rejects do
  not invoke the scorer. It is not fuzzing, TLS, dynamic reload,
  service-manager integration, or real pilot traffic.

HTTP daemon proof suite evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-proof-suite-v1
report: target/nando-wave/action-runtime-v1-daemon-proof-suite.product-proof.json
verdict: PHASE_ACTION_DAEMON_PROOF_SUITE_V1_PASS
artifact_count: 12
pass_count: 12
all_reports_pass: true
all_forbidden_flags_false: true
all_python_demo_false: true
all_server_runtime_hot_path_clean: true
all_false_local_accepts_zero: true
artifacts:
  daemon_smoke
  daemon_package_smoke
  daemon_hardening_smoke
  daemon_auth_smoke
  daemon_registry_smoke
  daemon_registry_config_smoke
  daemon_config_validation_smoke
  daemon_rate_limit_smoke
  daemon_observability_smoke
  daemon_audit_log_smoke
  daemon_error_taxonomy_smoke
  daemon_systemd_smoke
```

Boundary:
  this closes only a saved-report daemon proof bundle over existing product-proof
  JSON artifacts. It is not a live rerun, TLS, service-manager integration,
  dynamic reload, or real pilot traffic.

HTTP daemon live proof suite evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-live-proof-suite-v1
report: target/nando-wave/action-runtime-v1-daemon-live-proof-suite.product-proof.json
verdict: PHASE_ACTION_DAEMON_LIVE_PROOF_SUITE_V1_PASS
live_rerun_performed: true
live_rerun_step_count: 12
artifact_count: 12
pass_count: 12
all_reports_pass: true
all_forbidden_flags_false: true
all_python_demo_false: true
all_server_runtime_hot_path_clean: true
all_false_local_accepts_zero: true
```

Boundary:
  this freshly reruns the 12 local HTTP daemon and service-packaging smoke
  gates, then verifies the updated product-proof JSON artifacts as one bundle.
  It is not TLS, installed service, dynamic reload, or real pilot traffic.

HTTP daemon systemd packaging smoke evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-systemd-smoke-v1
service: target/nando-wave/nando-wave-action-daemon.service
env: target/nando-wave/nando-wave-action-daemon.env
report: target/nando-wave/action-runtime-v1-daemon-systemd-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_SYSTEMD_SMOKE_V1_PASS
package_count: 3
service_manager_artifacts_written: true
service_exec_serve_registry: true
service_environment_file_matches: true
service_restart_on_failure: true
service_hardening_pass: true
env_registry_config_matches: true
auth_token_placeholder_used: true
installed_to_systemd: false
systemctl_invoked: false
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
forbidden flags: false
```

Boundary:
  this writes and validates local systemd unit/env/registry artifacts under
  target for `phase-action-daemon-serve-registry-v1`. It does not install or
  start a service, configure TLS, dynamic reload, or real pilot traffic.

HTTP daemon deployment package evidence:

```text
proof command: cargo run -p nando-cli --release -- phase-action-daemon-deployment-package-v1
report: target/nando-wave/action-runtime-v1-daemon-deployment-package.product-proof.json
verdict: PHASE_ACTION_DAEMON_DEPLOYMENT_PACKAGE_V1_PASS
live_suite_artifact_count: 12
live_suite_step_count: 12
live_suite_contains_systemd: true
live_suite_hot_path_clean: true
live_suite_forbidden_flags_false: true
live_suite_python_demo_false: true
live_suite_false_local_accepts_zero: true
systemd_smoke_pass: true
systemd_artifacts_written: true
systemd_hardening_pass: true
systemd_auth_placeholder_used: true
systemd_not_installed: true
systemctl_not_invoked: true
systemd_hot_path_clean: true
systemd_forbidden_flags_false: true
service_unit_exec_matches: true
service_unit_env_matches: true
env_file_config_matches: true
registry_config_package_count: 3
registry_config_package_count_matches: true
deployment_artifacts_present: true
installed_to_systemd: false
```

Boundary:
  this verifies the daemon live proof suite, systemd smoke report, generated
  service unit, env file, and registry config as one local deployment package.
  It does not install/start systemd, configure TLS, dynamic reload, or prove
  real pilot traffic.

HTTP daemon deployment verify evidence:

```text
verify command: cargo run -p nando-cli --release -- phase-action-daemon-deployment-verify-v1
report: target/nando-wave/action-runtime-v1-daemon-deployment-package.product-proof.json
verdict: PHASE_ACTION_DAEMON_DEPLOYMENT_VERIFY_V1_PASS
report_gate_pass: true
rebuilt_gate_pass: true
report_matches_sources: true
live_suite_report_path: target/nando-wave/action-runtime-v1-daemon-live-proof-suite.product-proof.json
systemd_report_path: target/nando-wave/action-runtime-v1-daemon-systemd-smoke.product-proof.json
live_suite_artifact_count: 12
live_suite_step_count: 12
service_unit_exec_matches: true
registry_config_package_count: 3
deployment_artifacts_present: true
```

Boundary:
  this is a stale-proof check for the saved daemon deployment package report.
  It rebuilds expected deployment facts from the current live-suite and systemd
  proof sources without rerunning the daemon smoke gates.

Tamper check:

```text
tamper: live_suite_step_count 12 -> 11
verify command: cargo run -p nando-cli --release -- phase-action-daemon-deployment-verify-v1 \
  target/nando-wave/action-runtime-v1-daemon-live-proof-suite.product-proof.json \
  target/nando-wave/action-runtime-v1-daemon-systemd-smoke.product-proof.json \
  target/nando-wave/action-runtime-v1-daemon-deployment-package.tampered.product-proof.json
verdict: PHASE_ACTION_DAEMON_DEPLOYMENT_VERIFY_V1_WATCH
exit_code: 1
report_gate_pass: true
rebuilt_gate_pass: true
report_matches_sources: false
```

Preserve this boundary:
  coverage_action proves full operator-dimension coverage at release level,
  while generated_action/domain_action remain bounded WATCH coverage reports
  that still match their sources.
```

## 2026-07-02 - Workflow Replay Product Gate

Verdict:

```text
PHASE_ACTION_WORKFLOW_REPLAY_V1_PASS
PHASE_ACTION_WORKFLOW_REPLAY_VERIFY_V1_PASS
```

Command:

```text
cargo run -p nando-cli --release -- phase-action-workflow-replay-v1
cargo run -p nando-cli --release -- phase-action-workflow-replay-verify-v1
```

Artifact:

```text
target/nando-wave/action-runtime-v1-workflow-replay.product-proof.json
```

Current evidence:

```text
workflow_trace_calls: 3072
workflow_sessions: 256
steps_per_session: 12
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
all_packages_observed: true
sessions_cover_all_packages: true
total_unique_eval_rows: 308
replay_unique_rows: 308
exact_cache_llm_calls: 308
exact_cache_hits: 2764
exact_cache_plus_nando_llm_calls: 36
nando_local_operator_calls: 2780
nando_fallback_events: 292
incremental_llm_calls_removed_vs_cache: 272
incremental_llm_call_reduction_vs_cache_milli: 883
local_accuracy_milli: 1000
false_local_accepts: 0
max_bench_p99_latency_ns: 117
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Per-package replay:

```text
generated_action:
  trace_calls: 1024
  unique_replayed_rows: 80
  local_operator_calls: 868
  fallback_to_llm_calls: 156

domain_action:
  trace_calls: 1024
  unique_replayed_rows: 48
  local_operator_calls: 1024
  fallback_to_llm_calls: 0

coverage_action:
  trace_calls: 1024
  unique_replayed_rows: 180
  local_operator_calls: 888
  fallback_to_llm_calls: 136
```

Tamper check:

```text
tamper: replay_unique_rows 308 -> 307
verdict: PHASE_ACTION_WORKFLOW_REPLAY_VERIFY_V1_WATCH
exit_code: 1
report_gate_pass: false
report_matches_sources: false
log: target/nando-wave/action-runtime-v1-workflow-replay-verify-tamper.log
```

Boundary:

```text
This is a deterministic multi-package workflow replay over frozen `.nwpc`
packages and binary eval-packs. It improves on the old 48-row domain_action
workflow smoke, but it is still not raw action parsing, text generation,
dynamic real pilot traffic, or commercial license closure.
```

## 16-Slot / 32-Page Boundary

```text
Текущий зелёный strict ordered decoder:
  16-slot rung
  lengths 13..16
  strict ordered slot readout = 1000/1000
  sequence energy = 1000/1000
  flat parity mismatches = 0
  ablations collapse to 0

PAGE_COUNT = 32 означает 32 memory pages по 4096 centers.
Это не 32 output slots.

32-slot rung is now partially closed beyond smoke by the mixed+conditional
multi-seed combined gate, but the full product package proof is still open.
```

## 2026-07-03 - 32-Slot Mixed/Conditional Multi-Seed Rung

Verdict:

```text
SLOT32_MIXED_CONDITIONAL_MULTI_SEED_RUNG_PASS
```

Evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_MIXED_CONDITIONAL_MULTI_SEED_RUNG.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_mixed_conditional_multiseed_rung_release.log
runtime: 2294.10s
```

Core metrics:

```text
seeds: 3
page_count: 64
total_center_count: 262144
output_slot_count: 32
role_slot_count: 32
lengths: 17..32

mixed_min_slot_accuracy_milli: 1000
mixed_min_flat_slot_accuracy_milli: 1000
mixed_min_sequence_energy_accuracy_milli: 1000
mixed_total_flat_gap_parity_mismatches: 0
mixed_total_flat_sequence_energy_parity_mismatches: 0

conditional_min_slot_accuracy_milli: 1000
conditional_min_flat_slot_accuracy_milli: 1000
conditional_min_sequence_energy_accuracy_milli: 1000
conditional_total_flat_gap_parity_mismatches: 0
conditional_total_flat_sequence_energy_parity_mismatches: 0
conditional_total_direct_operator_pair_active_centers: 0
conditional_max_ablation_without_condition_action_accuracy_milli: 0
conditional_max_ablation_without_condition_action_energy_accuracy_milli: 0

max_role_binding_edges: 2202
max_hot_bytes_estimate: 681792
max_flat_eval_avg_ns_per_row: 172809
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Boundary:

```text
This closes 32-slot mixed-map plus conditional-branch multi-seed robustness
over Rust-generated symbolic operator tasks. It does not close raw-language
action parsing, autonomous action_tree induction, insert-new-constant edit
operators, packed product runtime proof, product p99, 64-slot capacity, broad
workflow reasoning, or text generation.
```

## 2026-07-03 - 32-Slot Mixed/Conditional Cache-Offload Bench

Verdict:

```text
SLOT32_MIXED_CONDITIONAL_CACHE_OFFLOAD_BENCH_PASS
```

Evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_MIXED_CONDITIONAL_CACHE_OFFLOAD_BENCH.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_mixed_conditional_cache_offload_bench_release.log
runtime: 281.40s
```

Core metrics:

```text
seeds: 3
simulated_repeats: 3
total_unique_rows: 12288
total_simulated_calls: 36864
total_exact_cache_llm_calls: 12288
total_exact_cache_plus_nando_llm_calls: 0
total_local_operator_calls: 36864
total_fallback_to_llm_calls: 0
total_false_local_accepts: 0
total_incremental_llm_calls_removed_vs_cache: 12288
total_incremental_llm_call_reduction_vs_cache_milli: 1000
min_local_accuracy_milli: 1000
min_offload_rate_milli: 1000
max_p99_latency_ns: 611686
max_hot_bytes_estimate: 681792
```

Boundary:

```text
This closes the 32-slot flat role-binding cache/offload benchmark over current
symbolic mixed/conditional task families. It does not close serialized .nwpc
packaging for the 32-slot role-binding runtime, raw-language action parsing,
autonomous action_tree induction, insert-new-constant edit operators, product
p99, 64-slot capacity, broad workflow reasoning, or text generation.
```

## 2026-07-03 - 32-Slot Role-Binding Package Rung

Verdict:

```text
SLOT32_ROLE_BINDING_PACKAGE_RUNG_PASS
```

Evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PACKAGE_RUNG.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_role_binding_package_rung_release.log
runtime: 735.91s
```

Structural claim-boundary check:

```text
nanda-gate-md /tmp/nanda-task-slot32-role-binding-package-boundary.md --task-id slot32-role-binding-package-boundary --domain code
verdict: PASS
complexity_score: 19
agent_action: SAFE_TO_EDIT
trace_path: /tmp/nanda-structural-gate/slot32-role-binding-package-boundary.trace.json
```

Core metrics:

```text
package_magic: NWRB0001
package_count: 6
seeds: 3
labels: conditional_branch, mixed_map
min_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
total_flat_gap_parity_mismatches: 0
total_flat_sequence_energy_parity_mismatches: 0
total_false_local_accepts: 0
rewrite_exact_all: true
nonzero_fingerprints: true
max_package_bytes: 26468
max_hot_bytes_estimate: 681792
max_edges: 2202
max_p99_latency_ns: 623242
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Boundary:

```text
This closes the serialized 32-slot role-binding `.nwrb` package proof for the
current mixed-map plus conditional-branch Rust runtime path. It does not close
the phase-center `.nwpc` package path, raw-language action parsing, autonomous
action_tree induction, insert-new-constant edit operators, 64-slot capacity,
broad workflow reasoning, text generation, or packaged daemon/API product p99.
```

## 2026-07-03 - 32-Slot Role-Binding Public SDK Smoke

Verdict:

```text
SLOT32_ROLE_BINDING_PUBLIC_SDK_SMOKE_PASS
```

Evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PUBLIC_SDK_SMOKE.md
test: cargo test -p nando-core --test wavepredictor_role_binding_sdk_public -- --nocapture
clippy: cargo clippy -p nando-core --test wavepredictor_role_binding_sdk_public -- -D warnings
```

Structural claim-boundary check:

```text
nanda-gate-md /tmp/nanda-task-slot32-role-binding-sdk-boundary.md --task-id slot32-role-binding-sdk-boundary --domain code
verdict: PASS
complexity_score: 29
agent_action: SAFE_TO_EDIT
trace_path: /tmp/nanda-structural-gate/slot32-role-binding-sdk-boundary.trace.json
```

Public API:

```text
nando_core::WavePredictorRoleBindingOffloadRuntime
nando_core::WavePredictorRoleBindingOffloadPolicy
nando_core::WavePredictorRoleBindingEvalTask
nando_core::WavePredictorRoleBindingDecision
nando_core::WavePredictorRoleBindingOffloadSummary
```

Boundary:

```text
This closes a public Rust SDK smoke for loading and scoring serialized
role-binding `.nwrb` packages. It does not close phase-center `.nwpc`,
CLI/daemon packaging, raw-language action parsing, broad workflow reasoning,
or text generation.
```

## 2026-07-03 - 32-Slot Public SDK-Loaded Role-Binding Package Rung

Verdict:

```text
SLOT32_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG_PASS
```

Evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_role_binding_public_sdk_package_rung_release.log
command: cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_role_binding_public_sdk_must_score_loaded_package_runtime --nocapture
runtime: 813.60s
```

Structural claim-boundary checks:

```text
runtime route:
  verdict: PASS
  complexity_score: 23
  trace_path: /tmp/nanda-structural-gate/slot32-role-binding-sdk-package-runtime.trace.json

boundary route:
  verdict: PASS
  complexity_score: 16
  trace_path: /tmp/nanda-structural-gate/slot32-role-binding-sdk-package-boundary-local.trace.json

note:
  the first aggregate packet returned VETO because runtime proof and boundary
  exclusions were mixed into one relation shape; route-local split resolved it.
```

Core metrics:

```text
package_magic: NWRB0001
seeds: 3
labels: sdk_conditional_branch, sdk_mixed_map
min_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
total_sdk_gap_parity_mismatches: 0
total_sdk_sequence_energy_parity_mismatches: 0
total_false_local_accepts: 0
rewrite_exact_all: true
nonzero_fingerprints: true
max_package_bytes: 26468
max_hot_bytes_estimate: 681792
max_edges: 2202
max_p99_latency_ns: 718891
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Red signal and fix:

```text
The naive SDK scoring path was correctness-green but performance-red at about
4.4-4.5 ms p99, so it was stopped before promotion. The promoted SDK runtime
uses a package-derived edge index and prepared active-fringe scoring, bringing
the release gate max p99 to 718891 ns.
```

Boundary:

```text
This closes public SDK-loaded scoring of real 32-slot `.nwrb` role-binding
packages. It does not close phase-center `.nwpc`, CLI/daemon registry,
raw-language action parsing, autonomous action_tree induction, broad workflow
reasoning, text generation, or the full operator catalog.
```

## 2026-07-03 - 32-Slot Role-Binding CLI Inspect/Verify Rung

Verdict:

```text
ROLE_BINDING_PACKAGE_INSPECT_V1_PASS
ROLE_BINDING_PACKAGE_VERIFY_V1_PASS
```

Evidence:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_CLI_INSPECT_RUNG.md
product_report: target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json

cargo run -p nando-cli --release -- role-binding-package-inspect-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json
cargo run -p nando-cli --release -- role-binding-package-verify-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json
```

Core metrics:

```text
package_magic: NWRB0001
package_bytes: 26468
edge_count: 2202
package_fingerprint64: 365065097387925697
sdk_load_matches_inspect: true
report_matches_package: true
forbidden_flags: false
rust_runtime_used: true
python_demo_used: false
corpus_jsonl_used: false
```

Boundary:

```text
This closes CLI inspect/verify for `.nwrb` role-binding package artifacts.
It does not close `.nwrb` CLI scoring, `.nwrb` daemon/registry routing,
phase-center `.nwpc`, raw-language action parsing, broad workflow reasoning,
or text generation.
```
