# Nando Wave L2 Profile Sharding Tests

Date: 2026-07-03

Purpose:

```text
Measure whether Nando Wave operator profiles should be rebuilt as separated
L2-resident instruction/profile shards instead of one larger monolithic runtime.
```

## Current Server Topology

Measured on the reserve iHosting Estonia VPS before the network outage:

```text
CPU: Intel Xeon E5-2699 v4 virtual CPU
vCPU: 2
L1d: 64 KiB total, 2 instances
L2: 8 MiB total, 2 instances, effectively ~4 MiB per vCPU
L3: 16 MiB shared
```

## Current Runtime Package Inventory

Local 32-slot artifacts:

```text
sdk_mixed_map seed 0..2:
  package_file_bytes: 17,948 each
  runtime_bytes_estimate: 83,448 each
  edge_count: 1,492 each

sdk_conditional_branch seed 0..2:
  package_file_bytes: 26,468 each
  runtime_bytes_estimate: 157,504 each
  edge_count: 2,202 each

all 6 runtime profiles:
  package_file_bytes: 133,248
  runtime_bytes_estimate: 722,856
  edge_count: 11,082
```

Interpretation:

```text
Runtime/operator data is small.
All 6 current profiles fit well under a 4 MiB L2 budget.
The large memory in the demo came from eval packs, not from the operator runtime.
```

## Eval Pack Boundary

Eval packs are not product serving data.

Current `.nwreb` files are roughly:

```text
sdk_mixed_map eval packs: ~59 MiB each
sdk_conditional_branch eval packs: ~60-61 MiB each
```

The demo HTTP daemon that preloaded eval packs used about:

```text
RSS per demo worker: ~512 MiB
```

This must not be copied into product architecture.

Product serving workers should load:

```text
runtime packages / operator profiles
request-local active fringe / slots
counters / route metadata
```

They should not preload:

```text
heldout eval packs
JSONL corpora
training data
compiler state
```

Current serving-only profile replay:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_REPLAY_SUITE.md
release_replay_report: target/nando-wave/role-binding-profile-runtime/profile-replay-suite-v1.product-proof.json
profile_count: 7
runtime_bytes_estimate: 790020
rss_bytes: 8101888
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
incremental_reduction_vs_exact_cache: 500 milli
false_local_accepts: 0
p99_latency_ns: 213509
serving_worker_loads: .nwrb only
eval_packs_used_by_replay_client: true
eval_packs_loaded_in_serving_worker: false
```

Interpretation:

```text
The serving-only profile worker keeps the hot runtime under the preferred
3 MiB per-worker estimate and beats the exact-cache baseline on sampled
release-suite traffic. This historical single-worker replay is superseded by
the local load-balancer and deployed cheap-VPS replay gates below.
```

Current serving-only worker-shard scaling:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_WORKER_SCALING.md
product_report: target/nando-wave/role-binding-profile-runtime/profile-worker-scaling-v1.product-proof.json
worker_count: 2
profile_split: 4 / 3
max_worker_runtime_bytes_estimate: 398456
max_worker_rss_bytes: 6557696
max_worker_p99_latency_ns: 6286
false_local_accepts: 0
wrong_worker_route_fallbacks: 2
serving_worker_loads: .nwrb only
```

Interpretation:

```text
The profile set now splits cleanly into multiple L2-sized serving workers. This
closes local shard acceptance, not real external load balancing or cheap-VPS
deployment.
```

Current sharded worker replay:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_WORKER_REPLAY.md
product_report: target/nando-wave/role-binding-profile-runtime/profile-worker-replay-v1.product-proof.json
worker_count: 2
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
incremental_reduction_vs_exact_cache: 500 milli
false_local_accepts: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_rss_bytes: 7135232
max_worker_p99_latency_ns: 265277
serving_worker_loads: .nwrb only
eval_packs_used_by_replay_client: true
```

Interpretation:

```text
The release-suite replay now runs through multiple local profile shards rather
than a single serving worker. This historical direct-shard replay is
superseded by the local load-balancer and deployed cheap-VPS replay gates
below.
```

Current local load-balancer replay:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_LB_REPLAY.md
product_report: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-v1.product-proof.json
lb_config: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-v1.lb.json
worker_count: 2
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
incremental_reduction_vs_exact_cache: 500 milli
false_local_accepts: 0
load_balancer_p99_latency_ns: 736030
core_score_p99_latency_ns: 78902
worker_score_p99_latency_ns: 167663
lb_upstream_roundtrip_p99_latency_ns: 735692
replay_client_wall_p99_latency_ns: 5489536
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
max_worker_runtime_bytes_estimate: 492792
max_worker_p99_latency_ns: 167663
serving_workers_load: .nwrb only
load_balancer_loads: route-to-upstream metadata only
eval_packs_used_by_replay_client: true
```

