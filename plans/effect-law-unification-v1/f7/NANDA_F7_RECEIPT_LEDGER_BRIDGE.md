# F7 Receipt To Ledger Bridge

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | F6 verified receipt | may create | VerifiedPass ledger row | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |
| s2 | F6 non-verified receipt | cannot create | VerifiedPass ledger row | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |
| s3 | receipt partition and freeze | must equal | ledger partition state | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |
| s4 | receipt generation ID | must equal | ledger generation ID | F7_GENERATION_PERSISTENCE_V1.md#one-generation-identity |
| s5 | live lineage and event roots | require | separate capture-owner join in F7-E | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |
| s6 | receipt-to-ledger bridge | grants | no execution authority | F7_GENERATION_PERSISTENCE_V1.md#f7-c-receipt-binding |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | F6 verified receipt | may create | VerifiedPass ledger row | crates/nando-operator-learning/src/generation_evidence_v3/receipt_bridge.rs#append_generation_verifier_receipt |
| c2 | F6 non-verified receipt | cannot create | VerifiedPass ledger row | crates/nando-operator-learning/src/generation_evidence_v3/receipt_bridge.rs#append_generation_verifier_receipt |
| c3 | receipt partition and freeze | must equal | ledger partition state | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#append_future |
| c4 | receipt generation ID | must equal | ledger generation ID | crates/nando-operator-learning/src/generation_evidence_v3/receipt_bridge.rs#append_generation_verifier_receipt |
| c5 | live lineage and event roots | require | separate capture-owner join in F7-E | plans/effect-law-unification-v1/STOP_F7_C_GENERATION_VERIFIER_RECEIPT.md |
| c6 | receipt-to-ledger bridge | grants | no execution authority | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#execution_authority |
