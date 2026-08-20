# Critique Of K2 Self-Formed Uncertainty V5 R7K Contract V4

Status: `CRITIQUE COMPLETE / REPAIRS INCORPORATED IN V4`

Date: `2026-08-20`

## Verdict

An environment-gated export can preserve R7J decision ownership, but only if
the packet is treated as untrusted transport and fully validated before decode.
Calling the export harmless without those checks would repeat the V3 error.

## Findings And Incorporated Repairs

| Severity | Finding | Incorporated repair |
|---|---|---|
| P0 | V3 required exact R7J test bytes, so the export hook cannot inherit V3 authority. | V4 names the drift, requires a new preflight revision and forbids an R7K result before it passes. |
| P0 | The current consumer does not validate the exported manifest before decode. | V4 requires exact set, count, type, mode and SHA-256 validation before any packet field is used. |
| P1 | A packet could be mistaken for K1-K12 process evidence. | V4 lists the only permitted predecessor fields and explicitly excludes all twelve process outcomes and every cleanup/result receipt. |
| P1 | Reusing an old export root could silently mix builds. | V4 requires a fresh nonexistent root and a same-command-chain producer/consumer run. |
| P1 | Export mode could change ordinary R7J behavior. | Both no-export and export regressions are mandatory; evaluator and terminal decision sources remain byte-identical. |
| P1 | A path manifest alone does not reject extra files. | The consumer checks the complete closed path set, including the manifest itself. |
| P2 | Persisted test evidence could be described as scientific evidence. | V4 keeps the Development-only claim boundary and forbids sealed, Natural K2 and production conclusions. |

## Remaining Non-Waivable Gate

The paper repair is not implementation evidence. R7K remains incomplete until
the manifest consumer, negative controls, full regressions, strict Clippy,
observed-source route gate and result audit all pass.
