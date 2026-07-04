# Git Control Request Side Safe Subfamily V1

## query

Verify that the current5k git_control admission audit found only a tiny
request-side safe candidate, that no CPU savings or local accepts are counted
from it yet, and that the next action is a separate promoted shadow/audit with
workspace mutations still disabled.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| git_control_admission_audit_current5k | reads_registry | profile-registry-git-control-v1-current5k.json | `git-control-admission-audit-v1-current5k.report.json.registry_config_path` |
| git_control_admission_audit_current5k | reads_trace | git-control-output-evidence-v1-current5k.trace.jsonl | `git-control-admission-audit-v1-current5k.report.json.evidence_trace_path` |
| git_control_admission_audit_current5k | scoreable_candidate_rows | 90 | `git-control-admission-audit-v1-current5k.report.json.scoreable_candidate_rows` |
| git_control_admission_audit_current5k | hook_ready_rows | 74 | `git-control-admission-audit-v1-current5k.report.json.hook_ready_rows` |
| git_control_admission_audit_current5k | label_true_rows | 35 | `git-control-admission-audit-v1-current5k.report.json.label_true_rows` |
| git_control_admission_audit_current5k | label_false_rows | 39 | `git-control-admission-audit-v1-current5k.report.json.label_false_rows` |
| git_control_admission_audit_current5k | unverified_rows | 16 | `git-control-admission-audit-v1-current5k.report.json.unverified_rows` |
| git_control_admission_audit_current5k | safe_policy_found | true | `git-control-admission-audit-v1-current5k.report.json.safe_policy_found` |
| git_control_admission_audit_current5k | best_safe_true_accepts | 3 | `git-control-admission-audit-v1-current5k.report.json.best_safe_true_accepts` |
| git_control_safe_policy_candidate | feature_conjunction | no_mutation_verbs_AND_has_push_terms | `git-control-admission-audit-v1-current5k.report.json.safe_policy_candidates[0].request_feature_conjunction` |
| git_control_safe_policy_candidate | energy_threshold | 1386496 | `git-control-admission-audit-v1-current5k.report.json.safe_policy_candidates[0].energy_threshold` |
| git_control_safe_policy_candidate | true_accepts | 3 | `git-control-admission-audit-v1-current5k.report.json.safe_policy_candidates[0].true_accepts` |
| git_control_safe_policy_candidate | false_accepts | 0 | `git-control-admission-audit-v1-current5k.report.json.safe_policy_candidates[0].false_accepts` |
| git_control_safe_policy_candidate | unverified_accepts | 0 | `git-control-admission-audit-v1-current5k.report.json.safe_policy_candidates[0].unverified_accepts` |
| cpu_catalog_current5k | current_incremental_unique_accepts_over_exact_cache | 104 | `cpu-operator-catalog-v1-current5k.combined.report.json.current_incremental_unique_cpu_accepts_over_exact_cache` |
| cpu_catalog_git_control_existing_profile | current_status | CANDIDATE | `cpu-operator-catalog-v1-current5k.combined.report.json.rows[git_control existing_profile_route].current_status` |
| cpu_catalog_git_control_existing_profile | expected_unique_cpu_accepts_over_exact_cache | 0 | `cpu-operator-catalog-v1-current5k.combined.report.json.rows[git_control existing_profile_route].expected_unique_cpu_accepts_over_exact_cache` |
| cpu_catalog_git_control_existing_profile | git_control_admission_best_safe_true_accepts | 3 | `cpu-operator-catalog-v1-current5k.combined.report.json.rows[git_control existing_profile_route].git_control_admission_best_safe_true_accepts` |
| cpu_catalog_git_control_existing_profile | false_accept_risk | MEDIUM_VERIFIER_READY_POLICY_PENDING_PROMOTE | `cpu-operator-catalog-v1-current5k.combined.report.json.rows[git_control existing_profile_route].false_accept_risk` |
| cpu_catalog_git_control_existing_profile | business_value_gate_failure_reason | expected_unique_cpu_accepts_zero_no_safe_local_accept_policy | `cpu-operator-catalog-v1-current5k.combined.report.json.rows[git_control existing_profile_route].business_value_gate_failure_reason` |
| cpu_catalog_git_control_gap_family | current_status | WATCH | `cpu-operator-catalog-v1-current5k.combined.report.json.rows[git_control route_gap_family].current_status` |
| cpu_catalog_git_control_gap_family | business_value_gate_failure_reason | expected_unique_cpu_accepts_zero | `cpu-operator-catalog-v1-current5k.combined.report.json.rows[git_control route_gap_family].business_value_gate_failure_reason` |
| promotion_boundary | local_accepts_enabled | false | `docs/CPU_CALL_CATALOG.md` |
| promotion_boundary | market_claim_allowed | false | `docs/CPU_CALL_CATALOG.md` |
| promotion_boundary | workspace_mutations_enabled | false | `docs/EXECUTOR_REVIEW_NOTES.md` |
| next_git_control_step | action | promoted_shadow_trace_registry_then_audit_feedback | `docs/EXECUTOR_REVIEW_NOTES.md` |
