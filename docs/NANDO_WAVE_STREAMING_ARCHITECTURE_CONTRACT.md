# Nando Wave Streaming Architecture Contract

Дата среза: 2026-07-06

Назначение: единый контракт текущей архитектуры, чтобы не смешивать research
gates, shadow compression, hot runtime, market claim и product local accept.

## Current Truth

```text
Active backend:
  phase-center / .nwpc / mutable operator memory

Forbidden backend:
  .nwrb role-binding serving path

Product direction:
  real stream
  -> source adapter
  -> source-neutral atoms
  -> L4 streaming selector/router/admission
  -> L3 drifting phase centers
  -> hot runtime score
  -> verifier-bound shadow/promotion gate
  -> local accept only after proof
```

## Best Current Compression Snapshot

Artifact:

```text
target/nando-wave/streaming/phase-atom-frontier-shadow-replay-diversity-p500-run-check-p500-latest-window-v1.report.json
```

Numbers:

```text
verdict: PHASE_ATOM_FRONTIER_SHADOW_REPLAY_V1_PASS_SHADOW_ONLY
denominator_rows: 29_770
denominator_tokens: 14_410_915
denominator_cost_microusd: 2_833_212
unique_request_fingerprints: 16_367

profile_count: 5
exact_cache_hits_in_routed_events: 5_920
local_operator_shadow_decisions: 12_564
unique_cpu_accepts_over_exact_cache: 6_644

calls_saved: 22.3177%
tokens_saved: 72.0541%
calls_saved_milli: 223
nando_cpu_tokens_saved: 10_383_658

false_accepts: 0
wrong_wins: 0
p99_shadow_latency_ns: 15_938
```

Boundary:

```text
local_accept_enabled: false
product_promotion_allowed: false
market_money_claim_allowed: false

This is a shadow replay result, not production local accept.
It does not claim provider-billing money yet.
```

Source traces used by that frontier:

```text
target/nando-wave/streaming/real-traffic-phase-atom-trace-v1.jsonl
target/nando-wave/streaming/codex-session-run-check-verifier-trace-v1.jsonl
target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.jsonl
```

Checked current row counts:

```text
real-traffic-phase-atom-trace-v1.jsonl: 17_000 rows
codex-session-run-check-verifier-trace-v1.jsonl: 770 rows
codex-session-tool-status-verifier-trace-append-v3-latest.jsonl: 12_000 rows
```

Open report debt:

```text
The frontier report should expose synthetic_rows/non_synthetic_rows directly.
Current trace scan found no obvious synthetic=true rows in the three source
trace files, but the report itself should carry that field for audit.
```

## Layer Contract

```text
SOURCE ADAPTER
|
+-- may be Codex-specific, Claude-specific, CI-specific, or app-specific
+-- may parse raw events
+-- must not leak source-specific logic into core
+-- emits source-neutral safe atoms and metadata

L1 / ATOMS
|
+-- safe request/state/result atoms
+-- numeric atom ids for hot path
+-- no raw prompt, raw answer, or raw stdout in core

L2 / EVENT SHAPE
|
+-- action_family
+-- route hint
+-- result shape
+-- cache/billing/verifier metadata

L4 / STREAMING GOAL-STATE LAYER
|
+-- chooses route/profile/bucket
+-- ranks opportunities by marginal denominator delta
+-- controls admission, eviction, top-K, and hot set
+-- prevents one L3 center from mixing unrelated actions
+-- does not become answer authority

L3 / PHASE-CENTER OPERATOR MEMORY
|
+-- score_before_update
+-- positive/negative center update after verifier label
+-- drifting phase center state
+-- margin/threshold statistics
+-- candidate quarantine

HOT RUNTIME
|
+-- route_id + fixed phase vector + scratch -> margin
+-- no JSON
+-- no String bucket key
+-- no BTreeMap
+-- no file IO
+-- no package compile
+-- no verifier replay

VERIFIER / PROMOTION
|
+-- false_accepts must be 0
+-- exact-cache overlap excluded
+-- future/heldout split required
+-- local accept remains disabled until promotion proof passes
```

