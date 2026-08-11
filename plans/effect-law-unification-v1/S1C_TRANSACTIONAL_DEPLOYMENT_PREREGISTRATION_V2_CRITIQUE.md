# S1C Transactional Deployment Preregistration V2 Critique

Status: `ADVERSARIAL REVIEW / REPAIRS ACCEPTED / NO DEPLOYMENT`

Date: `2026-08-12 Europe/Tallinn`

Reviewed artifact: `S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md`

## 1. Central Verdict

V2 is justified only as an environmental-validity repair. It may not relax the
5 ms durability limit or reopen V1. The final contract preserves both rules.

## 2. Findings And Repairs

| Severity | Finding | Failure mode | Accepted repair |
|---|---|---|---|
| P0 | V1 invoked Cargo inside the measured stage. | Compiler, linker, and test execution shared the host and cache state, so the environmental denominator was not isolated. | Prebuild the production binary, both lib-test harnesses, and both parity oracles; execute only frozen binaries after quiescence. |
| P0 | `pgrep -f` would match command text rather than running executables. | A harmless waiter containing `cargo build` could falsely block eligibility, while renamed executables could evade a text search. | Match exact `/proc/<pid>/comm` or executable basename against a frozen set; retain races and all matches. |
| P0 | Waiting for a quiet moment after observing a failed metric would be optional stopping. | Repeated windows could select a passing p99. | Quiescence occurs before the first candidate metric, has a maximum 1,800-second wait, and V2 has one attempt with no post-metric retry. |
| P0 | A quiet instant is weaker than a stable environment. | A build could end just before the metric or resume immediately after. | Require 30 consecutive intervals, freeze the receipt first, then monitor continuously through every metric and parity executable. |
| P0 | A monitor could be advisory and omit a contamination interval. | A compiler could overlap a latency run without invalidating the result. | Require at least one sample per two seconds plus every executable boundary; any gap, monitor failure, or build match is terminal contamination. |
| P0 | The quiescence receipt could be written after seeing results. | Environmental evidence could be fabricated around a desired outcome. | Atomically freeze and fsync the rooted mode-0400 receipt before the first metric starts; resource receipt binds it. |
| P0 | Rebuilding after a threshold failure could alter the candidate executable. | A second binary could replace the frozen candidate under the same source name. | Bind all executable hashes before eligibility; any later hash change is VETO. |
| P1 | Global load average is a poor proxy for interference on pinned CPU 4. | Host work on other cores could reject a valid environment, or CPU 4 contention could be missed. | Gate CPU 4 counters directly; retain loadavg as evidence only. |
| P1 | I/O PSI during the durability test includes candidate writes. | A second hard PSI limit could reject the behavior being measured. | Gate strict I/O PSI only in the pre-metric window; record it during metrics while unchanged p99 and hard-max limits decide the outcome. |
| P1 | One-second sampling can miss a very short compiler process. | A brief linker overlap could escape periodic scans. | Scan at every process boundary in addition to periodic sampling. Residual sub-sample races remain a disclosed limitation. |
| P1 | Cargo JSON could expose more than one test executable. | Selecting by mtime or glob could run the wrong harness. | Select one compiler-artifact by package, target kind, profile, and executable field; zero or multiple matches is VETO. |
| P1 | Direct harness invocation could silently change test selection. | A filter mismatch could report zero tests as PASS. | Preserve exact names and flags and require exactly one executed test plus the expected metric record and denominator. |
| P1 | Quiescence could become a way to stop unrelated services. | Cleaner measurements would widen the production change. | No service, timer, connector, or workload may be stopped for eligibility; the gate only waits and observes. |
| P1 | A preflight PASS could be promoted to K2 evidence. | Operational capture would be mislabeled as natural meaning. | Keep S1C-4, K2, dashboard claims, learning, phase mutation, and scientific authority blocked. |

## 3. Rejected Alternatives

```text
raise the p99 limit to 5.1 ms
  rejected: changes the hypothesis after observation

round 5,010,709 ns to 5.0 ms
  rejected: destroys the exact absolute gate

repeat V1 until it passes
  rejected: optional stopping

kill builds or stop unrelated services
  rejected: changes the host workload to manufacture evidence

use command-line substring matching
  rejected: false positives from shell waiters and comments

compile one harness immediately before each run
  rejected: measured-stage contamination and cache drift

apply a hidden loadavg ceiling
  rejected: wrong denominator for pinned CPU 4

edit the dashboard during S1C-3
  rejected: deployment is not natural decision evidence
```

## 4. Residual Limits

Periodic `/proc` sampling cannot prove that no forbidden process existed for a
fraction of a second between samples. Boundary scans and the absence of Cargo
invocations in the executor reduce this risk but do not make it mathematically
zero.

The host remains a live production machine. Even an eligible window cannot
eliminate every interrupt, kernel flush, or unrelated application. The fixed
absolute p99 and hard-max limits remain the final operational tolerance.

Passing V2 proves the candidate survives the declared host conditions. It does
not establish universal latency on other hardware or under arbitrary load.

## 5. Review Verdict

```text
same candidate                              yes
same 5,000,000 ns p99 limit                yes
single-ledger p99 limit                    5,000,000 ns PASS 3/3
three-ledger p99 limit                     5,000,000 ns PASS 3/3
V1 reinterpreted or retried                 no
compiler removed from measured route        required
quiescence frozen before first metric       required
continuous contamination evidence           required
one V2 attempt                              required
production changed by paper review          no
scientific authority                        false
ready for structural paper gate             yes
```
