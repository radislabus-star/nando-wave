# Nando Wave Product Runtime Task

Date: 2026-07-03

Status:

```text
PROFILE_RUNTIME_DEPLOYED_CHEAP_VPS_REPLAY_PASS
```

## Goal

Build the first product-shaped Nando Wave CPU offload runtime:

```text
route -> small L2-resident profile shard -> local score -> fallback
```

This task is not another research gate. It is the next serving architecture.

## Product Claim To Build Toward

```text
Nando Wave compiles repeated transferable operator workflows into small
CPU-local profile shards that reduce LLM calls beyond exact cache while keeping
false local accepts at zero.
```

The immediate product target:

```text
exact cache enabled
Nando enabled after exact-cache miss
Nando accepts only high-margin transferable actions
LLM fallback handles everything else
```

## Why This Task Exists

Measured facts:

```text
Current 32-slot runtime profiles are small:
  mixed_map profile: ~83 KiB runtime estimate
  conditional_branch profile: ~158 KiB runtime estimate
  current 7-profile serving registry: 790020 bytes runtime estimate

Reserve VPS cache:
  L2: ~4 MiB per vCPU
  L3: 16 MiB shared

HTTP/nginx replay:
  PASS
  false_local_accepts: 0
  missed_expected_local: 0
  strict_ordered_accuracy_milli: 1000
  p99: ~0.7 ms per sequence

Replay cache result:
  no cache LLM calls: 49,152
  exact cache LLM calls: 24,576
  exact cache + Nando LLM calls: 12,288
  incremental reduction vs exact cache: 50%
```

Boundary:

```text
The first product-shaped serving smoke is now green. This is still local smoke,
not real Codex production traffic yet.
```

Current real-traffic shadow path:

