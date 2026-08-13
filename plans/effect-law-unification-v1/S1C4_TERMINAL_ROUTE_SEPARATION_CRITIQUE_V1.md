# S1C-4 Terminal Route Separation Critique V1

Status: `ADVERSARIAL REVIEW APPLIED BEFORE IMPLEMENTATION`

| Priority | Finding | Failure if ignored | Frozen repair |
|---|---|---|---|
| P0 | The current blocker belongs to disabled legacy MS3. | A retired route is presented as the active scientific blocker. | Gate legacy blocker evaluation on `multi_source_research_enabled`; expose `LEGACY_MS3_RESEARCH_DISABLED` otherwise. |
| P0 | Remote transport recovery could be promoted into a terminal topology join. | Accepted frames become false Law #2 or K2 evidence. | Preserve route-bound frame counts as transport facts only; all authority flags remain false. |
| P0 | S1C-4 `EMPTY_GOAL_SURFACE` could be described as another collection wait. | The user waits for an immutable terminal window to change. | Render it as a completed negative result with exact `1,024 / 1,024` and `0` goals. |
| P0 | K1 Law #2 and K2 meaning can be merged because both are called learning. | Repairing one route appears to unblock the other. | Render K1 discovery, S1C-4 evidence surface, and K2 as separate rows with separate owners. |
| P1 | Removing old health fields can break existing observers. | A truthful repair causes compatibility regressions. | Preserve old fields and add explicit legacy route metadata; change only the disabled-route blocker value. |
| P1 | UI prose could prescribe retrospective goal extraction from the same traffic. | Post-hoc goals contaminate independent evidence. | State that the next K2 route requires a separately preregistered goal-bearing environment. |
| P1 | A dashboard-only repair can hide a wrong backend projection. | UI looks correct while API remains misleading. | Repair and test cold `/health` first, then project the already validated S1C/K1 sources in control. |
| P1 | Deployment can disturb natural evidence ledgers. | A status repair damages the evidence it describes. | Treat every evidence artifact as immutable or append-only; restart only scoped cold/control services with rollback. |
| P2 | Dynamic traffic counters may change during verification. | Whole-object equality produces false rollback. | Compare stable route flags, report roots, authority flags, safety counters, and PIDs; observe dynamic counters without equality. |

## Accepted Interpretation

```text
transport route                    recovered and live
legacy MS3 route                   intentionally off
K1 scheduler                       active, waiting for new readiness-PASS evidence
S1C-4                              complete negative evidence-surface result
K2                                 closed, not disproved
next K2 engineering work           separate goal-bearing environment contract
```

The critique authorizes only the scoped health and dashboard repair. It does
not authorize the next environment, another S1C-4 window, synthetic traffic,
Law #2 promotion, K2 training, or phase mutation.
