# NANDA Triad Worksheet

task_id: s1c3-v2-authority
domain: general
query: Does S1C-3 V2 keep transaction ownership, rollback, and scientific claim authority separated?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | S1C3 V2 transaction | changes_pid_of | transition-serving only | sole-owner boundary | 1.0 | deployment transaction | runtime owner | authority | s1c3-v2-authority | runtime | S1C3 transaction owner | stop swap start | one PID change | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:289 | S1C-3 V2 |
| s2 | S1C3 V2 rollback | restores | exact old binary and config while preserving journal prefixes | recovery boundary | 1.0 | rollback route | old runtime and evidence | authority | s1c3-v2-authority | recovery | S1C3 transaction owner | rollback | restored service | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:292 | S1C-3 V2 |
| s3 | S1C3 V2 operational PASS | cannot_prove | natural decision episode or K2 | claim boundary | 1.0 | operational evidence | scientific claim | authority | s1c3-v2-authority | epistemic | S1C3 transaction owner | deployment receipt | K2 blocked | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:300 | S1C-3 V2 |
| s4 | empty grounded decision journal | remains | valid deployment state without targeted traffic | natural-evidence boundary | 1.0 | capture state | deployment verdict | authority | s1c3-v2-authority | epistemic | S1C3 transaction owner | post-start check | no false claim | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:313 | S1C-3 V2 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C3 V2 transaction | changes_pid_of | transition-serving only | critique owner-isolation repair | 1.0 | deployment transaction | runtime owner | authority | s1c3-v2-authority | runtime | S1C3 transaction owner | stop swap start | one PID change | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:29 | S1C-3 V2 |
| c2 | S1C3 V2 rollback | restores | exact old binary and config while preserving journal prefixes | inherited V1 boundary | 1.0 | rollback route | old runtime and evidence | authority | s1c3-v2-authority | recovery | S1C3 transaction owner | rollback | restored service | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:9 | S1C-3 V2 |
| c3 | S1C3 V2 operational PASS | cannot_prove | natural decision episode or K2 | critique claim-authority repair | 1.0 | operational evidence | scientific claim | authority | s1c3-v2-authority | epistemic | S1C3 transaction owner | deployment receipt | K2 blocked | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:31 | S1C-3 V2 |
| c4 | empty grounded decision journal | remains | valid deployment state without targeted traffic | critique no-dashboard boundary | 1.0 | capture state | deployment verdict | authority | s1c3-v2-authority | epistemic | S1C3 transaction owner | post-start check | no false claim | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:56 | S1C-3 V2 |

## notes

- Structural PASS is coherence-only and must retain authority_ready false.
