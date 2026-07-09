# Phase-Center L4/L3 Coverage V17

Date: 2026-07-05

## Purpose

Separate three claims that must not be merged:

1. L4 Streaming Router/Packer coverage.
2. L3 phase-center coverage.
3. Product promotion / money claim boundary.

This report records the current state after the v17 planning phase-center
shadow run.

## Source Reports

```text
ranking:
  target/nando-wave/streaming/phase-atom-verifier-needed-ranking-v16-planning-tool-output.report.json

self-mining:
  target/nando-wave/streaming/phase-atom-live-self-mining-loop-v17-planning-background-eval-top128.report.json

compatible shadow:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v17-planning-background-eval-top128.report.json
```

## L4 Packing Coverage

Bucket:

```text
action_family:planning::route_hint:planning_update
```

Ranking facts:

```text
rows: 6059
exact_cache_hits: 120
exact_cache_misses_over_cache: 5939
verifier_true_over_exact_cache_ceiling: 5939
rows_with_result_atoms: 6059
rows_with_shadow_request: 0
rows_missing_shadow_request: 6059
```

Interpretation:

```text
L4 planning_update packing coverage is 0 / 5939 = 0%.
```

The daemon/live stream path is not closed for `planning_update` yet, because the
rows are verifier-ready and result-atom-ready, but they do not carry a
`shadow_request` payload.

## L3 Phase-Center Coverage

The same `planning_update` bucket is already scoreable by the offline
phase-center mining path:

```text
events_seen: 6059
train_positive_events: 3029
background_negative_train_events_used: 111
heldout_positive_events: 3030
background_negative_heldout_events_used: 112
heldout_local_operator_calls: 3030
unique_cpu_accepts_over_exact_cache: 2990
false_accepts: 0
wrong_wins: 0
accepted_for_shadow_review: true
```

Interpretation:

```text
L3 planning_update phase-center coverage over its verifier-true cache-miss ceiling:
  2990 / 5939 = 50.34%
```

This proves a useful phase-center for the route, not a completed live L4 packer.

## Compatible Denominator Shadow

Overall v17 compatible shadow:

```text
denominator.total_rows: 35829
profile_count: 27
heldout_routed_events: 6396
unique_cpu_accepts_over_exact_cache: 4160
calls_saved_pct: 11.610706411007843
tokens_saved_pct: 11.538996773062905
nando_cpu_tokens_saved: 1732993
false_accepts: 0
verdict: PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_V1_PASS_SHADOW_ONLY
```

Interpretation:

```text
CPU10 shadow is crossed:
  4160 / 35829 = 11.61% calls saved over exact cache
```

## Boundary

```text
local_accept_enabled: false
market_money_claim_allowed: false
product_promotion_allowed: false
nando_cpu_cost_saved_microusd: 0
```

This is not production local accept. This is not a market money claim. The
current planning class also depends on same-base background negatives because
the real `update_plan` rows in this trace are all verifier-positive.

## Next Required Work

1. Build the L4 streaming packer for `planning_update`.
2. It must create request-side `shadow_request` payloads from state/request/tool
   atoms, not from target/proof labels and not from response text.
3. Rerun verifier-needed ranking and require:

```text
planning_update rows_with_shadow_request > 0
planning_update rows_missing_shadow_request decreases
```

4. Rerun compatible denominator shadow and require:

```text
false_accepts = 0
local_accept_enabled = false
market_money_claim_allowed = false
```

5. Only after L4 packing coverage is proven should promotion/local shadow
manifest work continue.
