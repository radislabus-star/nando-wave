# Phase-Center L4/L3 Coverage V21

Date: 2026-07-05

Current status:

```text
Historical CPU10/early CPU12.66 shadow milestone.
Superseded as top-line status by the current frontier contract.
```

Current architecture contract:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Superseding top-line:

```text
calls_saved: 22.3177%
tokens_saved: 72.0541%
false_accepts: 0
local_accept_enabled: false
market_money_claim_allowed: false
```

## Purpose

Record the state after compatible phase-atom traces were enriched with
row-level estimated cost evidence from existing `token_cost.total_tokens` and
the model price config.

This report keeps three claims separate:

1. Calls/tokens/cost visible in shadow.
2. Product local accept boundary.
3. Market money claim boundary.

## Source Reports

```text
token/cost enrichment:
  target/nando-wave/streaming/real-traffic-token-cost-enrichment-v21-compatible-cost-filled.report.json

cost evidence audit:
  target/nando-wave/streaming/real-traffic-cost-evidence-audit-v21-compatible-cost-filled.report.json

compatible shadow:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v21-cost-filled.report.json

compatible shadow decisions:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v21-cost-filled-decisions.jsonl
```

## Token/Cost Enrichment

```text
input_rows: 35833
rows_with_shadow_request: 18833
matched_rows: 11967
rows_enriched_tokens: 11967
rows_enriched_cost: 30800
```

Session L4 traces were not matched by readiness `request_fingerprint`, but
already carried `token_cost.total_tokens`. The v21 enrichment therefore fills
missing row-level estimated cost from those token counts using the configured
input-token floor price.

```text
run_check rows_enriched_cost: 770
tool_status rows_enriched_cost: 12000
planning rows_enriched_cost: 6063
```

## Cost Evidence Audit

```text
shadow_request_rows: 18833
nonlegacy_candidate_rows: 18833
verifier_bound_token_or_cost_events: 18833
token_events: 18833
estimated_cost_events: 18833
provider_cost_events: 0
compile_ready_bucket_count: 2
money_proof_candidate_bucket_count: 2
```

Money-ready phase-center buckets:

```text
phase_center_tool_status_v1::tool_status_parse
  verifier_true_events: 9980
  verifier_false_events: 2020
  verifier_true_cost_events: 9980
  can_measure_money: true

phase_center_run_check_v1::test_output_parse
  verifier_true_events: 657
  verifier_false_events: 113
  verifier_true_cost_events: 657
  can_measure_money: true
```

Planning remains non-compile-ready because it has no negative verifier evidence:

```text
phase_center_planning_update_v1::planning_update
  verifier_true_events: 6063
  verifier_false_events: 0
  can_compile_phase_center: false
  recommended_next_action: add_verified_safe_negative_evidence
```

## Compatible Denominator Shadow

```text
denominator_rows: 35833
profile_count: 33
heldout_routed_events: 6627
local_operator_shadow_decisions: 5711
unique_cpu_accepts_over_exact_cache: 4537
calls_saved_pct: 12.6615131303547
tokens_saved_pct: 16.95134940808968
nando_cpu_tokens_saved: 2308568
nando_cpu_cost_saved_microusd: 2308568
accepted_token_evidence_missing_events: 0
accepted_cost_evidence_missing_events: 0
false_accepts: 0
verdict: PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_V1_PASS_SHADOW_ONLY
```

## Boundary

```text
local_accept_enabled: false
product_promotion_allowed: false
market_money_claim_allowed: false
provider_cost_events: 0
price_source: User-editable placeholder for cost-meter estimates.
```

Interpretation:

```text
The row-level estimated cost path is now connected for compatible shadow rows.
This allows cost math in reports, but it is still not a market money claim
because provider billing evidence is absent and the price config is explicitly
marked as a placeholder.
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

1. Attach real provider billing or user-approved price evidence before external
   money claims.
2. Add negative verifier evidence for `planning_update` if it should become a
   compile-ready money bucket.
3. Keep `local_accept_enabled=false` until promotion has verifier-bound gates,
   shadow stability, explicit admission policy, and runtime serving evidence.
