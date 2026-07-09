# Phase-Center L4/L3 Coverage V18

Date: 2026-07-05

## Purpose

Record the state after the `planning_update` L4 request-side packer was added.
This report separates:

1. L4 streaming packer coverage.
2. L3 phase-center shadow coverage.
3. Product promotion / money claim boundary.

## Source Reports

```text
l4 packer:
  target/nando-wave/streaming/codex-session-planning-verifier-trace-v2-l4-packer.report.json

ranking:
  target/nando-wave/streaming/phase-atom-verifier-needed-ranking-v18-planning-l4-packer.report.json

self-mining package source:
  target/nando-wave/streaming/phase-atom-live-self-mining-loop-v17-planning-background-eval-top128.report.json

compatible shadow:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v18-planning-l4-packer.report.json

compatible shadow decisions:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v18-planning-l4-packer-decisions.jsonl
```

## L4 Packing Coverage

Bucket:

```text
action_family:planning::route_hint:planning_update
```

Packer facts:

```text
rows_written: 6063
rows_with_shadow_request: 6063
rows_ready_for_existing_shadow_scoring: 6063
local_accept_enabled: false
market_money_claim_allowed: false
forbidden_flags: all false
```

Ranking facts:

```text
rows: 6063
exact_cache_hits: 120
exact_cache_misses_over_cache: 5943
verifier_true_rows: 6063
verifier_true_over_exact_cache_ceiling: 5943
rows_with_shadow_request: 6063
rows_missing_shadow_request: 0
rows_with_result_atoms: 6063
rows_missing_result_atoms: 0
recommended_next_action: eligible_for_shadow_phase_center_review
```

Interpretation:

```text
L4 planning_update packing coverage:
  6063 / 6063 = 100%

L4 planning_update cache-miss shadow-request coverage:
  5943 / 5943 = 100%
```

The packer emits request-side `nando_shadow_request` payloads from state,
request, route, tool, and plan-shape atoms. It does not use target/proof
authority, concrete lookup, manual `local_out_t`, response text, or legacy
`.nwrb` role-binding backend.

## L3 Phase-Center Coverage

The `planning_update` phase-center package remains the v17 self-mined candidate
with same-base background negatives:

```text
candidate_package_path:
  target/nando-wave/streaming/live-self-mining-v17-planning-background-eval-top128/planning__route_hint_planning_update-ab3b8d5585c5b8ce-live-self-mining.candidate.nwpc

package_records: 1
runtime_cells: 64
package_bytes: 2064
runtime_bytes_estimate: 2080
safe_accept_margin_threshold_micro: 100000
```

Compatible shadow facts for the planning profile:

```text
events_seen: 6063
train_events: 3031
heldout_events: 3032
heldout_local_operator_calls: 3032
heldout_fallback_calls: 0
unique_cpu_accepts_over_exact_cache: 2992
false_accepts: 0
package_matches_report: true
```

Decision-log cross-check for the planning profile:

```text
planning_decisions: 3032
exact_cache_hits: 40
cache_misses: 2992
unique_cpu_accept_over_exact_cache: 2992
local_operator_shadow_decision: 3032
fallback_shadow_decision: 0
false_accepts: 0
min_margin_micro: 388999
p10_margin_micro: 523317
median_margin_micro: 625228
```

Interpretation:

```text
L3 planning_update heldout cache-miss coverage:
  2992 / 2992 = 100%
```

Do not compare the heldout-only accept count to the full-class
`verifier_true_over_exact_cache_ceiling` of 5943. That ceiling spans the whole
`planning_update` class before the train/heldout split. The compatible shadow
decision log proves that the heldout cache-miss slice has zero planning
fallbacks and zero false accepts.

## Compatible Denominator Shadow

Overall v18 compatible shadow:

```text
denominator.total_rows: 35833
profile_count: 27
heldout_routed_events: 6398
local_operator_shadow_decisions: 5413
verified_safe_accepts: 5600
unique_cpu_accepts_over_exact_cache: 4162
calls_saved_pct: 11.614991767365277
tokens_saved_pct: 11.540527380649827
nando_cpu_tokens_saved: 1733274
nando_cpu_cost_saved_microusd: 0
false_accepts: 0
verdict: PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_V1_PASS_SHADOW_ONLY
```

Decision log cross-check:

```text
rows: 6398
unique_cpu_accept_over_exact_cache: 4162
verified_safe_accept: 5600
false_accepts: 0
local_operator_shadow_decision: 5413
local_accept_enabled_any: false
total_tokens_saved_shadow: 1733274
```

Interpretation:

```text
CPU10 shadow is crossed:
  4162 / 35833 = 11.615% calls saved over exact cache
```

This is a real shadow coverage win. It is not product local accept and not a
money claim.

## Boundary

```text
local_accept_enabled: false
market_money_claim_allowed: false
product_promotion_allowed: false
nando_cpu_cost_saved_microusd: 0
```

Forbidden paths:

```text
concrete_x_lookup_used: false
local_accept_without_verifier_used: false
lookup_used: false
manual_local_out_t_used: false
nwrb_used: false
role_binding_backend_used: false
target_id_or_proof_rule_id_authority_used: false
```

## Next Required Work

1. Keep `planning_update` as a green shadow profile for this trace, but do not
   promote it until admission policy evidence exists.
2. Push L4 request-side packers and phase-center packages into the remaining
   high-value families. The v18 ranking still reports many non-planning rows
   without a scoreable shadow request.
3. Keep `local_accept_enabled=false` until promotion has verifier-bound gates,
   shadow stability, and explicit admission policy evidence.
4. Add provider cost evidence before any money-savings claim.
