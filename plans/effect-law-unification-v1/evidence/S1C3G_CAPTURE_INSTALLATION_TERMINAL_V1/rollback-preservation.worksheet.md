# NANDA Triad Worksheet

task_id: s1c3g-rollback-preservation-v1
domain: code
query: Does the S1C-3G terminal packet prove rollback preservation without claiming installation?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | deployment receipt | restores | frozen baseline binary and config | installed hashes equal baseline hashes | 1.0 | rollback receipt | production bytes | rollback | preservation | evidence | rollback preservation projection | receipt | baseline restored | plans/effect-law-unification-v1/evidence/S1C3G_CAPTURE_INSTALLATION_TERMINAL_V1/deployment-receipt.json:1 | S1C-3G only |
| t2 | rollback journal | preserves | three zero-record NTF1 prefixes | exact manifest root and record count zero | 1.0 | append-only journal | natural evidence | rollback | preservation | evidence | rollback preservation projection | pending receipt | prefix preserved | plans/effect-law-unification-v1/evidence/S1C3G_CAPTURE_INSTALLATION_TERMINAL_V1/pending-receipt.json:1 | S1C-3G only |
| t3 | final verifier | rejects | capture installation claim | capture_installed false and S1C-4 closed | 1.0 | independent verifier | installation claim | rollback | preservation | conclusion | rollback preservation projection | final verification | no installation | plans/effect-law-unification-v1/evidence/S1C3G_CAPTURE_INSTALLATION_TERMINAL_V1/final-verification.json:1 | S1C-3G only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | deployment receipt | restores | frozen baseline binary and config | terminal report boundary | 1.0 | rollback receipt | production bytes | rollback | preservation | conclusion | rollback preservation projection | terminal report | baseline restored | candidate_answer:c1 | S1C-3G only |
| c2 | rollback journal | preserves | three zero-record NTF1 prefixes | terminal report boundary | 1.0 | append-only journal | natural evidence | rollback | preservation | conclusion | rollback preservation projection | terminal report | prefix preserved | candidate_answer:c2 | S1C-3G only |
| c3 | final verifier | rejects | capture installation claim | terminal report boundary | 1.0 | independent verifier | installation claim | rollback | preservation | conclusion | rollback preservation projection | terminal report | no installation | candidate_answer:c3 | S1C-3G only |
