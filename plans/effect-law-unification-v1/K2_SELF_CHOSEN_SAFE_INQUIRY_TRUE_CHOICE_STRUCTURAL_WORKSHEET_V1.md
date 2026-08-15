# K2 Active Inquiry True-Choice Boundary

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | public case | contains | four candidate model roots | preregistration:122-142 | 1.0 | public_hypothesis_set | candidate_roots | public_input | public-candidates |
| t2 | public selector input | contains | public hypothesis set | model.rs:415-468 | 1.0 | public_input | public_hypothesis_set | public_input | public-candidates |
| t3 | baseline request | receives | same public case | model.rs:473-526 | 1.0 | baseline_input | public_hypothesis_set | public_input | public-candidates |
| t4 | public candidate identity | belongs to | public hypothesis set | model.rs:277-285 | 1.0 | candidate_identity | public_hypothesis_set | public_input | public-candidates |
| t5 | private true-choice relation | identifies | one candidate only after selection | model.rs:955-1007 | 1.0 | private_choice | candidate_identity | private_outcome | private-choice |
| t6 | outcome verifier | receives after observation | private true-choice relation | verifier.rs:57-128 | 1.0 | proof_consumer | private_choice | private_outcome | private-choice |
| t7 | leakage guard | excludes from selector bytes | private true choice | model.rs:415-421 | 1.0 | exclusion_guard | private_choice | exclusion | leakage-guard |
| t8 | baseline request fields | exclude | private true-choice relation | model.rs:473-479 | 1.0 | baseline_input | private_choice | exclusion | leakage-guard |
| t9 | alternate true choices | produce identical | selector request bytes | repair-rule-selector-choice-invariance | 1.0 | private_choice_variants | selector_bytes | invariance | leakage-guard |
| t10 | alternate true choices | produce identical | baseline request bytes | repair-rule-baseline-choice-invariance | 1.0 | private_choice_variants | baseline_bytes | invariance | leakage-guard |
| t11 | selector | receives no | true-choice label | preregistration:43-45-true-choice | 1.0 | selector | private_choice_label | exclusion | leakage-guard |
| t12 | selector | receives no | post-outcome bytes | preregistration:45-46-post-outcome | 1.0 | selector | post_outcome | exclusion | leakage-guard |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| c1 | public case | contains | four candidate model roots | proposed repair | 1.0 | public_hypothesis_set | candidate_roots | public_input | public-candidates |
| c2 | public selector input | contains | public hypothesis set | proposed repair | 1.0 | public_input | public_hypothesis_set | public_input | public-candidates |
| c3 | baseline request | receives | same public case | proposed repair | 1.0 | baseline_input | public_hypothesis_set | public_input | public-candidates |
| c4 | public candidate identity | belongs to | public hypothesis set | proposed repair | 1.0 | candidate_identity | public_hypothesis_set | public_input | public-candidates |
| c5 | private true-choice relation | identifies | one candidate only after selection | proposed repair | 1.0 | private_choice | candidate_identity | private_outcome | private-choice |
| c6 | outcome verifier | receives after observation | private true-choice relation | proposed repair | 1.0 | proof_consumer | private_choice | private_outcome | private-choice |
| c7 | leakage guard | excludes from selector bytes | private true choice | proposed repair | 1.0 | exclusion_guard | private_choice | exclusion | leakage-guard |
| c8 | baseline request fields | exclude | private true-choice relation | proposed repair | 1.0 | baseline_input | private_choice | exclusion | leakage-guard |
| c9 | alternate true choices | produce identical | selector request bytes | proposed repair | 1.0 | private_choice_variants | selector_bytes | invariance | leakage-guard |
| c10 | alternate true choices | produce identical | baseline request bytes | proposed repair | 1.0 | private_choice_variants | baseline_bytes | invariance | leakage-guard |
| c11 | selector | receives no | true-choice label | proposed repair | 1.0 | selector | private_choice_label | exclusion | leakage-guard |
| c12 | selector | receives no | post-outcome bytes | proposed repair | 1.0 | selector | post_outcome | exclusion | leakage-guard |
