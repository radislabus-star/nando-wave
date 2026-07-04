# CPU Call Catalog

Status: review-only working catalog for CPU80.

Source report:

```text
target/nando-wave/real-traffic-shadow/cpu-call-catalog-business-value-v1.report.json
```

The catalog is a product filter before building another operator profile. It
answers one question:

```text
Which real call class can add unique verified CPU accepts over exact cache?
```

It is not a market claim. It does not enable local accepts. It does not count
candidate, scoreable, or broad-route rows as savings.

## Business Value Gate

A profile row passes the gate only when all conditions are true:

```text
call_class appears in non-synthetic real trace
non-exact candidate calls exist
deterministic verifier hook is ready
expected unique CPU accepts over exact cache > 0
false_accepts = 0
```

Anything else goes to a shelf:

```text
PROVEN          already has unique verified CPU accepts
CANDIDATE       payload/verifier evidence exists, but expected unique accepts are not proven
WATCH           low support, singleton-only, no verifier, or exhausted support
REJECT_FOR_NOW  broad or risky route; split before more work
```

## Current Snapshot

Window:

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

| rank | call class | candidates | non-exact | expected unique | status note |
| ---: | --- | ---: | ---: | ---: | --- |
| 1 | `role_binding_mixed_map_seed0` | 99 | 96 | 7 | Support is already covered; improve payload/evidence before another promote. |
| 2 | `role_binding_agent_control_seed0` | 111 | 74 | 5 | Duplicates/exact-cache overlap constrain unique value; split broader tool-state subfamilies. |
| 3 | `git_control` | 18 | 18 | 4 | Current safe support is exhausted; improve command outcome evidence or split. |
| 4 | `role_binding_conditional_branch_seed0` | 88 | 87 | 3 | Verifier-ready, but policy support is exhausted; split stronger conditional subfamily. |
| 5 | `metrics_report_readout` | 55 | 55 | 3 | Current robust metrics support is exhausted; split stronger report subfamily. |
| 6 | `serving_ops` | 25 | 25 | 3 | Current serving support is exhausted; split stronger daemon/health subfamily. |
| 7 | `role_binding_edit_marker_length_seed0` | 92 | 92 | 1 | Low support; improve edit evidence before another promote. |

CANDIDATE rows:

```text
uncatalogued / resource_pressure_budget
read_inspect
style_brevity
resource_pressure_budget
```

These do not count as savings. They are work candidates only if expected
unique accepts can be raised by verifier evidence or a narrower split.

REJECT_FOR_NOW rows:

```text
answer_or_explain
project_context_dialogue
agent_continue_execute
```

These are intentionally blocked as broad routes. Work only narrow
artifact-backed subfamilies, never the route as a whole.

## Next Engineering Rule

Do not build the next profile because it is interesting.

Build it only if the catalog row shows one of these:

```text
expected_unique_cpu_accepts_over_exact_cache > 0
or
clear deterministic verifier evidence that can raise expected unique accepts
```

The current highest-leverage pattern is not another generic profile. It is:

```text
split high-volume REJECT_FOR_NOW routes into narrow artifact-backed call classes
or improve evidence geometry for exhausted PROVEN routes
```

Immediate safe targets:

```text
metrics_report_readout split
git_control split
serving_ops split
read_inspect verifier/evidence
test_output_parse if found in real trace
```

## Broad Route Split Discovery V1

Source report:

```text
target/nando-wave/real-traffic-shadow/broad-route-split-discovery-v1.report.json
```

Purpose:

```text
Split REJECT_FOR_NOW broad routes into narrow artifact-backed call classes
before building another CPU profile.
```

Measured on the current 5k Codex history window:

```text
sampled_llm_calls: 5000
broad_candidate_events: 3330
non_exact_broad_candidate_events: 3089
candidate_split_rows: 11
watch_split_rows: 6
rejected_split_rows: 3
business_value_gate_passed_rows: 0
```

Top candidate splits:

