# NANDA Triad Worksheet

task_id: s1c3c-authority-envelope-v1
domain: code
query: Does the S1C-3C postmortem preserve the unsealed authority envelope without retroactive authority?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | frozen S1C-3C terminal verifier | rejects | parity-mismatch VETO sealing | exact candidate envelope error | 1.0 | authority verifier | authority envelope | authority | authority-envelope | evidence | authority owner | terminal verification | unsealed | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_TERMINAL_REPORT_2026-08-12.md:1 | exact attempt |
| t2 | authority-free postmortem | cannot grant | preregistered or scientific authority | report authority fields are false | 1.0 | diagnostic verifier | authority boundary | authority | authority-envelope | proof | authority owner | stored evidence only | facts only | ops/remote-backend/verify_s1c3c_postmortem_v1.py:1 | postmortem only |
| t3 | consumed S1C-3C attempt | forbids | rerun or automatic S1C-3D | frozen one-attempt contract and terminal status | 1.0 | consumed attempt | future authority | authority | authority-envelope | proof | authority owner | terminal result | route closed | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_TERMINAL_STATUS.json:1 | S1C-3C only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | frozen S1C-3C terminal verifier | rejects | parity-mismatch VETO sealing | terminal report | 1.0 | authority verifier | authority envelope | authority | authority-envelope | conclusion | authority owner | terminal report | unsealed | candidate_answer:c1 | exact attempt |
| c2 | authority-free postmortem | cannot grant | preregistered or scientific authority | terminal report | 1.0 | diagnostic verifier | authority boundary | authority | authority-envelope | conclusion | authority owner | terminal report | facts only | candidate_answer:c2 | postmortem only |
| c3 | consumed S1C-3C attempt | forbids | rerun or automatic S1C-3D | terminal report | 1.0 | consumed attempt | future authority | authority | authority-envelope | conclusion | authority owner | terminal report | route closed | candidate_answer:c3 | S1C-3C only |
