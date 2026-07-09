# Phase-Center Auto-Subcenter Discovery V26

Date: 2026-07-06

Current status:

```text
Historical bridge / diagnostic.
Not the current best compression frontier.
Not the final streaming selector.
```

Current architecture contract:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Superseding current frontier:

```text
artifact: target/nando-wave/streaming/phase-atom-frontier-shadow-replay-diversity-p500-run-check-p500-latest-window-v1.report.json
calls_saved: 22.3177%
tokens_saved: 72.0541%
false_accepts: 0
local_accept_enabled: false
```

Lesson from V26:

```text
Automatic subcenter discovery alone is not enough.
The selector must optimize marginal denominator delta, not pretty bucket score.
```

This step replaces manual `topN`/bucket picking with a batch automatic
subcenter discovery bridge. It still is not the final streaming miner daemon:
there is no live tail loop, no incremental reservoir update, and no product
promotion. It only proves the automatic search loop can generate quarantine
phase-center candidates, attach background negatives, and survive a compatible
denominator replay with zero false accepts.

## Commands

```bash
cargo run -q -p nando-cli -- phase-stream-auto-subcenter-discovery-v1 \
  target/nando-wave/streaming/auto-subcenter-discovery-v26.report.json \
  target/nando-wave/streaming/auto-subcenter-discovery-v26.candidates.jsonl \
  target/nando-wave/streaming/auto-subcenter-discovery-v26.rejections.jsonl \
  48 1200 1 \
  target/nando-wave/streaming/agent-continue-active-turn-state-v24.jsonl \
  target/nando-wave/streaming/agent-continue-command-result-followup-pack-v25.jsonl

cargo run -q -p nando-cli -- phase-stream-phase-atom-live-self-mining-loop-v1 \
  target/nando-wave/streaming/phase-atom-live-self-mining-loop-v26-auto-subcenter-top128.report.json \
  target/nando-wave/streaming/live-self-mining-v26-auto-subcenter-top128 \
  32 20 100000 800 128 \
  data/real_traffic/model_price_config.v1.json \
  target/nando-wave/streaming/auto-subcenter-discovery-v26.candidates.jsonl

cargo run -q -p nando-cli -- phase-stream-phase-atom-compatible-denominator-shadow-v1 \
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v26-auto-subcenter.report.json \
  target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v26-auto-subcenter-decisions.jsonl \
  target/nando-wave/streaming/phase-atom-live-self-mining-loop-v26-auto-subcenter-top128.report.json \
  target/nando-wave/streaming/auto-subcenter-discovery-v26.candidates.jsonl
```

## Discovery Result

```text
total_rows_seen: 23_063
eligible_rows: 18_063
enumerated_split_atoms: 196
selected_candidates: 48
rejected_candidates: 90
candidate_trace_rows_written: 109_300
candidate_positive_rows_written: 54_650
candidate_background_rows_written: 54_650
local_accept_enabled: false
```

Automation contract:

```text
ranked_candidate_split_atoms: true
chose_subcenters_by_score: true
selected_background_negatives: true
rejected_bad_candidates_with_reasons: true
compatible_denominator_delta_measured: measured by the later denominator report
```

The generated candidate trace contains 48 distinct
`route_hint:auto_subcenter_candidate:*` buckets. The route hint is used only to
keep candidate classes separable for quarantine mining; it is not a product
runtime authority.

## Mining Result

```text
total_rows: 109_300
action_families_seen: 48
high_value_classes: 48
compiled_quarantine_candidates: 32
shadow_accepted_candidates: 25
aggregate_heldout_local_operator_calls: 1_787
aggregate_heldout_fallback_calls: 10_213
aggregate_unique_cpu_accepts_over_exact_cache: 1_643
aggregate_nando_cpu_tokens_saved: 2_639_909
aggregate_nando_cpu_cost_saved_microusd: 2_639_909
accepted_false_accepts: 0
accepted_wrong_wins: 145
accepted_runtime_margin_parity_mismatches: 0
local_accept_enabled: false
market_money_claim_allowed: false
```

This mining number is useful but not sufficient as a product claim because
candidate classes overlap. The compatible denominator replay is the stricter
number.

## Compatible Denominator Result

```text
denominator_total_rows: 109_300
denominator_unique_request_fingerprints: 11_240
routed_events: 60_000
heldout_routed_events: 12_000
profile_count: 25
local_operator_shadow_decisions: 1_787
fallback_shadow_decisions: 10_213
exact_cache_hits_in_routed_events: 11_441
unique_cpu_accepts_over_exact_cache: 328
calls_saved_milli: 3
calls_saved_pct: 0.3000914913083257
nando_cpu_tokens_saved: 750_413
tokens_saved_pct: 0.21359162246005314
nando_cpu_cost_saved_microusd: 750_413
false_accepts: 0
wrong_wins: 145
latency_p99_ns: 17_567
local_accept_enabled: false
market_money_claim_allowed: false
auto_promote_enabled: false
product_promotion_allowed: false
verdict: PHASE_ATOM_COMPATIBLE_DENOMINATOR_SHADOW_V1_PASS_SHADOW_ONLY
```

The main finding is overlap pressure: quarantine mining sees 1,643 unique
accepts inside per-candidate heldout slices, but the compatible denominator
deduplicates this to 328 unique accepts over exact cache.

## Boundary

Allowed next work:

- build the real online miner daemon rung;
- reduce candidate overlap before claiming product compression;
- keep candidates in `.nwpc` quarantine form only;
- compare future deltas against this denominator report;
- split the giant streaming command file with a move-only spectral-budget pass.

Forbidden:

- `.nwrb` / role-binding backend;
- target/proof authority;
- concrete lookup;
- raw prompt/output text as runtime authority;
- local accept;
- auto promotion;
- market money claim.

## Verdict

`WATCH/PASS-SHADOW`: the automatic batch subcenter loop works and stays safe
under compatible denominator replay (`false_accepts=0`). It is not yet the final
streaming miner and does not yet provide a market compression claim.
