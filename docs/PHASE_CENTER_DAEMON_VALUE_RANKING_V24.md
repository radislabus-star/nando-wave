# Phase-Center Daemon Value Ranking V24

Date: 2026-07-05

## Purpose

Extend the phase-atom verifier-needed ranking into a daemon opportunity board.

The report now ranks action families by:

- traffic share;
- exact-cache overlap;
- verifier-positive ceiling over exact cache;
- expected tokens saved over exact cache;
- expected cost saved over exact cache;
- provider-vs-estimated cost evidence;
- false-accept risk;
- daemon next action.

This is still ranking only. It does not compile `.nwpc`, promote a candidate,
serve traffic, enable `local_accept`, or allow a market money claim.

## Source Report

```text
target/nando-wave/streaming/phase-atom-verifier-needed-ranking-v24-provider-billing-aware.report.json
```

## Command

```text
cargo run -q -p nando-cli -- phase-stream-phase-atom-verifier-needed-ranking-v1 \
  target/nando-wave/streaming/phase-atom-verifier-needed-ranking-v24-provider-billing-aware.report.json \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty/real-traffic-phase-atom-trace-v1.jsonl.token-cost-enriched.jsonl.provider-billing-enriched.jsonl \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty/codex-session-run-check-verifier-trace-v2-l4-packer.jsonl.token-cost-enriched.jsonl.provider-billing-enriched.jsonl \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty/codex-session-tool-status-verifier-trace-v4-l4-packer.jsonl.token-cost-enriched.jsonl.provider-billing-enriched.jsonl \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty/codex-session-planning-verifier-trace-v2-l4-packer.jsonl.token-cost-enriched.jsonl.provider-billing-enriched.jsonl
```

## Global Result

```text
total_rows: 35833
rows_with_action_family: 35833
rows_with_verifier_label: 19207
exact_cache_hits: 18535
exact_cache_misses_over_cache: 17298
exact_cache_overlap_milli: 517
verifier_true_over_exact_cache_ceiling: 11935
verifier_true_tokens_over_exact_cache_ceiling: 9370346
verifier_true_cost_microusd_over_exact_cache_ceiling: 9371730
token_events: 30800
provider_cost_events: 0
estimated_cost_events: 30800
cpu10_target_unique_accepts: 3584
remaining_verifier_true_accept_gap_to_cpu10: 0
compile_allowed: false
local_accept_enabled: false
market_money_claim_allowed: false
```

Interpretation:

```text
The real-trace labeled ceiling is large enough for CPU10:
  11935 verifier-true rows over exact cache vs 3584 required.

This is not a local-accept result.
It is a ranked opportunity ceiling for the daemon.
```

## Top Value Families

```text
tool_status:
  rows: 12000
  traffic_share_milli: 334
  exact_cache_overlap_milli: 463
  verifier_true_over_exact_cache_ceiling: 5711
  expected_tokens_saved_over_exact_cache: 8675251
  expected_cost_saved_microusd_over_exact_cache: 8675251
  daemon_next_action: run_shadow_with_internal_estimate_and_request_provider_billing

planning:
  rows: 11063
  traffic_share_milli: 308
  exact_cache_overlap_milli: 51
  verifier_true_over_exact_cache_ceiling: 6011
  expected_tokens_saved_over_exact_cache: 597818
  expected_cost_saved_microusd_over_exact_cache: 599202
  daemon_next_action: run_shadow_with_internal_estimate_and_request_provider_billing

run_check:
  rows: 770
  traffic_share_milli: 21
  exact_cache_overlap_milli: 650
  verifier_true_over_exact_cache_ceiling: 213
  expected_tokens_saved_over_exact_cache: 97277
  expected_cost_saved_microusd_over_exact_cache: 97277
  daemon_next_action: run_shadow_with_internal_estimate_and_request_provider_billing
```

Families to deprioritize for the next CPU10 push:

```text
metrics_report:
  exact_cache_overlap_milli: 979
  verifier_true_over_exact_cache_ceiling: 0
  daemon_next_action: deprioritize_exact_cache_overlap_until_unique_accepts_exist

serving_ops:
  exact_cache_overlap_milli: 1000
  verifier_true_over_exact_cache_ceiling: 0
  daemon_next_action: deprioritize_exact_cache_overlap_until_unique_accepts_exist
```

## Bucket-Level Warning

The largest planning bucket is valuable but lacks negative verifier evidence:

```text
bucket: action_family:planning::route_hint:planning_update
rows: 6063
exact_cache_overlap_milli: 19
verifier_true_over_exact_cache_ceiling: 5943
expected_tokens_saved_over_exact_cache: 597126
false_accept_risk: high_no_negative_verifier_evidence
daemon_next_action: collect_negative_verifier_rows_before_shadow
```

So the daemon should not blindly promote planning. It must collect or synthesize
verifier-bound negative rows before shadow mining that bucket.

## Boundary

V24 is an opportunity ranking report only.

It does not:

- compile `.nwpc`;
- promote a candidate;
- serve traffic;
- enable `local_accept`;
- allow a market money claim;
- use `.nwrb` or the legacy role-binding commercial backend.

Provider-cost evidence is still missing:

```text
provider_cost_events: 0
estimated_cost_events: 30800
```

The next aligned move is to run/strengthen shadow mining for high-value
verifier-bounded families while collecting provider billing evidence and
negative verifier rows where risk is high.
