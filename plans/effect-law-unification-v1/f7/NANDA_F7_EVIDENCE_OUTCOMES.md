# F7 Generation Evidence Outcomes

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | verified pass | maps to | positive reinforcement | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s2 | applicability negative | maps to | applicability counter-wave | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s3 | hard contradiction | maps to | structural revision | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s4 | censored outcome | maps to | no semantic update | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s5 | censored outcome | remains in | evidence accounting denominator | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s6 | duplicate event request or receipt | cannot create | second evidence row | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | verified pass | maps to | positive reinforcement | crates/nando-operator-learning/src/generation_evidence_v3/types.rs#semantic_update |
| c2 | applicability negative | maps to | applicability counter-wave | crates/nando-operator-learning/src/generation_evidence_v3/types.rs#semantic_update |
| c3 | hard contradiction | maps to | structural revision | crates/nando-operator-learning/src/generation_evidence_v3/types.rs#semantic_update |
| c4 | censored outcome | maps to | no semantic update | crates/nando-operator-learning/src/generation_evidence_v3/types.rs#semantic_update |
| c5 | censored outcome | remains in | evidence accounting denominator | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#accounting |
| c6 | duplicate event request or receipt | cannot create | second evidence row | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#ensure_unique |
