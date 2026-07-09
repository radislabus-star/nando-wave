# NANDA Task: phase-stream-live-admission-policy-smoke-boundary

## query

Check that the live admission policy smoke is only a shadow daemon admission
decision and does not enable product accept, runtime mutation, promotion, or
market money claims.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_admission_policy_smoke | decides | would_admit_shadow_only | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_decision |
| live_admission_policy_smoke | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#local_accept_enabled |
| live_admission_policy_smoke | keeps_disabled | product_runtime_mutation | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#product_runtime_changed |
| live_admission_policy_smoke | keeps_disabled | serving_runtime_mutation | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#serving_runtime_changed |
| live_admission_policy_smoke | keeps_disabled | package_promotion | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#promoted |
| live_admission_policy_smoke | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#market_money_claim_allowed |
| live_admission_policy_smoke | blocks_money_claim_on | provider_cost_missing | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_accept_guard.provider_cost_missing_blocks_money_claim |
| live_admission_policy_smoke | keeps_clear | forbidden_flags | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_accept_guard.forbidden_flags_clear |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_admission_policy_smoke | decides | would_admit_shadow_only | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_decision |
| live_admission_policy_smoke | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#local_accept_enabled |
| live_admission_policy_smoke | keeps_disabled | product_runtime_mutation | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#product_runtime_changed |
| live_admission_policy_smoke | keeps_disabled | serving_runtime_mutation | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#serving_runtime_changed |
| live_admission_policy_smoke | keeps_disabled | package_promotion | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#promoted |
| live_admission_policy_smoke | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#market_money_claim_allowed |
| live_admission_policy_smoke | blocks_money_claim_on | provider_cost_missing | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_accept_guard.provider_cost_missing_blocks_money_claim |
| live_admission_policy_smoke | keeps_clear | forbidden_flags | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_accept_guard.forbidden_flags_clear |
