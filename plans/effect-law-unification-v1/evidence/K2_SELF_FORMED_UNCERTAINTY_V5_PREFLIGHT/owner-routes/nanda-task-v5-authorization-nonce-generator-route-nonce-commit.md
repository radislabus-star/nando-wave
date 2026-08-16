# NANDA Split Worksheet

split_by: route
split_key: nonce-commit
source: plans/effect-law-unification-v1/evidence/K2_SELF_FORMED_UNCERTAINTY_V5_PREFLIGHT/nanda-task-v5-authorization-nonce-generator.md

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| t4 | frozen artifact descriptor | precedes | operating-system CSPRNG read | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:145 | 1.000 | frozen descriptor | nonce source | nonce-commit | nonce-source-owner |
| t5 | retained nonce file | commits as | hash-only NONCE_COMMITTED event | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:148 | 1.000 | private nonce artifact | public journal commitment | nonce-commit | nonce-artifact-owner |


## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| c4 | frozen artifact descriptor | precedes | operating-system CSPRNG read | V5 R7H nonce-owner candidate | 1.000 | frozen descriptor | nonce source | nonce-commit | nonce-source-owner |
| c5 | retained nonce file | commits as | hash-only NONCE_COMMITTED event | V5 R7H nonce-secrecy candidate | 1.000 | private nonce artifact | public journal commitment | nonce-commit | nonce-artifact-owner |
