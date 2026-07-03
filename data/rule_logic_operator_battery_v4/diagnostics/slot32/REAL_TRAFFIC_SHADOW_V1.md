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
  role-binding-real-traffic-shadow-v1
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
  candidate_events: 282
  no_candidate_events: 718
  full_shadow_request_payload_built: false
  route_counts:
    role_binding_edit_marker_length_seed0: 152
    role_binding_conditional_branch_seed0: 92
    role_binding_mixed_map_seed0: 38
```

Route-only candidate shadow analyzer:

```text
command: cargo run --release -p nando-cli -- role-binding-real-traffic-shadow-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.trace.jsonl target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.shadow-report.json
result:
  verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW
  total_requests: 1000
  total_llm_calls: 1000
  exact_cache_hits: 54
  operator_candidate_calls: 282
  nando_shadow_accepts: 0
  verified_safe_accepts: 0
  false_accepts: 0
  incremental_reduction_vs_exact_cache_milli: 0
  synthetic_trace_used: false
```

This closes route/profile candidate discovery only. The missing next piece is a
request-side builder for `active_fringe` and slot impulses. Until that exists,
route-only candidates must remain fallback-only.

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
