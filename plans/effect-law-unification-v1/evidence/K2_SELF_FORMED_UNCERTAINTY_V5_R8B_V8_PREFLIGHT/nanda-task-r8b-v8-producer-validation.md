# R8B V8 Producer Validation

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | producer request validation | precedes | durable producer mutation | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:397-409 | 1.0 | request proof owner | bounded producer contract | proof | pre-mutation |
| t2 | role-specific validator | decodes and reserializes | one concrete receipt type | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:482-486 | 1.0 | typed proof owner | producer output | proof | concrete-validation |
| t3 | generic root extraction | cannot validate | authority output | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:479-480 | 1.0 | forbidden shortcut | authority output | failure | no-generic-root |
| t4 | AuthoritySuccess completion | may bind | typed output descriptors | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:376-379 | 1.0 | successful process outcome | immutable output provenance | authority | authority-success |
| t6 | producer | publishes create-new and fsynced | final canonical files | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:501-504 | 1.0 | output mutation owner | immutable receipt files | mutation | canonical-channel |
| t7 | stdout and stderr | remain | diagnostics only | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:506-510 | 1.0 | process diagnostics | non-authority channel | observation | channel-separation |
| t8 | restart suite producer process | spawns and journals | direct setup generator invocations | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:493-494 | 1.0 | process spawn owner | direct generator invocations | execution | s02-direct-partition |
| t9 | each dispatching M01 | owns | its nested M02 request | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:495-497 | 1.0 | nested request owner | nested generator invocation | execution | m01-nested-partition |
| t10 | bound launch tools | possess no | receipt or writer authority | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:451-454 | 1.0 | execution adapters | evidence authority | authority | tool-boundary |
| t11 | cross-partition invocation | is rejected before | launch | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:456-459 | 1.0 | invalid request | process execution | failure | fail-before-spawn |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | producer request validation | precedes | durable producer mutation | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:397-407 | 1.0 | request proof owner | bounded producer contract | proof | pre-mutation |
| c2 | role-specific validator | decodes and reserializes | one concrete receipt type | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:482-485 | 1.0 | typed proof owner | producer output | proof | concrete-validation |
| c3 | generic root extraction | cannot validate | authority output | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:479-481 | 1.0 | forbidden shortcut | authority output | failure | no-generic-root |
| c4 | AuthoritySuccess completion | may bind | typed output descriptors | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:367-377 | 1.0 | successful process outcome | immutable output provenance | authority | authority-success |
| c6 | producer | publishes create-new and fsynced | final canonical files | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:499-510 | 1.0 | output mutation owner | immutable receipt files | mutation | canonical-channel |
| c7 | stdout and stderr | remain | diagnostics only | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:499-510 | 1.0 | process diagnostics | non-authority channel | observation | channel-separation |
| c8 | restart suite producer process | spawns and journals | direct setup generator invocations | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:493-495 | 1.0 | process spawn owner | direct generator invocations | execution | s02-direct-partition |
| c9 | each dispatching M01 | owns | its nested M02 request | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:495-497 | 1.0 | nested request owner | nested generator invocation | execution | m01-nested-partition |
| c10 | bound launch tools | possess no | receipt or writer authority | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:451-454 | 1.0 | execution adapters | evidence authority | authority | tool-boundary |
| c11 | cross-partition invocation | is rejected before | launch | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:456-459 | 1.0 | invalid request | process execution | failure | fail-before-spawn |

## notes

- Typed canonical validation, process exit and immutable publication are separate predicates.
- DiagnosticExpectedFailure remains diagnostics-only and has no output descriptor.
- Suite children cannot individually satisfy the suite aggregate receipt.
