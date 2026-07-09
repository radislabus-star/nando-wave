# Phase-Center Market Money Claim Gate V22

Date: 2026-07-05

## Purpose

Record the first explicit market-money claim gate for phase-center compatible
shadow results.

This gate does not compile, promote, serve, or enable `local_accept`. It only
answers whether the current shadow cost evidence is strong enough for an
external money-savings claim.

## Source Reports

```text
market-money claim gate:
  target/nando-wave/streaming/phase-atom-market-money-claim-gate-v22-placeholder-blocked.report.json

compatible shadow:
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v21-cost-filled.report.json

cost evidence audit:
  target/nando-wave/streaming/real-traffic-cost-evidence-audit-v21-compatible-cost-filled.report.json

price config:
  data/real_traffic/model_price_config.v1.json
```

## Command

```text
cargo run -q -p nando-cli -- phase-stream-phase-atom-market-money-claim-gate-v1 \
  target/nando-wave/streaming/phase-atom-market-money-claim-gate-v22-placeholder-blocked.report.json \
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v21-cost-filled.report.json \
  target/nando-wave/streaming/real-traffic-cost-evidence-audit-v21-compatible-cost-filled.report.json \
  data/real_traffic/model_price_config.v1.json
```

## Result

```text
claim_status: INTERNAL_ESTIMATE_ONLY
money_claim_gate_passed: false
market_money_claim_allowed: false
local_accept_enabled: false
```

Compatible shadow facts:

```text
shadow_gate_passed: true
false_accepts: 0
unique_cpu_accepts_over_exact_cache: 4537
nando_cpu_tokens_saved: 2308568
nando_cpu_cost_saved_microusd: 2308568
accepted_token_evidence_missing_events: 0
accepted_cost_evidence_missing_events: 0
```

Cost audit facts:

```text
estimated_cost_events: 18833
provider_cost_events: 0
money_proof_candidate_bucket_count: 2
```

Price evidence facts:

```text
price_source_is_placeholder: true
provider_billing_evidence_file_present: false
provider_row_cost_gate_passed: false
user_approved_price_config: false
user_approved_price_gate_passed: false
```

Passing gates:

```text
compatible_shadow_gate_passed: true
safety_gate_passed: true
cpu_savings_gate_passed: true
token_evidence_gate_passed: true
row_cost_evidence_gate_passed: true
```

Blocking gates:

```text
price_source_gate_passed: false
provider_billing_file_gate_passed: false
provider_row_cost_gate_passed: false
user_approved_price_gate_passed: false
```

Blockers:

```text
price_source_is_placeholder_or_estimate
no_provider_billing_evidence_file_provider_row_cost_or_user_approved_price_config
```

## Boundary

The internal estimated cost path is connected and useful for engineering
comparison. It is still not a market claim because the price source is a
placeholder and no provider billing or explicitly approved price evidence is
present.

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

1. Attach provider billing evidence or user-approved non-placeholder price
   evidence.
2. Rerun the market-money claim gate.
3. Keep `local_accept_enabled=false` until a separate product promotion gate
   proves serving safety.
