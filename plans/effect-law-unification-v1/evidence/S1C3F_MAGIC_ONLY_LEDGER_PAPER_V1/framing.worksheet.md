# NANDA Triad Worksheet

task_id: s1c3f-framing-v1
domain: code
query: Does S1C-3F distinguish format bytes from scientific records?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | NTF1 magic | identifies | framed-ledger segment format | frozen empty definition | 1.0 | format marker | segment format | framing | format-header | storage | Rust format | byte 0 | valid segment | plans/effect-law-unification-v1/S1C3F_MAGIC_ONLY_LEDGER_PREREGISTRATION_V1.md:26 | segment |
| t2 | independent parser | counts | frames after byte 4 | record-aware parser | 1.0 | verifier | record denominator | framing | record-parser | proof | S1C-3F verifier | bounded bytes | record count | plans/effect-law-unification-v1/S1C3F_MAGIC_ONLY_LEDGER_PREREGISTRATION_V1.md:52 | segment |
| t3 | opening authority | requires | decoded record count zero | pre-cursor boundary | 1.0 | cursor owner | evidence denominator | framing | cursor-zero | proof | S1C-3F verifier | parser output | empty cursor | plans/effect-law-unification-v1/S1C3F_MAGIC_ONLY_LEDGER_PREREGISTRATION_V1.md:66 | opening |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | NTF1 magic | identifies | framed-ledger segment format | candidate c1 | 1.0 | format marker | segment format | framing | format-header | conclusion | Rust format | byte 0 | valid segment | candidate_answer:c1 | segment |
| c2 | independent parser | counts | frames after byte 4 | candidate c2 | 1.0 | verifier | record denominator | framing | record-parser | conclusion | S1C-3F verifier | bounded bytes | record count | candidate_answer:c2 | segment |
| c3 | opening authority | requires | decoded record count zero | candidate c3 | 1.0 | cursor owner | evidence denominator | framing | cursor-zero | conclusion | S1C-3F verifier | parser output | empty cursor | candidate_answer:c3 | opening |
