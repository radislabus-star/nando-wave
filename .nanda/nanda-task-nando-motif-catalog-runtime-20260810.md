# NANDA Triad Worksheet

task_id: nando-motif-catalog-runtime-20260810
domain: general
query: Validate K1Runtime preparation of Catalog V2 from one validated motif archive

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | K1Runtime motif preparation | consumes | one validated MotifEvidenceArchive | archive validator reconstructs support and disposition manifests | 1.0 | runtime coordinator | validated input | scheduler | catalog-runtime | application | K1Runtime | prepare motif context | archive root | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/evidence/motif.rs | natural motif lane only |
| t2 | K1Runtime motif preparation | passes together | Evidence V4 rows support receipts and disposition summary | Catalog V2 builder requires all three inputs | 1.0 | runtime coordinator | catalog inputs | scheduler | catalog-runtime | application | K1Runtime | build motif catalog call | complete input tuple | crates/nando-operator-learning/src/multi_source/k1_natural_scheduler_v1/selection.rs | no partial input |
| t3 | K1Runtime motif preparation | preserves separately | source row and retained occurrence denominators | Catalog V2 validation rejects mixed totals | 1.0 | runtime coordinator | denominator pair | scheduler | catalog-runtime | application | K1Runtime | prepare motif context | exact Catalog V2 | crates/nando-operator-learning/src/multi_source/k1_natural_scheduler_v1/model/cohort.rs | no authority |
| t4 | K1Runtime historical preparation | preserves | active V1 to V5 catalog route | historical generation cannot be migrated retroactively | 1.0 | runtime coordinator | compatibility route | scheduler | catalog-runtime | application | K1Runtime | projection schema dispatch | unchanged historical context | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/lifecycle.rs | no freeze change in this cut |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | K1Runtime motif preparation | consumes | one validated MotifEvidenceArchive | candidate implementation | 0.99 | runtime coordinator | validated input | scheduler | catalog-runtime | application | K1Runtime | prepare motif context | archive root | candidate_answer | natural motif lane only |
| c2 | K1Runtime motif preparation | passes together | Evidence V4 rows support receipts and disposition summary | candidate implementation | 0.99 | runtime coordinator | catalog inputs | scheduler | catalog-runtime | application | K1Runtime | build motif catalog call | complete input tuple | candidate_answer | no partial input |
| c3 | K1Runtime motif preparation | preserves separately | source row and retained occurrence denominators | candidate implementation | 0.99 | runtime coordinator | denominator pair | scheduler | catalog-runtime | application | K1Runtime | prepare motif context | exact Catalog V2 | candidate_answer | no authority |
| c4 | K1Runtime historical preparation | preserves | active V1 to V5 catalog route | candidate implementation | 0.99 | runtime coordinator | compatibility route | scheduler | catalog-runtime | application | K1Runtime | projection schema dispatch | unchanged historical context | candidate_answer | no freeze change in this cut |

## notes

- Pure model builders do not own runtime selection; K1Runtime owns this preparation decision.
- Freeze, future, certification, and product execution are outside this packet.
