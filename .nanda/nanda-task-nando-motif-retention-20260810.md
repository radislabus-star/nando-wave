# NANDA Triad Worksheet

task_id: nando-motif-retention-20260810
domain: general
query: Validate bounded exact motif evidence retention and denominator ownership

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | ambient evidence arena | retains once | complete joined payload | existing EvidenceBinding owns optional boxed join | 1.0 | payload owner | immutable joined row | evidence | ambient | runtime | EvidenceArchive | EvidenceBindingAccumulator | indexed ambient evidence | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/evidence.rs | natural joined rows |
| t2 | exact motif descriptor | references | ambient topology root | embedding commits ambient root and local roles | 1.0 | derived descriptor | ambient evidence | motif | factorization | domain | MotifFactorizer | source_neutral_topology_motifs_v1 | motif plus embeddings | crates/nando-operator-learning/src/multi_source/factorizer.rs | no payload ownership |
| t3 | Evidence V4 | commits | immutable motif occurrence | contract evidence fields exclude aggregate overflow | 1.0 | immutable receipt | motif occurrence | motif | evidence-row | domain | EvidenceModel | seal_motif_v4 | stable row root | plans/nando-live-cpu-savings-v1/K1_EXACT_CONNECTED_MOTIF_DISCOVERY_CONTRACT_V1.md | no authority |
| t4 | motif support reservoir | retains | maximum 64 occurrence references | contract support bound | 1.0 | bounded selector | occurrence reference | motif | retention | application | MotifEvidenceAccumulator | observe occurrence | retained support references | plans/nando-live-cpu-savings-v1/K1_EXACT_CONNECTED_MOTIF_DISCOVERY_CONTRACT_V1.md | preserve independent lineage slots |
| t5 | motif overflow ledger | records | every excluded valid occurrence | contract rolling manifest requirement | 1.0 | disposition ledger | excluded occurrence | motif | retention | proof | MotifEvidenceAccumulator | append overflow | count and rolling root | plans/nando-live-cpu-savings-v1/K1_EXACT_CONNECTED_MOTIF_DISCOVERY_CONTRACT_V1.md | cannot alter retained evidence |
| t6 | Catalog V2 accounting | separates | source row and motif occurrence denominators | one source row can emit multiple motifs | 1.0 | denominator owner | independent denominators | accounting | denominator | reporting | CatalogBuilder | build motif catalog | exact separate counts | crates/nando-operator-learning/examples/k1_motif_frontier_census_v1.rs | fail on mixed partition |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | ambient evidence arena | retains once | complete joined payload | candidate_answer | 0.98 | payload owner | immutable joined row | evidence | ambient | runtime | EvidenceArchive | EvidenceBindingAccumulator | indexed ambient evidence | candidate_answer | never clone per motif |
| c2 | exact motif descriptor | references | ambient topology root | candidate_answer | 0.98 | derived descriptor | ambient evidence | motif | factorization | domain | MotifFactorizer | source_neutral_topology_motifs_v1 | motif plus lightweight binding index | candidate_answer | joined payload remains in ambient arena |
| c3 | Evidence V4 | commits | immutable motif occurrence | candidate_answer | 0.98 | immutable receipt | motif occurrence | motif | evidence-row | domain | EvidenceModel | seal_motif_v4 | stable row root | candidate_answer | evolving overflow excluded |
| c4 | motif support reservoir | retains | maximum 64 occurrence references | candidate_answer | 0.98 | bounded selector | occurrence reference | motif | retention | application | MotifEvidenceAccumulator | observe occurrence | retained support references | candidate_answer | reserve second lineage slot |
| c5 | motif overflow ledger | records | every excluded valid occurrence | candidate_answer | 0.98 | disposition ledger | excluded occurrence | motif | retention | proof | MotifEvidenceAccumulator | append overflow | count and rolling root | candidate_answer | candidate support receipt owns aggregate |
| c6 | Catalog V2 accounting | separates | source row and motif occurrence denominators | candidate_answer | 0.98 | denominator owner | independent denominators | accounting | denominator | reporting | CatalogBuilder | build motif catalog | exact separate counts | candidate_answer | reject mixed totals |

## notes

- Fill `triads` from source evidence.
- Fill `candidate_triads` from the answer being checked.
- Keep one coherent `group` per route, case, or local structure.
- Use `layer`, `owner`, `entrypoint`, `output`, `evidence_path`, and `scope` when checking architecture ownership.
