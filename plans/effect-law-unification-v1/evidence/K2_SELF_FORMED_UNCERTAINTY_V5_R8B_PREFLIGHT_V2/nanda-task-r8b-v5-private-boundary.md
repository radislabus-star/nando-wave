# R8B V5 Private Truth Boundary

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | full owner validator | reads | public and private split payloads | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md:83-83 | 1.0 | private validation owner | generated evidence | observation | full-validation |
| t2 | metadata runner loader | reads | public payloads only | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md:84-84 | 1.0 | public orchestrator | public evidence | observation | public-loader |
| t3 | private child | receives | one exact read only private mount | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md:55-55 | 1.0 | private execution owner | bounded private payload | execution | private-mount |
| t4 | public coordinator exit | precedes | every private child start | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4.md:260-283 | 1.0 | public barrier | private execution | stage-order | private-barrier |
| t5 | private result | cannot return to | public coordinator | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4.md:252-258 | 1.0 | private evidence | exited public process | authority | no-return |
| t6 | cleanup census | covers | actual linked attempt tree | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md:58-58 | 1.0 | cleanup proof owner | governed files | proof | actual-tree |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | full owner validator | reads | public and private split payloads | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:393-400 | 1.0 | private validation owner | generated evidence | observation | full-validation |
| c2 | metadata runner loader | reads | public payloads only | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:402-406 | 1.0 | public orchestrator | public evidence | observation | public-loader |
| c3 | private child | receives | one exact read only private mount | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:408-412 | 1.0 | private execution owner | bounded private payload | execution | private-mount |
| c4 | public coordinator exit | precedes | every private child start | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:414-416 | 1.0 | public barrier | private execution | stage-order | private-barrier |
| c5 | private result | cannot return to | public coordinator | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:415-416 | 1.0 | private evidence | exited public process | authority | no-return |
| c6 | cleanup census | covers | actual linked attempt tree | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:520-545 | 1.0 | cleanup proof owner | governed files | proof | actual-tree |

## notes

- Path metadata may be transported; private payload bytes may not enter the public loader.
