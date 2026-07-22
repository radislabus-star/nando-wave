# F7 Persistence Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | operator persistence crate | owns | checkpoint composition and filesystem durability | F7_GENERATION_PERSISTENCE_V1.md#owner-slices |
| s2 | kernel runtime proof and learning | do not depend on | operator persistence crate | F7_GENERATION_PERSISTENCE_V1.md#owner-slices |
| s3 | response actor | may later orchestrate but not own | checkpoint truth | F7_GENERATION_PERSISTENCE_V1.md#owner-slices |
| s4 | F7-D persistence | excludes | live capture join and serving swap | F7_GENERATION_PERSISTENCE_V1.md#f7-d-live-boundary |
| s5 | F7-D persistence | excludes | external admission and local accept | F7_GENERATION_PERSISTENCE_V1.md#f7-d-authority-boundary |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | operator persistence crate | owns | checkpoint composition and filesystem durability | crates/nando-operator-persistence/src/lib.rs |
| c2 | kernel runtime proof and learning | do not depend on | operator persistence crate | Cargo.toml |
| c3 | response actor | may later orchestrate but not own | checkpoint truth | crates/nando-response-actor/Cargo.toml |
| c4 | F7-D persistence | excludes | live capture join and serving swap | crates/nando-operator-persistence/src/lib.rs |
| c5 | F7-D persistence | excludes | external admission and local accept | crates/nando-operator-persistence/src/checkpoint/types.rs#execution_authority |
