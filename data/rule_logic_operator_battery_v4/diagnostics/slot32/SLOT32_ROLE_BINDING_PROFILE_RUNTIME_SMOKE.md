# Slot32 Role-Binding Profile Runtime Smoke

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_PROFILE_REGISTRY_FROM_RELEASE_V1_PASS
ROLE_BINDING_PROFILE_RUNTIME_SMOKE_V1_PASS
```

What this closes:

```text
The current green role-binding release-suite proof is frozen into a
serving-only profile registry. The serving worker loads only `.nwrb` runtime
packages, exposes product-shaped HTTP endpoints, and reports exact-cache versus
exact-cache-plus-Nando metrics.
```

Artifacts:

```text
registry_config: target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json
runtime_smoke_report: target/nando-wave/role-binding-profile-runtime/profile-runtime-smoke-v1.product-proof.json
release_suite_report: target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json
```

Commands:

```bash
cargo run -p nando-cli -- role-binding-profile-registry-from-release-v1 \
  target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json \
  target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json

cargo run -p nando-cli -- role-binding-profile-runtime-smoke-v1 \
  target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json \
  target/nando-wave/role-binding-profile-runtime/profile-runtime-smoke-v1.product-proof.json
```

Serving endpoints:

```text
GET /health
GET /profiles
POST /score
POST /replay
GET /metrics
```

Serving-only boundary:

```text
loaded runtime packages: .nwrb only
loaded eval packs: false
loaded corpora/jsonl: false
compiler/training in serving path: false
python demo used: false
```

Registry:

```text
profile_count: 7
profile_ids:
  role_binding_mixed_map_seed0
  role_binding_mixed_map_seed1
  role_binding_mixed_map_seed2
  role_binding_conditional_branch_seed0
  role_binding_conditional_branch_seed1
  role_binding_conditional_branch_seed2
  role_binding_edit_marker_length_seed0
runtime_bytes_estimate: 790020
package_bytes: 134912
edge_count: 11217
```

Smoke metrics:

```text
exact_cache_llm_calls: 2
exact_cache_plus_nando_llm_calls: 1
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
p99_latency_ns: 37468
rss_bytes: 10932224
runtime_bytes_estimate: 790020
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
compiler_used: false
eval_packs_loaded: false
corpus_jsonl_loaded: false
python_demo_used: false
```

Boundary:

```text
This is the first product-shaped serving runtime smoke for current `.nwrb`
role-binding profile shards. It proves registry loading, route/profile scoring,
fallback accounting, replay cache metrics, latency/RSS reporting, and clean
hot-path provenance.

It is not real Codex production traffic, not a full OPERATOR_BLUEPRINT close,
not `.nwpc` bridge proof, not raw-language action parsing, and not a
commercial deployment package.
```
