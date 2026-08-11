# NANDA Triad Worksheet

task_id: s1c-v3-authority
domain: general
query: Does one V3 slice authority separate source commit shadow work and deployment?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | V3 slice authority | grants_only | frozen S1C1 source commit after strict checks | protocol result matrix | 1.0 | slice authority | source action | authority | v3-authority-contract | proof | V3 slice authority | resource pass | commit eligibility | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:246 | S1C-1 |
| s2 | V3 slice authority | denies | production deployment | protocol production boundary | 1.0 | slice authority | production action | authority | v3-authority-contract | safety | V3 slice authority | resource verdict | deployment false | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:251 | S1C-1 |
| s3 | V3 slice authority | delegates | shadow to S1C2 and deployment to S1C3 | protocol slice boundary | 1.0 | slice authority | later slices | authority | v3-authority-contract | lifecycle | V3 slice authority | next stage | separate protocols | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:281 | S1C |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | V3 slice authority | grants_only | frozen S1C1 source commit after strict checks | post pass checks | 1.0 | slice authority | source action | authority | v3-authority-code | proof | V3 slice authority | final protocol gate | commit eligibility | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:259 | S1C-1 |
| c2 | V3 slice authority | denies | production deployment | verifier result field | 1.0 | slice authority | production action | authority | v3-authority-code | safety | V3 slice authority | verifier verdict | deployment false | ops/remote-backend/verify_s1c1_resource_v3.py:588 | S1C-1 |
| c3 | V3 slice authority | delegates | shadow to S1C2 and deployment to S1C3 | explicit production boundary | 1.0 | slice authority | later slices | authority | v3-authority-code | lifecycle | V3 slice authority | lifecycle boundary | separate protocols | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:275 | S1C |

## notes

- Structural PASS must retain authority_ready false.
