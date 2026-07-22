# F7 Generation Identity Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | operator kernel | solely owns | canonical generation identity | F7_GENERATION_PERSISTENCE_V1.md#one-generation-identity |
| s2 | generation identity | commits | sequence parent and seven component roots | F7_GENERATION_PERSISTENCE_V1.md#one-generation-identity |
| s3 | changed committed component | creates | different generation identity | F7_GENERATION_PERSISTENCE_V1.md#one-generation-identity |
| s4 | sequence one | excludes | parent generation | F7_GENERATION_PERSISTENCE_V1.md#one-generation-identity |
| s5 | later sequence | requires | valid parent generation root | F7_GENERATION_PERSISTENCE_V1.md#one-generation-identity |
| s6 | generation manifest | grants | no execution authority | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | operator kernel | solely owns | canonical generation identity | crates/nando-operator-kernel/src/operator_generation.rs |
| c2 | generation identity | commits | sequence parent and seven component roots | crates/nando-operator-kernel/src/operator_generation.rs |
| c3 | changed committed component | creates | different generation identity | crates/nando-operator-kernel/src/operator_generation.rs#every_component_change_creates_a_new_generation |
| c4 | sequence one | excludes | parent generation | crates/nando-operator-kernel/src/operator_generation.rs#validate_lineage |
| c5 | later sequence | requires | valid parent generation root | crates/nando-operator-kernel/src/operator_generation.rs#validate_lineage |
| c6 | generation manifest | grants | no execution authority | crates/nando-operator-kernel/src/operator_generation.rs#execution_authority |
