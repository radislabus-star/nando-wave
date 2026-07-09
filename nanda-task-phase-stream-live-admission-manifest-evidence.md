# NANDA Task: phase-stream-live-admission-manifest-evidence

## query

Check that the .nwpc live admission manifest is backed by matching package
evidence, verifier-bound profile loading, fresh shadow replay, and zero
false accepts.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_admission_manifest | reads | serving_admission_report | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#admission_report_path |
| live_admission_manifest | reads | fresh_append_shadow_replay_report | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#shadow_replay_report_path |
| live_admission_manifest | verifies | package_matches_admission_report | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#package.package_matches_admission_report |
| live_admission_manifest | verifies | package_matches_shadow_report | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#package.package_matches_shadow_report |
| live_admission_manifest | requires | verifier_bound_profile_loaded | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.verifier_bound_profile_loaded |
| live_admission_manifest | reports | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.unique_cpu_accepts_over_exact_cache |
| live_admission_manifest | reports_zero | false_accepts | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.false_accepts |
| live_admission_manifest | reports_zero | wrong_wins | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.wrong_wins |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_admission_manifest | reads | serving_admission_report | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#admission_report_path |
| live_admission_manifest | reads | fresh_append_shadow_replay_report | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#shadow_replay_report_path |
| live_admission_manifest | verifies | package_matches_admission_report | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#package.package_matches_admission_report |
| live_admission_manifest | verifies | package_matches_shadow_report | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#package.package_matches_shadow_report |
| live_admission_manifest | requires | verifier_bound_profile_loaded | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.verifier_bound_profile_loaded |
| live_admission_manifest | reports | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.unique_cpu_accepts_over_exact_cache |
| live_admission_manifest | reports_zero | false_accepts | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.false_accepts |
| live_admission_manifest | reports_zero | wrong_wins | target/nando-wave/streaming/phase-atom-live-admission-manifest-v1.report.json#evidence_gate.wrong_wins |
