# Route 01: Ingest And Induction

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|---|---|---|---|---|---:|---|---|---|---|---|---|---|---|---|---|
| t01 | grounded_observation_endpoint | captures | bounded_transition_with_provenance | raw text and upstream LLM self-report are not transition state | 1.0 | adapter | event | ingest | ingest-induction | adapter | transition intake coordinator | application/tool/environment boundary | bounded transition | /home/ubu/projects/nando-wave/docs/ROUTER_ACTOR_VERIFIER_ARCHITECTURE.md | production |
| t02 | evidence_receipt_validator | validates | observed_before_action_after | trusted client supplies applied state and Rust serving recomputes the content receipt | 1.0 | validator | transition | ingest | ingest-induction | application | transition intake coordinator | bounded transition | grounded observed transition | /home/ubu/projects/nando-wave/crates/nando-transition-serving/src/lib.rs | production |
| t03 | transition_inducer | compiles | induced_transition_package | A2 four-family Wave-guided induction contract | 1.0 | inducer | package | induction | ingest-induction | cold | transition intake coordinator | observed transitions | package candidate | /home/ubu/projects/nando-wave/crates/nando-transition-inducer/src/induce.rs | production |
| t04 | package_validator | validates | schema_version_hash_forbidden_flags | incompatible or authority-leaking packages are rejected | 1.0 | validator | package contract | induction | ingest-induction | cold | transition intake coordinator | package candidate | validated package | /home/ubu/projects/rsmod/plans/transition-program-induction-a2/A2_CONTRACT.md | production |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|---|---|---|---|---|---:|---|---|---|---|---|---|---|---|---|---|
| c01 | grounded_observation_endpoint | captures | bounded_transition_with_provenance | Rust POST /v2/transitions/observe accepts only explicit application/tool/environment evidence | 1.0 | adapter | event | ingest | ingest-induction | adapter | transition intake coordinator | application/tool/environment boundary | bounded transition | /home/ubu/projects/nando-wave/crates/nando-transition-serving/src/lib.rs | production |
| c02 | evidence_receipt_validator | validates | observed_before_action_after | Rust serving recomputes SHA-256 over before/action/after/source/verifier and rejects mismatch | 1.0 | validator | transition | ingest | ingest-induction | application | transition intake coordinator | bounded transition | grounded observed transition | /home/ubu/projects/nando-wave/crates/nando-transition-serving/src/lib.rs | production |
| c03 | transition_inducer | compiles | induced_transition_package | A2 PASS: four families, Wave ranking, CEGIS guards/verifiers, routing signature | 1.0 | inducer | package | induction | ingest-induction | cold | transition intake coordinator | support transitions | package candidate | /home/ubu/projects/rsmod/results/transition-program-induction-a2-2026-07-10/A2_PROOF_REPORT.md | production |
| c04 | package_validator | validates | schema_version_hash_forbidden_flags | validate_live_package checks schema, package budget, roundtrip, program, adapter, safety schemas, and routing margin | 1.0 | validator | package contract | induction | ingest-induction | cold | transition intake coordinator | package candidate | validated package | /home/ubu/projects/nando-wave/crates/nando-transition-inducer/src/live_profile.rs | production |
