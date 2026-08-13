# NANDA Triad Worksheet

task_id: s1c4-terminal-route-separation-v1
domain: code
query: Does the projection separate disabled legacy MS3, active K1 discovery, terminal S1C-4 evidence, and closed K2 without changing authority?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | legacy MS3 config | disables | MS3 freezer route | production has NANDO_MULTI_SOURCE_RESEARCH_ENABLED=0 | 1.0 | configuration owner | legacy route | authority | legacy-ms3 | runtime | cold learner config | cold startup | disabled route | plans/effect-law-unification-v1/S1C4_TERMINAL_ROUTE_SEPARATION_PREREGISTRATION_V1.md | production |
| t2 | remote evidence spool | reports | authenticated route-bound frames | cold health reports more than eleven thousand route-bound frames | 1.0 | evidence owner | transport fact | observation | transport | evidence | cold remote spool | cold health | transport count | plans/effect-law-unification-v1/S1C4_TERMINAL_ROUTE_SEPARATION_PREREGISTRATION_V1.md | production |
| t3 | disabled legacy route | returns | LEGACY_MS3_RESEARCH_DISABLED | inactive route cannot claim a missing terminal link | 1.0 | route owner | health blocker | authority | health-semantics | runtime | cold health | health request | compatibility projection | plans/effect-law-unification-v1/S1C4_TERMINAL_ROUTE_SEPARATION_PREREGISTRATION_V1.md | production |
| t4 | K1 scheduler | owns | Law number two discovery state | summary reports waiting_for_evidence and ready_now zero | 1.0 | candidate rank owner | discovery state | authority | k1-discovery | science | K1 scheduler | scheduler tick | scheduler summary | plans/effect-law-unification-v1/S1C4_TERMINAL_ROUTE_SEPARATION_PREREGISTRATION_V1.md | science |
| t5 | S1C-4 terminal report | proves | empty production goal surface in frozen window | 1024 requests classified and zero goals | 1.0 | evidence owner | terminal census result | proof | s1c4-result | science | S1C census | immutable report | EMPTY_GOAL_SURFACE | plans/effect-law-unification-v1/S1C4_TERMINAL_ROUTE_SEPARATION_PREREGISTRATION_V1.md | science |
| t6 | S1C-4 empty result | does_not_prove | grounded meaning false | missing evidence surface is not a mechanism refutation | 1.0 | evidence result | excluded claim | proof | s1c-claim-boundary | science | S1C census | terminal verdict | K2 closed | plans/effect-law-unification-v1/S1C4_TERMINAL_ROUTE_SEPARATION_CRITIQUE_V1.md | science |
| t7 | dashboard API | passes_to | three separate status rows | K1 S1C-4 and K2 retain distinct owners | 1.0 | display rank owner | renderer | authority | dashboard | display | gateway control | dashboard refresh | factual UI | plans/effect-law-unification-v1/S1C4_TERMINAL_ROUTE_SEPARATION_PREREGISTRATION_V1.md | production |
| t8 | scoped deployment | preserves | immutable reports and natural suffixes | status repair has no evidence mutation authority | 1.0 | mutation owner | evidence artifacts | authority | rollback | operations | deployment transaction | scoped install | preserved evidence | plans/effect-law-unification-v1/S1C4_TERMINAL_ROUTE_SEPARATION_PREREGISTRATION_V1.md | production |
| t9 | route separation PASS | does_not_grant | Law 2 K2 training or phase authority | claim boundary keeps every authority false | 1.0 | operational result | forbidden promotion | proof | route-repair-claim-boundary | science | independent verifier | final receipt | no promotion | plans/effect-law-unification-v1/S1C4_TERMINAL_ROUTE_SEPARATION_PREREGISTRATION_V1.md | science |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | legacy MS3 config | disables | MS3 freezer route | candidate c1 | 1.0 | configuration owner | legacy route | authority | legacy-ms3 | runtime | cold learner config | cold startup | disabled route | candidate_answer:c1 | production |
| c2 | remote evidence spool | reports | authenticated route-bound frames | candidate c2 | 1.0 | evidence owner | transport fact | observation | transport | evidence | cold remote spool | cold health | transport count | candidate_answer:c2 | production |
| c3 | disabled legacy route | returns | LEGACY_MS3_RESEARCH_DISABLED | candidate c3 | 1.0 | route owner | health blocker | authority | health-semantics | runtime | cold health | health request | compatibility projection | candidate_answer:c3 | production |
| c4 | K1 scheduler | owns | Law number two discovery state | candidate c4 | 1.0 | candidate rank owner | discovery state | authority | k1-discovery | science | K1 scheduler | scheduler tick | scheduler summary | candidate_answer:c4 | science |
| c5 | S1C-4 terminal report | proves | empty production goal surface in frozen window | candidate c5 | 1.0 | evidence owner | terminal census result | proof | s1c4-result | science | S1C census | immutable report | EMPTY_GOAL_SURFACE | candidate_answer:c5 | science |
| c6 | S1C-4 empty result | does_not_prove | grounded meaning false | candidate c6 | 1.0 | evidence result | excluded claim | proof | s1c-claim-boundary | science | S1C census | terminal verdict | K2 closed | candidate_answer:c6 | science |
| c7 | dashboard API | passes_to | three separate status rows | candidate c7 | 1.0 | display rank owner | renderer | authority | dashboard | display | gateway control | dashboard refresh | factual UI | candidate_answer:c7 | production |
| c8 | scoped deployment | preserves | immutable reports and natural suffixes | candidate c8 | 1.0 | mutation owner | evidence artifacts | authority | rollback | operations | deployment transaction | scoped install | preserved evidence | candidate_answer:c8 | production |
| c9 | route separation PASS | does_not_grant | Law 2 K2 training or phase authority | candidate c9 | 1.0 | operational result | forbidden promotion | proof | route-repair-claim-boundary | science | independent verifier | final receipt | no promotion | candidate_answer:c9 | science |
