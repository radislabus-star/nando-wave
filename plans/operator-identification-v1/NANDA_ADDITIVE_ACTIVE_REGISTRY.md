# Additive Active Registry

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | fresh admitted package | extends | active registry generation | ARCHITECTURE_CANON.md#generation-firewall |
| s2 | current external admission | revalidates | retained active package | ARCHITECTURE_CANON.md#generation-firewall |
| s3 | expired or invalid authority | cannot retain | active package | ARCHITECTURE_CANON.md#generation-firewall |
| s4 | equal package identifier with different runtime identity | blocks | registry merge | ARCHITECTURE_CANON.md#generation-firewall |
| s5 | additive registry merge | reissues | content-derived registry revision | ARCHITECTURE_CANON.md#generation-firewall |
| s6 | additive registry merge | rebinds | every package authority receipt | ARCHITECTURE_CANON.md#generation-firewall |
| s7 | unrelated active operator | remains available during | new operator admission | ARCHITECTURE_CANON.md#generation-firewall |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | fresh admitted package | extends | active registry generation | crates/nando-response-actor/src/online_admission.rs#merge_with_active_online_admission |
| c2 | current external admission | revalidates | retained active package | crates/nando-response-actor/src/online_admission.rs#merge_with_active_online_admission |
| c3 | expired or invalid authority | cannot retain | active package | crates/nando-response-actor/src/online_admission.rs#merge_with_active_online_admission |
| c4 | equal package identifier with different runtime identity | blocks | registry merge | crates/nando-response-actor/src/online_admission.rs#merge_with_active_online_admission |
| c5 | additive registry merge | reissues | content-derived registry revision | crates/nando-response-actor/src/online_admission.rs#authority_content_revision |
| c6 | additive registry merge | rebinds | every package authority receipt | crates/nando-response-actor/src/online_admission.rs#merge_online_admission_snapshots |
| c7 | unrelated active operator | remains available during | new operator admission | crates/nando-response-actor/src/online_admission.rs#merge_with_active_online_admission |
