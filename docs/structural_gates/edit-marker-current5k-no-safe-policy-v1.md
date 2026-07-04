# Edit Marker Current5k No Safe Policy V1

## query

Verify the current5k edit_marker_length boundary: the route has real traffic
and some verifier evidence, but no safe readout or admission policy, so it must
stay WATCH and must not be counted as CPU savings.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| edit_marker_current5k | candidate_events | 505 | `edit-payload-dry-run-v1-current5k.report.json` |
| edit_marker_current5k | scoreable_payload_events | 50 | `edit-payload-dry-run-v1-current5k.report.json` |
| edit_marker_current5k | output_evidence_matched_events | 42 | `edit-output-evidence-v1-current5k.report.json` |
| edit_marker_current5k | verified_true_events | 14 | `edit-output-evidence-v1-current5k.report.json` |
| edit_marker_current5k | verified_false_events | 28 | `edit-output-evidence-v1-current5k.report.json` |
| edit_marker_current5k | local_readout_safe_policy_found | false | `edit-local-accept-calibration-v1-current5k.report.json` |
| edit_marker_current5k | request_admission_safe_policy_found | false | `edit-admission-calibration-v1-current5k.report.json` |
| edit_marker_current5k | shadow_accepts | 0 | `edit-output-evidence-v1-current5k.shadow-report.json` |
| edit_marker_current5k | false_accepts | 0 | `edit-output-evidence-v1-current5k.shadow-report.json` |
| edit_marker_current5k | verified_cpu_accept_eligible_events | 0 | `edit-output-evidence-v1-current5k.verification-hook-audit.report.json` |
| edit_marker_current5k | shelf | WATCH_NO_SAFE_POLICY_CURRENT5K | `docs/CPU_CALL_CATALOG.md` |
| edit_marker_current5k | savings_policy | do_not_count | `docs/CPU_CALL_CATALOG.md` |
