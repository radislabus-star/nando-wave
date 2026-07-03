# Slot32 Role-Binding Profile Load-Balancer Throughput

Date: 2026-07-03

Verdict:

```text
LOCAL_ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_PASS
HOSTWORLD_ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
```

What this tests:

```text
Bounded concurrent POST /score pressure over the role-binding profile
load-balancer and serving-only `.nwrb` worker shards.

This is not the same as the green LB replay gate. Replay uses POST /replay
batches to generate sampled traffic. Throughput uses individual POST /score
requests, which is closer to a product serving endpoint.
```

Command:

```bash
cargo run --release -p nando-cli -- role-binding-profile-lb-throughput-v1
```

Local result:

```text
report: target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-v1.product-proof.json
verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_PASS
worker_count: 2
client_threads: 4
sequence_repetitions: 1
unique_sequences_replayed: 896
score_requests: 896
local_operator_calls: 448
fallback_to_llm_calls: 448
false_local_accepts: 0
missed_expected_local: 0
unexpected_local_accepts: 0
client_errors: 0
throughput_requests_per_second_milli: 1510939
client_p99_latency_ns: 3633433
load_balancer_p99_latency_ns: 743295
core_score_p99_latency_ns: 72666
worker_score_p99_latency_ns: 169460
lb_upstream_roundtrip_p99_latency_ns: 742942
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
max_worker_runtime_bytes_estimate: 492792
max_worker_p99_latency_ns: 169460
```

HostWorld capacity curve:

```text
hostworld 1 client:
  report: target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-hostworld-1c-v1.product-proof.json
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  score_requests: 896
  throughput_requests_per_second_milli: 323496
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 3507669
  lb_upstream_roundtrip_p99_latency_ns: 3507021
  worker_score_p99_latency_ns: 643303
  core_score_p99_latency_ns: 163584
  client_p99_latency_ns: 6214482

hostworld 2 clients:
  report: target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-hostworld-2c-v1.product-proof.json
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  score_requests: 896
  throughput_requests_per_second_milli: 436463
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 3250420
  lb_upstream_roundtrip_p99_latency_ns: 3249782
  worker_score_p99_latency_ns: 579524
  core_score_p99_latency_ns: 164370
  client_p99_latency_ns: 8152567

hostworld 4 clients:
  report: target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-hostworld-v1.product-proof.json
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  score_requests: 896
  throughput_requests_per_second_milli: 434243
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 3611864
  lb_upstream_roundtrip_p99_latency_ns: 3610931
  worker_score_p99_latency_ns: 577626
  core_score_p99_latency_ns: 221243
  client_p99_latency_ns: 17626188
```

Interpretation:

```text
The bounded concurrent command is safe and useful: it terminates by fixed
request count, prints per-client progress, and keeps child workers/LB scoped to
the proof run.

The local host passes the individual-score pressure gate. HostWorld does not:
even one client exceeds the 3 ms LB p99 target. Safety is green, packed parity
is green, and worker/core score are still sub-millisecond. The failing layer is
the deployed per-score HTTP/LB/upstream roundtrip envelope.
```

Rejected conclusion:

```text
Do not claim deployed concurrent throughput is closed.
Do not hide the HostWorld red result behind the green replay-batch gate.
Do not lower the p99 gate just to make the VPS pass.
```

Next proof-debt:

```text
The next speed work should target the per-score serving envelope:
  persistent LB -> worker upstream connections, or
  a compact binary upstream protocol, or
  a production route that sends traffic directly to the owning worker shard.

Any such change must keep:
  false_local_accepts = 0;
  packed/reference parity = 0 mismatches;
  workers serving-only `.nwrb`;
  load-balancer metadata-only unless a separate gate explicitly changes that.
```

Boundary:

```text
This is a bounded local/deployed pressure proof over sampled release-suite
traffic. It is not real Codex/API production traffic, not a long-running daemon
soak, and not full OPERATOR_BLUEPRINT coverage.
```