```text
report_doc: data/rule_logic_operator_battery_v4/diagnostics/slot32/REAL_TRAFFIC_SHADOW_V1.md
record_command: role-binding-real-traffic-record-v1
record_http_command: role-binding-real-traffic-record-serve-v1
ingest_events_command: role-binding-real-traffic-ingest-events-v1
codex_history_ingest_command: role-binding-real-traffic-codex-history-ingest-v1
codex_history_route_candidate_command: role-binding-real-traffic-codex-history-route-candidates-v1
analyze_command: role-binding-real-traffic-shadow-v1
cpu_route_forecast_command: role-binding-real-traffic-cpu-route-forecast-v1
edit_payload_readiness_command: role-binding-real-traffic-edit-payload-readiness-v1
edit_payload_dry_run_command: role-binding-real-traffic-edit-payload-dry-run-v1
verification_hook_audit_command: role-binding-real-traffic-verification-hook-audit-v1
feedback_loop_command: role-binding-real-traffic-feedback-loop-v1
smoke_command: role-binding-real-traffic-shadow-smoke-v1

purpose:
  record real requests/responses/tool-call fingerprints as JSONL;
  leave the live LLM flow untouched;
  score Nando in shadow mode only;
  compare against exact-cache baseline;
  rank operators by verified saved cost and runtime cost.

current smoke:
  real_shadow_pass_gate_requires_verified_savings: true.
  codex_history_route_candidate_events: 285.
  codex_history_route_no_candidate_events: 715.
  codex_history_route_full_shadow_request_payload_built: false.
  codex_history_route_shadow_verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW.
  codex_history_route_shadow_nando_shadow_accepts: 0.
  codex_history_route_shadow_incremental_reduction_vs_exact_cache_milli: 0.
  cpu_route_forecast_verdict: CPU_ROUTE_FORECAST_V1_REVIEW.
  cpu_route_forecast_report: target/nando-wave/real-traffic-shadow/cpu-route-forecast-v1.report.json.
  cpu_route_forecast_market_claim_allowed: false.
  cpu_route_forecast_priority_1: role_binding_edit_marker_length_seed0, 154 candidates.
  cpu_route_forecast_priority_2: role_binding_conditional_branch_seed0, 92 candidates.
  cpu_route_forecast_priority_3: role_binding_mixed_map_seed0, 39 candidates.
  cpu_route_forecast_50_percent_additional_savings: 141 calls.
  cpu_route_forecast_50_percent_total_calls_removed_with_exact_cache: 195 calls.
  edit_payload_readiness_verdict: EDIT_PAYLOAD_READINESS_V1_REVIEW_READY_CANDIDATES_FOUND.
  edit_payload_readiness_candidate_events: 154.
  edit_payload_readiness_payload_ready_events: 23.
  edit_payload_readiness_payload_ready_rate_milli: 149.
  edit_payload_dry_run_verdict: EDIT_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT.
  edit_payload_dry_run_trace: target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.trace.jsonl.
  edit_payload_dry_run_report: target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.report.json.
  edit_payload_dry_run_shadow_report: target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.shadow-report.json.
  edit_payload_dry_run_payload_built_events: 23.
  edit_payload_dry_run_scoreable_payload_events: 23.
  edit_payload_dry_run_raw_text_written: false.
  edit_payload_dry_run_response_text_used: false.
  edit_payload_dry_run_target_labels_used: false.
  edit_payload_dry_run_proof_labels_used: false.
  edit_payload_dry_run_shadow_operator_candidate_calls: 23.
  edit_payload_dry_run_shadow_nando_shadow_accepts: 0.
  edit_payload_dry_run_shadow_verified_safe_accepts: 0.
  edit_payload_dry_run_shadow_false_accepts: 0.
  edit_payload_dry_run_shadow_incremental_reduction_vs_exact_cache_milli: 0.
  edit_payload_dry_run_shadow_p99_shadow_score_latency_ns: 683880.
  verification_hook_audit_verdict: VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS.
  verification_hook_audit_report: target/nando-wave/real-traffic-shadow/verification-hook-audit-v1.report.json.
  verification_hook_audit_operator_candidate_calls: 23.
  verification_hook_audit_scoreable_candidate_calls: 23.
  verification_hook_audit_response_fingerprint_events: 0.
  verification_hook_audit_explicit_verified_safe_accept_events: 0.
  verification_hook_audit_candidates_missing_output_evidence: 23.
  verification_hook_audit_candidates_missing_explicit_verification: 23.
  verification_hook_audit_verification_hook_ready_events: 0.
  verification_hook_audit_verified_cpu_accept_eligible_events: 0.
  verification_hook_audit_market_claim_allowed: false.
  verification_hook_synthetic_control_verdict: VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND.
  verification_hook_synthetic_control_ready_events: 14.
  verification_hook_synthetic_control_shadow_accepts: 7.
  verification_hook_synthetic_control_market_claim_allowed: false.
  feedback_loop_verdict: CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW.
  feedback_loop_report: target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json.
  feedback_loop_exact_cache_hits: 54.
  feedback_loop_operator_candidate_calls: 285.
  feedback_loop_scoreable_candidate_calls: 23.
  feedback_loop_verification_hook_ready_events: 0.
  feedback_loop_verified_cpu_routability_milli: 0.
  feedback_loop_routing_gap_to_80_calls: 515.
  feedback_loop_verified_gap_to_80_calls: 800.
  codex_history_events_verdict: CODEX_HISTORY_EVENTS_V1_READY.
  codex_history_events_written: 1000.
  codex_history_raw_text_written: false.
  codex_history_shadow_verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW.
  codex_history_total_llm_calls: 1000.
  codex_history_exact_cache_hits: 54.
  codex_history_operator_candidate_calls: 0.
  codex_history_incremental_reduction_vs_exact_cache_milli: 0.
  ingest_events_verdict: REAL_TRAFFIC_INGEST_V1_REVIEW.
  ingest_events_total_events: 1.
  ingest_events_operator_candidate_events: 0.
  ingest_events_synthetic_events: 1.
  ingest_shadow_verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW.
  record_http_endpoints: /health /trace /metrics.
  record_http_requests_handled: 3.
  record_http_rows_written: 1.
  record_http_bad_requests: 0.
  record_http_exited_after_request_limit: true.
  verdict: REAL_TRAFFIC_SHADOW_V1_REVIEW.
  total_requests: 28.
  total_llm_calls: 28.
  nando_shadow_accepts: 14.
  verified_safe_accepts: 14.
  false_accepts: 0.
  incremental_reduction_vs_exact_cache_milli: 500.
  p99_shadow_score_latency_ns: 144392.
  synthetic_trace_used: true.

boundary:
  synthetic smoke is not a market savings claim.
  REAL_TRAFFIC_SHADOW_V1_PASS requires non-synthetic real trace rows.
```

