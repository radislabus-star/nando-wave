# K2 Self-Formed Uncertainty V5 R8B Contract V4

Status: `VETO / SUPERSEDED BY V5 / NO CODE AUTHORITY`

Date: `2026-08-20`

Supersedes:

```text
K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V2.md
K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V3.md
```

V3 critique:
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V3_CRITIQUE.md`

V4 critique:
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md`

Successor:
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md`

## 1. Exact Claim And Stop Boundary

R8B may prove only:

```text
one exact implementation commit
-> complete non-sealed package verification
-> one linked DevelopmentRehearsal process route
-> exact static, rehearsal and cross-mode denominators
-> bounded resources, restart and cleanup readiness
-> R8B_FROZEN
```

It cannot prove the self-formed-uncertainty hypothesis, Natural K2,
natural-traffic transfer, Wave-causal grokking, product value, CPU savings or
deployment readiness. It creates no nonce, authorization slot or sealed
attempt and grants no deployment authority.

## 2. Commit And Execution Chronology

```text
V4 paper and gate commit
-> exact implementation commit
-> fresh clean mini-PC checkout at that commit
-> build with 20 jobs
-> all non-sealed R8B executions
-> immutable evidence packet
-> result commit whose parent is the tested implementation commit
```

The server, connector, dashboard, traffic, K1 and phase memory are untouched.

## 3. Immutable Existing Bytes

Preflight freezes current bytes and root formulas for:

```text
Development generator request and response
Confirm generator request and response
Confirm owner request and V1 receipt
Confirm public/private split artifacts and receipt
Confirm pipe receipt
```

These existing schemas remain byte-for-byte and validation-compatible. No
existing V1 receipt changes meaning. Shared-helper edits are permitted only
when exact pre/post Confirm fixture hashes and all Confirm parity tests match.

## 4. Exact Type Matrix

### Development-only new types

```text
K2UncertaintyDevelopmentRehearsalStoredArtifactV1
K2UncertaintyDevelopmentRehearsalSplitReceiptV1
K2UncertaintyDevelopmentRehearsalOwnerReceiptV1
```

Each has a distinct schema string, explicit
`mode = DevelopmentRehearsal`, denied authority and its own root formula.

### Common split-bearing payloads

```text
K2UncertaintyPublicBatchV1
K2UncertaintyPrivateBatchV1
K2UncertaintyPublicCaseV1
K2UncertaintyPrivateCaseV1
```

Their existing validation binds `K2UncertaintySplitV1::Development` and the
frozen development seed commitment through public vocabulary and batch roots.

### Existing transitively bound payloads allowed in Development

```text
K2UncertaintyConfirmPublicDenominatorReceiptV1
K2UncertaintyConfirmResolverTableV1
K2UncertaintyConfirmFinalTruthCaseV1
K2UncertaintyConfirmPipeReceiptV1
```

Their names are historical. They may be reused only because their existing
root formulas bind the Development generator response, public batch, public
case or private case root. New Development constructors may be added, but the
existing structs, schemas, validators and Confirm constructors remain
unchanged. Every reuse has a positive Development test and a Confirm/Development
substitution negative test.

### Immutable Confirm-only types

```text
K2UncertaintyConfirmGeneratorRequestV1
K2UncertaintyConfirmGeneratorResponseV1
K2UncertaintyConfirmStoredArtifactV1
K2UncertaintyConfirmPrivateSplitReceiptV1
K2UncertaintyConfirmSplitReceiptV1
K2UncertaintyConfirmOwnerReceiptV1
```

No Development loader, owner or authorizer accepts these as its top-level
receipt.

## 5. Development Split Receipt

After exactly one anonymous pipe dispatch returns the unchanged validated
Development response, confirm-owner publishes:

```text
generated/public/public-batch.json                 0600
generated/public/denominator-receipt.json          0600
generated/private/resolver/<case>.json             0400
generated/private/final-truth/<case>.json           0400
generated/development-split-receipt.json            0600
```

The Development split receipt binds:

```text
schema and explicit mode
attempt root and owner request root
owner and generator executable SHA-256
generator request and response roots
complete pipe receipt and pipe receipt root
experiment ID and development seed commitment
public and private batch roots
public denominator root
exact sorted artifact descriptors
private-case reconstruction root
authority false
receipt root
```

Artifact descriptors bind kind, case ID when applicable, relative path, mode,
byte length, content SHA-256, semantic root and artifact root. The exact set is:

```text
public batch                       1
public denominator                 1
private resolver table            16
private final truth                16
total                              34
```

The sorted sixteen final-truth payloads contain the exact private cases and
denominator commitment needed to reconstruct the original private batch root.
The full-validation owner recomputes that root before issuing the split receipt.

## 6. Distinct Development Owner Receipt

`K2UncertaintyDevelopmentRehearsalOwnerReceiptV1` binds:

```text
schema and explicit mode
owner request and attempt roots
owner and generator executable SHA-256
generator request and response roots
public and private batch roots
Development split receipt root
pipe receipt root
journal CasesGenerated event root
generator dispatch count = 1
nonce commitment absent
authorization roots absent
sealed attempts = 0
authority false
owner receipt root
```

The confirm-owner binary selects stdout schema from the validated request mode:

```text
DevelopmentRehearsal -> DevelopmentRehearsalOwnerReceiptV1 bytes
Confirm               -> existing ConfirmOwnerReceiptV1 bytes
```

There is no wrapper enum on stdout and no change to Confirm bytes.

The owner persists the Development owner receipt atomically at
`attempt/development-owner-receipt.json` before writing the same bytes to
stdout.

## 7. Publication And Recovery State Machine

Every final artifact uses create-new temp, write, file fsync, chmod, rename and
parent-directory fsync. The Development split receipt is published after all
34 artifacts. `CasesGenerated` is appended only after the owner fully reopens
and validates every artifact. The owner receipt is published after the journal
append.

Exact recovery:

```text
owner receipt complete and valid
-> return identical receipt
-> dispatch count remains 1