Interpretation:

```text
The replay now enters through a separate local proxy/load-balancer endpoint
instead of being routed directly by the replay client to worker shards. This
closes the local load-balancer proof shape. The layered breakdown shows the
operator score loop is below 0.09 ms p99 after the packed serving-only runtime
split and compact worker response; the remaining envelope is local
HTTP/LB/replay-client path cost, not L2 profile miss cost.
```

Current deployed cheap-VPS load-balancer replay:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_DEPLOYED_HOSTWORLD_REPLAY.md
product_report: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1.product-proof.json
remote_host_alias: hostworld-ee
remote_root: /opt/nando-wave-profile-runtime-v1
binary: x86_64-unknown-linux-musl static nando-cli
worker_count: 2
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
incremental_reduction_vs_exact_cache: 500 milli
false_local_accepts: 0
verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
load_balancer_p99_latency_ns: 2744444
core_score_p99_latency_ns: 184328
worker_score_p99_latency_ns: 698145
lb_upstream_roundtrip_p99_latency_ns: 2743851
replay_client_wall_p99_latency_ns: 19478822
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
max_worker_runtime_bytes_estimate: 492792
max_worker_p99_latency_ns: 698145
serving_workers_load: .nwrb only
load_balancer_loads: route-to-upstream metadata only
eval_packs_used_by_replay_client: true
```

Interpretation:

```text
The current packed hot-path sampled release-suite profile runtime is inside
the cheap-VPS 3 ms p99 target through the load-balancer/proxy path after
transport cleanup and compact LB -> worker responses. Safety, exact-cache
reduction, and packed-vs-reference parity are green; the remaining open proof
is real Codex/API traffic and concurrent production-style throughput.
```

Current bounded individual-score throughput:

```text
local:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_PASS
  client_threads: 4
  score_requests: 896
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 743295
  worker_score_p99_latency_ns: 169460
  core_score_p99_latency_ns: 72666

hostworld:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  client_threads: 4
  score_requests: 896
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 3611864
  lb_upstream_roundtrip_p99_latency_ns: 3610931
  worker_score_p99_latency_ns: 577626
  core_score_p99_latency_ns: 221243
```

Interpretation:

```text
The replay-batch HostWorld proof is green, but individual POST /score pressure
is red on the cheap VPS. The L2/profile score loop is not the failing layer:
the red metric tracks the per-score LB/upstream HTTP envelope.
```

## Local L2 Probe

Temporary probe:

```text
/tmp/nando-l2-profile-probe
```

Commands:

```bash
cargo build --release --target x86_64-unknown-linux-musl

/tmp/nando-l2-profile-probe/target/x86_64-unknown-linux-musl/release/nando-l2-profile-probe \
  /home/ubu/projects/nando-wave/target/nando-wave/slot32-role-binding \
  4096 3 1000000 1

/tmp/nando-l2-profile-probe/target/x86_64-unknown-linux-musl/release/nando-l2-profile-probe \
  /home/ubu/projects/nando-wave/target/nando-wave/slot32-role-binding \
  4096 3 1000000 6
```

Saved local reports:

```text
/tmp/nando-l2-profile-probe/local-l2-one-profile.json
/tmp/nando-l2-profile-probe/local-l2-six-profiles.json
/tmp/nando-l2-profile-probe/local-l2-one-profile-r1.json
/tmp/nando-l2-profile-probe/local-l2-six-profiles-r1.json
/tmp/nando-l2-profile-probe/local-l2-one-profile-r2.json
/tmp/nando-l2-profile-probe/local-l2-six-profiles-r2.json
/tmp/nando-l2-profile-probe/local-l2-one-profile-r3.json
/tmp/nando-l2-profile-probe/local-l2-six-profiles-r3.json
```

## Local Probe Results

One profile loaded:

```text
runtime_profiles: 1
package_file_bytes: 17,948
runtime_bytes_estimate: 83,448
edge_count: 1,492
runtime-only RSS after load: ~760 KiB
```

Six profiles loaded:

```text
runtime_profiles: 6
package_file_bytes: 133,248
runtime_bytes_estimate: 722,856
edge_count: 11,082
runtime-only RSS after load: ~2,136 KiB
```

Representative scoring probe, `sdk_mixed_map/seed0`, 4096 sequences:

```text
one profile:
  strict_ordered_accuracy_milli: 1000
  false_local_accepts: 0
  missed_expected_local: 0
  p99_latency_ns: ~482,710 to ~659,462

