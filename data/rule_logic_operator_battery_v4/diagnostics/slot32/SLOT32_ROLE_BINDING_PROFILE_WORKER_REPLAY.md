# Slot32 Role-Binding Profile Worker Replay

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS
```

What this closes:

```text
The current profile runtime can replay sampled release-suite traffic through
multiple serving-only `.nwrb` profile workers. The replay client reads `.nwreb`
only to generate HTTP requests, then routes each profile request to the worker
that owns that profile shard.
```

Artifacts:

```text
source_registry_config: target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json
binary_suite_report: target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json
worker_replay_report: target/nando-wave/role-binding-profile-runtime/profile-worker-replay-v1.product-proof.json
worker_0_registry: target/nando-wave/role-binding-profile-runtime/profile-worker-replay-shards-v1/worker-0.registry.json
worker_1_registry: target/nando-wave/role-binding-profile-runtime/profile-worker-replay-shards-v1/worker-1.registry.json
```

Command:

```bash
cargo run --release -p nando-cli -- role-binding-profile-worker-replay-v1
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
total_runtime_bytes_estimate: 790020
max_worker_runtime_bytes_estimate: 398456
total_rss_bytes: 14221312
max_worker_rss_bytes: 7135232
max_worker_p99_latency_ns: 265277
all_workers_serving_only: true
```

Worker rows:

```text
worker 0:
  profile_count: 4
  unique_sequences_replayed: 512
  exact_cache_llm_calls: 512
  exact_cache_plus_nando_llm_calls: 256
  false_local_accepts: 0
  p99_latency_ns: 265277
  runtime_bytes_estimate: 391564
  rss_bytes: 7086080

worker 1:
  profile_count: 3
  unique_sequences_replayed: 384
  exact_cache_llm_calls: 384
  exact_cache_plus_nando_llm_calls: 192
  false_local_accepts: 0
  p99_latency_ns: 227158
  runtime_bytes_estimate: 398456
  rss_bytes: 7135232
```

Serving-only boundary:

```text
workers load: .nwrb runtime packages only
eval_packs_loaded: false
corpus_jsonl_loaded: false
compiler_used: false
python_demo_used: false
eval_packs_used_by_replay_client: true
```

Boundary:

```text
This is a local sharded HTTP replay over serving-only `.nwrb` profile workers.
It proves route-to-shard replay, exact-cache competition across shards,
RSS/runtime-byte reporting, and zero false local accepts.

It is not real Codex production traffic, not an external load-balancer proof,
not a concurrent throughput proof, not cheap-VPS deployment, and not full
OPERATOR_BLUEPRINT coverage.
```
