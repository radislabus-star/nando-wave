# R8B V8 C08 M16 M17 Authority

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | M24 child | freezes C08 before | C09-C20 downstream invocation schedule | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8_CRITIQUE_V1.md:15-20 | 1.0 | child request owner | expected downstream authority | authority | chronology |
| t3 | process ledger | observes | C09-C20 actual invocation projection | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:312-314 | 1.0 | observed provenance | downstream process events | observation | projection-boundary |
| t4 | M24 child | emits exactly | C08 plus three measured outputs | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8_CRITIQUE_V1.md:18-21 | 1.0 | child producer | four immutable objects | evidence | output-census |
| t6 | M16 Oracle batch | contains exactly | sixteen event roots and sixteen receipt roots | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:536-545 | 1.0 | derived aggregate | complete M16 dual sets | evidence | oracle-dual-set |
| t8 | M25 | reconstructs independently | exact M16 dual sets from ledger | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:547-549 | 1.0 | independent authorizer | observed Oracle provenance | proof | oracle-equality |
| t9 | M17 control census | contains exactly | four event roots and four receipt roots | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:551-553 | 1.0 | coverage aggregate | complete M17 dual sets | evidence | control-dual-set |
| t10 | four-scope census | is not | fifth control result | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:553-554 | 1.0 | coverage aggregate | control outcome | authority | denominator-separation |
| t11 | M16 or M17 root mismatch | causes | exact-set authorization VETO | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:556-557 | 1.0 | provenance conflict | authorization result | failure | exact-set |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | M24 child | freezes C08 before | C09-C20 downstream invocation schedule | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:173-182 | 1.0 | child request owner | expected downstream authority | authority | chronology |
| c3 | process ledger | observes | C09-C20 actual invocation projection | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:312-315 | 1.0 | observed provenance | downstream process events | observation | projection-boundary |
| c4 | M24 child | emits exactly | C08 plus three measured outputs | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:524-531 | 1.0 | child producer | four immutable objects | evidence | output-census |
| c6 | M16 Oracle batch | contains exactly | sixteen event roots and sixteen receipt roots | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:536-545 | 1.0 | derived aggregate | complete M16 dual sets | evidence | oracle-dual-set |
| c8 | M25 | reconstructs independently | exact M16 dual sets from ledger | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:547-549 | 1.0 | independent authorizer | observed Oracle provenance | proof | oracle-equality |
| c9 | M17 control census | contains exactly | four event roots and four receipt roots | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:551-552 | 1.0 | coverage aggregate | complete M17 dual sets | evidence | control-dual-set |
| c10 | four-scope census | is not | fifth control result | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:553-555 | 1.0 | coverage aggregate | control outcome | authority | denominator-separation |
| c11 | M16 or M17 root mismatch | causes | exact-set authorization VETO | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:556-557 | 1.0 | provenance conflict | authorization result | failure | exact-set |

## notes

- Expected schedule authority, observed process provenance and measured evidence are not interchangeable.
- C08 remains a typed non-evidence object with no twentieth evidence kind.
- Exact set equality is required; cardinality-only or subset checks are insufficient.
