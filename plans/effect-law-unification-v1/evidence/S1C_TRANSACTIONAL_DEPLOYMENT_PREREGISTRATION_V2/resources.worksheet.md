# NANDA Triad Worksheet

task_id: s1c3-v2-resources
domain: general
query: Does S1C-3 V2 preserve absolute resource denominators while running only preregistered binaries after quiescence?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | S1C3 V2 measurements | invoke_only | frozen direct executables after quiescence | direct execution route | 1.0 | measurement stage | executable set | resources | s1c3-v2-resources | measurement | S1C3 resource owner | taskset | raw metrics | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:210 | S1C-3 V2 |
| s2 | single-ledger durability | requires | p99 at most 5000000 ns PASS 3 of 3 | exact single-ledger limit | 1.0 | durability gate | absolute threshold | resources | s1c3-v2-resources | measurement | S1C3 resource owner | three direct runs | PASS or VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:237 | S1C-3 V2 |
| s3 | three-ledger durability | requires | p99 at most 5000000 ns PASS 3 of 3 | exact three-ledger limit | 1.0 | durability gate | absolute threshold | resources | s1c3-v2-resources | measurement | S1C3 resource owner | three direct runs | PASS or VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:239 | S1C-3 V2 |
| s4 | S1C3 V2 resource receipt | binds | quiescence and contamination roots | receipt composition | 1.0 | resource evidence | environment evidence | resources | s1c3-v2-resources | proof | S1C3 resource owner | receipt finalization | rooted evidence | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:253 | S1C-3 V2 |
| s5 | failed absolute metric | yields | terminal preflight failure without rerun | stop rule | 1.0 | failed measurement | terminal verdict | resources | s1c3-v2-resources | safety | S1C3 resource owner | verifier | terminal result | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:248 | S1C-3 V2 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C3 V2 measurements | invoke_only | frozen direct executables after quiescence | critique prebuild repair | 1.0 | measurement stage | executable set | resources | s1c3-v2-resources | measurement | S1C3 resource owner | taskset | raw metrics | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:18 | S1C-3 V2 |
| c2 | single-ledger durability | requires | p99 at most 5000000 ns PASS 3 of 3 | critique exact single-ledger limit | 1.0 | durability gate | absolute threshold | resources | s1c3-v2-resources | measurement | S1C3 resource owner | three direct runs | PASS or VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:80 | S1C-3 V2 |
| c3 | three-ledger durability | requires | p99 at most 5000000 ns PASS 3 of 3 | critique exact three-ledger limit | 1.0 | durability gate | absolute threshold | resources | s1c3-v2-resources | measurement | S1C3 resource owner | three direct runs | PASS or VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:81 | S1C-3 V2 |
| c4 | S1C3 V2 resource receipt | binds | quiescence and contamination roots | critique frozen receipt repair | 1.0 | resource evidence | environment evidence | resources | s1c3-v2-resources | proof | S1C3 resource owner | receipt finalization | rooted evidence | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:23 | S1C-3 V2 |
| c5 | failed absolute metric | yields | terminal preflight failure without rerun | critique optional-stopping repair | 1.0 | failed measurement | terminal verdict | resources | s1c3-v2-resources | safety | S1C3 resource owner | verifier | terminal result | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:20 | S1C-3 V2 |

## notes

- Structural PASS is coherence-only and must retain authority_ready false.
