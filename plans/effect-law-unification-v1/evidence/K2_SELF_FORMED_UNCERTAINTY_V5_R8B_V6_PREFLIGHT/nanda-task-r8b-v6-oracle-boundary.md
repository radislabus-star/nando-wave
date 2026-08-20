# R8B V6 Oracle Boundary

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | public coordinator | exits before | every private and oracle child | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:414-416 | 1.0 | public owner | private process set | chronology | public-barrier |
| t2 | runner | transports only | private path and immutable descriptor metadata | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:408-412 | 1.0 | metadata orchestrator | private mount descriptor | observation | runner-boundary |
| t3 | oracle child | reads and validates | private truth bytes | confirm_oracle_process.rs:129-174 | 1.0 | private proof owner | private truth | proof | oracle-read |
| t4 | oracle child | recomputes | evidence content SHA-256 | confirm_oracle_process.rs:250-265 | 1.0 | private proof owner | mounted bytes | proof | oracle-hash |
| t5 | private descriptor | remains open through | read-only child mount | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6_CRITIQUE_V1.md:23-24 | 1.0 | custody owner | mount operation | execution | no-toctou |
| t6 | oracle evaluator bytes | remain exact | predecessor oracle evaluator | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6_CRITIQUE_V1.md:76-88 | 1.0 | frozen proof owner | reused evaluator | compatibility | oracle-preserved |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | public coordinator | exits before | every private and oracle child | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:277-279 | 1.0 | public owner | private process set | chronology | public-barrier |
| c2 | runner | transports only | private path and immutable descriptor metadata | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:229-246 | 1.0 | metadata orchestrator | private mount descriptor | observation | runner-boundary |
| c3 | oracle child | reads and validates | private truth bytes | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:248-253 | 1.0 | private proof owner | private truth | proof | oracle-read |
| c4 | oracle child | recomputes | evidence content SHA-256 | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:248-253 | 1.0 | private proof owner | mounted bytes | proof | oracle-hash |
| c5 | private descriptor | remains open through | read-only child mount | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:241-246 | 1.0 | custody owner | mount operation | execution | no-toctou |
| c6 | oracle evaluator bytes | remain exact | predecessor oracle evaluator | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:472-474 | 1.0 | frozen proof owner | reused evaluator | compatibility | oracle-preserved |

## notes

- Runner source must not contain a private truth content read on the positive route.
