# NANDA Triad Worksheet

task_id: nando-motif-freeze-runtime-20260810
domain: general
query: Validate K1Runtime projection dispatch from Catalog V2 through Queue V2 to an authority-false Freeze V6 request

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | K1Runtime projection dispatch | preserves | an active historical Freeze V1 to V5 generation | active generations are immutable and are never reinterpreted under a newer DiscoveryBasis | 1.0 | runtime generation coordinator | active compatibility generation | scheduler | runtime-freeze | application | K1Runtime | advance | unchanged historical continuation | ARCHITECTURE_CANON.md | active generation only |
| t2 | K1Runtime projection dispatch | selects | shadow Catalog V2 only for a new generation with no active freeze | motif archive and Catalog V2 are prepared beside the historical catalog | 1.0 | runtime generation coordinator | new-generation catalog | scheduler | runtime-freeze | application | K1Runtime | advance waiting generation | selected Catalog V2 | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/lifecycle.rs | no retroactive migration |
| t3 | K1Runtime queue derivation | derives | Queue V2 from selected Catalog V2 deficit snapshot and exact exclusions | queue builder schema-dispatches from catalog schema and remains operator blind | 1.0 | runtime generation coordinator | immutable preregistration queue | scheduler | runtime-freeze | application | K1Runtime | build candidate queue | bounded Queue V2 | crates/nando-operator-learning/src/multi_source/k1_natural_scheduler_v1/selection.rs | no program hints |
| t4 | K1Runtime candidate selection | selects | first readiness PASS Queue V2 candidate | frozen generation cannot be replaced before a terminal result | 1.0 | runtime generation coordinator | preregistered structural candidate | scheduler | runtime-freeze | application | K1Runtime | first readiness pass | one candidate | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/lifecycle.rs | one active generation |
| t5 | K1Runtime freeze construction | seals | Freeze V6 request with Catalog V2 motif roots watermarks and DiscoveryBasis V4 | Freeze V6 model binds exact motif support embedding disposition and overflow roots | 1.0 | runtime generation coordinator | immutable authority request | scheduler | runtime-freeze | application | K1Runtime | candidate freeze seal | Freeze V6 request | crates/nando-operator-learning/src/multi_source/k1_natural_scheduler_v1/model/freeze.rs | authority false |
| t6 | K1Runtime freeze construction | forbids | execution authority and phase mutation | candidate freeze validation requires authority_ready false and phase_mutation_allowed false | 1.0 | runtime generation coordinator | forbidden authority effects | scheduler | runtime-freeze | application | K1Runtime | candidate freeze seal | fail-closed preregistration | crates/nando-operator-learning/src/multi_source/k1_natural_scheduler_v1/model/freeze.rs | future and certification excluded |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | K1Runtime projection dispatch | preserves | an active historical Freeze V1 to V5 generation | implementation assertion for schema-dispatched active-generation continuation | 0.99 | runtime generation coordinator | active compatibility generation | scheduler | runtime-freeze | application | K1Runtime | advance | unchanged historical continuation | candidate_answer | active generation only |
| c2 | K1Runtime projection dispatch | selects | shadow Catalog V2 only for a new generation with no active freeze | implementation assertion for new-generation Catalog V2 selection | 0.99 | runtime generation coordinator | new-generation catalog | scheduler | runtime-freeze | application | K1Runtime | advance waiting generation | selected Catalog V2 | candidate_answer | no retroactive migration |
| c3 | K1Runtime queue derivation | derives | Queue V2 from selected Catalog V2 deficit snapshot and exact exclusions | implementation assertion for operator-blind Queue V2 derivation | 0.99 | runtime generation coordinator | immutable preregistration queue | scheduler | runtime-freeze | application | K1Runtime | build candidate queue | bounded Queue V2 | candidate_answer | no program hints |
| c4 | K1Runtime candidate selection | selects | first readiness PASS Queue V2 candidate | implementation assertion for one immutable active generation | 0.99 | runtime generation coordinator | preregistered structural candidate | scheduler | runtime-freeze | application | K1Runtime | first readiness pass | one candidate | candidate_answer | one active generation |
| c5 | K1Runtime freeze construction | seals | Freeze V6 request with Catalog V2 motif roots watermarks and DiscoveryBasis V4 | implementation assertion for canonical Freeze V6 sealing | 0.99 | runtime generation coordinator | immutable authority request | scheduler | runtime-freeze | application | K1Runtime | candidate freeze seal | Freeze V6 request | candidate_answer | authority false |
| c6 | K1Runtime freeze construction | forbids | execution authority and phase mutation | implementation assertion for authority-free preregistration | 1.0 | runtime generation coordinator | forbidden authority effects | scheduler | runtime-freeze | application | K1Runtime | candidate freeze seal | fail-closed preregistration | candidate_answer | future and certification excluded |

## notes

- Pure Catalog and Queue functions are data derivations called by K1Runtime; they do not own generation selection.
- CertificationAuthority validation and signed journal append are checked in a separate owner-coherent packet.
