# F6 Raw Evidence Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | verifier input | consumes | exact raw provider bytes | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#independence-contract |
| s2 | request scene | derives from | raw provider bytes | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#independence-contract |
| s3 | independent surface | excludes | actor-selected request text | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#independence-contract |
| s4 | exhausted evidence walk | yields | ABSTAIN without semantic update | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#budgets |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | verifier input | consumes | exact raw provider bytes | crates/nando-operator-proof/src/independent_verifier_v3/input.rs |
| c2 | request scene | derives from | raw provider bytes | crates/nando-operator-proof/src/independent_verifier_v3/request_provenance.rs |
| c3 | independent surface | excludes | actor-selected request text | crates/nando-operator-proof/src/independent_verifier_v3/surface.rs |
| c4 | exhausted evidence walk | yields | ABSTAIN without semantic update | crates/nando-operator-proof/src/independent_verifier_v3/surface.rs |
