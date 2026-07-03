# Real Traffic Shadow V1

Date: 2026-07-03

Verdict:

```text
REAL_TRAFFIC_SHADOW_V1_READY
SYNTHETIC_SMOKE_FORCES_REVIEW
```

## What Changed

```text
Added a real-traffic shadow recorder and replay analyzer to nando-cli.

New commands:
  role-binding-real-traffic-record-v1
  role-binding-real-traffic-record-serve-v1
  role-binding-real-traffic-ingest-events-v1
  role-binding-real-traffic-codex-history-ingest-v1
  role-binding-real-traffic-codex-history-route-candidates-v1
  role-binding-real-traffic-shadow-v1
  role-binding-real-traffic-cpu-route-forecast-v1
  role-binding-real-traffic-edit-payload-readiness-v1
  role-binding-real-traffic-edit-payload-dry-run-v1
  role-binding-real-traffic-edit-output-evidence-v1
  role-binding-real-traffic-edit-local-accept-calibration-v1
  role-binding-real-traffic-verification-hook-audit-v1
  role-binding-real-traffic-feedback-loop-v1
  role-binding-real-traffic-shadow-smoke-v1
```

The recorder appends one validated JSONL row without touching the live LLM
flow. The analyzer loads only serving `.nwrb` profile registry packages and
computes shadow routability, exact-cache baseline, verified local accepts,
false accepts, latency, RSS, and operator rankings.

`role-binding-real-traffic-record-serve-v1` exposes the same recorder contract
over local HTTP:

```text
GET /health
POST /trace
GET /metrics
```

It supports a `request-limit`, so smoke tests and watchdog-controlled runs can
terminate without leaving a hanging daemon.

`role-binding-real-traffic-ingest-events-v1` converts agent/API event JSONL into
the same trace contract. It is a batch ingestion path for real traces collected
outside the recorder HTTP server.

`role-binding-real-traffic-codex-history-ingest-v1` converts local Codex prompt
history into privacy-safe event fingerprints. It never writes raw prompt text.
This is useful for non-synthetic traffic baselines, but it cannot prove savings
until another adapter supplies `nando_shadow_request`.

`role-binding-real-traffic-codex-history-route-candidates-v1` adds the first
route-only candidate adapter. It looks only at request-side prompt text, selects
a route/profile from the serving registry, and writes a `nando_shadow_request`
with empty `active_fringe` and `slots`. Empty payloads force safe fallback and
prevent fake local accepts.

## Market Claim Boundary

```text
Synthetic/release-suite savings are not market savings.

Market savings require:
  real trace traffic
  shadow-only Nando scoring
  verified_safe_accept = true
  exact-cache comparison
  false_local_accepts = 0
```

Synthetic traces always force:

```text
REAL_TRAFFIC_SHADOW_V1_REVIEW
```

even when the measured reduction looks good.

## Trace Contract

Each JSONL row stores the serving score request plus safe provenance fields:

```text
schema_version
trace_id
traffic_source
time_ms
request_fingerprint
response_fingerprint
tool_call_fingerprints
verification_source
llm_call
exact_cache_key
provider_cache_hit
provider_cost_microusd
nando_shadow_request
verified_safe_accept
synthetic_source
notes
```

Validation rule:

```text
If verified_safe_accept is present, the row must also carry:
  nando_shadow_request
  response_fingerprint or tool_call_fingerprints
  verification_source
```

So `verified_safe_accept` is not accepted as a naked label. It must be backed by
output/tool evidence and a named verification source.

The recorder/analyzer do not require raw prompts or raw responses. Real
integrations can store fingerprints and verification source first, then decide
later whether to retain redacted payloads in a separate private trace vault.

## Operator Ranking

The analyzer now reports `operator_rankings`.

Ranking components:

```text
traffic_share_milli
local_accept_rate_milli
verified_accept_rate_milli
incremental_savings_over_exact_cache
estimated_cost_saved_microusd
false_accepts
unverified_shadow_accepts
p99_shadow_score_latency_ns
value_score_microusd_per_ms
```

This is the first product-facing bridge from proof operators to money-ranked
operators:

```text
operator_value =
  frequency_in_real_traces
  * local_accept_rate
  * saved_llm_cost
  * safety_score
  / runtime_cost
```

The current number is only a ranking aid. The components are the real evidence.

## Local Synthetic Smoke

Local Codex history baseline:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-codex-history-ingest-v1 /home/ubu/.codex/history.jsonl target/nando-wave/real-traffic-shadow/codex-history-events-v1.events.jsonl target/nando-wave/real-traffic-shadow/codex-history-events-v1.report.json 1000
result:
  verdict: CODEX_HISTORY_EVENTS_V1_READY
  total_history_rows: 12187
  events_written: 1000
  raw_text_written: false