Current serving evidence:

```text
registry_config: target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json
runtime_smoke_report: target/nando-wave/role-binding-profile-runtime/profile-runtime-smoke-v1.product-proof.json
runtime_replay_report: target/nando-wave/role-binding-profile-runtime/profile-replay-suite-v1.product-proof.json
runtime_fallback_report: target/nando-wave/role-binding-profile-runtime/profile-fallback-smoke-v1.product-proof.json
runtime_worker_scaling_report: target/nando-wave/role-binding-profile-runtime/profile-worker-scaling-v1.product-proof.json
runtime_worker_replay_report: target/nando-wave/role-binding-profile-runtime/profile-worker-replay-v1.product-proof.json
runtime_lb_replay_report: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-v1.product-proof.json
runtime_deployed_hostworld_report: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1.product-proof.json
runtime_lb_throughput_report: target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-v1.product-proof.json
runtime_deployed_hostworld_throughput_report: target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-hostworld-v1.product-proof.json
runtime_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_RUNTIME_SMOKE.md
replay_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_REPLAY_SUITE.md
fallback_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_FALLBACK_SMOKE.md
worker_scaling_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_WORKER_SCALING.md
worker_replay_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_WORKER_REPLAY.md
load_balancer_replay_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_LB_REPLAY.md
deployed_hostworld_replay_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_DEPLOYED_HOSTWORLD_REPLAY.md
load_balancer_throughput_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_LB_THROUGHPUT.md
real_traffic_shadow_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/REAL_TRAFFIC_SHADOW_V1.md

profile_count: 7
runtime_bytes_estimate: 790020
exact_cache_llm_calls: 2
exact_cache_plus_nando_llm_calls: 1
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
p99_latency_ns: 21436
rss_bytes: 10805248
```

Current product replay evidence:

```text
verdict: ROLE_BINDING_PROFILE_REPLAY_SUITE_V1_PASS
command: cargo run --release -p nando-cli -- role-binding-profile-replay-suite-v1
unique_sequences_replayed: 896
http_replay_batches: 224
no_cache_llm_calls: 1792
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
missed_expected_local: 0
p50_latency_ns: 125821
p90_latency_ns: 148048
p99_latency_ns: 213509
rss_bytes: 8101888
runtime_bytes_estimate: 790020
serving_worker_loads: .nwrb only
eval_packs_loaded_in_serving_worker: false
corpus_jsonl_loaded_in_serving_worker: false
compiler_used: false
python_demo_used: false
```

Current fallback evidence:

```text
verdict: ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_PASS
command: cargo run -p nando-cli -- role-binding-profile-fallback-smoke-v1
local_accept_pass: true
bad_route_fallback_pass: true
low_margin_fallback_pass: true
bad_route_fallback_reason: profile_not_found
low_margin_fallback_reason: margin_below_threshold
local_operator_calls: 1
fallback_to_llm_calls: 2
false_local_accepts: 0
p99_latency_ns: 24312
runtime_bytes_estimate: 790020
```

Current worker scaling evidence:

