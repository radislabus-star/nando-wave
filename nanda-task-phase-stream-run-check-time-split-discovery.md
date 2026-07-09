# NANDA Task: Phase Stream Run-Check Time-Split Discovery

## Query

Verify that the run_check phase-stream time-split rung trains on older Codex
session verifier events, shadows newer events, writes only a quarantine `.nwpc`
candidate, and keeps promotion/product local_accept/market claim/legacy backend
disabled.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| time-split-discovery | artifact | phase-atom-run-check-time-split-discovery-v1.report.json | discovery report path |
| time-split-discovery | package | phase-atom-run-check-time-split-discovery-v1.candidate.nwpc | quarantine package path |
| time-split-discovery | package_magic | NWPCF001 | package magic |
| time-split-discovery | verifier_events | 770 | report parsed_verifier_events |
| time-split-discovery | train_events | 616 | report train_events |
| time-split-discovery | shadow_events | 154 | report heldout_events |
| time-split-discovery | split_granularity | event_timestamp_older_train_newer_shadow | report split_granularity |
| time-split-discovery | train_shadow_time_order | true | report train_heldout_time_order_ok |
| time-split-discovery | train_time_max | 2026-05-02T23:48:06.646Z | report train_time_max |
| time-split-discovery | shadow_time_min | 2026-05-02T23:48:44.353Z | report heldout_time_min |
| time-split-discovery | accuracy_milli | 1000 | report heldout_accuracy_milli |
| time-split-discovery | wrong_wins | 0 | report wrong_wins |
| time-split-discovery | false_accepts | 0 | report false_accepts |
| time-split-discovery | parity_mismatches | 0 | report runtime_margin_parity_mismatches |
| time-split-discovery | unique_cpu_over_exact | 53 | report unique_cpu_accepts_over_exact_cache |
| time-split-discovery | token_ceiling | 53390 | report nando_cpu_tokens_saved |
| time-split-discovery | money_claim | 0 | report nando_cpu_cost_saved_microusd |
| time-split-discovery | quarantine | true | report quarantine_only |
| time-split-discovery | promoted | false | report promoted |
| time-split-discovery | product_accept | false | report local_accept_enabled |
| time-split-discovery | market_claim | false | report market_money_claim_allowed |
| time-split-discovery | forbidden_legacy_backend | false | report forbidden_flags.legacy_backend_used |
| time-split-discovery | forbidden_target_authority | false | report target/proof/concrete/local_out_t flags |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| time-split-discovery | verifier_events | 770 | report parsed_verifier_events |
| time-split-discovery | train_events | 616 | report train_events |
| time-split-discovery | shadow_events | 154 | report heldout_events |
| time-split-discovery | train_shadow_time_order | true | report train_heldout_time_order_ok |
| time-split-discovery | accuracy_milli | 1000 | report heldout_accuracy_milli |
| time-split-discovery | wrong_wins | 0 | report wrong_wins |
| time-split-discovery | false_accepts | 0 | report false_accepts |
| time-split-discovery | parity_mismatches | 0 | report runtime_margin_parity_mismatches |
| time-split-discovery | unique_cpu_over_exact | 53 | report unique_cpu_accepts_over_exact_cache |
| time-split-discovery | quarantine | true | report quarantine_only |
| time-split-discovery | promoted | false | report promoted |
| time-split-discovery | product_accept | false | report local_accept_enabled |
| time-split-discovery | market_claim | false | report market_money_claim_allowed |
| time-split-discovery | forbidden_legacy_backend | false | report forbidden_flags.legacy_backend_used |
