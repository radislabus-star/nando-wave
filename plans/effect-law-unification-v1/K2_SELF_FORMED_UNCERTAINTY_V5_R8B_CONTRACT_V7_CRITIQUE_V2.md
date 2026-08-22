# K2 Self-Formed Uncertainty V5 R8B Contract V7 Critique V2

Status: `CRITIQUE COMPLETE / REPAIRED V7 STILL VETO / NARROW REPAIR`

Date: `2026-08-21`

## Verdict

The produced-receipt descriptor set closes the scalar-root defect. Four P1
implementation ambiguities remain. They are repairable without changing the
claim or adding a new authority owner.

## Findings

| Priority | Finding | Consequence | Required repair |
|---|---|---|---|
| P1 | S02-S05 themselves spawn Nando processes, but the hierarchical writer map names only M24, M01 and M10. | Restart, cleanup and authority suites could summarize unjournaled subprocesses. | Require each suite aggregate to journal its direct Nando children through the shared test support; M01/M10 still journal their own nested descendants. |
| P1 | P07 restart uses `/usr/bin/strace`, but the five-entry suite producer manifest does not bind this external tracer. | The real ptrace claim depends on an unmeasured executable. | Add a non-producer tool-dependency manifest inside S02 receipt binding canonical path, mode, length and SHA-256 of strace. Do not add it to linked or suite producer counts. |
| P1 | A produced descriptor has a relative path, but the draft does not say whether it names the producer output or final packet location. | M25 could match bytes while silently moving them under a different evidence-kind path. | Define it as the canonical final packet relative path; producer output mirrors that path and P06 copies bytes without changing path or content. |
| P1 | The V6 line budgets do not include the shared ledger appender, receipt vector or full parent/child route. | Artificially preserving old limits would force mixed ownership or compressed code that is difficult to audit. | Freeze revised per-owner budgets justified by the added responsibilities; keep total path scope at 23. |

## Additional Required Clarification

`FrozenControlScopes` must change schema ownership. In V6 it was decoded as one
ordinary M17 control receipt even though no M17 request emits a four-scope
receipt. In V7 it is a derived `K2UncertaintyR8BMeasuredReceiptV2` produced by
M24 child from four distinct M17 roots. The model and M25 dispatch must state
this explicitly.

## Next Legal Action

Apply the four repairs and schema clarification. Then run structural and
implementation gates against exact source baselines. A third prose critique is
not required unless those gates expose another semantic conflict.
