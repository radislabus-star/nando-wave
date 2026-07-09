# Phase-Center Provider Billing Evidence Join V23

Date: 2026-07-05

## Purpose

Add the first provider-billing evidence adapter for the phase-center real-traffic
stream.

This is a join/enrichment step only. It does not compile `.nwpc`, promote a
candidate, serve traffic, enable `local_accept`, or allow a market money claim.

## Source Reports

```text
provider billing join smoke:
  target/nando-wave/streaming/provider-billing-evidence-join-v23-empty.report.json

provider-billing-enriched cost audit:
  target/nando-wave/streaming/real-traffic-cost-evidence-audit-v23-empty-provider-billing-enriched.report.json

money claim gate with empty provider billing file:
  target/nando-wave/streaming/phase-atom-market-money-claim-gate-v23-empty-provider-billing-blocked.report.json

empty provider billing input:
  target/nando-wave/streaming/provider-billing-evidence-empty-v23.jsonl
```

## Command

```text
cargo run -q -p nando-cli -- phase-stream-provider-billing-evidence-join-v1 \
  target/nando-wave/streaming/provider-billing-evidence-join-v23-empty.report.json \
  target/nando-wave/streaming/provider-billing-evidence-empty-v23.jsonl \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty \
  target/nando-wave/streaming/token-cost-enriched-traces-v21-compatible-cost-filled/real-traffic-phase-atom-trace-v1.jsonl.token-cost-enriched.jsonl \
  target/nando-wave/streaming/token-cost-enriched-traces-v21-compatible-cost-filled/codex-session-run-check-verifier-trace-v2-l4-packer.jsonl.token-cost-enriched.jsonl \
  target/nando-wave/streaming/token-cost-enriched-traces-v21-compatible-cost-filled/codex-session-tool-status-verifier-trace-v4-l4-packer.jsonl.token-cost-enriched.jsonl \
  target/nando-wave/streaming/token-cost-enriched-traces-v21-compatible-cost-filled/codex-session-planning-verifier-trace-v2-l4-packer.jsonl.token-cost-enriched.jsonl
```

## Result

```text
billing_rows: 0
billing_rows_with_provider_cost: 0
rows_with_shadow_request: 18833
matched_rows: 0
rows_enriched_provider_cost: 0
rows_enriched_tokens: 0
market_money_claim_allowed: false
```

Provider-billing-enriched cost audit:

```text
provider_cost_events: 0
estimated_cost_events: 18833
token_events: 18833
verifier_bound_token_or_cost_events: 18833
compile_ready_bucket_count: 2
money_proof_candidate_bucket_count: 2
```

Money claim gate after the empty billing file:

```text
claim_status: INTERNAL_ESTIMATE_ONLY
money_claim_gate_passed: false
market_money_claim_allowed: false
provider_billing_evidence_file_present: true
provider_billing_file_gate_passed: false
provider_row_cost_gate_passed: false
```

## Boundary

The adapter may copy externally supplied provider cost/token counters into
matching real-traffic rows.

It must not:

- estimate missing billing;
- treat an empty billing file as evidence;
- compile, promote, serve, or enable `local_accept`;
- allow an external money claim;
- use `.nwrb` or the legacy role-binding commercial backend.

The next unlock is a real provider billing JSONL with matching fingerprints and
positive `provider_cost_microusd` rows, or an explicit non-placeholder
user-approved price config.
