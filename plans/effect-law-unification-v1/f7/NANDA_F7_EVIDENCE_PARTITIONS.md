# F7 Generation Evidence Partitions

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | generation evidence ledger | belongs to | one canonical generation ID | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s2 | support-open state | may transition once to | immutable support freeze | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s3 | immutable support freeze | precedes | future append | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s4 | support lineage | cannot reappear in | future partition | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s5 | future observation | must be at or after | frozen capture watermark | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s6 | future growth | changes | evidence root but not generation ID | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |
| s7 | evidence ledger | grants | no execution authority | F7_GENERATION_PERSISTENCE_V1.md#f7-b-evidence-ledger |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | generation evidence ledger | belongs to | one canonical generation ID | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#new |
| c2 | support-open state | may transition once to | immutable support freeze | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#freeze_support |
| c3 | immutable support freeze | precedes | future append | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#append_future |
| c4 | support lineage | cannot reappear in | future partition | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#append_future |
| c5 | future observation | must be at or after | frozen capture watermark | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#append_future |
| c6 | future growth | changes | evidence root but not generation ID | crates/nando-operator-learning/src/generation_evidence_v3/tests.rs#frozen_partitions_restart_byte_identically_without_changing_generation |
| c7 | evidence ledger | grants | no execution authority | crates/nando-operator-learning/src/generation_evidence_v3/ledger.rs#execution_authority |
