# R5 Authority Truth

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | package lifecycle state | supplies | admission input | decomposition-plan#package-policy |
| s2 | external authority lease | grants | execution authority | architecture-canon#authority |
| s3 | learner package state | cannot grant | execution authority | decomposition-plan#single-truth |
| s4 | admission validator | binds | registry package and proof digests | decomposition-plan#r5-stop |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | package lifecycle state | supplies | admission input | crates/nando-response-actor/src/package.rs#admission_candidate_blocker |
| c2 | external authority lease | grants | execution authority | crates/nando-operator-admission/src/authority.rs#ValidatedResponseAuthority |
| c3 | learner package state | cannot grant | execution authority | crates/nando-response-actor/src/package.rs#eligible_for_local_accept |
| c4 | admission validator | binds | registry package and proof digests | crates/nando-operator-admission/src/authority.rs#validate_response_authority_snapshot |
