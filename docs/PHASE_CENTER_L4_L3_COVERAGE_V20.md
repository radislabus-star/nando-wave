# Phase-Center L4/L3 Coverage V20

Date: 2026-07-05

## Purpose

Record the state after the `run_check` L4 request-side packer was added and
the phase-center self-mining / compatible denominator shadow reports were rerun.

This report keeps four claims separate:

1. L4 streaming packer coverage.
2. L3 phase-center `.nwpc` shadow coverage.
3. Calls/tokens saved in shadow.
4. Product promotion / money claim boundary.

## Source Reports

```text
run_check l4 packer:
  target/nando-wave/streaming/codex-session-run-check-verifier-trace-v2-l4-packer.report.json

ranking:
  target/nando-wave/streaming/phase-atom-verifier-needed-ranking-v20-run-check-l4-packer.report.json

self-mining:
  target/nando-wave/streaming/phase-atom-live-self-mining-loop-v20-run-check-l4-packer-top128.report.json

compatible shadow:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v20-run-check-l4-packer.report.json

compatible shadow decisions:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v20-run-check-l4-packer-decisions.jsonl

cost evidence audit:
  target/nando-wave/streaming/real-traffic-cost-evidence-audit-v20-compatible.report.json
```

## L4 Packing Coverage

`run_check` packer facts:

```text
rows_written: 770
pass_rows: 657
fail_rows: 26
compile_error_rows: 59
runtime_panic_rows: 28
unknown_failure_rows: 26
rows_with_shadow_request: 770
rows_ready_for_existing_shadow_scoring: 770
local_accept_enabled: false
market_money_claim_allowed: false
forbidden_flags: all false
```

`run_check` ranking facts:

```text
rows: 770
exact_cache_hits: 501
exact_cache_misses_over_cache: 269
verifier_true_rows: 657
verifier_false_rows: 113
verifier_true_over_exact_cache_ceiling: 213
rows_with_shadow_request: 770
rows_missing_shadow_request: 0
rows_ready_for_existing_shadow_scoring: 770
rows_with_result_atoms: 770
recommended_next_action: eligible_for_shadow_phase_center_review
```

Interpretation:

```text
L4 run_check packing coverage:
  770 / 770 = 100%
```

The packer emits request-side `nando_shadow_request` payloads from command,
cwd, check-kind, exit-code band, output-shape/status metadata, action, tool,
and route atoms. It does not store raw tool output, raw request/response text,
target/proof authority, concrete lookup, manual `local_out_t`, or legacy
`.nwrb` role-binding backend.

## L3 Phase-Center Coverage

Self-mining v20:

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

Compatible denominator v20:

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
estimated_nando_cpu_cost_saved_microusd: 2308568
accepted_cost_evidence_missing_events: 4537
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

The run_check L4 packer closes the request-side payload gap for this family.
It does not by itself increase the current accepted shadow profile set, because
the accepted top-128 phase-center candidates are unchanged from v19.

This is a shadow-only CPU coverage result. It is not product local accept and
not a money claim.

## Cost Evidence Audit

The cost evidence audit was corrected to read nested `token_cost` evidence
instead of only top-level token/cost fields.

```text
total_rows: 35833
shadow_request_rows: 18833
nonlegacy_candidate_rows: 18833
verifier_bound_token_or_cost_events: 18833
token_events: 18833
provider_cost_events: 0
estimated_cost_events: 0
compile_ready_bucket_count: 2
money_proof_candidate_bucket_count: 0
```

Compile-ready but not money-ready buckets:

```text
phase_center_tool_status_v1::tool_status_parse
  verifier_true_events: 9980
  verifier_false_events: 2020
  verifier_true_token_or_cost_events: 9980
  verifier_true_cost_events: 0
  recommended_next_action: attach_provider_or_estimated_cost_evidence_to_verified_safe_rows

phase_center_run_check_v1::test_output_parse
  verifier_true_events: 657
  verifier_false_events: 113
  verifier_true_token_or_cost_events: 657
  verifier_true_cost_events: 0
  recommended_next_action: attach_provider_or_estimated_cost_evidence_to_verified_safe_rows
```

Non-compile-ready token-visible bucket:

```text
phase_center_planning_update_v1::planning_update
  verifier_true_events: 6063
  verifier_false_events: 0
  verifier_true_token_or_cost_events: 6063
  verifier_true_cost_events: 0
  recommended_next_action: add_verified_safe_negative_evidence
```

Interpretation:

```text
Token evidence is now visible to the audit.
Money proof remains closed because verified-safe rows have no provider or
row-level estimated cost evidence.
```

## Boundary

```text
local_accept_enabled: false
market_money_claim_allowed: false
product_promotion_allowed: false
nando_cpu_cost_saved_microusd: 0
estimated_nando_cpu_cost_saved_microusd: 2308568
accepted_cost_evidence_missing_events: 4537
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
Estimated money savings are visible only as a placeholder-config estimate:
estimated_nando_cpu_cost_saved_microusd: 2308568
price_source: User-editable placeholder for cost-meter estimates.
accepted_cost_evidence_missing_events: 4537 / 4537
market_money_claim_allowed: false
```

Money savings are not market-claimable yet because provider billing or
row-level cost evidence is missing for accepted shadow rows.

## Next Required Work

1. Add provider billing/cost evidence to the denominator path before any money
   savings claim.
2. Convert the score-ready run_check family into stronger accepted phase-center
   candidates only if shadow gates keep `false_accepts = 0`.
3. Keep `local_accept_enabled=false` until promotion has verifier-bound gates,
   shadow stability, and explicit admission policy evidence.
