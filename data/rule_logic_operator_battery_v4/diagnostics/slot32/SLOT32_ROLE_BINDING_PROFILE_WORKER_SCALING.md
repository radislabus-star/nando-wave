# Slot32 Role-Binding Profile Worker Scaling

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_PROFILE_WORKER_SCALING_V1_PASS
```

What this closes:

```text
The serving-only `.nwrb` profile runtime can be split into multiple local
profile-shard workers. Each worker loads only its own `.nwrb` packages, accepts
valid high-margin local requests for its profiles, and rejects routes belonging
to a different worker by falling back instead of locally accepting.
```

Artifacts:

```text
source_registry_config: target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json
worker_scaling_report: target/nando-wave/role-binding-profile-runtime/profile-worker-scaling-v1.product-proof.json
worker_0_registry: target/nando-wave/role-binding-profile-runtime/profile-worker-shards-v1/worker-0.registry.json
worker_1_registry: target/nando-wave/role-binding-profile-runtime/profile-worker-shards-v1/worker-1.registry.json
```

Command:

```bash
cargo run --release -p nando-cli -- role-binding-profile-worker-scaling-v1
```

Metrics:

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

Worker shards:

```text
worker 0:
  profile_count: 4
  runtime_bytes_estimate: 391564
  rss_bytes: 6500352
  p99_latency_ns: 3975
  profiles:
    role_binding_mixed_map_seed0
    role_binding_mixed_map_seed2
    role_binding_conditional_branch_seed1
    role_binding_edit_marker_length_seed0

worker 1:
  profile_count: 3
  runtime_bytes_estimate: 398456
  rss_bytes: 6557696
  p99_latency_ns: 6286
  profiles:
    role_binding_mixed_map_seed1
    role_binding_conditional_branch_seed0
    role_binding_conditional_branch_seed2
```

Serving-only boundary:

```text
worker loads: .nwrb runtime packages only
eval_packs_loaded: false
corpus_jsonl_loaded: false
compiler_used: false
python_demo_used: false
```

Boundary:

```text
This is a local product acceptance gate for profile-shard workers. It proves
multi-worker shard loading, per-worker local accept, wrong-worker route
fallback, RSS/runtime-byte reporting, and zero false local accepts.

It is not real Codex production traffic, not an external load-balancer proof,
not a throughput scaling proof, not cheap-VPS deployment, and not full
OPERATOR_BLUEPRINT coverage.
```
