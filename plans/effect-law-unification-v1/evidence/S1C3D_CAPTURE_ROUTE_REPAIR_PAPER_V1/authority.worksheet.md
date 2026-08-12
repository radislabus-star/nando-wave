# NANDA Triad Worksheet

task_id: s1c3d-authority-v1
domain: code
query: Does S1C-3D limit authority to capture installation and keep science closed?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | predeployment verifier | grants | one mutation attempt for exact candidate identity | attempt discipline | 1.0 | deployment authority owner | bounded mutation authority | authority | predeployment-mutation | proof | predeployment verifier | frozen packet | install or rollback | plans/effect-law-unification-v1/S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md:172 | exact repair epoch |
| t2 | deployment PASS | opens | S1C-4 collecting at new append cursor | result matrix | 1.0 | operational installation result | natural census entry | authority | postinstall-census | operation | S1C-3D transaction owner | sealed deployment receipt | COLLECTING only | plans/effect-law-unification-v1/S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md:213 | post-install |
| t3 | S1C-3D paper owner | forbids | scientific authority training phase mutation and K2 | forbidden mutation list | 1.0 | scientific boundary owner | forbidden promotion | science | science-boundary | paper | S1C-3D paper owner | repair contract | authority false | plans/effect-law-unification-v1/S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md:160 | all outcomes |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | predeployment verifier | grants | one mutation attempt for exact candidate identity | candidate claim c1 | 1.0 | deployment authority owner | bounded mutation authority | authority | predeployment-mutation | conclusion | predeployment verifier | frozen packet | install or rollback | candidate_answer:c1 | exact repair epoch |
| c2 | deployment PASS | opens | S1C-4 collecting at new append cursor | candidate claim c2 | 1.0 | operational installation result | natural census entry | authority | postinstall-census | conclusion | S1C-3D transaction owner | sealed deployment receipt | COLLECTING only | candidate_answer:c2 | post-install |
| c3 | S1C-3D paper owner | forbids | scientific authority training phase mutation and K2 | candidate claim c3 | 1.0 | scientific boundary owner | forbidden promotion | science | science-boundary | conclusion | S1C-3D paper owner | repair contract | authority false | candidate_answer:c3 | all outcomes |
