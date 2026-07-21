# R5 Proof Reconstruction

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | serialized candidate | triggers | bounded reconstruction | decomposition-plan#proof-reconstruction |
| s2 | proof owner | compares | submitted and reconstructed commitments | decomposition-plan#r5 |
| s3 | commitment mismatch | blocks | admission | decomposition-plan#fail-closed |
| s4 | reconstruction receipt | does not grant | execution authority | decomposition-plan#structural-gates |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | serialized candidate | triggers | bounded reconstruction | crates/nando-response-actor/src/online_admission.rs#build_crystallized_admission_snapshot |
| c2 | proof owner | compares | submitted and reconstructed commitments | crates/nando-operator-proof/src/admission_reconstruction.rs#verify_admission_candidate_reconstruction |
| c3 | commitment mismatch | blocks | admission | crates/nando-response-actor/src/online_admission.rs#crystallized_admission_resynthesis_mismatch |
| c4 | reconstruction receipt | does not grant | execution authority | crates/nando-operator-proof/src/admission_reconstruction.rs#receipt |
