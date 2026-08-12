# NANDA Triad Worksheet

task_id: s1c3f-concurrency-v1
domain: code
query: Does S1C-3F preserve natural post-cursor records and rollback prefixes?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | opening verifier | freezes | record-count-zero cursor | opening-cursor clause | 1.0 | cursor owner | opening denominator | concurrency | opening-cursor | proof | S1C-3F verifier | startup parse | cursor | plans/effect-law-unification-v1/S1C3F_MAGIC_ONLY_LEDGER_PREREGISTRATION_V1.md:88 | opening |
| t2 | natural runtime | may append | valid suffix frames after cursor | post-cursor suffix clause | 1.0 | evidence producer | post-cursor evidence | concurrency | natural-suffix | runtime | ordinary traffic | later request | preserved frame | plans/effect-law-unification-v1/S1C3F_MAGIC_ONLY_LEDGER_PREREGISTRATION_V1.md:90 | survival |
| t3 | rollback owner | preserves | every pre-mutation prefix and natural suffix | rollback section | 1.0 | recovery owner | evidence bytes | rollback | preserve-prefix | transaction | root | failure | no evidence loss | plans/effect-law-unification-v1/S1C3F_MAGIC_ONLY_LEDGER_PREREGISTRATION_V1.md:98 | rollback |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | opening verifier | freezes | record-count-zero cursor | candidate c1 | 1.0 | cursor owner | opening denominator | concurrency | opening-cursor | conclusion | S1C-3F verifier | startup parse | cursor | candidate_answer:c1 | opening |
| c2 | natural runtime | may append | valid suffix frames after cursor | candidate c2 | 1.0 | evidence producer | post-cursor evidence | concurrency | natural-suffix | conclusion | ordinary traffic | later request | preserved frame | candidate_answer:c2 | survival |
| c3 | rollback owner | preserves | every pre-mutation prefix and natural suffix | candidate c3 | 1.0 | recovery owner | evidence bytes | rollback | preserve-prefix | conclusion | root | failure | no evidence loss | candidate_answer:c3 | rollback |