```text
verdict: ROLE_BINDING_PROFILE_WORKER_SCALING_V1_PASS
command: cargo run --release -p nando-cli -- role-binding-profile-worker-scaling-v1
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

Current sharded worker replay evidence:

```text
verdict: ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS
command: cargo run --release -p nando-cli -- role-binding-profile-worker-replay-v1
worker_count: 2
total_profile_count: 7
unique_sequences_replayed: 896
http_replay_batches: 224
no_cache_llm_calls: 1792
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
missed_expected_local: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_rss_bytes: 7135232
max_worker_p99_latency_ns: 265277
all_workers_serving_only: true
```

Current local load-balancer replay evidence:

```text
verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
command: cargo run --release -p nando-cli -- role-binding-profile-lb-replay-v1
worker_count: 2
total_profile_count: 7
unique_sequences_replayed: 896
http_replay_batches: 224
no_cache_llm_calls: 1792
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
missed_expected_local: 0
load_balancer_p50_latency_ns: 474665
load_balancer_p90_latency_ns: 580300
load_balancer_p99_latency_ns: 736030
core_score_p50_latency_ns: 40140
core_score_p90_latency_ns: 56192
core_score_p99_latency_ns: 78902
worker_score_p50_latency_ns: 99318
worker_score_p90_latency_ns: 134368
worker_score_p99_latency_ns: 167663
lb_upstream_roundtrip_p50_latency_ns: 474426
lb_upstream_roundtrip_p90_latency_ns: 579854
lb_upstream_roundtrip_p99_latency_ns: 735692
replay_client_wall_p50_latency_ns: 4136229
replay_client_wall_p90_latency_ns: 4786573
replay_client_wall_p99_latency_ns: 5489536
estimated_lb_overhead_p99_ns: 568367
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
load_balancer_rss_bytes: 6307840
max_worker_runtime_bytes_estimate: 492792
max_worker_p99_latency_ns: 167663
all_workers_serving_only: true
load_balancer_serving_only: true
```

Current deployed cheap-VPS replay evidence:

```text
verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
host_alias: hostworld-ee
bundle_root: /opt/nando-wave-profile-runtime-v1
binary: x86_64-unknown-linux-musl static nando-cli
command: ssh hostworld-ee 'cd /opt/nando-wave-profile-runtime-v1 && ./target/release/nando-cli role-binding-profile-lb-replay-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1-clean2.product-proof.json 2 128 4'
worker_count: 2
total_profile_count: 7
unique_sequences_replayed: 896
http_replay_batches: 224
no_cache_llm_calls: 1792
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
missed_expected_local: 0
load_balancer_p50_latency_ns: 1540141
load_balancer_p90_latency_ns: 1948988
load_balancer_p99_latency_ns: 2744444
core_score_p50_latency_ns: 59533
core_score_p90_latency_ns: 89933
core_score_p99_latency_ns: 184328
worker_score_p50_latency_ns: 229548
worker_score_p90_latency_ns: 311118
worker_score_p99_latency_ns: 698145
lb_upstream_roundtrip_p50_latency_ns: 1539701
lb_upstream_roundtrip_p90_latency_ns: 1948651
lb_upstream_roundtrip_p99_latency_ns: 2743851
replay_client_wall_p50_latency_ns: 13483605
replay_client_wall_p90_latency_ns: 16340101
replay_client_wall_p99_latency_ns: 19478822
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
load_balancer_rss_bytes: 2838528
max_worker_runtime_bytes_estimate: 492792
max_worker_p99_latency_ns: 698145
all_workers_serving_only: true
load_balancer_serving_only: true
```

Current bounded POST /score throughput evidence:

```text
local:
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
  load_balancer_p99_latency_ns: 743295
  client_p99_latency_ns: 3633433
  worker_score_p99_latency_ns: 169460
  core_score_p99_latency_ns: 72666
  packed_score_parity_mismatches: 0

hostworld 4 clients:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  score_requests: 896
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 3611864
  lb_upstream_roundtrip_p99_latency_ns: 3610931
  worker_score_p99_latency_ns: 577626
  core_score_p99_latency_ns: 221243

hostworld 1 client:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  load_balancer_p99_latency_ns: 3507669
  lb_upstream_roundtrip_p99_latency_ns: 3507021
  false_local_accepts: 0
  client_errors: 0
