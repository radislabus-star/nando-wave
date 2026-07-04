# Style Brevity Evidence Hook V1

This packet checks one narrow claim boundary: `style_brevity` has verifier
evidence, but the current evidence is negative, so it must not be promoted to
local accept or counted as CPU savings.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | style_brevity_verifier | verified_true_events | 0 | style-brevity-output-evidence-v1.verification-hook-audit.report.json | 0.99 | deterministic verifier | measured value | style_brevity | audit |
| t2 | style_brevity_verifier | verified_false_events | 1 | style-brevity-output-evidence-v1.verification-hook-audit.report.json | 0.99 | deterministic verifier | measured value | style_brevity | audit |
| t3 | style_brevity_audit | verified_cpu_accept_eligible_events | 0 | style-brevity-output-evidence-v1.verification-hook-audit.report.json | 0.99 | verification audit | measured value | style_brevity | audit |
| t4 | style_brevity_shadow | false_accepts | 0 | style-brevity-output-evidence-v1.shadow-report.json | 0.99 | shadow report | safety metric | style_brevity | shadow |
| t5 | style_brevity_catalog_row | style_brevity_verifier_true_support_zero | true | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | blocker state | style_brevity | catalog |
| t6 | style_brevity_catalog_row | priority_rank | 23 | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | measured value | style_brevity | catalog |
| t7 | style_brevity_decision | may_run_local_accept_calibration_now | false | docs/EXECUTOR_REVIEW_NOTES.md Style Brevity Evidence Hook V1 | 0.99 | engineering decision | permission state | style_brevity | decision |
| t8 | cpu_routability_current_state | market_claim_allowed | false | docs/EXECUTOR_REVIEW_NOTES.md Style Brevity Evidence Hook V1 | 0.99 | global CPU80 state | claim boundary | feedback_loop | current_state |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| c1 | style_brevity_verifier | verified_true_events | 0 | style-brevity-output-evidence-v1.verification-hook-audit.report.json | 0.99 | deterministic verifier | measured value | style_brevity | audit |
| c2 | style_brevity_verifier | verified_false_events | 1 | style-brevity-output-evidence-v1.verification-hook-audit.report.json | 0.99 | deterministic verifier | measured value | style_brevity | audit |
| c3 | style_brevity_audit | verified_cpu_accept_eligible_events | 0 | style-brevity-output-evidence-v1.verification-hook-audit.report.json | 0.99 | verification audit | measured value | style_brevity | audit |
| c4 | style_brevity_shadow | false_accepts | 0 | style-brevity-output-evidence-v1.shadow-report.json | 0.99 | shadow report | safety metric | style_brevity | shadow |
| c5 | style_brevity_catalog_row | style_brevity_verifier_true_support_zero | true | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | blocker state | style_brevity | catalog |
| c6 | style_brevity_catalog_row | priority_rank | 23 | cpu-operator-catalog-v1.report.json | 0.99 | catalog row | measured value | style_brevity | catalog |
| c7 | style_brevity_decision | may_run_local_accept_calibration_now | false | docs/EXECUTOR_REVIEW_NOTES.md Style Brevity Evidence Hook V1 | 0.99 | engineering decision | permission state | style_brevity | decision |
| c8 | cpu_routability_current_state | market_claim_allowed | false | docs/EXECUTOR_REVIEW_NOTES.md Style Brevity Evidence Hook V1 | 0.99 | global CPU80 state | claim boundary | feedback_loop | current_state |
