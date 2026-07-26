# STOP-MS3 T1 Adaptive Identification Implementation

Date: 2026-07-24 Europe/Tallinn.

## Scoped Verdict

```text
adaptive identification implementation       PASS
controlled 1 support + 1 future              PASS
same-lineage reuse rejection                 PASS
restart parity after future                  PASS
natural live T1 evidence            NOT_EVALUATED
runtime actor/verifier parity       NOT_EVALUATED
candidate execution authority                 false
STOP-MS3 scientific claim                    WATCH
MS4                                           BLOCKED
```

## Canonical MS3 State Machine

The selected observation is a proof result, never a generator input. The
canonical route is:

```text
MS3.0  FRAME_FROZEN
       provenance + lineage + event time + intent + all observations

MS3.1  EVIDENCE_CLASSIFIED
       missing | ambiguous | equivalent | conflicting | censored

MS3.2  VERSION_SPACE_BUILT
       role binding x temporal rule x scope x transform x renderer

MS3.3  SEMANTIC_QUOTIENT
       0 classes  -> representation gap or permanent ABSTAIN
       1 class    -> candidate freeze
       >1 classes -> distinguishing probe

MS3.4  PROBE_PENDING
       authority=false

MS3.5  UNIQUE_LAW_FROZEN
       source-neutral law + immutable support boundary

MS3.6  INDEPENDENT_FUTURE_PASS
       new lineage + exact actor/verifier receipts

MS3.7  BUNDLE_V4_SEALED
       CanonicalOperatorIR -> immutable crystal

MS3.8  EXTERNAL_ADMISSION
       applicability proof + lease + bounded rollout + revoke

MS3.9  ORDINARY_CPU_ACCEPT
       an ordinary request is served locally and upstream is prevented
```

## Route

```text
all admissible observations
-> source-neutral role hypotheses
-> bounded program hypotheses
-> exact replay
-> semantic quotient
-> highest marginal complete source-neutral shape
-> complete candidate search
-> exact observation evaluation
-> earliest unique-class CandidateFreezeReceiptV1
-> immutable support watermark
-> independent post-freeze SessionLineageId
-> exact transfer evidence
-> T1TransferReady | ABSTAIN
```

There is no `32 support + 32 future` rule. A simple T1 operator may reach
`T1TransferReady` with one support and one independent future only when the
bounded candidate generator is complete and exactly one executable semantic
class survives.

Repeated evidence from a support lineage is counted as `support_reuse_rows`.
It cannot become future evidence. Repeated identical observations can increase
neither information gain nor authority.

## Distinguishing Evidence

Ambiguity creates multiple passive probe candidates from the concrete
disagreement between surviving classes:

```text
role binding
temporal rule
renderer
routing atoms
```

The generic inquiry selector chooses the highest guaranteed semantic-class
split per cost unit. Wave may rank admissible probes, but it cannot create a
probe, reveal its result, or grant authority. A passive probe is a prediction
contract for a later ordinary frame, not an action against a user.

Two physical bindings that predict the same action remain one semantic class.
Missing or censored evidence changes neither the positive field nor an
anti-center.

## Applicability Boundary

`TransferReady` proves a unique executable law over its frozen basis. It does
not prove that every request containing a compatible scalar intends that law.
Request-derived projection therefore remains `HELD` until independent
applicability evidence creates a sealed semantic boundary:

```text
identified law
+ positive intent/topology evidence
+ real applicability negatives
-> learned anti-center / applicability proof
-> external admission
```

No fixed row count, manual anti-center, function name, field name, package id,
exact value hash, episode ordinal, or family-to-program lookup may satisfy this
boundary.

## Restart Repair

The generic identification machine previously committed the whole evidence
ledger as the candidate support root. Appending future changed that root and
made a valid frozen candidate fail restart.

The contract now has two roots:

```text
support_evidence_root_sha256   immutable at freeze
evidence_root_sha256           grows with future records
```

Checkpoint restore verifies the immutable support root against the freeze and
separately reconstructs the complete support/future ledger.

## Live API

`LiveMultiSourceDiscoverySnapshotV2` publishes:

```text
t1_identification.state
candidate_programs
semantic_classes_remaining
support_rows / support_lineages
zero_gain_observations
support_reuse_rows
independent_future_rows / lineages
wrong_role_bindings
negative_accepts
candidate_freeze
canonical_program
passive_probe
exact_transfer_parity
runtime_actor_verifier_parity
execution_authority = false
```

The snapshot distinguishes:

```text
NO_ELIGIBLE_T1_COHORT
T1_CANDIDATE_GENERATION_BLOCKED
T1_AMBIGUOUS
T1_AWAITING_INDEPENDENT_FUTURE
T1_FUTURE_CONTRADICTION
T1_TRANSFER_READY
```

## Remaining Natural Gate

The controlled test proves plumbing and threshold removal. It does not create
a natural operator claim. The scientific STOP remains WATCH until ordinary
traffic supplies:

```text
natural joined support
-> candidate freeze
-> new independent post-freeze lineage
-> actor execution
-> independent verifier parity
```

Only then may MS4 build and externally admit a natural package.

## First-Law Gate

```text
complete bounded version space          PASS
unique action-equivalent class          PASS
intent/applicability proved             PASS
support/future lineage overlap             0
wrong role bindings                        0
negative accepts                           0
verifier coverage                       100%
runtime parity failures                     0
exact episode authority removed         PASS
ordinary CPU receipt                    >= 1
```

Opportunity tokens are an economic denominator, not proof. MS3 scientific
evidence is counted only as independent frames, lineages, distinguishing
outcomes, verifier receipts, and the final ordinary CPU receipt.