| rank | parent route | split | candidates | non-exact | payload-ready | verifier-signal | status |
| ---: | --- | --- | ---: | ---: | ---: | ---: | --- |
| 1 | `answer_or_explain` | `file_path_evidence_answer` | 79 | 79 | 70 | 70 | `WATCH` |
| 2 | `project_context_dialogue` | `file_path_evidence_answer` | 65 | 63 | 51 | 52 | `WATCH` |
| 3 | `answer_or_explain` | `test_output_parse` | 57 | 57 | 3 | 35 | `CANDIDATE` |
| 4 | `answer_or_explain` | `metric_from_report` | 27 | 27 | 14 | 14 | `CANDIDATE` |
| 5 | `answer_or_explain` | `git_status_summary` | 10 | 10 | 5 | 10 | `CANDIDATE` |
| 6 | `agent_continue_execute` | `git_status_summary` | 8 | 8 | 4 | 8 | `CANDIDATE` |

Still rejected:

```text
project_context_dialogue / broad_reasoning_requires_llm: 1216 non-exact
answer_or_explain / broad_reasoning_requires_llm: 1029 non-exact
agent_continue_execute / artifact_progress: WATCH, high stateful singleton risk
```

Decision:

```text
The broad routes remain blocked as full routes.
The next high-value narrow work should not promote file_path_evidence_answer
as-is: its output verifier exists, but request-side admission is only
singleton-safe. Work should either collect more verifier-true non-synthetic
rows, split file_path_evidence_answer into a narrower artifact-backed family, or
move to test_output_parse / metric_from_report. All rows still have
expected_unique_cpu_accepts_over_exact_cache = 0 until output evidence,
admission audit, shadow, feedback, and CPU catalog prove false_accepts=0.
```

## Test Output Parse Payload Dry-Run V1

Source report:

```text
target/nando-wave/real-traffic-shadow/test-output-parse-payload-dry-run-v1.report.json
```

Purpose:

```text
Turn the narrow broad-route split `test_output_parse` into request-side
payloads, without promoting the blocked `answer_or_explain` or
`project_context_dialogue` routes as a whole.
```

Measured on the same 5k Codex history window:

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

Parent-route provenance:

```text
answer_or_explain: 57
project_context_dialogue: 47
```

Catalog status:

```text
CANDIDATE / REVIEW
```

Decision:

```text
`test_output_parse` is a real CPU-call candidate zone: 102 non-exact calls are
visible in the current trace window. It is not a savings claim yet. The request
payload builder only found 3 scoreable rows because the split needs actual
command-output/tool-output evidence. Next work should attach
test_status_and_error_excerpt/tool_output_validation_result verifier labels,
then compile a disabled-threshold profile and calibrate admission. Do not local
accept without verifier evidence.
```

### Test Output Parse Output Evidence V1

Source report:

```text
target/nando-wave/real-traffic-shadow/test-output-parse-output-evidence-v1.report.json
```

Measured on the 3 scoreable payload rows:

```text
operator_candidate_calls: 3
scoreable_candidate_calls: 3
output_evidence_matched_events: 3
deterministic_verification_events: 3
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

Catalog status after evidence:

```text
CANDIDATE / LOW_SUPPORT_REVIEW
```

Decision:

```text
Verifier labels exist for the scoreable subset and are currently clean, but the
support is only 3 rows. This is not enough to count savings or promote local
accepts. For CPU80, the larger debt is agent-loop tool-output state capture:
many test_output_parse candidates refer to command/test results, but the current
request-side payload builder sees explicit status evidence in only 3 of 104
candidate rows.
```

## File Path Evidence Payload Dry-Run V1

Source report:

```text
target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.report.json
```

Purpose:

```text
Turn the broad-route split `file_path_evidence_answer` into real request-side
payloads without promoting broad answer/project routes.
```

Measured on the same 5k Codex history window:

```text
file_path_evidence_candidate_events: 146
non_exact_candidate_events: 144
exact_cache_overlap_events: 2
payload_ready_events: 122
payload_built_events: 44
scoreable_payload_events: 44
profile_registered: false
shadow_score_ready: false
expected_unique_cpu_accepts_over_exact_cache: 0
expected_savings_milli: 0
false_accepts: 0
local_accepts_enabled: false
market_claim_allowed: false
```

Parent-route provenance:

```text
answer_or_explain: 79
project_context_dialogue: 65
agent_continue_execute: 2
```

Catalog status:

```text
CANDIDATE
```

Decision:

```text
This row has real traffic and scoreable request-side payloads, but it is not
CPU savings. It stays CANDIDATE until a disabled-threshold profile, deterministic
source/path verifier, admission audit, shadow, feedback, and CPU catalog prove
unique accepts over exact cache with false_accepts=0.
```

### Disabled Profile V1

Source reports:

```text
target/nando-wave/real-traffic-shadow/file-path-evidence-profile-v1.report.json
target/nando-wave/real-traffic-shadow/file-path-evidence-profile-shadow-v1.report.json
target/nando-wave/real-traffic-shadow/file-path-evidence-profile-v1.verification-hook-audit.report.json
```

Measured profile:

```text
profile_id: split_file_path_evidence_answer_profile_v1
package_bytes: 128
edge_count: 7
runtime_bytes_estimate: 32972
threshold: 2147483647
scoreable_payload_events: 44
package_training_requests: 44
positive_margin_rows: 44
strict_ordered_pass_rows: 44
unexpected_local_accepts_under_disabled_threshold: 0
median_energy_margin: 1123328
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
p99_shadow_score_latency_ns: 259065
verification_hook_ready_events: 0
verified_cpu_accept_eligible_events: 0
candidates_missing_output_evidence: 44
market_claim_allowed: false
```

Catalog status remains:

```text
CANDIDATE
```

Decision:

```text
The profile/scoring path is connected, but the route still adds zero CPU80
savings. The blocking item is verifier evidence, not score geometry. Do not
lower threshold or promote until source_path_or_url_presence_verifier_v1 proves
safe accepts and the feedback/catalog path reports unique value over exact cache.
```

### Output Evidence V1

Source reports:

```text
target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.report.json
target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-shadow-v1.report.json
target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.verification-hook-audit.report.json
```

Measured verifier labels:

```text
operator_candidate_calls: 44
scoreable_candidate_calls: 44
output_evidence_matched_events: 39
no_session_output_match_events: 5
deterministic_verification_events: 39
verified_true_events: 15
verified_false_events: 24
raw_prompt_text_written: false
raw_response_text_written: false
local_accepts_enabled: false
market_claim_allowed: false
```

Measured shadow/audit:

```text
nando_shadow_accepts: 0
nando_shadow_fallbacks: 44
verified_safe_accepts: 0
unverified_shadow_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
verification_hook_ready_events: 39
verified_cpu_accept_eligible_events: 0
candidates_missing_output_evidence: 5
provider_cost_events: 0
```

Catalog status becomes:

```text
WATCH
```

Decision:

```text
Verifier evidence exists now, but the disabled profile still accepts zero rows.
The route remains non-savings until request-side admission calibration finds a
robust safe policy and a promoted shadow run proves unique verified accepts over
exact cache with false_accepts=0.
```

### Admission Calibration V1

Source report:

```text
target/nando-wave/real-traffic-shadow/file-path-evidence-admission-calibration-v1.report.json
```

Measured request-side admission:

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

Catalog status:

```text
WATCH
```

Decision:

```text
This split has verifier labels, but no robust request-side admission policy.
The strongest zero-false policy accepts only one verifier-true row, below the
minimum robust support of 3. Do not promote. Either collect more verifier-true
non-synthetic rows, split the family more narrowly, or choose the next
higher-value artifact-backed profile.
```

Blocked for now:

```text
answer_or_explain as a whole
project_context_dialogue as a whole
agent_continue_execute as a whole
IME singleton-only routes
resource_pressure without verifier-true evidence
```

## Claim Boundary

Allowed:

```text
On the current non-synthetic 1000-call Codex trace, the CPU call catalog finds
7 proven call classes and 25 incremental unique verified CPU accepts over exact
cache, with false_accepts=0.
```

Not allowed:

```text
Nando saves 80%
Nando saves market traffic
scoreable rows are savings
broad answer routes are safe
candidate rows are verified CPU accepts
```
