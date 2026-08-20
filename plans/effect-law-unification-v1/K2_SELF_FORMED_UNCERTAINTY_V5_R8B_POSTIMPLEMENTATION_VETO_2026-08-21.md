# K2 Self-Formed Uncertainty V5 R8B Postimplementation VETO

Status: `VETO / IMPLEMENTATION COMMIT PRESERVED / NOT PUSHED / NO R8B AUTHORITY`

Date: `2026-08-21`

Frozen paper commit:
`4a01938bdda5`

Observed implementation commit:
`af18cad60054a70eb9bdeb8f815e174575ca664e`

## Verdict

The implementation commit does not satisfy the frozen V5 R8B contract. It is a
useful partial implementation and diagnostic artifact, but it cannot publish
`R8B_FROZEN` and cannot support R9B, R10B or R11B.

The defect is not a test flake. The required linked process route is absent from
the source. Creating a positive observed-source receipt for it would fabricate
evidence.

## What Passed

The following scoped results remain valid component evidence:

```text
implementation source scope          5 modified + 15 new paths
preserved confirm_terminal.rs         exact predecessor bytes
mode and legacy matrix                20 / 20 rejected
X18 terminal process control          real process, positive Development PASS
X18 foreign scopes                    Confirm and SealedAttempt rejected
strict Clippy                         PASS on delegated mini-PC build
```

These results prove compatibility and selected negative boundaries only. They
do not prove the aggregate R8B claim.

## Claim-To-Test Audit

| Frozen V5 claim | Observed source at `af18cad` | Verdict |
|---|---|---|
| One linked route from Development owner through aggregate publication | The linked test launches the owner, then the public coordinator, then replays the owner. It does not launch the private route, oracle, controls, terminal, cleanup authorizer, cleanup owner, cleanup verifier, R8B authorizer or publisher. | VETO |
| Exact 21-entry executable manifest | The list contains 21 hashes only by omitting the oracle, cleanup authorizer and cleanup owner. It inserts cleanup verifier in the predecessor set without running it. | VETO |
| Oracle and four frozen baselines | The linked test lists baseline-related executable hashes but launches no oracle process and produces no oracle batch from the Development route. | VETO |
| Runner never reads private truth | No complete Development oracle route exists. The R7J donor route used for design reads and decodes private truth in the test runner before launching the oracle. | UNPROVEN |
| Seven process restart states P01-P07 | The module-private suite contains known-answer vectors, 72 publication boundaries and link rejection. It contains no P01-P07 process restart tests. | VETO |
| Real classified cleanup transaction | The cleanup test performs an in-process census and manually removes one file. It never launches cleanup authorizer, cleanup owner or cleanup verifier and does not prove authorized deletion plus retained post-census. | VETO |
| Complete resource receipt for the linked route | No complete linked route was run in the delegated cgroup, so the route-wide memory, swap, OOM, timeout and network denominator does not exist. | VETO |
| Production survival receipt bound to this implementation | No complete postimplementation R8B run and no associated pre/post stable projection were produced. | VETO |
| Aggregate authorizer consumes actual linked receipts | The authority component test constructs synthetic conjunct roots. That is a useful validator test, but no positive request is assembled from actual linked-route receipts. | VETO |
| `DEVELOPMENT_REHEARSAL_COMPLETE` precedes `R8B_FROZEN` | Neither completion receipt nor an aggregate derived from it is produced by the linked test. | VETO |

## Manifest Count Error

The complete separated process identity set is 24, not 21:

```text
17 R7J process binaries
+ 1 Development owner
+ 3 cleanup processes
+ 1 linked runner
+ 1 R8B authorizer
+ 1 R8B publisher
= 24 identities
```

The 17 R7J identities include the oracle. The three cleanup identities are the
authorizer, mutation owner and independent verifier. None is interchangeable.

Manifest membership means one unique executable identity entry. It does not
mean one process invocation: per-case child cardinalities are recorded in a
separate process ledger.

## Source Evidence

At `af18cad`, the positive linked test contains process launches only for:

```text
nando-k2-self-formed-confirm-owner
nando-k2-self-formed-public-coordinator
nando-k2-self-formed-confirm-owner replay
```

The remaining binaries are collected into a vector of SHA-256 values. Listing a
hash is not execution evidence.

The restart test module contains three tests only:

```text
development_known_answer_vectors_are_byte_identical
immutable_publication_covers_72_boundaries
immutable_reader_rejects_symlink_and_foreign_hard_link
```

The cleanup test invokes only the Development owner process. Cleanup census and
damage injection are library calls in the test process.

## Consequence

```text
V5 postimplementation observed-source gate   VETO
V5 READY_TO_IMPLEMENT                        CONSUMED, NOT PROOF
af18cad                                      PRESERVED PARTIAL IMPLEMENTATION
push af18cad as R8B PASS                     FORBIDDEN
R8B_FROZEN                                   ABSENT
R9B / R10B / R11B                            LOCKED
production / dashboard / services            UNTOUCHED
```

The repair starts from the paper commit in a separate worktree. It must not
rewrite or force-reset the branch that preserves `af18cad`.