split complete, journal at GeneratorDispatched
-> full owner validation
-> append CasesGenerated with split root
-> reconstruct and persist owner receipt from split-bound pipe receipt
-> return identical receipt

split complete, journal at CasesGenerated
-> full owner validation
-> reconstruct and persist owner receipt
-> return identical receipt

split incomplete or invalid
-> append GeneratorResultIndeterminate when legal
-> no downstream execution
-> no redispatch and no overwrite
-> classified cleanup
```

The same attempt path never overwrites a final file. Renamed partial artifacts
are retained as failure evidence until cleanup classifies them. Temporary files
without rename authority are disposable. A retry uses a fresh route/attempt ID.

## 8. Private-Truth Boundary

There are two loaders:

```text
full owner validator
  called only inside confirm-owner/recovery
  reads and validates all public and private payloads

metadata/public loader
  called by linked runner
  validates owner/split receipt bytes and roots
  reads public batch and denominator only
  never opens private resolver/final-truth files
```

The linked runner may transport private artifact paths, modes, hashes and
semantic roots. It mounts each exact private file read-only only into the
corresponding private resolver or final verifier process. Those child processes
validate payload bytes against their request roots. The independent cleanup
verifier later reopens and hashes the retained/final tree.

Private results never return to the public coordinator.

## 9. One Linked Development Route

One fresh route ID binds:

```text
Development owner request
-> one unchanged generator pipe dispatch
-> owner-owned typed Development split
-> durable Development owner receipt
-> public coordinator over exact Development public artifacts
-> learner, probe, safety and closure processes
-> ALL_CASES_PRECOMMITTED
-> public coordinator exits
-> private resolver and final verifier under exact mounts
-> oracle and four frozen baselines
-> fresh R8B K1-K12 processes
-> unchanged control evaluator
-> unchanged terminal evaluator
-> DEVELOPMENT_REHEARSAL_PASS
-> classified cleanup authorization
-> cleanup owner
-> independent cleanup verifier
-> DEVELOPMENT_REHEARSAL_COMPLETE
```

The runner transports typed requests and roots only. It cannot derive a split,
dispatch the generator, inspect private truth, construct a positive
disposition, rewrite a child receipt, replace a child process with a library
call, authorize or publish `R8B_FROZEN`.

## 10. Frozen Cross-Mode Matrix

The following fourteen negative cases are mandatory and separately named:

```text
X01 Development metadata loader <- Confirm split receipt
X02 Confirm loader <- Development split receipt
X03 Development owner <- Confirm generator response
X04 Confirm owner <- Development generator response
X05 Development split <- Confirm public batch
X06 Development split <- non-development seed commitment
X07 Development owner <- missing split receipt
X08 Development owner <- substituted split root
X09 runner <- mismatched owner request root
X10 runner <- mismatched attempt root
X11 runner <- mismatched generator response root
X12 public coordinator <- mismatched denominator/public root
X13 private child <- wrong case/root/mount
X14 terminal <- Confirm or SealedAttempt control scope
```

Expected denominator: `14 / 14` rejected. Confirm positive/parity fixtures and
Development positive/recovery fixtures are separate denominators.

## 11. Other Exact Denominators

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
R8B linked route                                    1 / 1
R8B cross-mode controls                            14 / 14
```

