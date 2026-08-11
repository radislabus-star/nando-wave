# NANDA Triad Worksheet

task_id: s1c-v3-identity
domain: general
query: Does one V3 identity owner bind the parent epoch commit type and candidate source?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | V3 protocol identity | binds | exact parent commit and epoch root | protocol V3 identity constants | 1.0 | identity owner | frozen roots | identity | v3-identity-contract | proof | V3 identity owner | paper contract | root binding | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:45 | S1C-1 |
| s2 | V3 protocol identity | types | protocol commit as 40 hex Git SHA1 | protocol V3 commit type | 1.0 | identity owner | typed commit | identity | v3-identity-contract | schema | V3 identity owner | measurement contract | commit type | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:52 | S1C-1 |
| s3 | V3 protocol identity | binds | frozen candidate source manifest | protocol V3 candidate identity | 1.0 | identity owner | source identity | identity | v3-identity-contract | source | V3 identity owner | candidate freeze | source binding | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:86 | S1C-1 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | V3 protocol identity | binds | exact parent commit and epoch root | verifier constants | 1.0 | identity owner | frozen roots | identity | v3-identity-code | proof | V3 identity owner | verifier constants | root binding | ops/remote-backend/verify_s1c1_resource_v3.py:18 | S1C-1 |
| c2 | V3 protocol identity | types | protocol commit as 40 hex Git SHA1 | commit regular expression | 1.0 | identity owner | typed commit | identity | v3-identity-code | schema | V3 identity owner | require commit | commit type | ops/remote-backend/verify_s1c1_resource_v3.py:38 | S1C-1 |
| c3 | V3 protocol identity | binds | frozen candidate source manifest | verifier source constant | 1.0 | identity owner | source identity | identity | v3-identity-code | source | V3 identity owner | measurement validation | source binding | ops/remote-backend/verify_s1c1_resource_v3.py:20 | S1C-1 |

## notes

- Coherence only; no measurement or deployment authority.
