# R8B V8 Format Normalization

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | pinned rustfmt | normalizes | exact eight measured Rust files | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V8_FORMAT_NORMALIZATION_AMENDMENT_V1.md:19-20,37-49 | 1.0 | mechanical formatter | bounded source set | mutation | exact-scope |
| t2 | post-formatter source lines | define | V8 engineering budget census | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V8_FORMAT_NORMALIZATION_AMENDMENT_V1.md:56-64 | 1.0 | budget measurement | source line census | proof | formatter-budget |
| t3 | global cargo fmt check | rejects | every remaining formatting deviation | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V8_FORMAT_NORMALIZATION_AMENDMENT_V1.md:61-71 | 1.0 | format verifier | workspace source | proof | no-exclusion |
| t4 | format-only operation | preserves | process and authority semantics | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V8_FORMAT_NORMALIZATION_AMENDMENT_V1.md:88-101 | 1.0 | mechanical mutation | frozen behavior | mutation | format-vetoes |
| t5 | post-format gates | verify | route, byte, tests, Clippy and budgets | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V8_FORMAT_NORMALIZATION_AMENDMENT_V1.md:103-106 | 1.0 | independent checks | normalized checkpoint | proof | post-format-proof |
| t6 | format amendment | grants no | R8B execution or scientific authority | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V8_FORMAT_NORMALIZATION_AMENDMENT_V1.md:108-113 | 1.0 | paper-only amendment | excluded authority | authority | claim-boundary |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | pinned rustfmt | normalizes | exact eight measured Rust files | candidate:c1 | 1.0 | mechanical formatter | bounded source set | mutation | exact-scope |
| c2 | post-formatter source lines | define | V8 engineering budget census | candidate:c2 | 1.0 | budget measurement | source line census | proof | formatter-budget |
| c3 | global cargo fmt check | rejects | every remaining formatting deviation | candidate:c3 | 1.0 | format verifier | workspace source | proof | no-exclusion |
| c4 | format-only operation | preserves | process and authority semantics | candidate:c4 | 1.0 | mechanical mutation | frozen behavior | mutation | format-vetoes |
| c5 | post-format gates | verify | route, byte, tests, Clippy and budgets | candidate:c5 | 1.0 | independent checks | normalized checkpoint | proof | post-format-proof |
| c6 | format amendment | grants no | R8B execution or scientific authority | candidate:c6 | 1.0 | paper-only amendment | excluded authority | authority | claim-boundary |

## notes

- Structural coherence is not implementation or execution authority.
- Any new formatter exclusion or ninth touched file is a VETO.
- Existing frozen skip directives prevent a skip-free complexity claim.
- The post-format gates, not this worksheet, establish implementation parity.
