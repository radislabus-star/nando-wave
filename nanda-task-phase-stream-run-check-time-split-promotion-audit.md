# NANDA Task: Phase Stream Run-Check Time-Split Promotion Audit

## Query

Verify that the run_check time-split promotion audit accepts only a quarantine
promotion candidate, does not enable product local_accept, does not allow a
market money claim, and keeps the legacy `.nwrb` / role-binding backend
forbidden.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| promotion-audit | artifact | phase-atom-run-check-time-split-promotion-audit-v1.report.json | audit report path |
| promotion-audit | source_report | phase-atom-run-check-time-split-discovery-v1.report.json | discovery_report_path |
| promotion-audit | package | phase-atom-run-check-time-split-discovery-v1.candidate.nwpc | candidate_package_path |
| promotion-audit | package_magic_ok | true | package.package_magic_ok |
| promotion-audit | package_fingerprint_match | true | package.inspect_matches_discovery_report |
| promotion-audit | time_order | true | discovery_gate.train_heldout_time_order_ok |
| promotion-audit | false_accepts | 0 | discovery_gate.false_accepts |
| promotion-audit | wrong_wins | 0 | discovery_gate.wrong_wins |
| promotion-audit | parity_mismatches | 0 | discovery_gate.runtime_margin_parity_mismatches |
| promotion-audit | unique_cpu_over_exact | 53 | discovery_gate.unique_cpu_accepts_over_exact_cache |
| promotion-audit | token_saving | 53390 | economics.nando_cpu_tokens_saved |
| promotion-audit | provider_cost_evidence | false | economics.provider_cost_evidence_present |
| promotion-audit | model_price_estimate | true | economics.explicit_model_price_estimate_used |
| promotion-audit | estimated_cost_saving | 53390 | economics.estimated_nando_cpu_cost_saved_microusd |
| promotion-audit | money_claim_blocker | provider_cost_missing_internal_price_estimate_only | economics.money_claim_blocker |
| promotion-audit | promotion_candidate | true | promotion_candidate_allowed |
| promotion-audit | product_promotion | false | product_promotion_allowed |
| promotion-audit | local_accept | false | local_accept_enabled |
| promotion-audit | market_money_claim | false | market_money_claim_allowed |
| promotion-audit | forbidden_legacy_backend | false | forbidden_flags.legacy_backend_used |
| promotion-audit | forbidden_target_authority | false | target/proof/concrete/local_out_t/bind flags |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| promotion-audit | package_magic_ok | true | package.package_magic_ok |
| promotion-audit | package_fingerprint_match | true | package.inspect_matches_discovery_report |
| promotion-audit | time_order | true | discovery_gate.train_heldout_time_order_ok |
| promotion-audit | false_accepts | 0 | discovery_gate.false_accepts |
| promotion-audit | wrong_wins | 0 | discovery_gate.wrong_wins |
| promotion-audit | parity_mismatches | 0 | discovery_gate.runtime_margin_parity_mismatches |
| promotion-audit | unique_cpu_over_exact | 53 | discovery_gate.unique_cpu_accepts_over_exact_cache |
| promotion-audit | token_saving | 53390 | economics.nando_cpu_tokens_saved |
| promotion-audit | provider_cost_evidence | false | economics.provider_cost_evidence_present |
| promotion-audit | model_price_estimate | true | economics.explicit_model_price_estimate_used |
| promotion-audit | estimated_cost_saving | 53390 | economics.estimated_nando_cpu_cost_saved_microusd |
| promotion-audit | money_claim_blocker | provider_cost_missing_internal_price_estimate_only | economics.money_claim_blocker |
| promotion-audit | promotion_candidate | true | promotion_candidate_allowed |
| promotion-audit | product_promotion | false | product_promotion_allowed |
| promotion-audit | local_accept | false | local_accept_enabled |
| promotion-audit | market_money_claim | false | market_money_claim_allowed |
| promotion-audit | forbidden_legacy_backend | false | forbidden_flags.legacy_backend_used |
| promotion-audit | forbidden_target_authority | false | target/proof/concrete/local_out_t/bind flags |
