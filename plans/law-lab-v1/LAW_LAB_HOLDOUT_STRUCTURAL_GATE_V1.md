# Law Lab Holdout Structural Gate V1

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t01 | ordinary traffic residual | seeds | candidate area | LAW_LAB_PREREGISTRATION_V1.md section 4 | 1.0 | natural source | candidate area | natural-holdout | natural-holdout |
| t02 | generated fixture or teacher output | cannot seed | candidate area | LAW_LAB_PREREGISTRATION_V1.md section 4 | 1.0 | excluded source | candidate area | natural-holdout | natural-holdout |
| t03 | lab probe | cannot satisfy | natural holdout | LAW_LAB_PREREGISTRATION_V1.md section 4 | 1.0 | experimental evidence | natural evidence | natural-holdout | natural-holdout |
| t04 | post-candidate natural holdout | may feed | external LawCertificate authority | LAW_LAB_PREREGISTRATION_V1.md section 9 | 1.0 | natural evidence | certification | natural-holdout | natural-holdout |
| t05 | external LawCertificate authority | may grant | Epistemic Registry membership | LAW_LAB_PREREGISTRATION_V1.md section 9 | 1.0 | certification | K1 evidence | natural-holdout | natural-holdout |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c01 | ordinary traffic residual | seeds | candidate area | law_lab_contract.rs evidence_policy_v1 | 1.0 | natural source | candidate area | natural-holdout | natural-holdout |
| c02 | generated fixture or teacher output | cannot seed | candidate area | law_lab_contract.rs evidence_policy_v1 | 1.0 | excluded source | candidate area | natural-holdout | natural-holdout |
| c03 | lab probe | cannot satisfy | natural holdout | law_lab_contract.rs evidence_policy_v1 | 1.0 | experimental evidence | natural evidence | natural-holdout | natural-holdout |
| c04 | post-candidate natural holdout | may feed | external LawCertificate authority | law_lab_contract.rs authority_boundary_v1 | 1.0 | natural evidence | certification | natural-holdout | natural-holdout |
| c05 | external LawCertificate authority | may grant | Epistemic Registry membership | operator_certification.rs LawCertificateV1 | 1.0 | certification | K1 evidence | natural-holdout | natural-holdout |
