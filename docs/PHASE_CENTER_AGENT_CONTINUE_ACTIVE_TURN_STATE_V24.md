# Phase-Center Agent Continue Active Turn State V24

Date: 2026-07-05

This is the first reviewer-directed `agent_continue_execute` split after the
`.nwrb` commercial path was quarantined. It does not mine a broad
`agent_continue_execute` operator. It only builds an active-turn trace and a
subroute scoreboard for future verifier-bound phase-center mining.

## Commands

```bash
cargo run -q -p nando-cli -- phase-stream-agent-continue-active-turn-state-v1 \
  target/nando-wave/streaming/agent-continue-active-turn-state-v24.report.json \
  target/nando-wave/streaming/agent-continue-active-turn-state-v24.jsonl \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty/real-traffic-phase-atom-trace-v1.jsonl.token-cost-enriched.jsonl.provider-billing-enriched.jsonl \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty/codex-session-planning-verifier-trace-v2-l4-packer.jsonl.token-cost-enriched.jsonl.provider-billing-enriched.jsonl \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty/codex-session-tool-status-verifier-trace-v4-l4-packer.jsonl.token-cost-enriched.jsonl.provider-billing-enriched.jsonl \
  target/nando-wave/streaming/provider-billing-enriched-traces-v23-empty/codex-session-run-check-verifier-trace-v2-l4-packer.jsonl.token-cost-enriched.jsonl.provider-billing-enriched.jsonl

cargo run -q -p nando-cli -- phase-stream-agent-continue-subroute-scoreboard-v1 \
  target/nando-wave/streaming/agent-continue-subroute-scoreboard-v24.report.json \
  target/nando-wave/streaming/agent-continue-active-turn-state-v24.jsonl
```

## Result

```text
total_rows_seen: 35_833
agent_continue_rows_written: 11_063
exact_cache_misses_over_cache: 10_489
verifier_true_rows: 6_137
verifier_false_rows: 223
rows_with_result_atoms: 6_063
rows_with_shadow_request: 6_063
local_accept_enabled: false
market_money_claim_allowed: false
```

Subroute scoreboard:

```text
artifact_progress:
  rows: 6_063
  exact_cache_misses: 5_943
  verifier_true: 6_063
  verifier_false: 0
  rows_with_result_atoms: 6_063
  rows_with_shadow_request: 6_063
  ready_for_subroute_mining: false
  blocker: high_no_negative_verifier_evidence
  next: collect_negative_or_background_verifier_rows_before_mining

command_result_followup:
  rows: 5_000
  exact_cache_misses: 4_546
  verifier_true: 74
  verifier_false: 223
  rows_with_result_atoms: 0
  rows_with_shadow_request: 0
  ready_for_subroute_mining: false
  blocker: missing result atoms and shadow request
  next: capture_result_atoms_before_subroute_mining
```

## Verdict

`WATCH`, not progress against CPU10.

The useful discovery is structural:

- broad `agent_continue_execute` stays unsafe;
- `artifact_progress` is large and valuable, but positive-only;
- `command_result_followup` has both verifier signs, but lacks the state/result
  payload needed for phase-center mining;
- therefore no V24 `.nwpc` mining or compatible denominator replay is allowed
  yet.

## Boundary

Allowed next work:

- collect negative/background verifier evidence for `artifact_progress`;
- attach result atoms and shadow request payloads to `command_result_followup`;
- mine only eligible subroutes after the scoreboard gate turns green.

Forbidden:

- broad `agent_continue_execute` mining;
- hardcoded agent operator stores;
- `.nwrb` / role-binding backend;
- target/proof authority;
- local accept.
