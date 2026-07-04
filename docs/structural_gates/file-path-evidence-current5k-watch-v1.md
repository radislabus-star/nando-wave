# File Path Evidence Current5k Watch V1

## query

Verify the current5k file_path_evidence_answer split boundary: it is a real
artifact-backed candidate split, but it has no robust safe policy and must stay
WATCH until more verifier-true evidence or a stricter subfamily exists.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| broad_split_current5k | sampled_llm_calls | 5000 | `broad-route-split-discovery-v1-current5k.report.json` |
| broad_split_current5k | top_split_key | file_path_evidence_answer | `broad-route-split-discovery-v1-current5k.report.json` |
| file_path_evidence_current5k | candidate_events | 146 | `file-path-evidence-payload-dry-run-v1-current5k.report.json` |
| file_path_evidence_current5k | non_exact_candidate_events | 144 | `file-path-evidence-payload-dry-run-v1-current5k.report.json` |
| file_path_evidence_current5k | payload_ready_events | 122 | `file-path-evidence-payload-dry-run-v1-current5k.report.json` |
| file_path_evidence_current5k | scoreable_payload_events | 44 | `file-path-evidence-payload-dry-run-v1-current5k.report.json` |
| file_path_evidence_current5k | profile_edges | 7 | `file-path-evidence-profile-v1-current5k.report.json` |
| file_path_evidence_current5k | disabled_threshold_accepts | 0 | `file-path-evidence-profile-v1-current5k.report.json` |
| file_path_evidence_current5k | output_evidence_matched_events | 39 | `file-path-evidence-output-evidence-v1-current5k.report.json` |
| file_path_evidence_current5k | verified_true_events | 16 | `file-path-evidence-output-evidence-v1-current5k.report.json` |
| file_path_evidence_current5k | verified_false_events | 23 | `file-path-evidence-output-evidence-v1-current5k.report.json` |
| file_path_evidence_current5k | robust_safe_policy_found | false | `file-path-evidence-admission-calibration-v1-current5k.report.json` |
| file_path_evidence_current5k | best_robust_true_accepts | 0 | `file-path-evidence-admission-calibration-v1-current5k.report.json` |
| file_path_evidence_current5k | singleton_safe_policy_found | true | `file-path-evidence-admission-calibration-v1-current5k.report.json` |
| file_path_evidence_current5k | best_singleton_true_accepts | 1 | `file-path-evidence-admission-calibration-v1-current5k.report.json` |
| file_path_evidence_current5k | verified_cpu_accept_eligible_events | 0 | `file-path-evidence-output-evidence-v1-current5k.verification-hook-audit.report.json` |
| file_path_evidence_current5k | shelf | WATCH_SINGLETON_ONLY | `docs/CPU_CALL_CATALOG.md` |
| file_path_evidence_current5k | promote_policy | forbidden_without_robust_policy | `docs/CPU_CALL_CATALOG.md` |
