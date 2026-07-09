# NANDA Task: phase-stream-live-admission-policy-smoke-evidence

## query

Check that the live admission policy smoke consumes the .nwpc manifest, verifies
the package file still matches the manifest, and preserves zero false accepts.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_admission_policy_smoke | reads | live_admission_manifest | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#manifest_report_path |
| live_admission_policy_smoke | inspects | candidate_nwpc_package | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#candidate_package_path |
| live_admission_policy_smoke | verifies | package_file_matches_manifest | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#package_file_matches_manifest |
| live_admission_policy_smoke | requires | manifest_live_accept_eligible | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_accept_guard.manifest_live_accept_eligible |
| live_admission_policy_smoke | requires | verifier_bound_profile_loaded | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_accept_guard.verifier_bound_profile_loaded |
| live_admission_policy_smoke | reports | would_local_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#would_local_accepts_over_exact_cache |
| live_admission_policy_smoke | reports_zero | false_accepts | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#false_accepts |
| live_admission_policy_smoke | reports_zero | wrong_wins | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#wrong_wins |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_admission_policy_smoke | reads | live_admission_manifest | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#manifest_report_path |
| live_admission_policy_smoke | inspects | candidate_nwpc_package | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#candidate_package_path |
| live_admission_policy_smoke | verifies | package_file_matches_manifest | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#package_file_matches_manifest |
| live_admission_policy_smoke | requires | manifest_live_accept_eligible | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_accept_guard.manifest_live_accept_eligible |
| live_admission_policy_smoke | requires | verifier_bound_profile_loaded | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#policy_accept_guard.verifier_bound_profile_loaded |
| live_admission_policy_smoke | reports | would_local_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#would_local_accepts_over_exact_cache |
| live_admission_policy_smoke | reports_zero | false_accepts | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#false_accepts |
| live_admission_policy_smoke | reports_zero | wrong_wins | target/nando-wave/streaming/phase-atom-live-admission-policy-smoke-v1.report.json#wrong_wins |
