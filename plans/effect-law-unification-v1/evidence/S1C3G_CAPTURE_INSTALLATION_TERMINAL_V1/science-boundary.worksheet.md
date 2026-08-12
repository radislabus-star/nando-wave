# NANDA Triad Worksheet

task_id: s1c3g-science-boundary-v1
domain: code
query: Does the S1C-3G terminal packet keep operational rollback separate from scientific authority?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | rollback pass | does not prove | natural decision episode | capture was not installed | 1.0 | operational result | scientific claim | science | claim-boundary | proof | scientific claim projection | terminal state | authority false | plans/effect-law-unification-v1/evidence/S1C3G_CAPTURE_INSTALLATION_TERMINAL_V1/s1c3g-state.json:1 | S1C-3G only |
| t2 | startup diagnostic | does not prove | complete route mismatch cause | candidate projection was not persisted | 1.0 | diagnostic observation | causal claim | science | claim-boundary | evidence | scientific claim projection | startup log | proximate only | plans/effect-law-unification-v1/S1C3G_CAPTURE_INSTALLATION_TERMINAL_REPORT_2026-08-12.md:28 | S1C-3G only |
| t3 | terminal status | forbids | training phase mutation and S1C-4 | all authority flags false and state closed | 1.0 | terminal status | downstream authority | science | claim-boundary | conclusion | scientific claim projection | rooted sidecar | no promotion | plans/effect-law-unification-v1/S1C3G_CAPTURE_INSTALLATION_TERMINAL_STATUS.json:1 | S1C-3G only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | rollback pass | does not prove | natural decision episode | terminal report boundary | 1.0 | operational result | scientific claim | science | claim-boundary | conclusion | scientific claim projection | terminal report | authority false | candidate_answer:c1 | S1C-3G only |
| c2 | startup diagnostic | does not prove | complete route mismatch cause | terminal report boundary | 1.0 | diagnostic observation | causal claim | science | claim-boundary | conclusion | scientific claim projection | terminal report | proximate only | candidate_answer:c2 | S1C-3G only |
| c3 | terminal status | forbids | training phase mutation and S1C-4 | terminal report boundary | 1.0 | terminal status | downstream authority | science | claim-boundary | conclusion | scientific claim projection | terminal report | no promotion | candidate_answer:c3 | S1C-3G only |