No denominator is summed into another.

## 12. Owners And Executable Manifest

The V2 21-entry executable manifest remains exact. No new decision owner is
introduced:

```text
confirm-owner          generator dispatch, full split validation, persistence
linked runner          process orchestration and metadata transport only
aggregate authorizer   complete-conjunct validation only
evidence publisher     atomic aggregate mutation only
```

Each child rechecks its executable SHA-256. Missing, duplicate, substituted or
extra identity is terminal failure.

## 13. Fault, Cleanup And Resource Evidence

Fault injection covers every Development artifact and owner-receipt publication
transition before rename and after rename. Every legal journal/file prefix has
an exact restart disposition and a test. Complete recovery proves identical
receipt and dispatch count one; incomplete recovery proves indeterminate and no
downstream execution.

Resource limits remain:

```text
MemoryPeak descendant-inclusive      <= 512 MiB
MemorySwapPeak                              0
OOMKills                                    0
each sandboxed case                    <= 60 s
complete linked route                  <= 20 min
protocol object                         < 1 MiB
manifest page entries                   <= 256
manifest entries                      <= 8,192
network calls                                 0
production/K1/dashboard mutations             0
```

Compilation completes before measurement. The linked test runs alone in a
fresh delegated cgroup. Measurement failure is indeterminate, never PASS.

## 14. Atomic Evidence And Authorization

The existing R8B packet layout, separate suite/control/resource/fault/cleanup
receipts, aggregate authorizer and atomic publisher remain. The authorizer
validates the distinct Development owner/split schemas, full denominator set,
resource evidence, production census, cleanup and zero authority effects.

`R8B_FROZEN` is absent unless every conjunct passes. Individual PASS and failure
evidence is retained. Structural gates remain `authority_ready=false`.

## 15. Required Gate Sequence

```text
V4 owner-bounded structural routes
-> V4 code-route design gate
-> V4 implementation preflight over exact current source and bytes
-> READY_TO_IMPLEMENT
-> implementation commit only
-> clean mini-PC build with CARGO_BUILD_JOBS=20
-> non-sealed R8B suites and resource run
```

V2 and V3 preflights are superseded and grant no code authority.

## 16. Successor Boundary

R8B PASS publishes only `R8B_FROZEN` and unlocks a separate R9B
source/executable/test freeze. R10B is a mandatory exact-root authorization
stop. R11B alone may own exactly one sealed attempt.
