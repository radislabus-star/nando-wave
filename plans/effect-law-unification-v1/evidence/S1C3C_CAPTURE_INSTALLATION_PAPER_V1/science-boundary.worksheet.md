# NANDA Triad Worksheet

task_id: s1c3c-science-boundary-v1
domain: code
query: Does S1C-3C keep capture installation separate from natural S1C-4 and grounded S2 claims?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | S1C-3C paper owner | limits | DEPLOYMENT_PASS to capture installation | preregistration scoped claim | 1.0 | scientific claim boundary owner | operational receipt scope | science | science-boundary | paper | S1C-3C paper owner | successor contract | no scientific authority | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md:27 | successor only |
| t2 | S1C-3C paper owner | permits | S1C-4 natural collection only after deployment PASS | preregistration census opening rule | 1.0 | scientific claim boundary owner | later census entry | science | science-boundary | paper | S1C-3C paper owner | successor contract | COLLECTING or CLOSED | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md:197 | successor boundary |
| t3 | S1C-3C paper owner | forbids | S2 entry without S1C-4 PASS two lineages and sufficient K1 vocabulary | preregistration S2 boundary | 1.0 | scientific claim boundary owner | forbidden promotion | science | science-boundary | paper | S1C-3C paper owner | successor contract | S2 remains blocked | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md:209 | successor boundary |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C-3C paper owner | limits | DEPLOYMENT_PASS to capture installation | candidate claim c1 | 1.0 | scientific claim boundary owner | operational receipt scope | science | science-boundary | conclusion | S1C-3C paper owner | successor contract | no scientific authority | candidate_answer:c1 | successor only |
| c2 | S1C-3C paper owner | permits | S1C-4 natural collection only after deployment PASS | candidate claim c2 | 1.0 | scientific claim boundary owner | later census entry | science | science-boundary | conclusion | S1C-3C paper owner | successor contract | COLLECTING or CLOSED | candidate_answer:c2 | successor boundary |
| c3 | S1C-3C paper owner | forbids | S2 entry without S1C-4 PASS two lineages and sufficient K1 vocabulary | candidate claim c3 | 1.0 | scientific claim boundary owner | forbidden promotion | science | science-boundary | conclusion | S1C-3C paper owner | successor contract | S2 remains blocked | candidate_answer:c3 | successor boundary |
