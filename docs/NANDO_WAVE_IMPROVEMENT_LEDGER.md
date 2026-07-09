# Nando Wave Improvement Ledger

Дата среза: 2026-07-06.

Назначение: единый журнал улучшений. Каждый новый механизм должен попадать сюда
только как измеримый delta, а не как красивый report.

Правило:

```text
improvement = before/after + denominator + false_accepts=0 + artifact
```

Если есть только новый adapter/report, но нет прироста accepts/tokens/$, статус
остается `WATCH`.

## Current Scoreboard

```text
best shadow frontier:
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  unique_cpu_accepts_over_exact_cache: 6_644 / 29_770
  false_accepts: 0
  local_accept_enabled: false
  market_money_claim_allowed: false

frozen NP-rescue shadow baseline:
  accepts: 591
  tokens_saved: 994_409
  false_accepts: 0
  parity_mismatches: 0
  provider_keys: 0
  market_money_claim_allowed: false

provider-correlated run-check chain:
  accepts: 23
  tokens_saved: 15_032
  false_accepts: 0
  provider_correlation: present
  compression_delta_vs_frozen_baseline: no

constrained split miner with verifier sources:
  safe_future_accepts_over_exact_cache: 5_657
  safe_future_tokens_saved: 8_454_877
  false_accepts: 0
  selected_split_count: 2
  boundary: cold automatic discovery, not product local_accept

selected split .nwpc quarantine/replay:
  promoted_shadow_packages: 1
  future_unique_accepts_over_exact_cache: 18
  future_tokens_saved: 26_724
  false_accepts: 0
  replay_mismatch_count: 0
  value_collapse_vs_split_miner: 5_657 -> 18 accepts
  boundary: shadow registry/runtime replay only, not product local_accept
```

## Improvement Table

| ID | Change | Layer | Metric Delta | Status | Artifact |
| --- | --- | --- | --- | --- | --- |
| IMP-001 | L4 marginal-denominator selector | L4 selector | moved selection away from pretty buckets toward unique accepts over exact cache | PASS as selector direction, not final product | `docs/EXECUTOR_REVIEW_NOTES.md` |
| IMP-002 | Learned / risk-aware selector curve | L4 selector | exposes fixed_greedy vs learned vs risk-aware tradeoff | WATCH: needs product-safe policy dominance | `target/nando-wave/streaming/*selector*curve*` |
| IMP-003 | NP-rescue safe recovery | L4/L3 selector + phase-center | 591 accepts / 994_409 tokens / 0 false in frozen shadow baseline | PASS shadow-safe | `target/nando-wave/streaming/phase-stream-online-miner-portfolio-np-rescue-v1-autogate-v28.report.json` |
| IMP-004 | NP-rescue runtime replay | Runtime proof | runtime replay parity 0, false_accepts 0 | PASS review-only | `target/nando-wave/streaming/phase-stream-online-miner-portfolio-np-rescue-runtime-replay-v1-autogate-v28.report.json` |
| IMP-005 | Provider-correlated run-check chain | Billing/evidence | provider keys present, but only 23 accepts / 15_032 tokens | WATCH: evidence improved, compression did not | `target/nando-wave/streaming/provider-boundary-np-chain-codex-token-backfill-run-check-v1.report.json` |
| IMP-006 | Selector billing-request adapter | Cold evidence adapter | 23 billing-request rows exported | WATCH: request-only, not money claim | `target/nando-wave/streaming/provider-boundary-np-chain-codex-token-backfill-run-check-v1.selector-billing-request.report.json` |
| IMP-007 | Hot/cold path separation | Architecture/runtime | hot runtime kept separate from JSONL/report/proof path | PASS as boundary, not savings claim | `docs/NANDO_WAVE_REVIEW_CONTROL.md` |
| IMP-008 | L4 Constrained Split Miner with verifier sources | L4 discovery / class split | 16_260 candidate splits -> 2 selected; 5_657 safe future accepts; 8_454_877 tokens; 0 false | PASS as cold automatic discovery; WATCH for product accept | `target/nando-wave/streaming/phase-stream-constrained-split-miner-v1-realtrace-plus-verifier-sources-v1.report.json` |
| IMP-009 | Selected split `.nwpc` quarantine + shadow replay | L3 package/runtime proof | 1 promoted shadow package; 18 future unique accepts; 26_724 tokens; 0 false; 0 replay mismatch | PASS runtime replay; WATCH due value collapse from 5_657 to 18 | `target/nando-wave/streaming/phase-stream-selected-split-nwpc-shadow-replay-v1-realtrace-plus-verifier-sources.report.json` |

## Required Tracking Fields

Every future entry must include:

```text
before_accepts
after_accepts
before_tokens
after_tokens
false_accepts
wrong_wins
exact_cache_overlap
denominator_rows
provider_billing_evidence
local_accept_enabled
market_money_claim_allowed
artifact_path
verdict
```

## Current P0

```text
P0:
  run billing-request / provider evidence on the large frozen NP-rescue baseline:
    591 accepts / 994_409 tokens / 0 false

P0:
  do not spend cycles polishing the 23-row run-check chain as if it were
  product compression.

P0:
  any new selector must beat the frozen baseline on comparable denominator:
    accepts > 591 or tokens_saved > 994_409
    false_accepts = 0
    parity_mismatches = 0

P0:
  market-money claim stays blocked until external provider billing evidence
  joins to accepted rows.

P0:
  Next step after constrained split miner:
    selected split
    -> verifier-bound .nwpc quarantine candidate
    -> fresh-future shadow
    -> runtime parity
    -> promotion only if false_accepts = 0

P0:
  Diagnose the split-miner -> .nwpc value collapse:
    split miner safe accepts: 5_657
    selected split .nwpc replay accepts: 18
  Keep the PASS for runtime replay, but do not call this a product compression
  improvement until the compiled package preserves much more of the split value.
```
