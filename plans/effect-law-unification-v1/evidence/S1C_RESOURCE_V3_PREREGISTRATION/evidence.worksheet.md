# NANDA Triad Worksheet

task_id: s1c-v3-evidence
domain: general
query: Does one V3 evidence owner bind the bounded raw set manifest and transcribed measurements?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | V3 evidence contract | bounds | exact 40 raw evidence files | protocol file denominator | 1.0 | evidence owner | raw evidence set | evidence | v3-evidence-contract | proof | V3 evidence owner | runner contract | bounded set | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:146 | S1C-1 |
| s2 | V3 evidence contract | binds | raw files to canonical SHA256 manifest | protocol manifest rule | 1.0 | evidence owner | evidence root | evidence | v3-evidence-contract | persistence | V3 evidence owner | manifest contract | exact root | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:159 | S1C-1 |
| s3 | V3 evidence contract | compares | raw metrics with measurements JSON | protocol transcription rule | 1.0 | evidence owner | typed measurements | evidence | v3-evidence-contract | proof | V3 evidence owner | verifier contract | exact equality | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:166 | S1C-1 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | V3 evidence contract | bounds | exact 40 raw evidence files | runner file count | 1.0 | evidence owner | raw evidence set | evidence | v3-evidence-code | proof | V3 evidence owner | runner finalization | bounded set | ops/remote-backend/run_s1c1_resource_v3.sh:198 | S1C-1 |
| c2 | V3 evidence contract | binds | raw files to canonical SHA256 manifest | canonical manifest function | 1.0 | evidence owner | evidence root | evidence | v3-evidence-code | persistence | V3 evidence owner | verifier manifest | exact root | ops/remote-backend/verify_s1c1_resource_v3.py:100 | S1C-1 |
| c3 | V3 evidence contract | compares | raw metrics with measurements JSON | raw evidence validator | 1.0 | evidence owner | typed measurements | evidence | v3-evidence-code | proof | V3 evidence owner | verifier evidence gate | exact equality | ops/remote-backend/verify_s1c1_resource_v3.py:350 | S1C-1 |

## notes

- Synthetic parser tests do not supply measurement authority.
