# S1C Pre-Action Decision Owner Resource Protocol V2 Critique

Status: `ADVERSARIAL REVIEW PASS / STRUCTURAL 4 OF 4 PASS / MEASUREMENTS NOT STARTED`

Date: `2026-08-11 Europe/Tallinn`

Reviewed artifact:
`S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V2.md`

## 1. Scope

This review asks only whether V2 can distinguish an S1C-specific resource
regression from ordinary mini-PC scheduling interference without rewriting the
failed V1 result or manufacturing deployment authority.

It does not review S1C semantics again, reopen the K1 scientific claim, or
authorize production changes.

## 2. Adversarial Findings And Repairs

| Priority | Finding | Risk | Repair in final V2 |
|---|---|---|---|
| P0 | The first draft described ratios in prose but had no frozen executable verdict owner. | An executor could change rounding, omit a failed run, or reinterpret a nonzero test exit after seeing measurements. | Added `verify_s1c1_resource_v2.py`, exact binary/source constants, strict schemas, rational arithmetic, terminal exit codes, and seven pre-measurement mini-PC tests. |
| P0 | A median-only paired rule could hide one catastrophic candidate regression behind two clean pairs. | A severe candidate stall could be dismissed as load noise. | Added a hard `2.00` ceiling for every individual pair regression factor while retaining the `1.10` median ceiling. |
| P0 | Replacing the inherited absolute gate with a relative gate could silently weaken the product contract. | S1C-1 might be presented as production-latency PASS. | Preserved V1 as VETO, retained absolute classification for every inherited invocation, kept targeted S1C budgets absolute 3/3, and explicitly denied deployment authority. S1C-3 still requires a new absolute product gate. |
| P0 | Candidate source, binaries, and test names could drift independently. | Measurements could be attached to code other than the candidate later committed. | Froze base, tracked diff, each untracked source file, source-manifest root, all executable hashes, exact test names, and sample counts. Any mismatch is terminal `INVALID_ENVIRONMENT`. |
| P0 | A failing assertion could abort the shell before its metrics were preserved. | Absolute failures could disappear from the evidence set. | The runner must preserve every metrics line and exit code with failure-tolerant orchestration; exit-code/budget disagreement is rejected by the verifier. |
| P1 | Three pairs have an unavoidable first-run imbalance. | A fixed baseline-first order could systematically favor one side. | Froze alternating `BC / CB / BC`, one process at a time, fixed two-second gaps, and no idle-conditioned starts. The same order cannot be changed after seeing load. |
| P1 | A single noisy absolute sentinel could remain unexplained. | The result could be overclaimed as a runtime or product fact. | Record load, service survival, and both absolute-pass counts. `ABSOLUTE_ENVIRONMENT_FAIL / RELATIVE_NON_REGRESSION_PASS` is scoped to pure S1C-1 acceptance only. |
| P1 | Reusing earlier passing receipts could permit code changes after those receipts. | RSS, sync, idle CPU, or parity evidence could refer to stale source. | Bound all carried receipts to the frozen source-manifest root and prohibited any implementation edit before the final result. Identity is rechecked before commit. |
| P1 | A fourth run could be selected after an inconvenient result. | Optional stopping would invalidate the paired denominator. | Exactly three targeted runs and three pairs are terminal. Missing data, PID drift, malformed output, or service restart creates `INVALID_ENVIRONMENT`; V2 allows no replacement run. |
| P1 | Structural coherence could be mistaken for authority. | A NANDA PASS might be cited to deploy. | NANDA remains coherence-only with `authority_ready=false`; V2 and its verifier always emit `deployment_allowed=false`. |

## 3. Remaining Limitations

V2 does not estimate a population latency distribution from three pairs. It is
a bounded regression sentinel for one frozen candidate under ordinary load.
The 10% paired margin is an engineering non-regression budget, not a claim that
10% slower production latency is acceptable.

The baseline and candidate executables are not reproducible-build proofs. Their
SHA-256 identities are measurement identities, while source acceptance is bound
separately by the source manifest, strict tests, parity oracle, and remote/local
diff equality. Reproducible build authority remains outside S1C-1.

Normal server load is intentionally uncontrolled. Paired order and exact run
count reduce confounding but do not remove it. Therefore the inherited sentinel
cannot grant product authority even if all six invocations pass absolutely.

## 4. Rejected Alternatives

```text
wait for an idle server
  rejected: violates the user-approved ordinary-load condition

rerun until three passes appear
  rejected: optional stopping and denominator fraud

drop absolute numbers from the report
  rejected: hides product-relevant evidence

use only candidate measurements
  rejected: cannot attribute shared scheduling interference

use CPU time instead of wall time
  rejected: removes scheduling delay that real serving experiences

relax the original S1C hot-path budgets
  rejected: targeted budgets remain frozen and absolute

deploy shadow code to measure live
  rejected: deployment belongs to S1C-3, not resource-protocol repair
```

## 5. Review Verdict

The repaired V2 protocol is specific, bounded, fail-closed, and does not erase
V1. It is ready for split structural coherence checks. Measurements remain
forbidden until all required NANDA routes pass without `WATCH` or `VETO` and the
paper-only protocol commit is pushed.

```text
protocol identity route                 READY FOR GATE
measurement chronology route           READY FOR GATE
metric and terminal math route          READY FOR GATE
slice authority route                   READY FOR GATE
measurement started                     NO
implementation changed after freeze     NO
deployment authority                    false
```

## 6. Structural Result

The first packet set returned `VETO`: it represented source and candidate
claims as separate owners and therefore created false duplicate-authority
routes. The repair kept the four routes separate and bound source and candidate
evidence to the same contract roles.

```text
identity                                PASS
chronology                              PASS
metric and terminal math                PASS
slice authority                         PASS
WATCH                                   none
conflicts                               none
foreign pull                            none
owner conflicts                         none
negative hits                           none
repair queue                            empty
authority_ready                         false
```

This is coherence-only PASS. It authorizes the frozen measurement schedule and
nothing else.
