# NANDA Triad Worksheet

task_id: s1c3h-evidence-boundary-route-v1
domain: code
query: Does S1C-3H preserve prior attempts, append-only natural evidence, repair provenance, and the boundary between installation and K2 claims?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | S1C recorder | records_before_and_after | goal available actions selected action verified result | exact recorder sequence lines 9-20 | 1.0 | evidence instrument | decision evidence | science | capture-route | evidence | S1C owner | ordinary pre-action boundary | append-only receipts | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:9 | evidence only |
| t9 | rollback | preserves | decision journal prefix and natural suffix | exact append-only rule lines 97-99 | 1.0 | rollback owner | append-only evidence | rollback | journal-preservation | evidence | S1C journal owner | frozen cursor | byte-preserved records | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:97 | scientific future |
| t10 | S1C-3H | preserves | S1C-3G immutable terminal evidence | exact successor identity lines 107-108 | 1.0 | successor repair | prior attempt | provenance | prior-attempt-provenance | evidence | attempt ledger owner | new transaction identity | no relabel | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:107 | evidence |
| t11 | engineering repair | may_repeat_only_after | terminal seal new commit preflight and identity | exact engineering lifecycle lines 110-114 | 1.0 | repair lifecycle | bounded new attempt | provenance | engineering-repair-lifecycle | operations | repair lifecycle owner | exact blocker | separately rooted transaction | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:110 | engineering only |
| t12 | natural evidence | may_not_be | retried deleted generated or post-hoc | exact natural evidence prohibitions lines 113-118 | 1.0 | scientific evidence | forbidden mutation | authority | natural-evidence-boundary | science | natural evidence owner | post-install append cursor | immutable suffix | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:113 | science |
| t13 | installation PASS | proves_only | recorder installed fail-closed and READY | exact immediate acceptance lines 122-141 | 1.0 | operational result | scoped claim | authority | installation-claim | science | installation claim owner | deployment receipt | no K2 claim | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:122 | claim boundary |
| t14 | S1C-4 | starts_after | installed recorder and new natural cursor | exact claim boundary lines 143-150 | 1.0 | census stage | operational prerequisite | science | s1c4-census | science | census owner | natural journal suffix | collecting | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:143 | science |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C recorder | records_before_and_after | goal available actions selected action verified result | candidate c1 | 1.0 | evidence instrument | decision evidence | science | capture-route | evidence | S1C owner | ordinary pre-action boundary | append-only receipts | candidate_answer:c1 | evidence only |
| c9 | rollback | preserves | decision journal prefix and natural suffix | candidate c9 | 1.0 | rollback owner | append-only evidence | rollback | journal-preservation | evidence | S1C journal owner | frozen cursor | byte-preserved records | candidate_answer:c9 | scientific future |
| c10 | S1C-3H | preserves | S1C-3G immutable terminal evidence | candidate c10 | 1.0 | successor repair | prior attempt | provenance | prior-attempt-provenance | evidence | attempt ledger owner | new transaction identity | no relabel | candidate_answer:c10 | evidence |
| c11 | engineering repair | may_repeat_only_after | terminal seal new commit preflight and identity | candidate c11 | 1.0 | repair lifecycle | bounded new attempt | provenance | engineering-repair-lifecycle | operations | repair lifecycle owner | exact blocker | separately rooted transaction | candidate_answer:c11 | engineering only |
| c12 | natural evidence | may_not_be | retried deleted generated or post-hoc | candidate c12 | 1.0 | scientific evidence | forbidden mutation | authority | natural-evidence-boundary | science | natural evidence owner | post-install append cursor | immutable suffix | candidate_answer:c12 | science |
| c13 | installation PASS | proves_only | recorder installed fail-closed and READY | candidate c13 | 1.0 | operational result | scoped claim | authority | installation-claim | science | installation claim owner | deployment receipt | no K2 claim | candidate_answer:c13 | claim boundary |
| c14 | S1C-4 | starts_after | installed recorder and new natural cursor | candidate c14 | 1.0 | census stage | operational prerequisite | science | s1c4-census | science | census owner | natural journal suffix | collecting | candidate_answer:c14 | science |
