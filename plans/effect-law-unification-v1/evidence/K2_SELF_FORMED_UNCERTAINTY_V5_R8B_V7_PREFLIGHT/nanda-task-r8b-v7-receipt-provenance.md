# R8B V7 Receipt Provenance

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | libtest stdout | remains distinct from | canonical authority outputs | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V1.md:20-26 | 1.0 | process diagnostics | typed evidence bytes | evidence | channel-separation |
| t2 | one finished process event | binds | bounded produced-receipt descriptor set | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V1.md:54-65 | 1.0 | producer event | canonical output set | evidence | multi-output |
| t3 | S01-S05 | produce | their own immutable suite receipts | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V2.md:9-13 | 1.0 | suite producer | suite evidence | authority | suite-owner |
| t4 | M24 child | produces | linked receipt, Oracle batch and control census | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V1.md:38-48 | 1.0 | linked aggregate producer | linked aggregate outputs | authority | child-owner |
| t5 | M25 | matches exactly once | packet entry to producer descriptor | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V1.md:25-26 | 1.0 | independent authorizer | producer evidence | proof | no-reuse |
| t6 | four-scope census | derives from | four distinct M17 roots | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V2.md:30-34 | 1.0 | coverage census | control process receipts | denominator | derived-not-additive |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | libtest stdout | remains distinct from | canonical authority outputs | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:164-178 | 1.0 | process diagnostics | typed evidence bytes | evidence | channel-separation |
| c2 | one finished process event | binds | bounded produced-receipt descriptor set | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:112-136 | 1.0 | producer event | canonical output set | evidence | multi-output |
| c3 | S01-S05 | produce | their own immutable suite receipts | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:148-200 | 1.0 | suite producer | suite evidence | authority | suite-owner |
| c4 | M24 child | produces | linked receipt, Oracle batch and control census | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:202-217 | 1.0 | linked aggregate producer | linked aggregate outputs | authority | child-owner |
| c5 | M25 | matches exactly once | packet entry to producer descriptor | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:222-225 | 1.0 | independent authorizer | producer evidence | proof | no-reuse |
| c6 | four-scope census | derives from | four distinct M17 roots | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:211-217 | 1.0 | coverage census | control process receipts | denominator | derived-not-additive |

## notes

- Output path, bytes, schema and semantic root must all match.
- A producer exit code alone is never a PASS receipt.
