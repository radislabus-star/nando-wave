# NANDA Triad Worksheet

task_id: s1c3-resources
domain: general
query: Does S1C-3 keep absolute resource denominators fixed and reject optional stopping or incomparable measurements?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | S1C3 hot latency gate | requires | exact absolute thresholds PASS 3 of 3 | frozen latency math | 1.0 | resource gate | absolute thresholds | resources | s1c3-resources | measurement | S1C3 resource owner | three fixed runs | PASS or VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:196 | S1C-3 |
| s2 | S1C3 durability gate | requires | exact sync thresholds PASS 3 of 3 | frozen durability math | 1.0 | resource gate | absolute thresholds | resources | s1c3-resources | measurement | S1C3 resource owner | three fixed runs | PASS or VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:206 | S1C-3 |
| s3 | S1C3 idle CPU gate | is_valid_only_with | unchanged isolated inputs and counters | frozen idle denominator | 1.0 | resource gate | comparable denominator | resources | s1c3-resources | validity | S1C3 resource owner | 60-second interval | valid observation | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:232 | S1C-3 |
| s4 | S1C3 RSS gate | compares_only | same fixture allocator authority warmup and schedule | frozen RSS denominator | 1.0 | resource gate | comparable denominator | resources | s1c3-resources | validity | S1C3 resource owner | isolated pair | valid delta | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:233 | S1C-3 |
| s5 | failed or incomparable resource evidence | yields | VETO or INVALID_ENVIRONMENT without rerun | stop rule | 1.0 | failed measurement | terminal verdict | resources | s1c3-resources | safety | S1C3 resource owner | verifier | terminal result | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:235 | S1C-3 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C3 hot latency gate | requires | exact absolute thresholds PASS 3 of 3 | critique optional-stopping repair | 1.0 | resource gate | absolute thresholds | resources | s1c3-resources | measurement | S1C3 resource owner | three fixed runs | PASS or VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:29 | S1C-3 |
| c2 | S1C3 durability gate | requires | exact sync thresholds PASS 3 of 3 | critique optional-stopping repair | 1.0 | resource gate | absolute thresholds | resources | s1c3-resources | measurement | S1C3 resource owner | three fixed runs | PASS or VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:29 | S1C-3 |
| c3 | S1C3 idle CPU gate | is_valid_only_with | unchanged isolated inputs and counters | critique idle repair | 1.0 | resource gate | comparable denominator | resources | s1c3-resources | validity | S1C3 resource owner | 60-second interval | valid observation | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:34 | S1C-3 |
| c4 | S1C3 RSS gate | compares_only | same fixture allocator authority warmup and schedule | critique RSS repair | 1.0 | resource gate | comparable denominator | resources | s1c3-resources | validity | S1C3 resource owner | isolated pair | valid delta | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:33 | S1C-3 |
| c5 | failed or incomparable resource evidence | yields | VETO or INVALID_ENVIRONMENT without rerun | critique no-rerun repair | 1.0 | failed measurement | terminal verdict | resources | s1c3-resources | safety | S1C3 resource owner | verifier | terminal result | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:29 | S1C-3 |

## notes

- Structural PASS is coherence-only and must retain authority_ready false.
