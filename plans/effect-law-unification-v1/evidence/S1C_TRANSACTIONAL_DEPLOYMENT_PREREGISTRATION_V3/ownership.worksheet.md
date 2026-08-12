# NANDA Triad Worksheet

task_id: s1c3-v3-ownership
domain: general
query: Does S1C-3 V3 repair only oracle workspace ownership while preserving the frozen attempt and claim boundaries?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | V3 oracle workspace | is_owned_by | build user e with bounded modes | ownership contract | 1.0 | build workspace | build principal | ownership | s1c3-v3-ownership | filesystem | S1C3 V3 owner | workspace creation | bounded writable workspace | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3.md:24 | S1C-3 V3 |
| s2 | V3 ownership probe | is_executed_by | same user e before Cargo | principal parity | 1.0 | write probe | build principal | ownership | s1c3-v3-ownership | verification | S1C3 V3 owner | prebuild | PASS or terminal failure | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3.md:59 | S1C-3 V3 |
| s3 | V3 ownership probe | is_removed_and_fsynced_before | oracle build | workspace identity | 1.0 | temporary probe | oracle input | ownership | s1c3-v3-ownership | persistence | S1C3 V3 owner | probe cleanup | clean oracle input | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3.md:73 | S1C-3 V3 |
| s4 | V3 artifacts | are_fresh_from | new checkout and target directories | attempt isolation | 1.0 | V3 artifact set | V3 transaction | ownership | s1c3-v3-ownership | provenance | S1C3 V3 owner | new transaction | no V2 reuse | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3.md:84 | S1C-3 V3 |
| s5 | V3 deployment PASS | cannot_prove | natural decision episode or K2 | claim boundary | 1.0 | operational result | scientific claim | ownership | s1c3-v3-ownership | epistemic | S1C3 V3 owner | deployment receipt | K2 blocked | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3.md:119 | S1C-3 V3 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | V3 oracle workspace | is_owned_by | build user e with bounded modes | critique directory ownership repair | 1.0 | build workspace | build principal | ownership | s1c3-v3-ownership | filesystem | S1C3 V3 owner | workspace creation | bounded writable workspace | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3_CRITIQUE.md:11 | S1C-3 V3 |
| c2 | V3 ownership probe | is_executed_by | same user e before Cargo | critique principal repair | 1.0 | write probe | build principal | ownership | s1c3-v3-ownership | verification | S1C3 V3 owner | prebuild | PASS or terminal failure | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3_CRITIQUE.md:12 | S1C-3 V3 |
| c3 | V3 ownership probe | is_removed_and_fsynced_before | oracle build | critique cleanup repair | 1.0 | temporary probe | oracle input | ownership | s1c3-v3-ownership | persistence | S1C3 V3 owner | probe cleanup | clean oracle input | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3_CRITIQUE.md:14 | S1C-3 V3 |
| c4 | V3 artifacts | are_fresh_from | new checkout and target directories | critique no-reuse repair | 1.0 | V3 artifact set | V3 transaction | ownership | s1c3-v3-ownership | provenance | S1C3 V3 owner | new transaction | no V2 reuse | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3_CRITIQUE.md:13 | S1C-3 V3 |
| c5 | V3 deployment PASS | cannot_prove | natural decision episode or K2 | critique claim-boundary repair | 1.0 | operational result | scientific claim | ownership | s1c3-v3-ownership | epistemic | S1C3 V3 owner | deployment receipt | K2 blocked | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V3_CRITIQUE.md:17 | S1C-3 V3 |

## notes

- Structural PASS is coherence-only and must retain authority_ready false.