```

Codex history -> trace ingest:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-ingest-events-v1 target/nando-wave/real-traffic-shadow/codex-history-events-v1.events.jsonl target/nando-wave/real-traffic-shadow/codex-history-events-v1.trace.jsonl target/nando-wave/real-traffic-shadow/codex-history-events-v1.ingest-report.json
result:
  verdict: REAL_TRAFFIC_INGEST_V1_REVIEW
  total_events: 1000
  llm_calls: 1000
  operator_candidate_events: 0
  synthetic_events: 0
```

Codex history shadow analyzer:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-shadow-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/codex-history-events-v1.trace.jsonl target/nando-wave/real-traffic-shadow/codex-history-events-v1.shadow-report.json
result:
  verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
  total_requests: 1000
  total_llm_calls: 1000
  exact_cache_hits: 54
  operator_candidate_calls: 0
  nando_shadow_accepts: 0
  false_accepts: 0
  incremental_reduction_vs_exact_cache_milli: 0
```

Interpretation: we now have a real, non-synthetic local traffic baseline, but it
contains no Nando shadow candidates. The next missing piece is a router/candidate
adapter that turns a real agent event into a `nando_shadow_request` without
reading target answers or using proof labels.

Codex history route-only candidate adapter:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-codex-history-route-candidates-v1 /home/ubu/.codex/history.jsonl target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.events.jsonl target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.report.json 1000
result:
  verdict: CODEX_HISTORY_ROUTE_CANDIDATES_V1_REVIEW
  events_written: 1000
  candidate_events: 285
  no_candidate_events: 715
  full_shadow_request_payload_built: false
  route_counts:
    role_binding_edit_marker_length_seed0: 154
    role_binding_conditional_branch_seed0: 92
    role_binding_mixed_map_seed0: 39
```

Route-only candidate shadow analyzer:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-shadow-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.trace.jsonl target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.shadow-report.json
result:
  verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
  total_requests: 1000
  total_llm_calls: 1000
  exact_cache_hits: 54
  operator_candidate_calls: 285
  nando_shadow_accepts: 0
  verified_safe_accepts: 0
  false_accepts: 0
  incremental_reduction_vs_exact_cache_milli: 0
  synthetic_trace_used: false
```

This closes route/profile candidate discovery only. The missing next piece is a
request-side builder for `active_fringe` and slot impulses. Until that exists,
route-only candidates must remain fallback-only.

Edit payload dry-run builder:

```text
command: cargo run -p nando-cli -- role-binding-real-traffic-edit-payload-dry-run-v1 /home/ubu/.codex/history.jsonl target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.trace.jsonl target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.report.json 1000
result:
  verdict: EDIT_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT
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

Edit payload dry-run shadow analyzer:

```text
command: cargo run -p nando-cli -- role-binding-real-traffic-shadow-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.trace.jsonl target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.shadow-report.json
result:
  verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
  total_requests: 1000
  total_llm_calls: 1000
  exact_cache_hits: 54
  operator_candidate_calls: 23
  nando_shadow_accepts: 0
  nando_shadow_fallbacks: 23
  verified_safe_accepts: 0
  unverified_shadow_accepts: 0
  false_accepts: 0
  incremental_reduction_vs_exact_cache_milli: 0
  p99_shadow_score_latency_ns: 683880
  synthetic_trace_used: false
```

Interpretation: the first request-side edit builder now emits non-empty
`active_fringe` and slot impulses for 23 real prompt-side rows. This proves the
empty-payload blocker is removed for the priority edit slice. It does not prove
savings: verified accepts remain disabled, and every scoreable dry-run payload
falls back safely. The observed margins are positive at sequence-energy level
but not at strict slot level (`min_slot_margin = 0`), so the next engineering
debt is an edit verifier/output hook, not a market claim.

Verification hook audit over edit dry-run trace:

```text
command: cargo run -p nando-cli -- role-binding-real-traffic-verification-hook-audit-v1 target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.trace.jsonl target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.shadow-report.json target/nando-wave/real-traffic-shadow/verification-hook-audit-v1.report.json
result:
  verdict: VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS
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
  market_claim_allowed: false
```

Synthetic positive-control hook audit:

```text
command: cargo run -p nando-cli -- role-binding-real-traffic-verification-hook-audit-v1 target/nando-wave/real-traffic-shadow/verification-hook-synthetic-smoke.trace.jsonl target/nando-wave/real-traffic-shadow/verification-hook-synthetic-smoke.shadow-report.json target/nando-wave/real-traffic-shadow/verification-hook-synthetic-smoke.audit-report.json
result:
  verdict: VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
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
  verified_cpu_accept_eligible_events: 0
  market_claim_allowed: false
```

