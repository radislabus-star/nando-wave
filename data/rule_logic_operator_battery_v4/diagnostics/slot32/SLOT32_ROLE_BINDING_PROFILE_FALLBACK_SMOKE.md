# Slot32 Role-Binding Profile Fallback Smoke

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_PASS
```

What this closes:

```text
The serving-only `.nwrb` profile runtime now has a direct product fallback
guard. It verifies:

1. a valid high-margin request is accepted locally;
2. an unknown route falls back to the LLM;
3. a valid route with too-low margin falls back to the LLM;
4. false_local_accepts remains 0.
```

Command:

```bash
cargo run -p nando-cli -- role-binding-profile-fallback-smoke-v1
```

Artifact:

```text
target/nando-wave/role-binding-profile-runtime/profile-fallback-smoke-v1.product-proof.json
```

Current metrics:

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
p50_latency_ns: 23689
p90_latency_ns: 24312
p99_latency_ns: 24312
runtime_bytes_estimate: 790020
rss_bytes: 11104256
```

Serving-only boundary:

```text
compiler_used: false
eval_packs_loaded: false
corpus_jsonl_loaded: false
python_demo_used: false
```

Claim boundary:

```text
This proves local accept, missing-route fallback, and low-margin fallback over
the serving-only `.nwrb` role-binding profile runtime.

This historical fallback-only boundary was later superseded by the worker
scaling and worker replay gates. It still does not prove real Codex traffic,
external load-balancer routing, cheap-VPS deployment, real route
classification, or full OPERATOR_BLUEPRINT closure.
```

Next product target:

```text
real Codex/API traffic replay, if a trace exists
otherwise external load-balancer / cheap-VPS deployment proof
keep exact-cache baseline and false_local_accepts = 0
```
