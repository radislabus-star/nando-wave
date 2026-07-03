# Slot32 Mixed/Conditional Cache-Offload Benchmark

Date: 2026-07-03

Verdict:

```text
SLOT32_MIXED_CONDITIONAL_CACHE_OFFLOAD_BENCH_PASS
```

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_mixed_conditional_cache_offload_benchmark_must_stay_local_without_false_accepts --nocapture
```

Log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_mixed_conditional_cache_offload_bench_release.log
```

Runtime:

```text
finished in 281.40s
```

Structural claim-boundary check:

```text
nanda-gate-md /tmp/nanda-task-slot32-cache-offload-status.md --task-id slot32-cache-offload-status --domain code
verdict: PASS
complexity_score: 23
agent_action: SAFE_TO_EDIT
trace_path: /tmp/nanda-structural-gate/slot32-cache-offload-status.trace.json
```

Scope:

```text
32 output slots
32 role slots
lengths 17..32
mixed-map + conditional-branch rows
seeds = 3
simulated_repeats = 3
total_unique_rows = 12288
total_simulated_calls = 36864
local_margin_threshold = 1000000
p99_latency_gate_ns = 1000000
```

Per-seed results:

```text
mixed seed=0 local_accuracy=1000 false_local_accepts=0 local_calls=6144 fallback_calls=0 exact_cache_llm_calls=2048 nando_llm_calls=0 removed_vs_cache=2048 reduction_milli=1000 p99_ns=485839 min_margin=2330624 hot_bytes=607736 role_edges=1492
conditional seed=0 local_accuracy=1000 false_local_accepts=0 local_calls=6144 fallback_calls=0 exact_cache_llm_calls=2048 nando_llm_calls=0 removed_vs_cache=2048 reduction_milli=1000 p99_ns=569299 min_margin=2354176 hot_bytes=681792 role_edges=2202
mixed seed=1 local_accuracy=1000 false_local_accepts=0 local_calls=6144 fallback_calls=0 exact_cache_llm_calls=2048 nando_llm_calls=0 removed_vs_cache=2048 reduction_milli=1000 p99_ns=512918 min_margin=2411776 hot_bytes=607736 role_edges=1492
conditional seed=1 local_accuracy=1000 false_local_accepts=0 local_calls=6144 fallback_calls=0 exact_cache_llm_calls=2048 nando_llm_calls=0 removed_vs_cache=2048 reduction_milli=1000 p99_ns=581582 min_margin=2449664 hot_bytes=681792 role_edges=2202
mixed seed=2 local_accuracy=1000 false_local_accepts=0 local_calls=6144 fallback_calls=0 exact_cache_llm_calls=2048 nando_llm_calls=0 removed_vs_cache=2048 reduction_milli=1000 p99_ns=528044 min_margin=2352640 hot_bytes=607736 role_edges=1492
conditional seed=2 local_accuracy=1000 false_local_accepts=0 local_calls=6144 fallback_calls=0 exact_cache_llm_calls=2048 nando_llm_calls=0 removed_vs_cache=2048 reduction_milli=1000 p99_ns=611686 min_margin=2380544 hot_bytes=681792 role_edges=2202
```

Aggregate:

```text
total_no_cache_llm_calls: 36864
total_exact_cache_llm_calls: 12288
total_exact_cache_hits: 24576
total_exact_cache_plus_nando_llm_calls: 0
total_exact_cache_plus_nando_cache_hits: 0
total_local_operator_calls: 36864
total_fallback_to_llm_calls: 0
total_false_local_accepts: 0
total_incremental_llm_calls_removed_vs_cache: 12288
total_incremental_llm_call_reduction_vs_cache_milli: 1000
min_local_accuracy_milli: 1000
min_offload_rate_milli: 1000
min_energy_margin: 2330624
max_p99_latency_ns: 611686
max_hot_bytes_estimate: 681792
max_role_binding_edges: 2202
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Interpretation:

```text
This proves a 32-slot flat role-binding local-operator offload benchmark for
the current mixed-map plus conditional-branch Rust-generated symbolic task
family. Exact cache alone would still need one LLM call per unique row. The
local Nando scorer accepts all simulated calls with false_local_accepts = 0.
```

Boundary:

```text
This is not a serialized .nwpc package proof.
It does not close raw-language action parsing, autonomous action_tree induction,
insert-new-constant edit operators, packed product runtime proof, product p99,
64-slot capacity, broad workflow reasoning, or text generation.
```
