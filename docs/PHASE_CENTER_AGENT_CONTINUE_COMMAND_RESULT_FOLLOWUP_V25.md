# Phase-Center Agent Continue Command Result Followup V25

Date: 2026-07-05

Current status:

```text
Historical subroute packer milestone.
Still useful as evidence, but not the current top-line compression number.
```

Current architecture contract:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Current top-line frontier:

```text
calls_saved: 22.3177%
tokens_saved: 72.0541%
false_accepts: 0
local_accept_enabled: false
```

This step fixes the V24 `command_result_followup` blocker without reopening the
legacy `.nwrb` path. V24 had verifier positives/negatives but no result atoms
and no shadow request. V25 repacks existing `tool_status` phase rows into
`agent_continue` active-turn rows for this narrow subroute only.

## Commands

```bash
cargo run -q -p nando-cli -- phase-stream-agent-continue-command-result-followup-pack-v1 \
  target/nando-wave/streaming/agent-continue-command-result-followup-pack-v25.report.json \
  target/nando-wave/streaming/agent-continue-command-result-followup-pack-v25.jsonl \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty/codex-session-tool-status-verifier-trace-v4-l4-packer.jsonl.token-cost-enriched.jsonl.provider-billing-enriched.jsonl

cargo run -q -p nando-cli -- phase-stream-agent-continue-subroute-scoreboard-v1 \
  target/nando-wave/streaming/agent-continue-subroute-scoreboard-v25-command-result.report.json \
  target/nando-wave/streaming/agent-continue-command-result-followup-pack-v25.jsonl

cargo run -q -p nando-cli -- phase-stream-phase-atom-live-self-mining-loop-v1 \
  target/nando-wave/streaming/phase-atom-live-self-mining-loop-v25-agent-command-result-top128.report.json \
  target/nando-wave/streaming/live-self-mining-v25-agent-command-result-top128 \
  32 20 200 800 128 \
  data/real_traffic/model_price_config.v1.json \
  target/nando-wave/streaming/agent-continue-command-result-followup-pack-v25.jsonl

cargo run -q -p nando-cli -- phase-stream-phase-atom-live-self-mining-loop-v1 \
  target/nando-wave/streaming/phase-atom-live-self-mining-loop-v25-agent-command-result-top256.report.json \
  target/nando-wave/streaming/live-self-mining-v25-agent-command-result-top256 \
  32 20 200 800 256 \
  data/real_traffic/model_price_config.v1.json \
  target/nando-wave/streaming/agent-continue-command-result-followup-pack-v25.jsonl
```

## Packer Result

```text
total_rows_seen: 12_000
agent_continue_rows_written: 12_000
verifier_true_rows: 9_980
verifier_false_rows: 2_020
rows_with_shadow_request: 12_000
exact_cache_misses_over_cache: 6_435
local_accept_enabled: false
```

The command also removes the duplicate `action_family:tool_status` family from
forced `agent_continue` rows. `tool_status` remains as provenance/context atoms,
but the operator family is `planning`.

## Scoreboard

```text
subroute: command_result_followup
rows: 12_000
exact_cache_misses: 6_435
verifier_true: 9_980
verifier_false: 2_020
rows_with_result_atoms: 12_000
rows_with_shadow_request: 12_000
ready_for_subroute_mining: true
false_accept_risk: bounded_by_positive_negative_verifier_evidence
recommended_next_action: run_phase_center_mining_for_this_subroute
```

This turns the V24 blocker into a mining-ready subroute.

## Top-128 Mining Result

```text
total_rows: 12_000
parsed_verifier_events: 12_000
action_families_seen: 268
compiled_quarantine_candidates: 25
shadow_accepted_candidates: 21
aggregate_heldout_local_operator_calls: 726
aggregate_heldout_fallback_calls: 2_408
aggregate_unique_cpu_accepts_over_exact_cache: 513
aggregate_nando_cpu_tokens_saved: 784_026
accepted_false_accepts: 0
accepted_runtime_margin_parity_mismatches: 0
local_accept_enabled: false
market_money_claim_allowed: false
```

## Top-256 Ceiling Probe

```text
total_rows: 12_000
parsed_verifier_events: 12_000
action_families_seen: 268
high_value_classes: 62
compiled_quarantine_candidates: 48
shadow_accepted_candidates: 35
aggregate_heldout_local_operator_calls: 822
aggregate_heldout_fallback_calls: 5_954
aggregate_unique_cpu_accepts_over_exact_cache: 568
aggregate_nando_cpu_tokens_saved: 823_375
accepted_false_accepts: 0
accepted_runtime_margin_parity_mismatches: 0
local_accept_enabled: false
market_money_claim_allowed: false
```

Top-256 adds only 55 unique accepts over top-128, so the current
`command_result_followup` ceiling is visible: useful, but not enough alone for
the full CPU10 product target.

Notes:

- 568 is the current shadow-only ceiling for this V25 subroute pack, not a
  product deployment accept count.
- Total false accepts across rejected/non-accepted top-256 classes were 24.
  Accepted classes had 0 false accepts.
- `.nwpc` files were quarantine candidates only.
- The cost field is still not a market money claim. The money gate remains
  blocked without provider billing or an approved price source.

## Verdict

`command_result_followup` is no longer blocked by missing result/shadow payload.
It now has a verifier-bound phase-center mining path and produces meaningful
shadow savings. It still is not promoted to serving.

## Boundary

Allowed next work:

- combine this V25 result with the current compatible denominator report;
- mine/promote only after a separate promotion gate proves zero false accepts;
- continue collecting negative/background rows for `artifact_progress`;
- keep provider billing / price-source gate closed for external money claims.

Forbidden:

- broad `agent_continue_execute` mining;
- `.nwrb` / role-binding backend;
- target/proof authority;
- raw prompt/answer payloads;
- local accept.
