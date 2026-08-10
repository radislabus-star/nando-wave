# NANDA Triad Worksheet

task_id: nando-motif-frame-budget-20260810
domain: general
query: Validate bounded frame materialization for frozen K1 motif identification without weakening future evidence

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | Frame archive | indexes | canonical frame root to immutable RelationFrame | canonical root is validated at restore and append | 1.0 | evidence owner | immutable frame | evidence | frame-index | IO | MultiSourceFrameArchive | archive open and append | restart-stable root index | crates/nando-transition-serving/src/multi_source_frame_archive.rs | no authority |
| t2 | Frozen candidate support | selects | exact completed frame roots bound before identification | support rows remain bound to immutable freeze and watermark | 1.0 | frozen evidence | bounded frame roots | identification | identification-budget | proof | K1Runtime | frozen support lookup | exact root set | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/evidence/identification.rs | support only |
| t3 | Initial frozen identification | consumes | only frames named by frozen support roots | archive lookup fails closed when any requested root is missing | 1.0 | identifier caller | bounded frame set | identification | identification-budget | application | K1Runtime | candidate frozen transition | at most frozen support frames | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/service.rs | before identification freeze |
| t4 | Identification freeze | restores before | full topology and frame snapshot for future settlement | future selection and missing-frame fences retain their original complete inputs | 1.0 | durable identification | complete future evidence | future | future-boundary | application | K1Runtime | post-identification transition | unchanged future route | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/service.rs | after identification freeze |
| t5 | Bounded frame materialization | does not grant | execution authority or phase mutation | scheduler reports remain authority false until independent lifecycle gates pass | 1.0 | performance optimization | authority boundary | authority | authority-boundary | proof | certification authority | scheduler advance | no authority promotion | crates/nando-transition-serving/src/k1_natural_scheduler_runtime/lifecycle.rs | all K1 lanes |
| t6 | Dashboard Law 2 verdict | separates from | current discovery state and previous terminal blocker | LawCertificate count alone controls the Law 2 verdict | 1.0 | control plane | claim boundaries | dashboard | claim-boundary | interface | gateway control | live dashboard refresh | separate live facts | crates/nando-gateway-control/src/live_dashboard.rs | display only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | Frame archive | indexes | canonical frame root to immutable RelationFrame | candidate implementation | 0.99 | evidence owner | immutable frame | evidence | frame-index | IO | MultiSourceFrameArchive | archive open and append | restart-stable root index | candidate_answer | no authority |
| c2 | Frozen candidate support | selects | exact completed frame roots bound before identification | candidate implementation | 0.99 | frozen evidence | bounded frame roots | identification | identification-budget | proof | K1Runtime | frozen support lookup | exact root set | candidate_answer | support only |
| c3 | Initial frozen identification | consumes | only frames named by frozen support roots | candidate implementation | 0.99 | identifier caller | bounded frame set | identification | identification-budget | application | K1Runtime | candidate frozen transition | at most frozen support frames | candidate_answer | before identification freeze |
| c4 | Identification freeze | restores before | full topology and frame snapshot for future settlement | candidate implementation | 0.99 | durable identification | complete future evidence | future | future-boundary | application | K1Runtime | post-identification transition | unchanged future route | candidate_answer | after identification freeze |
| c5 | Bounded frame materialization | does not grant | execution authority or phase mutation | candidate implementation | 0.99 | performance optimization | authority boundary | authority | authority-boundary | proof | certification authority | scheduler advance | no authority promotion | candidate_answer | all K1 lanes |
| c6 | Dashboard Law 2 verdict | separates from | current discovery state and previous terminal blocker | candidate implementation | 0.99 | control plane | claim boundaries | dashboard | claim-boundary | interface | gateway control | live dashboard refresh | separate live facts | candidate_answer | display only |

## notes

- The optimization changes only pre-identification frame materialization.
- Full evidence remains mandatory once an identification freeze can select or settle future evidence.
- A NANDA PASS is coherence-only and does not authorize deployment.