Interpretation: the audit now separates three states:

```text
route candidate
  -> scoreable payload
  -> evidence-backed verification hook
  -> verified CPU accept eligible for savings
```

Before the output-evidence join, current real Codex edit rows had reached
`scoreable payload`, but not `evidence-backed verification hook`.

Edit output-evidence join:

```text
command: cargo run -p nando-cli -- role-binding-real-traffic-edit-output-evidence-v1 target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.trace.jsonl /home/ubu/.codex/sessions target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.trace.jsonl target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.report.json
result:
  verdict: EDIT_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED
  total_trace_rows: 1000
  operator_candidate_calls: 23
  scoreable_candidate_calls: 23
  output_evidence_matched_events: 17
  no_session_output_match_events: 6
  deterministic_verification_events: 17
  verified_true_events: 3
  verified_false_events: 14
  raw_prompt_text_written: false
  raw_response_text_written: false
  response_text_used_for_verification: true
  target_labels_used: false
  proof_labels_used: false
  local_accepts_enabled: false
  market_claim_allowed: false
```

Shadow + audit over the evidence-enriched trace:

```text
shadow:
  verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
  exact_cache_hits: 54
  nando_shadow_accepts: 0
  verified_safe_accepts: 0
  false_accepts: 0
  incremental_reduction_vs_exact_cache_milli: 0
  p99_shadow_score_latency_ns: 555869

audit:
  verdict: VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND
  scoreable_candidate_calls: 23
  verification_hook_ready_events: 17
  verified_cpu_accept_eligible_events: 0
  market_claim_allowed: false
```

Updated interpretation: the first edit route has moved past the blanket
`missing output evidence` blocker. The remaining blocker is local CPU accept:
the current profile runtime accepts 0/17 hook-ready real edit rows and therefore
still proves no real savings.

Edit local-accept calibration:

```text
command: cargo run -p nando-cli -- role-binding-real-traffic-edit-local-accept-calibration-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.trace.jsonl target/nando-wave/real-traffic-shadow/edit-local-accept-calibration-v1.report.json
result:
  verdict: EDIT_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY
  hook_ready_rows: 17
  label_true_rows: 3
  label_false_rows: 14
  safe_policy_found: false
  best_safe_true_accepts: 0
```

Policy sweep:

```text
current_strict_all_slots:
  accepts: 0
  true_accepts: 0
  false_accepts: 0

energy_only_no_slot_order:
  accepts: 17
  true_accepts: 3
  false_accepts: 14

marker_slot_only_ignore_end_slot:
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

Updated interpretation: tuning readout cannot safely unlock this edit route.
The end slot causes current fallback, but ignoring it would accept 14
verifier-false rows. The next safe step is request-side admission / richer edit
payload features, not threshold lowering.

CPU route feedback-loop report:

```text
command: cargo run -p nando-cli -- role-binding-real-traffic-feedback-loop-v1 target/nando-wave/real-traffic-shadow/cpu-route-forecast-v1.report.json target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.report.json target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.verification-hook-audit.report.json target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json
result:
  verdict: CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW
  total_llm_calls: 1000
  exact_cache_hits: 54
  exact_cache_coverage_milli: 54
  operator_candidate_calls: 285
  operator_candidate_coverage_milli: 285
  scoreable_candidate_calls: 23
  scoreable_candidate_coverage_milli: 23
  verification_hook_ready_events: 17
  verified_cpu_accept_eligible_events: 0
  verified_cpu_routability_milli: 0
  target_routability_milli: 800
  target_verified_cpu_calls: 800
  routing_gap_to_80_calls: 515
  verified_gap_to_80_calls: 800
  market_claim_allowed: false
```

Feedback stages:

```text
role_binding_edit_marker_length_seed0:
  candidate_events: 154
  scoreable_payload_events: 23
  verification_hook_ready_events: 17
  stage: verification_hook_ready_waiting_local_accept
  next_action: Run local-accept calibration; if no safe policy exists, improve request-side admission or payload features.

role_binding_conditional_branch_seed0:
  candidate_events: 92
  stage: payload_builder_missing
  next_action: Build the request-side payload builder for this route family.

role_binding_mixed_map_seed0:
  candidate_events: 39
  stage: payload_builder_missing
  next_action: Build the request-side payload builder for this route family.
