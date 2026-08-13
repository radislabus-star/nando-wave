# K1 Terminal Failure Quotient V1 Preregistration

Status: REJECTED by `K1_TERMINAL_FAILURE_QUOTIENT_CRITIQUE_V2.md`. Historical
contract only. It MUST NOT authorize implementation or deployment.

Date: 2026-08-13.

## Measured Blocker

The bounded K1 authority wire is operational. Generation 556 crossed

```text
freeze -> bounded wire -> authority -> identifier
```

and generations 556 through 586 continued autonomously. The current production
catalog contains 6,097 natural motif cohorts and the retained queue contains
256 readiness-PASS rows. Waiting for more traffic is therefore not the active
blocker.

The identifier repeatedly terminates different frozen candidates with
`motif_program_candidates_empty`. The exact candidate root changes with the
evidence manifest, so the existing completed-candidate exclusion does not
prevent a structurally equivalent terminal family from dominating the next
queue.

The measured terminal suffix contains these repeated operator-blind families:

```text
capture generation 2cee... / scalar     15 empty identifier terminals
capture generation 1255... / scalar      6 empty identifier terminals
capture generation b914... / collection  7 empty identifier terminals
```

This proves repeated terminal selection. It does not prove that every member of
any consequence type is unidentifiable.

## Decision

Add a deterministic `K1TerminalFailureQuotientV1` derived only from the signed,
append-only scheduler ledger and current preregistered candidate fields.

```text
signed freeze + terminal pairs
-> exact terminal diagnostic observations
-> operator-blind family quotient
-> bounded repeated-failure threshold
-> queue demotion, never evidence deletion
-> authority recomputation and exact queue parity
```

The quotient is a scheduler policy receipt. It cannot issue a certificate,
alter an immutable freeze, create a program, mutate phase, or activate a
package.

## Diagnostic Observation

For every completed V6 generation the diagnostic projection binds:

```text
generation sequence
candidate freeze root
candidate root
candidate structural root
capture generation root
consequence type
semantic novelty signature root
generator schema
discovery basis root
bounded discovery cost units
terminal verdict root
terminal class
terminal blocker
identifier candidate count
```

`identifier_candidate_count = 0` is asserted only for the exact blocker
`motif_program_candidates_empty`. Other blockers are not silently interpreted.

## Operator-Blind Family Key

The family key contains only fields available before identifier synthesis:

```text
schema
current Epistemic Registry root
fixture exclusion root
capture generation root
consequence type
semantic novelty signature root
generator schema
discovery basis root
bounded discovery cost units
terminal blocker class
```

Forbidden family-key inputs:

```text
FILTER / COUNT / BRANCH / renderer names
generated programs or program roots
teacher output
post-freeze identifier choices
active package family mapping
manual semantic labels
```

The structural motif root remains in each observation as audit evidence, but is
not the family key: live evidence shows that exact motif roots almost never
repeat even when the same larger terminal surface repeats.

## Threshold And Queue Semantics

A family becomes exhausted only after all of the following hold:

```text
terminal blocker                  motif_program_candidates_empty
terminal class                    ACQUISITION_FAIL
distinct completed generations   >= 4
distinct structural motif roots  >= 4
identifier candidate count       exactly 0 for every observation
```

Exhausted families are demoted after K1 gain and readiness, but before bounded
cost, token opportunity, and stable hash:

```text
safety and provenance veto
-> K1 gain
-> readiness
-> unexhausted family first
-> bounded discovery cost
-> expected verified tokens
-> stable hash
```

Demotion is not exclusion. If no unexhausted readiness-PASS family remains, an
exhausted family is still reachable. No evidence row or terminal receipt is
removed.

## Reopening Contract

The family key changes, and therefore the family reopens, when any of these
preregistered worlds changes:

```text
Epistemic Registry root
fixture exclusion root
capture generation root
generator schema
discovery basis root
consequence type or semantic signature
bounded discovery cost class
```

Ordinary growth of a candidate evidence manifest does not erase terminal
history. A new exact motif in the same frozen world remains part of that family,
but demotion cannot make it unreachable.

## Rooted Receipt

`K1TerminalFailureQuotientV1` commits:

```text
scheduler ledger revision and root
current registry and fixture roots
current discovery basis
threshold
all qualifying diagnostic observations
all family summaries
demoted family roots
demoted current candidate roots
authority_ready = false
phase_mutation_allowed = false
```

The queue V3 root commits the quotient root and a one-bit family novelty rank on
every retained row. The certification authority independently restores the
ledger, rebuilds the quotient, rebuilds the queue, and reseals the selected
freeze. Client-provided quotient bytes have no authority.

## Invariants

1. Existing scheduler journal bytes, terminal receipts, anchors, and freezes
   remain byte-identical.
2. The quotient is a pure projection of signed history plus current catalog and
   deficit roots.
3. A single failure never demotes a family.
4. Four failures on one exact motif do not satisfy the distinct-motif rule.
5. Repairable or unrelated blockers never enter this quotient.
6. Demotion never removes the last readiness-PASS route.
7. Learner and authority derive byte-identical queue and quotient roots.
8. `authority_ready` and `phase_mutation_allowed` remain false.
9. No generated traffic, synthetic fixture, retrospective label, program hint,
   or manual family mapping is introduced.
10. Law #2 remains unproved until the existing independent future, BundleV4,
    CPU, economics, cleanup, and LawCertificate route passes.

## Verification

The implementation is accepted only when:

```text
unit: threshold below 4 does not demote
unit: 4 distinct empty motifs demote the current family
unit: 4 repeats of one exact motif do not demote
unit: registry/capture/basis changes reopen
unit: non-empty and unrelated blockers do not demote
unit: exhausted family remains in a queue when it is the only ready route
parity: learner queue == authority queue
restart: signed journal replay yields the same quotient root
production-copy shadow:
  old scalar families stop owning the leading ready rows
  collection/boolean/record surfaces become reachable
  production source bytes remain unchanged
live:
  cold/authority/control only restart transactionally
  hot serving, Nginx, and connector PIDs remain unchanged
  false accepts = 0
  parity failures = 0
```

## Claim Boundary

A PASS proves only that repeated terminal families no longer monopolize K1
candidate selection. It does not prove a unique semantic class, Law #2, K1
OPEN, Natural L2, answer quality, or Wave causality.