six profiles:
  strict_ordered_accuracy_milli: 1000
  false_local_accepts: 0
  missed_expected_local: 0
  p99_latency_ns: ~646,611 to ~856,174 typical
  observed noisy outlier: 2,356,140 ns
```

Interpretation:

```text
Multiple profiles still fit in L2 by size.
But p99 is cleaner when the hot worker has fewer active profiles.
For product latency, prefer route -> dedicated profile shard over one growing monolith.
```

## Server HTTP Tests Already Run

Single nginx-backed worker:

```text
route: /nando-wave/
total_sequences: 24,576
verdict: PASS
false_local_accepts: 0
missed_expected_local: 0
strict_ordered_accuracy_milli: 1000
max_p99_latency_ns: ~0.70 ms
throughput at concurrency 1..8: ~0.8 req/s
```

Two local workers, nginx round-robin:

```text
route: /nando-wave-rr/
workers:
  127.0.0.1:18081
  127.0.0.1:18082

concurrency 1: ~0.79 req/s
concurrency 2: ~1.59 req/s
concurrency 4: ~1.22 req/s
concurrency 8: ~1.30 req/s
all_pass: true
false_local_accepts: 0
missed_expected_local: 0
```

Interpretation:

```text
One daemon is effectively serial.
Two daemons on two vCPU almost doubled throughput.
The next product server should use a worker pool / per-core shard model.
```

Server logs created before outage:

```text
/opt/nando-wave-proof-demo/logs/nginx-preload-suite-20260703T021124Z.json
/opt/nando-wave-proof-demo/logs/nginx-concurrency-probe-20260703T021539Z.json
/opt/nando-wave-proof-demo/logs/two-daemon-round-robin-probe-20260703T021653Z.json
/opt/nando-wave-proof-demo/logs/nginx-two-worker-round-robin-probe-20260703T021904Z.json
```

## Server Outage Boundary

After the heavy two-worker demo, the reserve VPS stopped responding to:

```text
ping 38.180.45.212
TCP 22
TCP 443
```

This is likely caused by the cheap VPS being overloaded by two eval-heavy demo
workers, or by provider-side network instability. The important boundary:

```text
The current demo workers are not product workers.
They preload eval packs and consume ~512 MiB RSS each.
Do not use that memory shape as the product architecture.
```

When the server returns, first recovery action should be:

```bash
kill "$(cat /opt/nando-wave-proof-demo/logs/nando-role-http-2.pid)" 2>/dev/null || true
```

Then replace eval-heavy workers with serving-only profile workers.

## Architecture Decision From These Tests

The next runtime architecture should be:

```text
request
  -> route classifier
  -> profile_id
  -> one small hot profile shard
  -> local operator score
  -> accept if margin high
  -> fallback to LLM if margin low
```

Not:

```text
one huge L3 runtime
one daemon with every profile hot
demo worker that preloads eval data
```

## L2-Oriented Serving Target

Initial target per profile shard:

```text
hot runtime estimate: <= 512 KiB
hard profile budget: <= 1 MiB
per-core active profile budget: <= 2-3 MiB
reserve in 4 MiB L2: keep at least ~1 MiB free for request-local data
```

For the current 32-slot operator profiles:

```text
mixed_map profile: ~83 KiB runtime estimate
conditional_branch profile: ~158 KiB runtime estimate
six current profiles together: ~723 KiB runtime estimate
```

This is good enough to continue with separated profile shards.

## Required Next Tests

1. Serving-only daemon:

```text
preload .nwrb packages only
do not preload .nwreb eval packs
accept compact request payload
return accept/fallback/margin/latency
```

2. Profile-shard routing:

```text
one worker per vCPU
one or few profiles per worker
nginx upstream or internal router
```

3. Product replay:

```text
exact cache enabled
route cache miss to Nando
measure incremental LLM-call reduction
track false accepts
```

4. Cache stress:

```text
1 profile
4 profiles
8 profiles
16 profiles
measure p50/p90/p99 and RSS
```

5. Hardware counters if allowed:

```text
cache-references
cache-misses
cycles
instructions
```

Current local kernel blocks `perf` counters:

```text
perf_event_paranoid = 4
```

Do not change this silently.

## Current Verdict

```text
L2/profile-shard architecture is justified.

The operator packages are small enough.
The current product risk is not operator size.
The risk is worker shape:
  eval-heavy demo workers,
  serial request handling,
  too many profiles per hot process,
  no route-level profile isolation yet.

Next rebuild should target serving-only, per-core, separated profile workers.
```