```

Interpretation: this report prevents the exact-cache, route-candidate,
scoreable-payload, verification-hook, and verified-CPU stages from being mixed.
CPU Routability 80 remains red until verified CPU routability reaches 800 milli
on non-synthetic real traffic with false accepts at zero.

CPU route forecast over real Codex route candidates:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-cpu-route-forecast-v1 target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.report.json target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.shadow-report.json target/nando-wave/real-traffic-shadow/cpu-route-forecast-v1.report.json
result:
  verdict: CPU_ROUTE_FORECAST_V1_REVIEW
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
   candidate_share_of_all_llm_calls: 154 milli
   recommended_payload_builder: edit_marker_length_payload_builder_v1
   work: detect edit intent, affected file/text marker, requested length/shape
         constraint, and deterministic patch slots from request text only.

2. role_binding_conditional_branch_seed0
   candidate_events: 92
   candidate_share_of_all_llm_calls: 92 milli
   exact_cache_hits_inside_route: 2
   recommended_payload_builder: conditional_branch_payload_builder_v1
   work: extract condition, evidence slots, allowed/refused branch, and
         fallback threshold from request text only.

3. role_binding_mixed_map_seed0
   candidate_events: 39
   candidate_share_of_all_llm_calls: 39 milli
   recommended_payload_builder: mixed_map_payload_builder_v1
   work: extract source slots, destination slots, ordered mapping action, and
         invariant checks from request text only.
```

Forecast boundary:

```text
This is route-zone capacity, not verified savings.
The route forecast may become a market claim only after real request-side
payload builders produce verified_safe_accepts > 0 with false_accepts = 0.
```

Edit route payload readiness over the same real Codex window:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-edit-payload-readiness-v1 /home/ubu/.codex/history.jsonl target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/edit-payload-readiness-v1.report.json 1000
result:
  verdict: EDIT_PAYLOAD_READINESS_V1_REVIEW_READY_CANDIDATES_FOUND
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

Interpretation:

```text
The edit route is the largest real CPU offload candidate zone, but only
23/154 current edit candidates have enough request-side structure for the first
payload-builder attempt. That is the honest first sub-target for verified
local accepts; the rest need better route taxonomy or richer event capture.
```

Batch event ingestion smoke:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-ingest-events-v1 target/nando-wave/real-traffic-shadow/real-traffic-ingest-contract-smoke.events.jsonl target/nando-wave/real-traffic-shadow/real-traffic-ingest-contract-smoke.trace.jsonl target/nando-wave/real-traffic-shadow/real-traffic-ingest-contract-smoke.report.json
result:
  verdict: REAL_TRAFFIC_INGEST_V1_REVIEW
  total_events: 1
  llm_calls: 1
  operator_candidate_events: 0
  synthetic_events: 1
```

Analyzer on the ingested smoke trace:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-shadow-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/real-traffic-ingest-contract-smoke.trace.jsonl target/nando-wave/real-traffic-shadow/real-traffic-ingest-contract-smoke.shadow-report.json
result:
  verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
  total_requests: 1
  total_llm_calls: 1
  nando_shadow_accepts: 0
  verified_safe_accepts: 0
  false_accepts: 0
  incremental_reduction_vs_exact_cache_milli: 0
```

Recorder server smoke:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-record-serve-v1 target/nando-wave/real-traffic-shadow/real-traffic-record-server-smoke.trace.jsonl 127.0.0.1:38991 3
requests:
  GET /health
  POST /trace
  GET /metrics
result:
  rows_written: 1
  requests_handled: 3
  bad_requests: 0
  server exited after request_limit
```

Shadow analyzer smoke:

Command:

```text
cargo run --release -p nando-cli -- role-binding-real-traffic-shadow-smoke-v1 target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json target/nando-wave/real-traffic-shadow/real-traffic-shadow-smoke-v1.trace.jsonl 4
```

Result:

```text
rows_written: 28
synthetic_source: true
```

Analyzer command:

```text
cargo run --release -p nando-cli -- role-binding-real-traffic-shadow-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/real-traffic-shadow-smoke-v1.trace.jsonl target/nando-wave/real-traffic-shadow/real-traffic-shadow-smoke-v1.product-proof.json
```

Analyzer result:

```text
verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
total_requests: 28
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

This is a smoke test only. It proves the analyzer and safety boundary, not
real-market savings.

## No SSH

This rung was run locally only. No SSH or remote deployment command was used.

## Next Proof Debt

```text
real agent/API trace
  -> shadow-only scoring
  -> operator mining
  -> rank top operators by verified saved cost
  -> compile top profiles
  -> compare exact-cache vs exact-cache+Nando
```

Do not claim "we save X%" until the trace is non-synthetic and the report
passes `REAL_TRAFFIC_SHADOW_V1_PASS`.
