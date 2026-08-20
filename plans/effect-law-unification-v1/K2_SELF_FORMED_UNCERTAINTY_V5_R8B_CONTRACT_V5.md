# K2 Self-Formed Uncertainty V5 R8B Contract V5

Status: `STRUCTURAL ROUTES PASS / DESIGN CODE-ROUTE PASS / IMPLEMENTATION PREFLIGHT PENDING / NO CODE AUTHORITY`

Date: `2026-08-20`

Supersedes:

```text
K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V2.md
K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V3.md
K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4.md
```

Critique authority:
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md`

## 1. Claim And Stop Boundary

R8B may prove only:

```text
one exact implementation commit
-> complete non-sealed package verification
-> one linked DevelopmentRehearsal process route
-> exact control, restart, cleanup and resource denominators
-> R8B_FROZEN
```

It cannot prove the self-formed-uncertainty hypothesis, Natural K2,
natural-traffic transfer, Wave-causal grokking, product value, CPU savings or
deployment readiness. It creates no nonce, authorization slot or sealed
attempt. It grants no runtime or deployment authority.

R9B, R10B and R11B remain locked.

## 2. Source-Grounded Blocker

At source commit `bdcae5351c7de75f325b0ebe752804066823cc38`, Development
confirm-owner does this:

```text
validated Development owner request
-> one generator dispatch
-> validated in-memory Development response
-> CasesGenerated(response root)
-> historical ConfirmOwnerReceiptV1 with split root absent
-> stdout
```

It persists neither the response, a split nor the owner receipt. Existing
restart handling never returns a Development receipt. R7I and R7J start from a
separate Confirm fixture and therefore prove downstream components, not the
linked Development route.

R8B changes only this missing non-sealed connection and its R8B harness. It does
not change the scientific mechanism, scoring, learner, verifier, control
evaluator or terminal evaluator.

## 3. Chronology

```text
V5 paper and gate commit
-> exact implementation commit
-> clean mini-PC checkout at that commit
-> build with CARGO_BUILD_JOBS=20
-> all non-sealed R8B executions
-> immutable evidence packet
-> result commit whose parent is the tested implementation commit
```

The server, connector, dashboard, traffic, K1 registry and phase memory are
untouched.

## 4. Confirm Compatibility And One Development Producer

These existing schemas, constructors, validators and canonical bytes remain
unchanged:

```text
K2UncertaintyConfirmGeneratorRequestV1
K2UncertaintyConfirmGeneratorResponseV1
K2UncertaintyConfirmStoredArtifactV1
K2UncertaintyConfirmPrivateSplitReceiptV1
K2UncertaintyConfirmSplitReceiptV1
K2UncertaintyConfirmOwnerReceiptV1
K2UncertaintyConfirmPipeReceiptV1
```

Historical Development-shaped `K2UncertaintyConfirmOwnerReceiptV1` bytes remain
decodable and validation-compatible as historical evidence. They are
superseded route evidence: no new Development owner emits them and no R8B
loader, runner or authorizer accepts them.

The execution API is split by mode:

```text
execute_self_formed_development_rehearsal_owner_v1
  accepts DevelopmentRehearsal only
  returns DevelopmentRehearsalOwnerReceiptV1 only

execute_self_formed_confirm_owner_v1
  accepts Confirm only
  returns unchanged ConfirmOwnerReceiptV1 only

run_self_formed_confirm_owner_process_v1
  validates the shared request envelope
  dispatches to exactly one mode-specific API
  writes that concrete receipt directly to stdout
  never writes a wrapper enum
