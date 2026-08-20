# K2 Self-Formed Uncertainty V5 R8B Contract V3

Status: `DRAFT AFTER V2 IMPLEMENTATION DISCREPANCY / REQUIRES ADVERSARIAL CRITIQUE`

Date: `2026-08-20`

Supersedes: `K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V2.md`

Discrepancy:
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_IMPLEMENTATION_DISCREPANCY_2026-08-20.md`

## 1. Exact Claim

R8B may prove only non-sealed process readiness:

```text
one exact implementation commit
-> one owner-linked DevelopmentRehearsal route
-> exact independent suite/control denominators
-> bounded resource, restart and cleanup evidence
-> R8B_FROZEN
```

It cannot prove the self-formed-uncertainty hypothesis, Natural K2,
natural-traffic transfer, Wave-causal grokking, product value, CPU savings or
deployment readiness.

## 2. Immutable Wire And Authority Boundary

The implementation must preserve byte-for-byte:

```text
K2UncertaintyGeneratorRequestV1 Development wire schema
K2UncertaintyGeneratorResponseV1 Development wire schema
K2UncertaintyConfirmGeneratorRequestV1 Confirm wire schema
K2UncertaintyConfirmGeneratorResponseV1 Confirm wire schema
existing Confirm split artifacts and receipt bytes
existing Confirm owner receipt bytes
```

DevelopmentRehearsal creates:

```text
generator dispatches          1
request artifact writes       0
nonce commitments             0
authorization slot claims     0
sealed attempts               0
authority                     false
```

## 3. Development Split Ownership

After the single generator pipe returns a validated
`K2UncertaintyGeneratorResponseV1`, the existing confirm-owner process owns all
Development split persistence. It publishes under the attempt root before it
returns:

```text
generated/public/public-batch.json                 0600
generated/public/denominator-receipt.json          0600
generated/private/resolver/<case>.json             0400
generated/private/final-truth/<case>.json          0400
generated/development-split-receipt.json            0600
```

The public/private batch payloads are the exact objects from the one generator
response. Resolver and final-truth payloads are derived by confirm-owner from
that private batch. The linked runner cannot derive or rewrite them.

The new top-level receipt is
`K2UncertaintyDevelopmentRehearsalSplitReceiptV1`. It binds:

```text
schema and DevelopmentRehearsal mode
attempt root
owner request root
generator request root
generator response root
experiment ID
development seed commitment
public batch root
private batch root
public denominator root
exact sorted public/private artifact descriptors
authority false
receipt root
```

Each new `K2UncertaintyDevelopmentRehearsalStoredArtifactV1` binds kind, case ID
when applicable, relative path, mode, byte length, content SHA-256, semantic
root and artifact root. Confirm artifact descriptors are not accepted.

Existing split-neutral downstream payload types may be reused only when their
semantic roots transitively bind the Development public/private batch roots.
No Confirm split receipt may stand in for the new Development receipt.

## 4. Owner Receipt And Journal

V3 proposes using the existing optional owner-receipt split field:

```text
K2UncertaintyConfirmOwnerReceiptV1
  mode                                DevelopmentRehearsal
  split_receipt_root_sha256           Some(Development split root)
  nonce_commitment_sha256             None
  sealed_attempts                     0
  generator_dispatch_count            1
