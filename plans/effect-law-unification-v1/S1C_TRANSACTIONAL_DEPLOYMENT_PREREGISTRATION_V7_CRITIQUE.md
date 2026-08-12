# S1C Transactional Deployment Preregistration V7 Critique

Status: `ADVERSARIAL REVIEW / REPAIRS APPLIED / NOT YET FROZEN`

Date: `2026-08-12 Europe/Tallinn`

## Findings

| Severity | Finding | Failure mode | Accepted repair |
|---|---|---|---|
| P0 | V6 treated every missing `exe` as an unresolved race. | Stable kernel threads made every interval globally ineligible; V6 could never pass. | Separate observable, proven non-executing, and unresolved statuses with durable classified rows. |
| P0 | Ignoring `ENOENT` would be equally unsafe. | A vanished/reused PID or user process with a terminated main thread could conceal a build process. | Require stable `starttime`; accept only vanished PID, stable zombie, or stable `Kthread: 1` plus empty cmdline. |
| P0 | PID can be reused between `/proc` reads. | Comm, executable, state, and starttime could come from different processes. | Bind opening and closing stat rows by PID plus starttime; mismatch is unresolved. |
| P0 | A summary counter can be forged independently of observations. | Verifier could accept a claimed zero unresolved count without classifying source rows. | Persist all classification rows and make verifier recompute endpoint summaries and interval blockers. |
| P0 | Checking only comm or only executable is incomplete. | A wrapper or renamed executable could hide a forbidden compiler process. | Compare both comm and resolved executable basename; either match blocks. |
| P1 | Synthetic `/proc` fixtures could be confused with host quietness. | Unit PASS could become deployment authority. | Fixtures prove parser/classifier behavior only; one live V7 receipt owns quiescence. |
| P1 | Background build load was also present in V6. | Fixing the detector does not guarantee a valid window. | Preserve all CPU/IO/build gates and the 1,800-second deadline; do not stop workloads or change affinity. |
| P1 | A V7 deployment could be called grounded meaning. | Operational capture would be promoted into S1C-4 or K2 evidence. | Keep natural episode, grounded meaning, S1C-4, K2, training, phase mutation, and dashboard claims closed until ordinary evidence. |

## Rejected Alternatives

```text
retry V6
  rejected: V6 is terminal and its detector makes PASS impossible

ignore all ENOENT rows
  rejected: can hide PID exit/reuse and non-kernel no-exe processes

inspect only /proc/<pid>/comm
  rejected: executable identity remains unchecked

stop other mini-PC builds
  rejected: changes the production environment under test

change CPU, IO, latency, or deadline bounds
  rejected: unrelated denominator adaptation

reuse V6-built executables
  rejected: violates fresh-attempt evidence symmetry
```

## Verdict

```text
runtime candidate changed                       no
production config changed                       no
offline oracle contract changed                 no
resource thresholds changed                     no
production affinity changed                     no
process observation made total and typed        yes
PID reuse fail-closed                            yes
kernel-thread proof conjunctive                  yes
classified rows verifier-owned                  required
V7 remote attempts                              one after final freeze
scientific authority                            false
ready for structural gate                       yes
```
