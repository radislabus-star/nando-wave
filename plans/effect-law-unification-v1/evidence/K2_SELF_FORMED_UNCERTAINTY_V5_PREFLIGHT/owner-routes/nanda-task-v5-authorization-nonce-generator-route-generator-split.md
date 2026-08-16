# NANDA Split Worksheet

split_by: route
split_key: generator-split
source: plans/effect-law-unification-v1/evidence/K2_SELF_FORMED_UNCERTAINTY_V5_PREFLIGHT/nanda-task-v5-authorization-nonce-generator.md

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| t8 | Confirm generator request | uses | separate closed Confirm wire schema | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:104 | 1.000 | confirm request | typed generator contract | generator-split | request-schema-owner |
| t9 | validated Confirm generator response | separates into | public batch resolver tables and per-case truth files | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:176 | 1.000 | generator result | isolated artifact classes | generator-split | output-split-owner |


## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| c8 | Confirm generator request | uses | separate closed Confirm wire schema | V5 R7G split-schema candidate | 1.000 | confirm request | typed generator contract | generator-split | request-schema-owner |
| c9 | validated Confirm generator response | separates into | public batch resolver tables and per-case truth files | V5 R7H output-publication candidate | 1.000 | generator result | isolated artifact classes | generator-split | output-split-owner |
