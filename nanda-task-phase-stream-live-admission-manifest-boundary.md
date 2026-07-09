# NANDA Task: phase-stream-live-admission-manifest-boundary

## query

Check that the .nwpc live admission manifest marks only next-step eligibility
and keeps product local_accept, runtime mutation, market money claim, and legacy
backend use disabled.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_admission_manifest | marks | live_accept_eligible_next_step_only | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#live_accept_recommendation |
| live_admission_manifest | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#local_accept_enabled |
| live_admission_manifest | keeps_disabled | product_promotion | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#product_promotion_allowed |
| live_admission_manifest | keeps_disabled | serving_profile_artifact | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#serving_profile_artifact |
| live_admission_manifest | keeps_disabled | runtime_mutation | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#serving_runtime_changed |
| live_admission_manifest | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#market_money_claim_allowed |
| live_admission_manifest | keeps_false | provider_cost_evidence_present | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.provider_cost_evidence_present |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_admission_manifest | marks | live_accept_eligible_next_step_only | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#live_accept_recommendation |
| live_admission_manifest | keeps_disabled | product_local_accept | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#local_accept_enabled |
| live_admission_manifest | keeps_disabled | product_promotion | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#product_promotion_allowed |
| live_admission_manifest | keeps_disabled | serving_profile_artifact | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#serving_profile_artifact |
| live_admission_manifest | keeps_disabled | runtime_mutation | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#serving_runtime_changed |
| live_admission_manifest | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#market_money_claim_allowed |
| live_admission_manifest | keeps_false | provider_cost_evidence_present | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.provider_cost_evidence_present |
| legacy_nwrb_backend | remains | forbidden_guard_only | crates/nando-cli/src/main.rs#FORBIDDEN_LEGACY_NWRB_BACKEND |
