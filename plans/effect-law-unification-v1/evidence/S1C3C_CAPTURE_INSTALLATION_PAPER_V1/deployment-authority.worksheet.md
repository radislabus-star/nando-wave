# NANDA Triad Worksheet

task_id: s1c3c-deployment-authority-v1
domain: code
query: Does S1C-3C preserve one-attempt deployment and rollback authority?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | independent verifier | requires | paper-authorized single attempt identity | preregistration remote attempt contract | 1.0 | operational authority owner | frozen authority input | deployment | deployment-authority | operation | independent verifier | verified successor envelope | one transaction identity | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md:151 | successor only |
| t2 | independent verifier | requires | unchanged mechanism thresholds and complete denominator | preregistration resource contract | 1.0 | operational authority owner | frozen mechanism receipt | deployment | deployment-authority | operation | independent verifier | verified successor envelope | accepted or rejected resource root | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md:127 | successor only |
| t3 | independent verifier | grants | operational deployment authority after rollback checks | preregistration transaction contract | 1.0 | operational authority owner | capture installation | deployment | deployment-authority | operation | independent verifier | verified successor envelope | DEPLOYMENT_PASS or terminal failure | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md:163 | successor only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | independent verifier | requires | paper-authorized single attempt identity | candidate claim c1 | 1.0 | operational authority owner | frozen authority input | deployment | deployment-authority | conclusion | independent verifier | verified successor envelope | one transaction identity | candidate_answer:c1 | successor only |
| c2 | independent verifier | requires | unchanged mechanism thresholds and complete denominator | candidate claim c2 | 1.0 | operational authority owner | frozen mechanism receipt | deployment | deployment-authority | conclusion | independent verifier | verified successor envelope | accepted or rejected resource root | candidate_answer:c2 | successor only |
| c3 | independent verifier | grants | operational deployment authority after rollback checks | candidate claim c3 | 1.0 | operational authority owner | capture installation | deployment | deployment-authority | conclusion | independent verifier | verified successor envelope | DEPLOYMENT_PASS or terminal failure | candidate_answer:c3 | successor only |
