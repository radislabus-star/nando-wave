# Slot32 Role-Binding Profile Deployed HostWorld Replay

Date: 2026-07-03

Current verdict after packed hot-path deployment:

```text
ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
```

What this closes:

```text
The current profile runtime was copied to the cheap VPS host `hostworld-ee` and
replayed there through the local load-balancer/proxy endpoint over two
serving-only `.nwrb` worker shards.

The first remote attempt exposed a packaging issue: the local glibc binary
required GLIBC_2.43. The deployed proof therefore uses the
x86_64-unknown-linux-musl static `nando-cli` binary.

Historical note: the previous non-packed deployed static-binary bundle passed
the cheap-VPS target once with load_balancer_p99_latency_ns: 2834992. After the
packed edge-group runtime was deployed, the first packed-dual clean sequential
HostWorld run was WATCH/FAIL at 3090366 ns p99. The current serving packed-only
split lowered runtime bytes but initially remained WATCH/FAIL on the 3 ms
latency gate at load_balancer_p99_latency_ns: 3612312. Transport cleanup then
enabled TCP_NODELAY, stopped HTTP response reads at Content-Length, and switched
LB -> worker traffic to the compact internal POST /score-compact response. After
that cleanup, two clean HostWorld runs passed:
  profile-lb-replay-hostworld-v1-clean.product-proof.json p99 = 2993688 ns;
  profile-lb-replay-hostworld-v1-clean2.product-proof.json p99 = 2744444 ns.
Safety, packed-score parity, and exact-cache reduction remain green.
```

Remote host facts:

```text
host_alias: hostworld-ee
hostname: tt-ee-test
cpu: 2 vCPU, Intel Core Processor (Haswell, no TSX, IBRS)
L1d: 64 KiB (2 instances)
L2: 8 MiB (2 instances)
L3: 32 MiB (2 instances)
memory: 1.9 GiB
deploy_root: /opt/nando-wave-profile-runtime-v1
bundle_size: 360M
```

Artifacts copied back:

```text
remote_report_clean: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1-clean.product-proof.json
remote_report_clean2: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1-clean2.product-proof.json
remote_report_latest_default: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1.product-proof.json
remote_lb_config: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1.lb.json
```

Remote command:

```bash
ssh hostworld-ee 'cd /opt/nando-wave-profile-runtime-v1 && ./target/release/nando-cli role-binding-profile-lb-replay-v1 target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1-clean2.product-proof.json 2 128 4'
```

Remote aggregate metrics:

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
estimated_lb_overhead_p99_ns: 2046299
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
load_balancer_rss_bytes: 2838528
total_worker_runtime_bytes_estimate: 969492
max_worker_runtime_bytes_estimate: 492792
total_worker_rss_bytes: 7000064
max_worker_rss_bytes: 3510272
max_worker_p99_latency_ns: 698145
all_workers_serving_only: true
load_balancer_serving_only: true
```

Second clean run:

```text
profile-lb-replay-hostworld-v1-clean.product-proof.json:
  verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
  load_balancer_p99_latency_ns: 2993688
  core_score_p99_latency_ns: 187721
  worker_score_p99_latency_ns: 545095
  lb_upstream_roundtrip_p99_latency_ns: 2993349
  packed_score_parity_mismatches: 0
  false_local_accepts: 0
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
This closes the deployed cheap-VPS replay target for the current packed runtime
on sampled release-suite traffic: two clean HostWorld runs are inside the 3 ms
p99 envelope, with safety, package parity, and exact-cache reduction green.
Serving packed-only removed the reference table and reference HashMap from
workers; the remaining latency fat is still in the remote HTTP/LB/virtualization
path rather than in a score-parity, false-accept, or runtime-byte failure.

It is still not real Codex/API production traffic, not a concurrent throughput
benchmark, and not full OPERATOR_BLUEPRINT coverage.
```