```

The owner appends `CasesGenerated` with the Development split root only after
the split receipt and every referenced file reopen and validate. It returns the
owner receipt only after that journal append is durable.

Confirm mode retains its existing semantics and byte representation.

## 5. Linked Process Route

One fresh route ID binds this exact chronology:

```text
DevelopmentRehearsal owner request
-> owner dispatches unchanged Development generator once
-> owner publishes typed Development split
-> owner returns split-bound receipt
-> runner reads public payload and split metadata only
-> public coordinator consumes Development public batch and denominator
-> learner, probe, safety and closure owners
-> ALL_CASES_PRECOMMITTED
-> public coordinator exits
-> private resolver receives only exact read-only resolver mount
-> final verifier receives only exact read-only truth mount
-> oracle and four frozen baselines
-> fresh R8B K1-K12 process controls
-> unchanged control evaluator
-> unchanged terminal evaluator
-> DEVELOPMENT_REHEARSAL_PASS
-> classified cleanup authorization
-> cleanup owner
-> independent cleanup verifier
-> DEVELOPMENT_REHEARSAL_COMPLETE
```

The runner may read artifact descriptors, paths, modes, content hashes and
semantic roots. It may not read private resolver or final-truth bytes, construct
a positive disposition, rewrite a child receipt, replace a process with a
library call, or return private results to the public coordinator.

## 6. Cross-Mode Controls

Required negative tests:

```text
Development loader rejects Confirm split receipt
Confirm loader rejects Development split receipt
Development owner rejects Confirm generator response
Development route rejects Confirm public batch
Development route rejects non-development seed commitment
Development owner receipt rejects absent or substituted split root
linked runner rejects owner/split request-root mismatch
linked runner rejects attempt-root mismatch
linked runner rejects response/batch-root mismatch
public coordinator rejects denominator/public-root mismatch
private resolver rejects wrong case/root/mount
terminal rejects Confirm or SealedAttempt control scope
```

Existing Confirm positive and negative bytes are frozen as parity fixtures and
must remain unchanged.

## 7. Persistence, Faults And Restart

Every artifact is published by create-new temp, write, file fsync, chmod,
rename and parent-directory fsync. The top-level Development split receipt is
last. A loader treats the split as complete only when that receipt and every
referenced artifact reopen with exact bytes and modes.

Fault injection covers before rename and after rename for every file. On
restart:

```text
complete typed split exists
-> loader validates it
-> journal may recover GeneratorDispatched to CasesGenerated
-> generator is never dispatched again

complete typed split absent
-> GeneratorResultIndeterminate
-> no downstream execution
-> classified cleanup only
```

A restarted existing attempt never creates a second dispatch or a replacement
split. Failure evidence is retained.

## 8. Exact Denominators

R8B records, never sums:

```text
package library/integration/doc tests             observed exact counts
static legacy controls                             32 / 32
static V3 controls                                  4 / 4
static V4 controls                                 16 / 16
DevelopmentRehearsal V5 controls                   12 / 12
R7G route                                           3 / 3
R7H route                                           9 / 9
R7I route                                           2 / 2
R7J route                                           2 / 2
R7K controls/cleanup                                2 / 2
R7K durability/restart                              7 / 7
R8B linked Development route                        1 / 1
R8B cross-mode controls                            observed exact count
```

## 9. Owner Separation

The 21-entry executable manifest from V2 remains, with these roles:

```text
confirm-owner             dispatch and Development split persistence
linked runner             typed request/root transport only
aggregate authorizer      validates complete conjunct set only
evidence publisher        atomic aggregate filesystem mutation only
```

The runner cannot authorize or publish `R8B_FROZEN`. The authorizer cannot run
tests or inspect private truth. The publisher cannot reinterpret evidence.

## 10. Resource And Production Boundary

The V2 resource limits remain unchanged: isolated delegated cgroup, MemoryPeak
`<= 512 MiB`, swap and OOM kills zero, each case `<= 60 s`, linked route
`<= 20 min`, protocol object `< 1 MiB`, no network calls and no production/K1
mutation. Compilation precedes measurement.

Production service, connector, dashboard, traffic, K1, phase memory,
authorization-slot and sealed-attempt roots are censused before and after.
R8B does not deploy, restart or pause the server.

## 11. Atomic R8B Packet

The V2 packet, aggregate authorizer and publisher contracts remain unchanged.
Individual evidence survives failure. `R8B_FROZEN` is published only after all
conjuncts, including the typed Development split and cross-mode controls, pass.

## 12. Required Gate Sequence

```text
V3 adversarial critique
-> repaired contract if any P0/P1 remains
-> owner-bounded structural routes
-> code-route design gate
-> implementation preflight with exact current bytes
-> READY_TO_IMPLEMENT
-> implementation commit
-> clean mini-PC build with 20 jobs
-> non-sealed R8B execution
```

No earlier paper or preflight receipt authorizes code.

## 13. Successor Boundary

R8B PASS may publish only `R8B_FROZEN` and unlock a separate R9B
source/executable/test freeze. R10B remains a mandatory exact-root authorization
stop. R11B remains the sole owner of exactly one sealed attempt.
