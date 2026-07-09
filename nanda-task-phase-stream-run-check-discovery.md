# NANDA Task: Phase Stream Run-Check Discovery

## Query

Verify the compact structure of the run_check phase-stream rung:
real Codex session verifier labels build a quarantine `.nwpc` candidate, while
promotion, product local_accept, market money claim, target/proof authority,
and legacy backend stay disabled.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| verifier-trace | artifact | codex-session-run-check-verifier-trace-v1.report.json | trace-report path |
| verifier-trace | jsonl | codex-session-run-check-verifier-trace-v1.jsonl | trace-jsonl path |
| verifier-trace | rows | 770 | trace-report rows_written |
| verifier-trace | pass | 657 | trace-report pass_rows |
| verifier-trace | negative | 113 | trace-report fail+compile+panic |
| verifier-trace | raw_tool_output | false | trace-report raw_tool_output_written |
| verifier-trace | raw_request_text | false | trace-report raw_request_text_written |
| verifier-trace | raw_response_text | false | trace-report raw_response_text_written |
| verifier-trace | product_accept | false | trace-report local_accept_enabled |
| verifier-trace | market_claim | false | trace-report market_money_claim_allowed |
| discovery | artifact | phase-atom-run-check-discovery-v1.report.json | discovery-report path |
| discovery | package | phase-atom-run-check-discovery-v1.candidate.nwpc | discovery-package path |
| discovery | package_magic | NWPCF001 | package magic |
| discovery | verifier_events | 770 | discovery-report parsed_verifier_events |
| discovery | train | 617 | discovery-report train_events |
| discovery | heldout | 153 | discovery-report heldout_events |
| discovery | accuracy_milli | 1000 | discovery-report heldout_accuracy_milli |
| discovery | wrong_wins | 0 | discovery-report wrong_wins |
| discovery | false_accepts | 0 | discovery-report false_accepts |
| discovery | parity_mismatches | 0 | discovery-report runtime_margin_parity_mismatches |
| discovery | unique_cpu_over_exact | 59 | discovery-report unique_cpu_accepts_over_exact_cache |
| discovery | token_ceiling | 20638 | discovery-report nando_cpu_tokens_saved |
| discovery | money_claim | 0 | discovery-report nando_cpu_cost_saved_microusd |
| discovery | quarantine | true | discovery-report quarantine_only |
| discovery | promoted | false | discovery-report promoted |
| discovery | product_accept | false | discovery-report local_accept_enabled |
| discovery | market_claim | false | discovery-report market_money_claim_allowed |
| discovery | forbidden_legacy_backend | false | discovery-report forbidden_flags.legacy_backend_used |
| discovery | forbidden_target_authority | false | discovery-report target/proof/concrete/local_out_t flags |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| verifier-trace | rows | 770 | trace-report rows_written |
| verifier-trace | raw_tool_output | false | trace-report raw_tool_output_written |
| verifier-trace | product_accept | false | trace-report local_accept_enabled |
| discovery | verifier_events | 770 | discovery-report parsed_verifier_events |
| discovery | accuracy_milli | 1000 | discovery-report heldout_accuracy_milli |
| discovery | wrong_wins | 0 | discovery-report wrong_wins |
| discovery | false_accepts | 0 | discovery-report false_accepts |
| discovery | parity_mismatches | 0 | discovery-report runtime_margin_parity_mismatches |
| discovery | unique_cpu_over_exact | 59 | discovery-report unique_cpu_accepts_over_exact_cache |
| discovery | quarantine | true | discovery-report quarantine_only |
| discovery | promoted | false | discovery-report promoted |
| discovery | forbidden_legacy_backend | false | discovery-report forbidden_flags.legacy_backend_used |
