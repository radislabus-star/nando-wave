# S1C-3B Production-Load Absolute Gate Critique V1

Status: `ADVERSARIAL REVIEW / REPAIRS INCORPORATED / FROZEN`

Date: `2026-08-12 Europe/Tallinn`

## Findings

| Severity | Finding | Failure mode | Accepted repair |
|---|---|---|---|
| P0 | Subtracting baseline p99 from candidate p99 is mathematically invalid. | Quantiles are not additive; a noisy run could be normalized into a false PASS. | Retain unchanged absolute candidate bounds. Floor probes and ratios are diagnostic only. |
| P0 | Searching for a quiet interval selected the environment before observing the candidate. | Ordinary mini-PC load made the gate nonterminating without testing capture. | Remove quiescence and CPU selection completely; freeze CPU 4 and exactly three rounds. |
| P0 | Letting a baseline probe veto a passing candidate creates a second arbitrary performance gate. | Filesystem-floor drift could reject a candidate that satisfies every product bound. | Persist all floor samples but give them no PASS or VETO authority. |
| P0 | Allowing unrelated work while using relative thresholds could create a false PASS. | Candidate and baseline could experience different interference. | Use only absolute candidate thresholds; relative values never enter the verdict. |
| P0 | Compiling during measurement would mix proof construction with proof observation. | Transaction-owned rustc/cargo/cc could contaminate denominators and executable identity. | Build and hash every executable first; transaction-owned compiler after measurement start is instrument failure. |
| P0 | A failed first test could stop the sequence and hide the remaining fixed denominator. | The receipt would contain only the first bad metric and invite a selected rerun. | Parse and retain all three frozen rounds even when individual test assertions fail; final verdict is computed after the complete sequence. |
| P0 | A monitor summary could claim boundaries that were never observed. | Missing intervals or forged labels could pass contamination checks. | Persist raw samples and boundaries; verifier recomputes ordering, coverage, gaps, affinity and executable roots. |
| P0 | Current production receipt changed when the dashboard was deployed. | Reusing V7's older receipt as current production authority would fail or roll back unrelated control-plane state. | Bind current receipt `d02a3d7...`, while separately binding the unchanged transition binary and config roots. |
| P0 | A deployment PASS could be promoted into grounded meaning. | Operational capture would be reported as K2 evidence without a natural goal or alternative. | Keep S1C-4, decision episodes, K2, training and phase mutation closed; expose only capture availability. |
| P1 | Fixed CPU 4 can be busy. | One attempt can receive a false negative under ordinary load. | Accept the false-negative risk as the price of a bounded nonselective test; no retry or CPU shopping. |
| P1 | Unrelated compiler processes may run during a metric. | They can slow the candidate and make attribution less clean. | Record complete process evidence. Because verdicts are absolute, unrelated load cannot manufacture a PASS and does not censor one. |
| P1 | A filesystem microprobe does not reproduce the three-ledger Rust path. | It could be misrepresented as a semantic baseline. | Name it filesystem floor, freeze exact operations, and forbid parity, resource or scientific authority. |
| P1 | A fourth run after a near miss would be tempting. | `5.010709 ms` could become post-hoc threshold adaptation. | Exactly three runs; the frozen gate decides without user approval or rerun. |
| P1 | Starting S1C-4 immediately could call an empty journal a result. | Installed capture would be conflated with observed decision evidence. | Start S1C-4 only as `COLLECTING`; zero ordinary episodes remains an explicit valid state. |

## Rejected Alternatives

```text
retry V7 at night
  rejected: V7 is terminal and environment selection remains unbounded

raise p99 to 5.1 or 6 ms
  rejected: changes the product budget after observing a near miss

subtract baseline or filesystem-floor p99
  rejected: invalid quantile arithmetic and false-PASS risk

stop other agents or builds
  rejected: changes another owner's ordinary production environment

select the quieter of CPUs 4 and 6
  rejected: benchmark shopping after host observation

run until three passes are accumulated
  rejected: selected denominator and unlimited retry

skip resource evidence and deploy because code tests pass
  rejected: durability fsync sits on the request path

call capture installation K2
  rejected: no natural goal, alternative, outcome or decision episode follows
```

## Improvements Over V7

```text
30-second selected quiet window
  -> fixed ordinary-load environment with no wait

environment PASS before first metric
  -> absolute candidate metrics own PASS

one unstructured measurement stream
  -> three frozen rounds with paired diagnostic floor probes

contamination summary
  -> raw monitor rows, exact boundaries and verifier recomputation

near miss requires discussion
  -> frozen verdict with no approval request

deployment dashboard ambiguity
  -> capture availability separated from ordinary evidence and K2
```

## Verdict

```text
runtime candidate changed                         no
production config changed                         no
durability chronology changed                     no
5 ms p99 retained                                 yes
20 ms hard maximum retained                       yes
quantile subtraction                              forbidden
quiet-window search                               removed
measurement CPU                                   fixed before implementation
candidate rounds                                  exactly three
floor-probe authority                             diagnostic only
independent verifier                              required
remote transactions                               exactly one after freeze
scientific authority                              false
ready for structural gate                         yes
```
