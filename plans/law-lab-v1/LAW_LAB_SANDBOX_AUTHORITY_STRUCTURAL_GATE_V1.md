# Law Lab Sandbox Authority Structural Gate V1

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t01 | natural candidate and version space | are produced by | existing OperatorIdentificationMachineV1 | LAW_LAB_SANDBOX_ADAPTER_V1.md section 1 | 1.0 | natural evidence | identifier | sandbox-authority | sandbox-authority |
| t02 | external durable prediction ledger | binds before | sandbox execution | LAW_LAB_SANDBOX_ADAPTER_V1.md section 3 | 1.0 | external commitment | experimental executor | sandbox-authority | sandbox-authority |
| t03 | sandbox adapter | cannot write | prediction commitments | LAW_LAB_SANDBOX_ADAPTER_V1.md section 3 | 1.0 | experimental executor | external commitment | sandbox-authority | sandbox-authority |
| t04 | generated capability fixture | cannot seed | natural candidate or holdout | LAW_LAB_SANDBOX_ADAPTER_V1.md section 9 | 1.0 | generated evidence | natural evidence | sandbox-authority | sandbox-authority |
| t05 | exact sandbox outcome | may distinguish only | already frozen hypotheses | LAW_LAB_SANDBOX_ADAPTER_V1.md section 9 | 1.0 | experimental outcome | hypothesis set | sandbox-authority | sandbox-authority |
| t06 | sandbox receipt | cannot grant | LawCertificate, K1, package, phase, or economics authority | LAW_LAB_SANDBOX_ADAPTER_V1.md section 9 | 1.0 | experimental receipt | production authority | sandbox-authority | sandbox-authority |
| t07 | UniqueLawCandidate | still requires | new independent natural holdout | LAW_LAB_SANDBOX_ADAPTER_V1.md section 9 | 1.0 | lab candidate | natural evidence | sandbox-authority | sandbox-authority |
| t08 | external LawCertificate | alone may feed | Epistemic Registry and K1 | LAW_LAB_SANDBOX_ADAPTER_V1.md section 9 | 1.0 | certification authority | K1 evidence | sandbox-authority | sandbox-authority |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c01 | natural candidate and version space | are produced by | existing OperatorIdentificationMachineV1 | law_lab_sandbox/model.rs request roots | 1.0 | natural evidence | identifier | sandbox-authority | sandbox-authority |
| c02 | external durable prediction ledger | binds before | sandbox execution | law_lab_sandbox/model.rs durable_prediction_ledger_root_sha256 | 1.0 | external commitment | experimental executor | sandbox-authority | sandbox-authority |
| c03 | sandbox adapter | cannot write | prediction commitments | law_lab_sandbox/model.rs LawLabSandboxAuthorityBoundaryV1 | 1.0 | experimental executor | external commitment | sandbox-authority | sandbox-authority |
| c04 | generated capability fixture | cannot seed | natural candidate or holdout | law_lab_sandbox/model.rs capability report | 1.0 | generated evidence | natural evidence | sandbox-authority | sandbox-authority |
| c05 | exact sandbox outcome | may distinguish only | already frozen hypotheses | law_lab_sandbox/model.rs worker outcome | 1.0 | experimental outcome | hypothesis set | sandbox-authority | sandbox-authority |
| c06 | sandbox receipt | cannot grant | LawCertificate, K1, package, phase, or economics authority | law_lab_sandbox/model.rs LawLabSandboxAuthorityBoundaryV1 | 1.0 | experimental receipt | production authority | sandbox-authority | sandbox-authority |
| c07 | UniqueLawCandidate | still requires | new independent natural holdout | law_lab_sandbox/model.rs natural_holdout_satisfied | 1.0 | lab candidate | natural evidence | sandbox-authority | sandbox-authority |
| c08 | external LawCertificate | alone may feed | Epistemic Registry and K1 | law_lab_contract.rs authority_boundary_v1 | 1.0 | certification authority | K1 evidence | sandbox-authority | sandbox-authority |
