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

## Route

```text
accepted joined T1 transitions
-> highest marginal complete source-neutral shape
-> bounded ResponseProgram candidate generation
-> complete candidate search
-> exact observation evaluation
-> semantic quotient
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