```

Boundary:

```text
The replay-suite has a local HTTP product replay PASS through a separate
load-balancer/proxy endpoint over sampled release-suite traffic. The deployed
cheap-VPS packed hot-path replay is now inside the 3 ms p99 envelope while
safety, exact-cache reduction, and packed-score parity remain green. It is
stronger than direct worker replay.

The bounded individual POST /score throughput gate is different and currently
red on HostWorld: the local host passes, but the deployed cheap-VPS per-score
LB/upstream p99 is above 3 ms even at one client. This means concurrent
throughput is not closed. The next product speed debt is persistent/binary
upstream scoring or a route that sends traffic directly to the owning worker
shard. This is still not real Codex/API production traffic and not
OPERATOR_BLUEPRINT closure.
```

## Hard Architectural Rule

Do not build one giant runtime.

Build:

```text
profile registry
  -> route classifier / route key
  -> profile shard
  -> local operator score
  -> accept/fallback decision
  -> counters
```

Do not build:

```text
one daemon with all eval packs preloaded
one L3 monolith
one lookup table of answers
one hidden proof_rule_id router
one hand-coded target shortcut
```

## Serving-Only Worker Requirement

Product worker may load:

```text
.nwrb runtime packages
profile metadata
route metadata
thresholds
counters
```

Product worker must not load:

```text
.nwreb eval packs
JSONL corpora
training data
compiler state
Python scripts
heldout reports
```

Reason:

```text
Demo workers with eval packs used ~512 MiB RSS each.
Runtime packages alone are small enough for L2-oriented serving.
```

## Target Runtime Shape

### 1. Profile Registry

Add a registry layer:

```text
ProfileRegistry
  profile_id
  profile_kind
  operator_classes
  package_path
  runtime_bytes_estimate
  edge_count
  slot_count
  threshold
  accepted_route_keys
