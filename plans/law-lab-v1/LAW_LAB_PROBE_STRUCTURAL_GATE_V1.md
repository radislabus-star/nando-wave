# Law Lab Probe Structural Gate V1

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t01 | law-lab | delegates identification to | OperatorIdentificationMachineV1 | LAW_LAB_PREREGISTRATION_V1.md section 3 | 1.0 | preregistration | identifier | law-lab | law-lab |
| t02 | surviving hypotheses | precommit before | probe execution | LAW_LAB_PREREGISTRATION_V1.md section 6 | 1.0 | predictors | execution | law-lab | law-lab |
| t03 | independent oracle | verifies | exact probe outcome | LAW_LAB_PREREGISTRATION_V1.md section 5 | 1.0 | verifier | outcome | law-lab | law-lab |
| t04 | lab probe | may produce only | UniqueLawCandidate | LAW_LAB_PREREGISTRATION_V1.md section 9 | 1.0 | experimental evidence | candidate | law-lab | law-lab |
| t05 | law-lab | cannot issue | LawCertificate or execution authority | LAW_LAB_PREREGISTRATION_V1.md section 9 | 1.0 | research owner | production authority | law-lab | law-lab |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c01 | law-lab | delegates identification to | OperatorIdentificationMachineV1 | law_lab_contract.rs hypothesis_policy_v1 | 1.0 | preregistration | identifier | law-lab | law-lab |
| c02 | surviving hypotheses | precommit before | probe execution | law_lab_contract.rs lifecycle_policy_v1 | 1.0 | predictors | execution | law-lab | law-lab |
| c03 | independent oracle | verifies | exact probe outcome | law_lab_contract.rs probe_policy_v1 | 1.0 | verifier | outcome | law-lab | law-lab |
| c04 | lab probe | may produce only | UniqueLawCandidate | law_lab_contract.rs authority_boundary_v1 | 1.0 | experimental evidence | candidate | law-lab | law-lab |
| c05 | law-lab | cannot issue | LawCertificate or execution authority | law_lab_contract.rs authority_boundary_v1 | 1.0 | research owner | production authority | law-lab | law-lab |