```

The former Development branch of `execute_self_formed_confirm_owner_v1` is
removed. A mode mismatch fails before attempt mutation.

The frozen R7H result remains valid only for its historical commit. Its old
current-tree assertion that Development has no split is intentionally
superseded. R8B records a separately named `R7H invariant compatibility`
denominator for slot, nonce, sandbox and Confirm-byte invariants; it does not
pretend the modified test file is the frozen R7H execution.

## 5. New Development Byte Types

The only new owner/split types are:

```text
K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1
K2UncertaintyDevelopmentRehearsalStoredArtifactV1
K2UncertaintyDevelopmentRehearsalSplitReceiptV1
K2UncertaintyDevelopmentRehearsalOwnerReceiptV1
```

Their schema strings are frozen:

```text
nando.k2-self-formed-development-rehearsal-stored-artifact.v1
nando.k2-self-formed-development-rehearsal-split-receipt.v1
nando.k2-self-formed-development-rehearsal-owner-receipt.v1
```

The kind enum has exactly:

```text
PublicBatch
PublicDenominator
ResolverTable
FinalTruth
```

Every new struct contains explicit
`mode = K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal`, denied
authority and its own canonical root.

### Stored artifact root

The artifact root binds, in this order:

```text
schema
mode
kind
case ID option
private-case ordinal option
relative path
Unix mode
byte length
content SHA-256
semantic root
denied authority
```

Public artifacts have no case ID or ordinal. Resolver and final-truth artifacts
have one case ID and one ordinal in `0..15`. Logical identity is unique by
`(kind, case ID, ordinal)` and relative path. Exact relative paths are:

```text
public/public-batch.json
public/denominator-receipt.json
private/resolver/<case-id>.json
private/final-truth/<case-id>.json
```

The public modes are `0600`; private modes are `0400`; all directories are
`0700`.

### Development split root

The split receipt root binds, in this order:

```text
schema and explicit DevelopmentRehearsal mode
attempt root and owner request root
owner and generator executable SHA-256
generator request and response roots
complete pipe receipt and pipe receipt root
experiment ID and development seed commitment
public and private batch roots
public denominator root
exact sorted 34 artifact descriptors
private reconstruction root
denied authority
```

The reconstruction root binds:

```text
nando.k2-self-formed-development-rehearsal-private-reconstruction.v1
ordered private case roots for ordinals 0..15
reconstructed private batch root
reconstructed generator response root
canonical response byte length
canonical response byte SHA-256
```

The split has exactly 34 descriptors:

```text
public batch                       1
public denominator                 1
private resolver table            16
private final truth                16
total                              34
```

### Development owner root

The owner receipt root binds, in this order:

```text
schema and explicit DevelopmentRehearsal mode
owner request and attempt roots
owner and generator executable SHA-256
generator request and response roots
public and private batch roots
Development split receipt root
pipe receipt root
CasesGenerated event root
generator dispatch count = 1
nonce commitment absent
authorization roots absent
sealed attempts = 0
denied authority
```

The exact nested tuple layout used by canonical serialization is frozen in the
implementation preflight before code. Any field, order or schema drift requires
a new contract revision.

## 6. Reused Payloads And Exact Response Reconstruction

These payload schemas may be reused unchanged in Development because their
roots bind a Development public batch, public case or private case transitively:

```text
K2UncertaintyPublicBatchV1
K2UncertaintyPrivateBatchV1
K2UncertaintyPublicCaseV1
K2UncertaintyPrivateCaseV1
K2UncertaintyConfirmPublicDenominatorReceiptV1
K2UncertaintyConfirmResolverTableV1
K2UncertaintyConfirmFinalTruthCaseV1
K2UncertaintyConfirmPipeReceiptV1
```

New Development constructors may be added for the denominator, resolver and
truth payloads. Existing Confirm constructors and validators do not change.

Before split publication succeeds, and again during full owner recovery, the
owner must:

1. Validate all 34 descriptor and file identities.
2. Match every resolver mapping exactly to the corresponding final-truth
   private-case mapping.
3. Match all case IDs and public-case roots to the public batch.
4. Rebuild private cases in persisted ordinal order `0..15`.
5. Rebuild and validate the exact `K2UncertaintyPrivateBatchV1`.
6. Rebuild and validate the exact `K2UncertaintyGeneratorResponseV1`.
7. Canonically serialize that response.
8. Match response root, byte count and byte SHA-256 to the complete pipe
   receipt.
9. Canonically serialize the Development generator request and match its byte
   count, request root and generator executable to the pipe receipt.
10. Match experiment ID to the attempt descriptor and seed commitment to the
    frozen Development commitment.

No raw generator response is persisted.

## 7. Immutable Publication

The owner publishes:

```text
attempt/generated/public/public-batch.json
attempt/generated/public/denominator-receipt.json
attempt/generated/private/resolver/<case-id>.json
attempt/generated/private/final-truth/<case-id>.json
attempt/generated/development-split-receipt.json
attempt/development-owner-receipt.json
```

Every immutable Development file uses:

```text
create-new temp
-> bounded write
-> file fsync
-> chmod
-> atomic no-clobber final publication
-> parent-directory fsync
-> remove any same-inode publication temp
-> parent-directory fsync
```

The implementation may use a same-filesystem hard-link publication or an
equivalent checked no-replace primitive. Ordinary overwriting rename is not
sufficient. Recovery may remove a known temp only after proving it is the same
inode and bytes as its already published final file. It never overwrites a
final path.

Every read checks, before content use:

```text
bounded path under canonical attempt root
all parents are real directories
final component is a regular non-symlink file
link count = 1 after publication recovery
exact mode and byte bound
descriptor content SHA-256
descriptor semantic root
```

A symlink, foreign hard link, path escape, device, FIFO, socket, extra file,
missing file or unclassified temp is failure.

The Development split receipt is published only after all 34 payloads pass
full validation. `CasesGenerated` is appended only after the owner reopens and
revalidates the complete split. The owner receipt is published only after the
journal append. The owner receipt is durable before the same bytes are written
to stdout.

## 8. Single Writer And Recovery State Machine

Before inspecting the Development attempt path, the owner acquires one
nonblocking exclusive lock on the canonical private lab-root directory and
holds it until the exact stdout receipt bytes have been prepared. A concurrent
owner returns `DEVELOPMENT_ATTEMPT_OWNER_BUSY` without attempt or journal
mutation. Process death releases the lock.

The exact restart states are:

```text
D0 attempt absent
  -> create attempt and journal
  -> append ArtifactsFrozen
  -> D1