```

Expected initial profiles:

```text
role_binding_mixed_map_seed0
role_binding_mixed_map_seed1
role_binding_mixed_map_seed2
role_binding_conditional_branch_seed0
role_binding_conditional_branch_seed1
role_binding_conditional_branch_seed2
role_binding_edit_marker_length_seed0
```

This seed naming is allowed for the proof bundle, but product-facing naming
should eventually move to operator/profile names, not seed names.

### 2. Route Classifier

Initial router can be simple and explicit:

```text
route_key -> profile_id
```

Allowed:

```text
HTTP route
profile_id parameter
workflow kind
operator class
```

Forbidden as authority:

```text
target_id
proof_rule_id
concrete_x_lookup
heldout answer id
```

### 3. Profile Worker

Each worker should hold one or a few hot profiles.

Target:

```text
one worker per vCPU
one/few profiles per worker
profile hot data <= 1 MiB preferred
per-core profile set <= 2-3 MiB preferred
leave L2 room for request-local active fringe / slots
```

### 4. Local Score

For each request:

```text
prepare active fringe
score requested operator/profile
compute local margin
accept if margin >= threshold and strict checks pass
otherwise fallback
```

Response must include:

```text
accepted: bool
fallback: bool
profile_id
margin
threshold
latency_ns
forbidden_flags
```

### 5. Fallback

Fallback is not failure.

Fallback is correct behavior when:

```text
profile not found
margin below threshold
strict slot check fails
input outside profile contract
route confidence below threshold
```

## HTTP API Requirement

Build product-shaped endpoints:

```text
GET /health
GET /profiles
POST /score
POST /replay
GET /metrics
```

### GET /health

Return:

```json
{
  "status": "ok",
  "runtime": "nando-wave-profile-runtime",
  "compiler_used": false,
  "eval_packs_loaded": false,
  "corpus_jsonl_loaded": false
}
```

### GET /profiles

Return every loaded profile:

```json
{
  "profiles": [
    {
      "profile_id": "role_binding_mixed_map_seed0",
      "package_bytes": 17948,
      "runtime_bytes_estimate": 83448,
      "edge_count": 1492,
      "slot_count": 32,
      "threshold": 1000000
    }
  ]
}
```

### POST /score

Input must be compact binary or compact JSON first.

Minimum JSON shape for first implementation:

```json
{
  "request_id": "demo-001",
  "route_key": "sdk_mixed_map",
  "profile_id": "role_binding_mixed_map_seed0",
  "active_fringe": [
    {"center_id": 1, "strength": 100}
  ],
  "slots": [
    {
      "output_slot": 0,
      "positive": [{"lane_id": 10, "signed_strength": 1}],
      "negative": [{"lane_id": 11, "signed_strength": 1}]
    }
  ]
}
```

Output:

```json
{
  "request_id": "demo-001",
  "accepted": true,
  "fallback": false,
  "profile_id": "role_binding_mixed_map_seed0",
  "strict_ok": true,
  "margin": 2330624,
  "threshold": 1000000,
  "latency_ns": 700000,
  "forbidden_flags": {
    "target_id_used": false,
    "proof_rule_id_authority_used": false,
    "concrete_x_lookup_used": false,
    "local_out_t_used": false,
    "eval_pack_loaded": false
  }
}
```

### POST /replay

Purpose:

```text
Product-shaped replay, not research eval.
```

Input:

```json
{
  "request_id": "codex-edit-mini-v1",
  "requests": []
}
```

Output:

```json
{
  "request_id": "codex-edit-mini-v1",
  "no_cache_llm_calls": 1000,
  "exact_cache_llm_calls": 700,
  "exact_cache_plus_nando_llm_calls": 490,
  "exact_cache_incremental_reduction_milli": 300,
  "local_operator_calls": 210,
  "fallback_to_llm_calls": 490,
  "false_local_accepts": 0,
  "missed_expected_local": 0,
  "p99_latency_ns": 1000000
}
```

### GET /metrics

Return counters:

```text
requests_total
cache_hits_total
nando_accept_total
nando_fallback_total
false_local_accepts_total
profile_not_found_total
margin_reject_total
p50_latency_ns
p90_latency_ns
p99_latency_ns
rss_kib
runtime_bytes_estimate
```

## Product Line Requirements

### Line A: Role-Binding Offload Core

Purpose:

```text
Offload repeatable structured transformations after exact-cache miss.
```

Required operator classes:

```text
ORDER
MOVE_COPY
CONDITION_ROUTE
COMPOSE
VERIFY_REPAIR
EDIT
FIELD
FILTER_GROUP
```

Minimum package goal:

```text
30-50 operator families
32 slots
0 false accepts
exact-cache-plus-Nando replay reduction >= 20% target
```

### Line B: Codex/Edit Offload

Purpose:

```text
Handle repeated code-edit workflows:
rename, move, insert, delete, replace, normalize, verify.
```

Required profiles:

```text
rename_symbol
update_imports
move_block
replace_config_value
insert_guard
delete_dead_branch
normalize_json_yaml_toml
verify_patch_shape
```

Product replay must compare against exact cache.

### Line C: Structured Workflow Offload

Purpose:

```text
Handle business/process transitions:
state_t + action -> state_t+1
```

Required profiles:

```text
ticket_status_update
field_extract_normalize
route_by_condition
merge_split_fields
deduplicate_records
verify_constraints
repair_missing_field
```

## Pass/Fail Gates

### Gate 1: Serving Boundary

Pass only if:

```text
eval_packs_loaded: false
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
runtime_packages_loaded: true
```

### Gate 2: Profile Registry

Pass only if:

```text
GET /profiles returns all loaded profiles
each profile has package bytes, edge_count, runtime_bytes_estimate
profile total per worker is reported
```

### Gate 3: Score Endpoint

Pass only if:

```text
POST /score works with compact request payload
returns accept/fallback
returns margin and threshold
returns forbidden flags
p99 measured
```

### Gate 4: Exact Cache Competition

Pass only if:

```text
replay includes exact_cache_enabled=true
reports exact_cache_llm_calls
reports exact_cache_plus_nando_llm_calls
reports incremental reduction vs exact cache
```

### Gate 5: Safety

Pass only if:

```text
false_local_accepts = 0
fallback is used on uncertainty
bad route/profile returns fallback, not accept
```

### Gate 6: L2 Budget

Pass only if:

```text
runtime_bytes_estimate per profile <= 1 MiB preferred
runtime_bytes_estimate per worker <= 3 MiB preferred
RSS excludes eval packs
```

### Gate 7: Worker Scaling

Pass only if:

```text
1 worker baseline measured
N workers measured
throughput scales with vCPU count
no false accepts under concurrency
```

## Concrete Executor Task

Build:

```text
crates/nando-cli or a small nando-runtime binary:
  nando-profile-runtime
