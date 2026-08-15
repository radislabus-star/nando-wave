# K2 Generated Capability Dashboard V24 Critique

Status: `REVIEW COMPLETE / P0-P1 REPAIRS FROZEN`

Date: `2026-08-15`

## Findings

| Severity | Finding | Repair |
|---|---|---|
| P0 | Three green PASS labels beside live K2 can look like natural K2 proof. | Put them in a separately titled `Generated causal AI` section and repeat `Natural K2 NOT PROVED` plus `production authority FALSE`. |
| P0 | Hand-written dashboard constants can drift from evidence. | Compile the checked-in evidence bytes, validate exact claim and verdict fields, and fail closed to `UNVERIFIED`. |
| P0 | A display deployment could accidentally restart hot serving or Nginx. | Build and install only `nando-gateway-control`; pin hot, Nginx and local connector PIDs before and after. |
| P1 | `61 / 8,659` can be read as a speedup ratio. | Display `61 / 67 evaluations` and label `8,659` as the complete search denominator; do not claim latency or compute speedup. |
| P1 | The older hidden-effect result has Markdown evidence rather than a machine receipt. | Require exact PASS and authority-false strings plus its frozen SHA; do not infer any additional counters from it. |
| P1 | Static evidence can become stale after a later experiment. | Bind it to dashboard build V24 and display the frozen receipt root prefix. A later result requires a new build. |
| P1 | Mobile cells can overflow on long scientific labels. | Use fixed responsive tracks, `overflow-wrap:anywhere`, and verify 390 px plus desktop screenshots. |

No finding requires changing runtime, API, evidence, or the scientific verdict.
The remaining risk is interpretive: generated capability remains a lab result,
not a natural or production authority claim.
