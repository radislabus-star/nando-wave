# NANDA Task: phase-stream-live-daemon-shadow-gate-evidence

## query

Check that the live daemon shadow gate loads the exact .nwpc package from the
policy manifest, scores live trace rows, writes a decision log, and keeps zero
false accepts.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_daemon_shadow_gate | consumes | live_admission_policy_smoke | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#manifest_report_path |
| live_daemon_shadow_gate | loads | candidate_nwpc_package | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#profile.candidate_package_path |
| live_daemon_shadow_gate | verifies | package_file_matches_manifest | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#profile.package_file_matches_manifest |
| live_daemon_shadow_gate | reads | live_trace_rows | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#live_trace_path |
| live_daemon_shadow_gate | writes | shadow_decision_log | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#decision_log_path |
| shadow_decision_log | has_rows | routed_events | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#audit.decision_log_rows |
| live_daemon_shadow_gate | reports | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#audit.unique_cpu_accepts_over_exact_cache |
| live_daemon_shadow_gate | reports | safety_zero_counts | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#audit.false_accepts |
| live_daemon_shadow_gate | explicitly_tests_synthetic | fallback_probe | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#fallback_probe.probe_fell_back |
| live_daemon_shadow_gate | reports | process_rss_after_score | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#audit.process_rss_kib_after_score |
| fallback_probe | is_kind | synthetic_reversed_vector_probe | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#fallback_probe.probe_kind |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_daemon_shadow_gate | consumes | live_admission_policy_smoke | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#manifest_report_path |
| live_daemon_shadow_gate | loads | candidate_nwpc_package | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#profile.candidate_package_path |
| live_daemon_shadow_gate | verifies | package_file_matches_manifest | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#profile.package_file_matches_manifest |
| live_daemon_shadow_gate | reads | live_trace_rows | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#live_trace_path |
| live_daemon_shadow_gate | writes | shadow_decision_log | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#decision_log_path |
| shadow_decision_log | has_rows | routed_events | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#audit.decision_log_rows |
| live_daemon_shadow_gate | reports | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#audit.unique_cpu_accepts_over_exact_cache |
| live_daemon_shadow_gate | reports | safety_zero_counts | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#audit.false_accepts |
| live_daemon_shadow_gate | explicitly_tests_synthetic | fallback_probe | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#fallback_probe.probe_fell_back |
| live_daemon_shadow_gate | reports | process_rss_after_score | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#audit.process_rss_kib_after_score |
| fallback_probe | is_kind | synthetic_reversed_vector_probe | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#fallback_probe.probe_kind |
