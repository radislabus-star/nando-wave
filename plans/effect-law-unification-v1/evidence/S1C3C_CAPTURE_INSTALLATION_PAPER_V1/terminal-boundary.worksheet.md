# NANDA Triad Worksheet

task_id: s1c3c-terminal-boundary-v1
domain: code
query: Does S1C-3C preserve the consumed terminal authority of S1C-3B?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | S1C-3C paper owner | preserves | S1C-3B PREFLIGHT_FAILURE and consumed attempt | terminal report and status | 1.0 | prospective authority owner | immutable historical input | terminal | terminal-boundary | paper | S1C-3C paper owner | successor contract | closed old attempt | plans/effect-law-unification-v1/S1C3B_PRODUCTION_LOAD_TERMINAL_REPORT_2026-08-12.md:1 | both protocols |
| t2 | S1C-3C paper owner | creates | separate prospective attempt namespace | successor preregistration section 3 | 1.0 | prospective authority owner | new attempt contract | terminal | terminal-boundary | paper | S1C-3C paper owner | successor contract | one new namespace | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md:39 | S1C-3C only |
| t3 | S1C-3C paper owner | forbids | old S1C-3B launcher invocation | successor entry gate | 1.0 | prospective authority owner | forbidden operation | terminal | terminal-boundary | paper | S1C-3C paper owner | successor contract | no disguised retry | plans/effect-law-unification-v1/S1C3C_CAPTURE_INSTALLATION_PREREGISTRATION_V1.md:216 | both protocols |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C-3C paper owner | preserves | S1C-3B PREFLIGHT_FAILURE and consumed attempt | candidate claim c1 | 1.0 | prospective authority owner | immutable historical input | terminal | terminal-boundary | conclusion | S1C-3C paper owner | successor contract | closed old attempt | candidate_answer:c1 | both protocols |
| c2 | S1C-3C paper owner | creates | separate prospective attempt namespace | candidate claim c2 | 1.0 | prospective authority owner | new attempt contract | terminal | terminal-boundary | conclusion | S1C-3C paper owner | successor contract | one new namespace | candidate_answer:c2 | S1C-3C only |
| c3 | S1C-3C paper owner | forbids | old S1C-3B launcher invocation | candidate claim c3 | 1.0 | prospective authority owner | forbidden operation | terminal | terminal-boundary | conclusion | S1C-3C paper owner | successor contract | no disguised retry | candidate_answer:c3 | both protocols |
