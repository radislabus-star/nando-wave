# S1C Transactional Deployment Preregistration V5 Critique

Status: `ADVERSARIAL REVIEW / ACCEPTED WITH REPAIRS / PAPER VERIFIED`

Date: `2026-08-12 Europe/Tallinn`

## Findings

| Severity | Finding | Failure mode | Accepted repair |
|---|---|---|---|
| P0 | Choosing a CPU after timing the candidate would be adaptive benchmark shopping. | V5 could select the fastest result and understate latency. | Freeze the CPU before every candidate metric using environment-only counters. |
| P0 | Observing CPUs sequentially would give each CPU a different workload interval. | The selector could compare incomparable time windows. | Read all pool counters from the same start and end snapshots. |
| P0 | Using sibling CPUs 4 and 5 as alternatives would not create independent contention surfaces. | Both share one physical core and interfere with each other. | Freeze one logical thread from each equal-frequency physical core: CPUs 4 and 6. |
| P0 | Watching only the representative thread would miss contention on its SMT sibling. | CPU 4 could look idle while CPU 5 consumes the same physical core. | Require both siblings `[4,5]` or `[6,7]` to pass the unchanged per-logical-CPU gate. |
| P0 | A passing CPU could be selected while a build runs elsewhere. | Compiler IO and cache pressure could contaminate the measurement despite a quiet selected CPU. | Keep the global forbidden-process and IO vetoes unchanged for every CPU. |
| P0 | V4 timeout retained only an error string. | The environment blocker could not be independently verified or improved. | Persist all attempted samples and recomputable blocker census on PASS and TIMEOUT. |
| P0 | A timeout receipt could accidentally flow into the normal preparation path. | Production might mutate without a valid quiescence window. | Independent verifier accepts TIMEOUT only as terminal non-authority; prepare exits before resource tests. |
| P1 | Choosing the CPU with the smallest mean among passers would still optimize the benchmark environment. | Stable but adaptive selection could bias latency. | Select the lowest CPU number among simultaneous full-window passers. |
| P1 | RSS on CPU 5 did not match V4's CPU 4 quiescence denominator. | One resource metric used an unproven execution CPU. | Bind hot, durability, idle, and RSS measurements to the single selected CPU. |
| P1 | Expanding the pool until something passes could erase the attempt boundary. | Every failure would invite a larger search. | Freeze pool `[4,6]`, order, topology contract, and exactly one V5 transaction in paper. |
| P1 | Changing the CPU gate could be presented as a runtime improvement. | Proof-plane repair could be confused with product performance. | Reuse exact V4 candidate and config; retain all resource thresholds and claim boundaries. |

## Rejected Alternatives

```text
retry V4 during a quiet hour
  rejected: V4 is terminal and timeout evidence was incomplete

stop learner or unrelated builds
  rejected: changes the ordinary production environment and another owner's work

raise CPU mean above 5%
  rejected: weakens the frozen measurement quality

ignore global build processes when selected CPU is idle
  rejected: cache and IO contamination remain host-wide

pick the lowest observed latency CPU
  rejected: adaptive benchmark selection

use all 20 CPUs and take the first pass
  rejected: unnecessary search surface and heterogeneous core classes

reuse V4 build artifacts
  rejected: spent attempt state
```

## Improvements Applied

```text
fixed CPU 4 only
  -> preregistered equal-class physical-core pool 4:[4,5], 6:[6,7]

timeout error string only
  -> atomic rooted timeout receipt with complete sample series

different quiescence and RSS CPUs
  -> one selected CPU for every resource metric

implicit selector behavior
  -> simultaneous snapshots plus lowest-index tie-break

diagnostic observations outside receipt
  -> recomputable per-condition blocker census
```

## Verdict

```text
runtime candidate changed                       no
resource thresholds changed                     no
production affinity changed                     no
CPU pool frozen before implementation           yes
SMT sibling contamination covered               yes
selection precedes every candidate metric        yes
selection tie-break non-adaptive                 yes
timeout independently verifiable                required
fresh V5 artifacts                              required
V5 remote attempts                              one
scientific authority                            false
ready for structural gate                       yes
```