D0I attempt directory exists without one valid ArtifactsFrozen journal
  -> ATTEMPT_INITIALIZATION_INDETERMINATE
  -> no dispatch
  -> retain and classify

D1 journal at ArtifactsFrozen, no GeneratorDispatched
  -> safe to append GeneratorDispatched and dispatch exactly once

D2 journal at GeneratorDispatched, no complete split
  -> append GeneratorResultIndeterminate exactly once when legal
  -> no redispatch and no downstream execution

D3 strict complete split, journal at GeneratorDispatched
  -> full response reconstruction and validation
  -> append CasesGenerated(split root)
  -> D4

D4 strict complete split, journal at CasesGenerated, no owner receipt
  -> require event payload = split root
  -> reconstruct and publish owner receipt
  -> D5

D5 strict complete split and owner receipt
  -> full cross-validation against request and journal
  -> return byte-identical owner receipt
  -> dispatch count remains one
```

Historical Development journals whose `CasesGenerated` payload is a raw
response root remain historical evidence. They cannot enter D4 or D5 and cannot
support R8B.

After `GeneratorDispatched`, absence or invalidity of a complete split is
indeterminate, never permission to rerun the generator. A retry after an
indeterminate attempt uses a fresh route and attempt ID.

## 9. Private-Truth Boundary

There are exactly two Development loaders:

```text
full owner validator
  used only inside Development owner and recovery
  reads and validates all public and private payloads

metadata/public runner loader
  validates owner and split receipt bytes and roots
  reads public batch and denominator only
  never reads resolver or final-truth content
```

The runner may transport private artifact path, mode, byte hash and semantic
root from the validated split receipt. Before each mount it checks path custody
without reading private content. It mounts exactly one private file read-only
into the corresponding resolver or final-verifier process. The child validates
the payload bytes against its typed request roots.

The public coordinator must exit successfully and its barrier receipt must be
durable before any private child starts. Private results return only to the
private orchestration/evaluation route, never to the exited public coordinator.

## 10. One Linked Development Route

One fresh route ID binds:

```text
Development owner request
-> one generator process and one anonymous pipe dispatch
-> owner-owned typed Development split
-> durable Development owner receipt
-> metadata/public runner loader
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

## 11. Mode And Legacy Negative Matrix

The following twenty cases are mandatory and separately named:

```text
X01 Development metadata loader <- Confirm split receipt
X02 Confirm loader <- Development split receipt
X03 Development loader <- historical Development ConfirmOwnerReceiptV1
X04 Confirm loader <- Development owner receipt
X05 Development owner <- Confirm generator response
X06 Confirm owner <- Development generator response
X07 Development split <- Confirm public batch
X08 Development split <- Confirm public denominator
X09 Development split <- Confirm pipe receipt
X10 Development split <- non-development seed commitment
X11 Development owner <- missing split receipt
X12 Development owner <- substituted split root
X13 runner <- mismatched owner request root
X14 runner <- mismatched attempt root
X15 runner <- mismatched generator response root
X16 public coordinator <- mismatched denominator/public root
X17 private child <- wrong case/root/mount
X18 terminal <- Confirm or SealedAttempt control scope
X19 R8B recovery <- historical CasesGenerated(response root)
X20 second concurrent owner <- live first owner on same attempt
```

Expected denominator: `20 / 20` rejected with zero attempt mutation for X20.

Separate positive denominators are:

```text
unchanged Confirm canonical byte fixtures
unchanged Confirm process and recovery parity
fresh Development owner/split route
Development D3/D4/D5 byte-identical recovery
```

