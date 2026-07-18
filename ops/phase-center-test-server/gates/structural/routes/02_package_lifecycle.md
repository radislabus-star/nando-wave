# Route 02: Package Lifecycle

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|---|---|---|---|---|---:|---|---|---|---|---|---|---|---|---|---|
| t01 | profile_compiler | decomposes | routing_execution_safety_profiles | all children retain one package_id and version | 1.0 | compiler | profile set | package | package-lifecycle | cold | package lifecycle coordinator | validated package | linked profiles | /home/ubu/projects/rsmod/NANDO_WAVE_OWNER_VISION_RU.md | production |
| t02 | quarantine_registry | stores | linked_candidate_profiles | quarantine never performs local accept | 1.0 | registry | profile state | lifecycle | package-lifecycle | warm | package lifecycle coordinator | linked profiles | quarantined version | /home/ubu/projects/rsmod/NANDO_WAVE_OWNER_VISION_RU.md | production |
| t03 | future_shadow | evaluates | fixed_package_on_fresh_events | score-before-update, cache exclusion, global dedupe, independent labels | 1.0 | evaluator | future evidence | lifecycle | package-lifecycle | proof | package lifecycle coordinator | quarantined version | frozen evidence | /home/ubu/projects/rsmod/NANDO_WAVE_OWNER_VISION_RU.md | production |
| t04 | automatic_promotion_policy | promotes | package_version | denominator, zero-error bound, parity, drift, budgets, and fallback gates | 1.0 | policy | active version | lifecycle | package-lifecycle | application | package lifecycle coordinator | frozen evidence | atomic registry update | /home/ubu/projects/nando-wave/docs/ROUTER_ACTOR_VERIFIER_ARCHITECTURE.md | production |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|---|---|---|---|---|---:|---|---|---|---|---|---|---|---|---|---|
| c01 | profile_compiler | decomposes | routing_execution_safety_profiles | import_package creates linked profiles from every validated transition | 1.0 | compiler | profile set | package | package-lifecycle | cold | package lifecycle coordinator | validated package | linked profiles | /home/ubu/projects/nando-wave/crates/nando-transition-inducer/src/live_profile.rs | production |
| c02 | quarantine_registry | stores | linked_candidate_profiles | imported profiles start in Quarantine and active_profile_indices excludes them | 1.0 | registry | profile state | lifecycle | package-lifecycle | warm | package lifecycle coordinator | linked profiles | quarantined version | /home/ubu/projects/nando-wave/crates/nando-transition-inducer/src/live_profile.rs | production |
| c03 | future_shadow | evaluates | fixed_package_on_fresh_events | daemon watermark and seen_trace_ids keep fresh evidence separate before promotion | 1.0 | evaluator | future evidence | lifecycle | package-lifecycle | proof | package lifecycle coordinator | quarantined version | frozen evidence | /home/ubu/projects/nando-wave/crates/nando-transition-inducer/src/bin/nando-transition-profile-daemon.rs | production |
| c04 | automatic_promotion_policy | promotes | package_version | maybe_promote_profile applies the versioned zero-error denominator, parity, and latency policy | 1.0 | policy | active version | lifecycle | package-lifecycle | application | package lifecycle coordinator | frozen evidence | atomic registry update | /home/ubu/projects/nando-wave/crates/nando-transition-inducer/src/bin/nando-transition-profile-daemon.rs | production |