## Hot/Warm/Cold Memory Contract

```text
HOT:
  1-4 active profile shards per worker
  bounded top-K per route
  L2/cache-resident target where possible

WARM:
  bounded process-memory registry
  candidate profiles and route metadata

COLD:
  JSONL traces
  reports
  .nwpc snapshots
  audit artifacts
  rollback/deploy packages
```

`.nwpc` is a snapshot/export/deployment artifact. It is not compiled on every
request and is not the streaming mechanism itself.

## Current Hot Runtime Snapshot

Artifact:

```text
target/nando-wave/streaming/phase-stream-hot-path-benchmark-v1-release.report.json
```

Numbers:

```text
verdict: HOT_PATH_BENCHMARK_PASS
total_rows: 17_000
parsed_rows: 374
online_bucket_count: 33
candidate_bucket_count: 1
unique_cpu_accepts_over_exact_cache: 8
false_accepts: 0
tokens_saved: 4_526
cost_saved_microusd: 13_578

hot_runtime_bytes_estimate: 544
hot_route_table_bytes_estimate: 48
hot_bytes_estimate: 592
warm_metadata_bytes_estimate: 39_136
product_runtime_budget_passed: true
```

Boundary:

```text
This proves the hot path shape and budget.
It does not replace the best shadow frontier compression number.
```

## Current Live Store Snapshot

Artifact:

```text
target/nando-wave/streaming/phase-stream-live-store-adapter-smoke-v1.report.json
```

Numbers:

```text
verdict: LIVE_STORE_ADAPTER_SMOKE_WATCH_FALSE_ACCEPTS
total_rows: 17_000
parsed_rows: 374
route_count: 5
route_bucket_count: 33
active_bucket_count: 12
candidate_bucket_count: 1
unique_cpu_accepts_over_exact_cache: 8
false_accepts: 2
```

Boundary:

```text
This is a live-store wiring smoke.
Because false_accepts = 2, it cannot promote.
The later hot benchmark has false_accepts = 0 but only proves the bounded hot
runtime lane on the selected candidate.
```

## Automatic Streaming Process

Required product loop:

```text
real event stream
|
+-- class/opportunity board
|   |
|   +-- traffic share
|   +-- exact-cache overlap
|   +-- token/cost share
|   +-- verifier availability
|   +-- result atoms present
|   +-- current false-accept risk
|
+-- L4 self-learning selector
|   |
|   +-- choose by marginal denominator delta
|   +-- not by pretty bucket score alone
|   +-- penalize overlap, hot bytes, latency, false-risk
|
+-- L3 phase-center mining
|   |
|   +-- bounded positive/background evidence
|   +-- multi-center/subcenter split when needed
|
+-- shadow on future events
|   |
|   +-- count only unique accepts over exact cache
|   +-- false_accepts must be 0
|
+-- promotion queue
    |
    +-- verifier-bound evidence required
    +-- local accept disabled until proof passes
```

## Current Problem To Solve

```text
The 22.32% / 72.05% frontier proves a strong shadow opportunity.

The next product problem is not "more manual buckets".
The next product problem is automatic streaming selection:
  which classes to mine,
  which subcenters to keep hot,
  which profiles to evict,
  and which verified shadow wins actually increase the global denominator.
```

P0 next work:

```text
automatic L4 opportunity board
learned marginal-denominator selector
future-window shadow replay for selected profiles
synthetic/non-synthetic fields in every compression report
provider billing join with real provider_cost_events
```

## Forbidden Regressions

```text
Do not revive .nwrb role-binding backend.
Do not use lookup, target_id, proof_rule_id authority, concrete_x_lookup, or
manual local_out_t.
Do not turn verifier label into score authority.
Do not claim market money from placeholder prices.
Do not count exact-cache hits as Nando savings.
Do not count scoreable rows as accepted rows.
Do not call batch candidate generation the final streaming miner.
Do not enable product local_accept without verifier-bound promotion and
false_accepts = 0.
```
