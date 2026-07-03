# Slot32 Role-Binding Profile Load-Balancer Replay

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
```

What this closes:

```text
The current profile runtime can replay sampled release-suite traffic through a
local external load-balancer/proxy endpoint that dispatches to multiple
serving-only `.nwrb` profile workers. The replay client reads `.nwreb` only to
generate HTTP requests, then sends traffic to the load-balancer rather than to
worker shards directly.

The current transport path keeps per-row score semantics. A grouped LB -> worker
POST /replay experiment was tested and rejected because it made the measured
p99 worse and changed the interpretation of per-row latency.
```

Artifacts:

```text
source_registry_config: target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json
binary_suite_report: target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json
lb_config: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-v1.lb.json
lb_replay_report: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-v1.product-proof.json
worker_0_registry: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-shards-v1/worker-0.registry.json
worker_1_registry: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-shards-v1/worker-1.registry.json
```

Command:

```bash
cargo run --release -p nando-cli -- role-binding-profile-lb-replay-v1
```

Aggregate metrics:

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
total_worker_runtime_bytes_estimate: 969492
max_worker_runtime_bytes_estimate: 492792
total_worker_rss_bytes: 12390400
max_worker_rss_bytes: 6230016
max_worker_p99_latency_ns: 167663
all_workers_serving_only: true
load_balancer_serving_only: true
```

Worker rows:

```text
worker 0:
  profile_count: 4
  local_operator_calls: 256
  fallback_to_llm_calls: 256
  false_local_accepts: 0
  p99_latency_ns: 158014
  core_score_p99_latency_ns: 71442
  worker_score_p99_latency_ns: 158014
  lb_upstream_roundtrip_p99_latency_ns: 0
  runtime_bytes_estimate: 476700
  rss_bytes: 6160384

worker 1:
  profile_count: 3
  local_operator_calls: 192
  fallback_to_llm_calls: 192
  false_local_accepts: 0
  p99_latency_ns: 167663
  core_score_p99_latency_ns: 78902
  worker_score_p99_latency_ns: 167663
  lb_upstream_roundtrip_p99_latency_ns: 0
  runtime_bytes_estimate: 492792
  rss_bytes: 6230016
```

Layered latency interpretation:

```text
core_score_p99_latency_ns: 78902
worker_score_p99_latency_ns: 167663
load_balancer_score_envelope_p99_latency_ns: 736030
lb_upstream_roundtrip_p99_latency_ns: 735692
replay_client_wall_p99_latency_ns: 5489536
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
```

The current fat is not in the role-binding score loop. The local score loop is
below 0.08 ms p99 after the packed serving-only runtime split and compact worker
response, the worker score function is below 0.17 ms p99, and the millisecond-
class envelope is introduced above the core by the local HTTP/LB path and replay
batch client wall time.

Packed score update:

```text
previous_reference_core_score_p99_latency_ns: 88086
packed_serving_only_core_score_p99_latency_ns: 78902
core_p99_delta_ns: -9184
core_p99_reduction_milli: 104
packed_score_parity_mismatches: 0
```

Serving-only split update:

```text
previous_packed_dual_max_worker_runtime_bytes_estimate: 891248
packed_serving_only_max_worker_runtime_bytes_estimate: 492792
runtime_bytes_delta: -398456
runtime_bytes_reduction_milli: 447
```

Serving-only boundary:

```text
workers load: .nwrb runtime packages only
load-balancer loads: route-to-upstream metadata only
eval_packs_loaded: false
corpus_jsonl_loaded: false
compiler_used: false
python_demo_used: false
eval_packs_used_by_replay_client: true
```

Boundary:

```text
This is a local external load-balancer replay over serving-only `.nwrb`
profile workers. It proves route-to-upstream dispatch through a separate proxy
endpoint, exact-cache competition across shards, RSS/runtime-byte reporting,
proxy p50/p90/p99 reporting, and zero false local accepts.

It is not real Codex/API traffic, not a concurrent throughput proof, not
cheap-VPS deployment, and not full OPERATOR_BLUEPRINT coverage.
```
