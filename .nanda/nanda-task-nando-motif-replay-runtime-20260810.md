# NANDA Triad Worksheet

task_id: nando-motif-replay-runtime-20260810
domain: general
query: Validate K1Runtime exact motif occurrence replay for Freeze V6 support and bounded future evidence

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | K1Runtime V6 support replay | consumes | validated MotifEvidenceArchive occurrences | retained V4 occurrences reference one ambient EvidenceBinding arena | 1.0 | runtime evidence coordinator | immutable motif support | scheduler | motif-replay-runtime | application | K1Runtime | frozen support replay | exact support occurrence set | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/evidence/motif.rs | Freeze V6 only |
| t2 | K1Runtime V6 support replay | matches | every frozen candidate identity root and support watermark | Freeze V6 binds motif topology semantic consequence generation and evidence manifest roots | 1.0 | runtime evidence coordinator | frozen candidate identity | scheduler | motif-replay-runtime | application | K1Runtime | frozen support replay | manifest-matched support | crates/nando-operator-learning/src/multi_source/k1_natural_scheduler_v1/model/freeze.rs | no ambient widening |
| t3 | K1Runtime exact occurrence replay | reconstructs | motif descriptor from ambient topology and stored V4 embedding roots | deterministic bounded motif enumeration must reproduce the archived V4 row | 1.0 | runtime evidence coordinator | exact motif descriptor | scheduler | motif-replay-runtime | application | K1Runtime | exact occurrence replay | row and embedding parity | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/evidence/motif.rs | fail closed on mismatch |
| t4 | K1Runtime V6 identification input | aligns | each selected ambient joined row with one exact motif descriptor | identifier input ordering binds support applied and trial evidence | 1.0 | runtime evidence coordinator | bounded aligned identification evidence | scheduler | motif-replay-runtime | application | K1Runtime | identify frozen candidate | joined rows plus motifs | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/evidence/identification.rs | no joined payload clone per archive occurrence |
| t5 | K1Runtime V6 future replay | enumerates only | post-freeze or explicitly applied bounded rows | overflow support is not retroactively support and future remains after watermark | 1.0 | runtime evidence coordinator | bounded future motif occurrence | scheduler | motif-replay-runtime | application | K1Runtime | future evidence selection | exact future occurrence | ARCHITECTURE_CANON.md | no support backfill |
| t6 | K1Runtime compatibility dispatch | preserves | historical Freeze V1 to V5 replay | existing evidence row and topology schema branches remain unchanged | 1.0 | runtime evidence coordinator | compatibility evidence route | scheduler | motif-replay-runtime | application | K1Runtime | frozen evidence dispatch | unchanged historical replay | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/evidence/identification.rs | no retroactive migration |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | K1Runtime V6 support replay | consumes | validated MotifEvidenceArchive occurrences | implementation assertion for archive-owned support selection | 0.99 | runtime evidence coordinator | immutable motif support | scheduler | motif-replay-runtime | application | K1Runtime | frozen support replay | exact support occurrence set | candidate_answer | Freeze V6 only |
| c2 | K1Runtime V6 support replay | matches | every frozen candidate identity root and support watermark | implementation assertion for exact V6 identity filtering | 0.99 | runtime evidence coordinator | frozen candidate identity | scheduler | motif-replay-runtime | application | K1Runtime | frozen support replay | manifest-matched support | candidate_answer | no ambient widening |
| c3 | K1Runtime exact occurrence replay | reconstructs | motif descriptor from ambient topology and stored V4 embedding roots | implementation assertion for deterministic row parity | 0.99 | runtime evidence coordinator | exact motif descriptor | scheduler | motif-replay-runtime | application | K1Runtime | exact occurrence replay | row and embedding parity | candidate_answer | fail closed on mismatch |
| c4 | K1Runtime V6 identification input | aligns | each selected ambient joined row with one exact motif descriptor | implementation assertion for aligned identifier input | 0.99 | runtime evidence coordinator | bounded aligned identification evidence | scheduler | motif-replay-runtime | application | K1Runtime | identify frozen candidate | joined rows plus motifs | candidate_answer | no joined payload clone per archive occurrence |
| c5 | K1Runtime V6 future replay | enumerates only | post-freeze or explicitly applied bounded rows | implementation assertion for bounded future-only enumeration | 0.99 | runtime evidence coordinator | bounded future motif occurrence | scheduler | motif-replay-runtime | application | K1Runtime | future evidence selection | exact future occurrence | candidate_answer | no support backfill |
| c6 | K1Runtime compatibility dispatch | preserves | historical Freeze V1 to V5 replay | implementation assertion for unchanged legacy branch | 1.0 | runtime evidence coordinator | compatibility evidence route | scheduler | motif-replay-runtime | application | K1Runtime | frozen evidence dispatch | unchanged historical replay | candidate_answer | no retroactive migration |

## notes

- Exact motif replay is an evidence projection; it grants no execution authority.
- Pre-action prediction and outcome authority are checked separately after support identification parity.
