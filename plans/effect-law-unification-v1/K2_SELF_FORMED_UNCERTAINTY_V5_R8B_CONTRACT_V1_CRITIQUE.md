# K2 Self-Formed Uncertainty V5 R8B Contract V1 Critique

Status: `V1 REJECTED / REPAIRS REQUIRED BEFORE IMPLEMENTATION`

Date: `2026-08-20`

## Verdict

V1 states the right stage boundary but is not executable evidence. It could turn
several green component tests into a false claim that the required end-to-end
DevelopmentRehearsal route ran. It also leaves resource and receipt ownership
ambiguous. R8B must not execute under V1.

## Findings

| Severity | Finding | Failure if left open | Required repair |
|---|---|---|---|
| P0 | V1 aggregates R7H-R7K component tests but does not require one request to traverse confirm-owner, generator pipe, split, coordinator, resolver, terminal and cleanup. | Disconnected component PASS could masquerade as full route readiness. | Add one process-level DevelopmentRehearsal runner with one immutable route ID and linked receipt chain. |
| P0 | `cargo test` success is accepted as the static-control evidence. | Test output could omit a control, change a denominator or bypass the control evaluator. | Export exact `32 + 4 + 16` outcomes, independently evaluate them and freeze the denominator/root. |
| P0 | V1 does not distinguish the R8B rehearsal K1-K12 receipt from the earlier R7K receipt or a future attempt-bound receipt. | Cross-stage receipt substitution becomes possible. | Bind mode, tested commit, executable manifest, development freeze root and absent attempt root in a fresh R8B receipt. |
| P0 | The tested commit and result commit are not separated. | Publishing the result changes the commit claimed to have been tested. | Freeze an implementation commit before execution; publish results only in a child commit that names the tested parent. |
| P1 | Resource measurement has no owner or observation boundary. | Measuring Cargo or only the parent could hide descendant RSS and confuse compile cost with route cost. | Build first, then run the test binary in a fresh cgroup and bind `MemoryPeak`, CPU time, wall time, swap, OOM and exit status. |
| P1 | The executable manifest is not defined. | Missing, duplicate or substituted route binaries could pass through test-harness helpers. | Freeze the exact 18 route executables, each path and SHA-256 exactly once, and exercise every self-hash check. |
| P1 | Receipt publication is described but not crash-atomic. | A partial aggregate could be mistaken for `R8B_FROZEN`. | Publish temp, fsync, rename and directory fsync; retain individual receipts and remove only unpublished aggregate temp on failure. |
| P1 | The full package suite and R8B scientific route are not separate denominators. | Unrelated package PASS could inflate route coverage. | Record package, static, rehearsal, fault/restart, route and resource denominators separately. |
| P1 | V1 does not require negative checks for nonce, slot, attempt and production effects after the run. | A readiness run could accidentally consume authority or mutate production while still passing tests. | Census these effects before and after and require exact zero deltas. |
| P2 | Structural gates are unnamed. | One broad worksheet could mix execution, proof, resources and claim ownership. | Require four owner-bounded routes and an explicit all-routes claim-boundary gate. |

## Required Replacement

V2 must define the process runner, immutable inputs, independent evaluators,
resource cgroup, atomic evidence packet, failure transitions and exact claim
boundary before code or execution begins.
