# NANDA Triad Worksheet

task_id: s1c3d-parity-snapshot-v1
domain: code
query: Does S1C-3D repair parity access without weakening live authority ownership?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | root transaction owner | reads and binds | live registry and admission source roots | snapshot owner route | 1.0 | authority snapshot owner | live authority artifacts | parity | parity-snapshot | operation | root transaction owner | authority paths | source roots | plans/effect-law-unification-v1/S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md:52 | predeployment |
| t2 | root transaction owner | creates | immutable root-owned transaction-local snapshots | snapshot mode contract | 1.0 | authority snapshot owner | bounded parity inputs | parity | parity-snapshot | operation | root transaction owner | source bytes | root:e 0440 files in root:e 0550 directory | plans/effect-law-unification-v1/S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md:60 | transaction only |
| t3 | baseline and candidate oracles | consume | exact same bound snapshot paths and bytes | parity comparison contract | 1.0 | parity evidence consumers | bounded parity inputs | parity | parity-verdict | proof | parity verifier | frozen oracle binaries | byte-identical rows | plans/effect-law-unification-v1/S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md:68 | predeployment |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | root transaction owner | reads and binds | live registry and admission source roots | candidate claim c1 | 1.0 | authority snapshot owner | live authority artifacts | parity | parity-snapshot | conclusion | root transaction owner | authority paths | source roots | candidate_answer:c1 | predeployment |
| c2 | root transaction owner | creates | immutable root-owned transaction-local snapshots | candidate claim c2 | 1.0 | authority snapshot owner | bounded parity inputs | parity | parity-snapshot | conclusion | root transaction owner | source bytes | root:e 0440 files in root:e 0550 directory | candidate_answer:c2 | transaction only |
| c3 | baseline and candidate oracles | consume | exact same bound snapshot paths and bytes | candidate claim c3 | 1.0 | parity evidence consumers | bounded parity inputs | parity | parity-verdict | conclusion | parity verifier | frozen oracle binaries | byte-identical rows | candidate_answer:c3 | predeployment |