## 12. Fault And Restart Denominators

Immutable Development publication has 36 final objects:

```text
34 payloads
1 split receipt
1 owner receipt
```

The pure persistence suite injects one fault before final publication and one
after final publication but before temp cleanup for every object:

```text
publication boundaries                 72 / 72
```

The process-level restart suite separately covers:

```text
P01 incomplete attempt initialization -> indeterminate, zero dispatch
P02 ArtifactsFrozen restart            -> one dispatch
P03 GeneratorDispatched no split       -> indeterminate, no redispatch
P04 complete split before journal      -> CasesGenerated and owner recovery
P05 CasesGenerated before owner        -> owner recovery
P06 owner durable before stdout        -> byte-identical replay
P07 concurrent live owner              -> busy, zero mutation by contender
```

Expected denominator: `7 / 7`. Pure publisher tests do not count as process
restart evidence, and process restart tests do not inflate publication
coverage.

## 13. Cleanup Ownership

For a successful non-sealed Development route:

```text
RetainAlways
  public batch and denominator
  Development split and owner receipts
  attempt journal and downstream typed receipts
  R8B suite, resource, production-census and aggregate receipts

DeleteAfterTerminalAndObserverFsync
  resolver tables and final-truth payloads
  sandbox workspaces, logs and known publication temps

SupersededNeverUse
  historical Development ConfirmOwnerReceipt fixtures
  superseded V2-V4 R8B preflight receipts
```

Every actual path in the linked attempt tree appears exactly once in the
cleanup registry. The independent cleanup verifier hashes the before-census,
validates terminal and observer roots, verifies required deletion/retention and
rejects any unclassified residue. A failed or indeterminate attempt receives a
separate failure receipt and complete path census before any deterministic
Development private payload is removed.

## 14. Other Exact Denominators

```text
package library/integration/doc tests             observed exact counts
static legacy controls                             32 / 32
static V3 controls                                  4 / 4
static V4 controls                                 16 / 16
DevelopmentRehearsal V5 controls                   12 / 12
R7G frozen route evidence                           3 / 3 historical
R7H invariant compatibility                        observed exact count
R7I component regression                           2 / 2
R7J independent component regression               2 / 2
R7K controls/cleanup                                2 / 2
R7K durability/restart                              7 / 7
R8B linked route                                    1 / 1
R8B mode/legacy negatives                          20 / 20
R8B publication faults                             72 / 72
R8B process restart                                 7 / 7
```

No denominator is summed into another. Historical evidence is never reported
as execution at the R8B implementation commit.

## 15. Owners And Executable Manifest

The 21-entry R8B manifest remains:

```text
18 predecessor process binaries, each exactly once
linked R8B runner test binary
R8B aggregate authorizer
R8B evidence publisher
```

Decision ownership remains separate:

```text
Development owner      lock, dispatch, split, full validation, persistence
linked runner          process orchestration and metadata transport only
aggregate authorizer   complete-conjunct validation only
evidence publisher     atomic aggregate mutation only
```

Every child rechecks its executable SHA-256. Missing, duplicate, substituted or
extra identity is terminal failure.

## 16. Resources And Atomic R8B Evidence

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

The R8B authorizer requires all suite, linked-route, cross-mode, fault,
restart, resource, production-census and cleanup receipts. It validates the
distinct Development schemas and zero authority effects. The publisher writes
the aggregate through temp, fsync, atomic publication and directory fsync.

`R8B_FROZEN` is absent unless every conjunct passes. Individual PASS and failure
evidence is retained. Structural gates remain `authority_ready=false`.

## 17. Required Gate Sequence

```text
V5 owner-bounded structural routes
-> V5 design code-route gate
-> V5 implementation preflight over exact current source and bytes
-> READY_TO_IMPLEMENT
-> implementation commit only
-> postimplementation observed-source code-route gate
-> clean mini-PC build with CARGO_BUILD_JOBS=20
-> non-sealed R8B suites and resource run
```

V2, V3 and V4 contracts, code-route designs and preflights are superseded and
grant no code authority.

The pre-implementation byte-root repair is recorded in
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5_PREFLIGHT_CRITIQUE.md`. Any
structural receipt issued against the earlier V5 candidate bytes is superseded
and cannot support implementation authority.

## 18. Successor Boundary

R8B PASS publishes only `R8B_FROZEN` and unlocks a separate R9B
source/executable/test freeze. R10B remains an exact-root authorization stop.
R11B alone may own exactly one sealed attempt.
