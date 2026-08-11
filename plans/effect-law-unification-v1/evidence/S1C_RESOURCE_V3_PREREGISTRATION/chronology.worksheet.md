# NANDA Triad Worksheet

task_id: s1c-v3-chronology
domain: general
query: Does one V3 chronology owner enforce order gaps nonzero exits and service survival?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | V3 chronology contract | orders | exact nine run labels | protocol exact schedule | 1.0 | chronology owner | run sequence | chronology | v3-chronology-contract | proof | V3 chronology owner | schedule contract | exact order | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:188 | S1C-1 |
| s2 | V3 chronology contract | preserves | nonzero inherited exit and two second gap | protocol failure rule | 1.0 | chronology owner | run observation | chronology | v3-chronology-contract | runtime | V3 chronology owner | invocation contract | durable chronology | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:198 | S1C-1 |
| s3 | V3 chronology contract | compares | service PID and restart state across snapshots | protocol survival rule | 1.0 | chronology owner | service state | chronology | v3-chronology-contract | safety | V3 chronology owner | evidence contract | drift rejection | plans/effect-law-unification-v1/S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md:169 | S1C-1 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | V3 chronology contract | orders | exact nine run labels | runner invocation sequence | 1.0 | chronology owner | run sequence | chronology | v3-chronology-code | proof | V3 chronology owner | runner schedule | exact order | ops/remote-backend/run_s1c1_resource_v3.sh:181 | S1C-1 |
| c2 | V3 chronology contract | preserves | nonzero inherited exit and two second gap | run one function | 1.0 | chronology owner | run observation | chronology | v3-chronology-code | runtime | V3 chronology owner | runner invocation | durable chronology | ops/remote-backend/run_s1c1_resource_v3.sh:171 | S1C-1 |
| c3 | V3 chronology contract | compares | service PID and restart state across snapshots | evidence chronology validator | 1.0 | chronology owner | service state | chronology | v3-chronology-code | safety | V3 chronology owner | verifier snapshots | drift rejection | ops/remote-backend/verify_s1c1_resource_v3.py:390 | S1C-1 |

## notes

- Malformed evidence is terminal and cannot be rerun under V3.
