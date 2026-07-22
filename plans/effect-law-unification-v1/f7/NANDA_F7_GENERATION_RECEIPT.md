# F7 Generation Verifier Receipt

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | generation verifier receipt | binds | exact F6 receipt to one generation | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |
| s2 | generation verifier receipt | preserves | exact F6 verifier verdict | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |
| s3 | rejected F6 verdict | cannot become | verified positive evidence | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |
| s4 | future receipt | requires | post-watermark sequence and exact support freeze | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |
| s5 | foreign generation or artifact set | yields | fail-closed rejection | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |
| s6 | generation verifier receipt | grants | no execution authority | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | generation verifier receipt | binds | exact F6 receipt to one generation | crates/nando-operator-proof/src/generation_receipt_v3/seal.rs#seal_generation_verifier_receipt_v3 |
| c2 | generation verifier receipt | preserves | exact F6 verifier verdict | crates/nando-operator-proof/src/generation_receipt_v3/types.rs#f6_verdict |
| c3 | rejected F6 verdict | cannot become | verified positive evidence | crates/nando-operator-learning/src/generation_evidence_v3/receipt_bridge.rs#append_generation_verifier_receipt |
| c4 | future receipt | requires | post-watermark sequence and exact support freeze | crates/nando-operator-proof/src/generation_receipt_v3/seal.rs#validate_partition_binding |
| c5 | foreign generation or artifact set | yields | fail-closed rejection | crates/nando-operator-proof/tests/f7_generation_receipt_v3.rs#partition_generation_and_artifact_mismatches_fail_closed |
| c6 | generation verifier receipt | grants | no execution authority | crates/nando-operator-proof/src/generation_receipt_v3/types.rs#execution_authority |
