# Slot32 Role-Binding Profile Replay Suite

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_PROFILE_REPLAY_SUITE_V1_PASS
```

What this closes:

```text
The current `.nwrb` profile serving runtime was replayed through real HTTP
`/replay` requests against the current role-binding release-suite profiles.
The serving worker loaded only `.nwrb` runtime packages. The `.nwreb` eval
packs were used only by the external replay client to generate requests.
```

Artifacts:

```text
registry_config: target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json
binary_suite_report: target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json
default_release_replay_report: target/nando-wave/role-binding-profile-runtime/profile-replay-suite-v1.product-proof.json
sample_release_replay_report: target/nando-wave/role-binding-profile-runtime/profile-replay-suite-v1.release.product-proof.json
```

Command:

```bash
cargo run --release -p nando-cli -- role-binding-profile-replay-suite-v1
```

Release replay metrics:

```text
profile_count: 7
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
```

Serving-only boundary:

```text
serving worker loads: .nwrb runtime packages only
eval_packs_loaded_in_serving_worker: false
corpus_jsonl_loaded_in_serving_worker: false
compiler_used: false
python_demo_used: false
eval_packs_used_by_replay_client: true
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

HTTP body finding and fix:

```text
old default batch_unique_sequences: 32 returned HTTP 413 request body too large.
old default max_unique_sequences_per_profile: 256 produced a p99 outlier above the 3ms gate.
new default max_unique_sequences_per_profile: 128 passed.
new default batch_unique_sequences: 4 passed.
```

This is a product-serving finding, not a model failure. The profile runtime
should either keep replay batches small or introduce an explicit, audited body
limit change later.

Boundary:

```text
This is a local HTTP replay-suite over the serving-only `.nwrb` role-binding
profile runtime. It proves product-cache competition on sampled release-suite
traffic, not real Codex production traffic, not broad raw-language action
parsing, not full OPERATOR_BLUEPRINT coverage, and not cheap-VPS deployment.

OPERATOR_BLUEPRINT remains WATCH:
  partial_classes: 7
  missing_classes: 2
  missing: FIELD, FILTER_GROUP
```
