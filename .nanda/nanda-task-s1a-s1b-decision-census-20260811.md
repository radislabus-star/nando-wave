# S1A Transition Projection To S1B Decision Census

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | anchored certification CPU completion topology outcome and transport binding | grounds only | read-only GroundedTransitionEpisodeV1 | plans/effect-law-unification-v1/GROUNDED_MEANING_ARCHITECTURE_V1.md:132 | 1.0 | immutable evidence | transition fact | s1a | decision-census |
| t2 | GroundedTransitionEpisodeV1 | supports at most | dynamics evidence | plans/effect-law-unification-v1/GROUNDED_MEANING_ARCHITECTURE_V1.md:132 | 1.0 | transition fact | scoped claim | s1a | decision-census |
| t3 | pre-action goal K1 alternative frozen horizon and independent satisfaction | permits only | GroundedDecisionEpisodeV1 | plans/effect-law-unification-v1/GROUNDED_MEANING_ARCHITECTURE_V1.md:158 | 1.0 | decision contract surface | decision evidence | s1b | decision-census |
| t4 | S1A and S1B reports | grant neither | authority or phase mutation | plans/effect-law-unification-v1/GROUNDED_MEANING_ARCHITECTURE_V1.md:446 | 1.0 | cold report | runtime authority | boundary | decision-census |
| t5 | gateway control | exposes only | fail-closed decision roots and exact counters | plans/effect-law-unification-v1/GROUNDED_MEANING_ARCHITECTURE_V1.md:446 | 1.0 | observer | report projection | dashboard | decision-census |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | anchored certification CPU completion topology outcome and transport binding | grounds only | read-only GroundedTransitionEpisodeV1 | crates/nando-transition-serving/src/grounded_decision_census.rs:93 | 1.0 | immutable evidence | transition fact | s1a | decision-census |
| c2 | GroundedTransitionEpisodeV1 | supports at most | dynamics evidence | crates/nando-operator-learning/src/grounded_decision/census.rs:168 | 1.0 | transition fact | scoped claim | s1a | decision-census |
| c3 | pre-action goal K1 alternative frozen horizon and independent satisfaction | permits only | GroundedDecisionEpisodeV1 | crates/nando-operator-learning/src/grounded_decision/census.rs:280 | 1.0 | decision contract surface | decision evidence | s1b | decision-census |
| c4 | S1A and S1B reports | grant neither | authority or phase mutation | crates/nando-operator-learning/src/grounded_decision/census.rs:402 | 1.0 | cold report | runtime authority | boundary | decision-census |
| c5 | gateway control | exposes only | fail-closed decision roots and exact counters | crates/nando-gateway-control/src/main.rs:1686 | 1.0 | observer | report projection | dashboard | decision-census |

## notes

- The S1B contract surface includes a goal bound before action, a meaningful nonselected K1 alternative, a pre-frozen outcome horizon, independently verified satisfaction, and natural provenance.
- Structural coherence only; this packet does not grant scientific or runtime authority.
- S1A transition evidence remains `DYNAMICS_ONLY` while the S1B decision surface is empty.