```

It must:

```text
1. Load `.nwrb` packages only.
2. Build an in-memory profile registry.
3. Serve HTTP locally on 127.0.0.1.
4. Expose /health, /profiles, /score, /replay, /metrics.
5. Never load `.nwreb` in serving mode.
6. Support one profile or many profiles.
7. Report runtime bytes, RSS, p50/p90/p99 latency.
8. Produce a product-proof JSON report.
```

First report name:

```text
target/nando-wave/product-runtime/profile-runtime-serving-v1.product-proof.json
```

First markdown report:

```text
data/rule_logic_operator_battery_v4/diagnostics/product_runtime/PROFILE_RUNTIME_SERVING_V1.md
```

Current markdown reports:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_RUNTIME_SMOKE.md
data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_REPLAY_SUITE.md
```

## Required Test Matrix

Run:

```text
profile_count: 1, 2, 6
workers: 1, 2
concurrency: 1, 2, 4, 8
exact_cache: enabled
bad_route_cases: included
low_margin_cases: included
```

Record:

```text
accepted
fallback
false_local_accepts
missed_expected_local
incremental reduction vs exact cache
p50/p90/p99 latency
RSS
runtime_bytes_estimate
worker count
profile count
```

## Current Known Server Boundary

The reserve VPS became unreachable after heavy eval-preloaded two-worker tests.

Do not repeat eval-heavy worker tests as product tests.

Recovery when server returns:

```bash
kill "$(cat /opt/nando-wave-proof-demo/logs/nando-role-http-2.pid)" 2>/dev/null || true
```

Then deploy only serving-only workers.

## Non-Goals

Do not do these in this task:

```text
do not add new model architecture
do not add local_out_t
do not use target_id
do not use proof_rule_id as authority
do not train during serving
do not preload eval packs in serving mode
do not claim real Codex traffic savings until real replay exists
```

## Done Definition

The first local runtime/replay slice is green when there is a report proving:

```text
serving-only runtime
profile registry
route -> profile shard
score -> accept/fallback
exact-cache comparison
0 false accepts
reported p99 latency
reported RSS/runtime bytes
worker scaling report
local load-balancer replay report
deployed cheap-VPS load-balancer replay report
```

Closed locally:

```text
serving-only runtime
profile registry
route -> profile shard
score -> accept/fallback
exact-cache comparison
bad-route fallback
low-margin fallback
0 false accepts
reported p99 latency
reported RSS/runtime bytes
worker scaling report
local load-balancer replay report
deployed cheap-VPS load-balancer replay report
```

Still open:

```text
real Codex traffic replay
concurrent throughput under production routing
```

Minimum acceptable result:

```text
incremental LLM-call reduction vs exact cache: >= 20%
false_local_accepts: 0
p99 score latency target: <= 1-3 ms on cheap VPS, <= 1 ms on local decent CPU
runtime_bytes_estimate per hot worker: <= 3 MiB preferred
```

If the reduction is lower than 20%, keep the report as WATCH and diagnose:

```text
wrong profile routing
too narrow operator coverage
too strict threshold
bad replay construction
cache already captures most repeats
```

## Short Command For Executor

```text
Stop building research-only gates.
Build the serving-only profile runtime.
No eval packs in serving.
Route to L2-sized profile shards.
Compete against exact cache.
Report accepts, fallbacks, false accepts, latency, RSS, and LLM-call reduction.
```
