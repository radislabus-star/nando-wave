# NANDA Triad Worksheet

task_id: s1c3-v4-measurement
domain: general
query: Does S1C-3 V4 correct the durability measurement chronology without weakening runtime durability or deployment authority?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | production durability | has_two_stages | pre-action precommit and post-action settlement | runtime chronology | 1.0 | runtime path | measured stages | measurement | s1c3-v4 | runtime | S1C3 V4 owner | final freeze | two critical sections | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4.md:9 | S1C-3 V4 |
| s2 | each real critical stage | must_pass | p99 5 ms and hard max 20 ms | frozen threshold | 1.0 | measured stage | authority bound | measurement | s1c3-v4 | proof | S1C3 V4 owner | final freeze | PASS 3/3 or terminal | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4.md:64 | S1C-3 V4 |
| s3 | aggregate episode cost | remains_bound_by | diagnostic p99 and hard max 20 ms | anti-hiding boundary | 1.0 | total storage cost | retained evidence | measurement | s1c3-v4 | proof | S1C3 V4 owner | final freeze | visible aggregate | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4.md:74 | S1C-3 V4 |
| s4 | V4 candidate | may_change_only | cfg(test) measurement with identical release binary | runtime preservation | 1.0 | candidate source | production artifact | candidate | s1c3-v4 | build | S1C3 V4 owner | final freeze | no runtime delta | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4.md:32 | S1C-3 V4 |
| s5 | V4 evidence | must_be_fresh_from | new checkout targets harnesses and quiescence | attempt isolation | 1.0 | evidence set | V4 transaction | provenance | s1c3-v4 | build | S1C3 V4 owner | final freeze | no V3 reuse | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4.md:86 | S1C-3 V4 |
| s6 | final V4 paper | authorizes | exactly one terminal transaction | attempt boundary | 1.0 | paper authority | V4 attempt | authority | s1c3-v4 | deployment | S1C3 V4 owner | final freeze | one attempt | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4.md:93 | S1C-3 V4 |
| s7 | V4 deployment PASS | cannot_prove | natural decision episode grounded meaning or K2 | claim boundary | 1.0 | operational result | scientific claim | epistemic | s1c3-v4 | science | S1C3 V4 owner | final freeze | K2 remains blocked | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4.md:104 | S1C-3 V4 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | production durability | has_two_stages | pre-action precommit and post-action settlement | accepted chronology repair | 1.0 | runtime path | measured stages | measurement | s1c3-v4 | runtime | S1C3 V4 owner | final freeze | two critical sections | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4_CRITIQUE.md:9 | S1C-3 V4 |
| c2 | each real critical stage | must_pass | p99 5 ms and hard max 20 ms | accepted threshold repair | 1.0 | measured stage | authority bound | measurement | s1c3-v4 | proof | S1C3 V4 owner | final freeze | PASS 3/3 or terminal | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4_CRITIQUE.md:9 | S1C-3 V4 |
| c3 | aggregate episode cost | remains_bound_by | diagnostic p99 and hard max 20 ms | accepted anti-hiding repair | 1.0 | total storage cost | retained evidence | measurement | s1c3-v4 | proof | S1C3 V4 owner | final freeze | visible aggregate | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4_CRITIQUE.md:10 | S1C-3 V4 |
| c4 | V4 candidate | may_change_only | cfg(test) measurement with identical release binary | accepted runtime-preservation repair | 1.0 | candidate source | production artifact | candidate | s1c3-v4 | build | S1C3 V4 owner | final freeze | no runtime delta | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4_CRITIQUE.md:11 | S1C-3 V4 |
| c5 | V4 evidence | must_be_fresh_from | new checkout targets harnesses and quiescence | accepted no-reuse repair | 1.0 | evidence set | V4 transaction | provenance | s1c3-v4 | build | S1C3 V4 owner | final freeze | no V3 reuse | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4_CRITIQUE.md:12 | S1C-3 V4 |
| c6 | final V4 paper | authorizes | exactly one terminal transaction | accepted retry rejection | 1.0 | paper authority | V4 attempt | authority | s1c3-v4 | deployment | S1C3 V4 owner | final freeze | one attempt | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4_CRITIQUE.md:27 | S1C-3 V4 |
| c7 | V4 deployment PASS | cannot_prove | natural decision episode grounded meaning or K2 | accepted claim-boundary repair | 1.0 | operational result | scientific claim | epistemic | s1c3-v4 | science | S1C3 V4 owner | final freeze | K2 remains blocked | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V4_CRITIQUE.md:15 | S1C-3 V4 |

## notes

- Structural PASS is coherence-only and must retain authority_ready false.
