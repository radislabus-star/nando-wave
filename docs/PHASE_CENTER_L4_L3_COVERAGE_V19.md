# Phase-Center L4/L3 Coverage V19

Date: 2026-07-05

## Purpose

Record the state after the `tool_status` L4 request-side packer was added and
phase-center self-mining was rerun on the updated trace mix.

This report keeps four claims separate:

1. L4 streaming packer coverage.
2. L3 phase-center `.nwpc` shadow coverage.
3. Calls/tokens saved in shadow.
4. Product promotion / money claim boundary.

## Source Reports

```text
tool_status l4 packer:
  target/nando-wave/streaming/codex-session-tool-status-verifier-trace-v4-l4-packer.report.json

ranking:
  target/nando-wave/streaming/phase-atom-verifier-needed-ranking-v19-tool-status-l4-packer.report.json

self-mining:
  target/nando-wave/streaming/phase-atom-live-self-mining-loop-v19-tool-status-l4-packer-top128.report.json

compatible shadow:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v19-tool-status-l4-packer.report.json

compatible shadow decisions:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v19-tool-status-l4-packer-decisions.jsonl
```

## L4 Packing Coverage

`tool_status` packer facts:

```text
rows_written: 12000
rows_with_shadow_request: 12000
rows_ready_for_existing_shadow_scoring: 12000
pass_rows: 9980
negative_rows: 2020
local_accept_enabled: false
market_money_claim_allowed: false
forbidden_flags: all false
```

`tool_status` ranking facts:

```text
rows: 12000
exact_cache_misses_over_cache: 6435
verifier_true_over_exact_cache_ceiling: 5711
rows_with_shadow_request: 12000
rows_missing_shadow_request: 0
rows_ready_for_existing_shadow_scoring: 12000
rows_with_result_atoms: 12000
recommended_next_action: eligible_for_shadow_phase_center_review
```

Interpretation:

```text
L4 tool_status packing coverage:
  12000 / 12000 = 100%
```

The packer emits request-side `nando_shadow_request` payloads from command,
cwd, output-shape/status metadata, action, tool, and route atoms. It does not
store raw tool output, raw request/response text, target/proof authority,
concrete lookup, manual `local_out_t`, or legacy `.nwrb` role-binding backend.

## L3 Phase-Center Coverage

Self-mining v19:

```text
total_rows: 35833
high_value_classes: 71
compiled_quarantine_candidates: 37
shadow_accepted_candidates: 33
aggregate_heldout_local_operator_calls: 5711
aggregate_heldout_fallback_calls: 3844
aggregate_unique_cpu_accepts_over_exact_cache: 4542
local_accept_enabled: false
market_money_claim_allowed: false
```

Compatible denominator v19:

```text
denominator.total_rows: 35833
profile_count: 33
heldout_routed_events: 6627
local_operator_shadow_decisions: 5711
verified_safe_accepts: 5817
unique_cpu_accepts_over_exact_cache: 4537
calls_saved_pct: 12.6615131303547
tokens_saved_pct: 16.95134940808968
nando_cpu_tokens_saved: 2308568
nando_cpu_cost_saved_microusd: 0
false_accepts: 0
verdict: PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_V1_PASS_SHADOW_ONLY
```

Accepted shadow contribution by action family:

```text
planning:    2992 unique CPU accepts over exact cache, 1 profile
tool_status: 1472 unique CPU accepts over exact cache, 25 profiles
run_check:     73 unique CPU accepts over exact cache, 6 profiles
metrics:        0 unique CPU accepts over exact cache, 1 profile
```

Interpretation:

```text
CPU10 shadow remains crossed:
  4537 / 35833 = 12.66% calls saved over exact cache

Token shadow lift:
  2308568 tokens = 16.95% tokens saved over exact cache
```

This is a shadow-only CPU coverage win. It is not product local accept and not
a money claim.

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

Money boundary:

```text
Token savings are visible.
Money savings are not market-claimable yet because provider billing evidence is
not connected to the compatible denominator report.
```

## Next Required Work

1. Add an L4 request-side packer for `run_check`.
2. Add provider billing/cost evidence to the denominator path before any money
   savings claim.
3. Keep `local_accept_enabled=false` until promotion has verifier-bound gates,
   shadow stability, and explicit admission policy evidence.
