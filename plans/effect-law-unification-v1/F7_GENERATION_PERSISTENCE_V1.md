# F7 Generation And Persistence V1

Status: `F7-A/B/C/D COMPLETE / F7-E UNLOCKED`

Authority: `false`

## Objective

Make the controlled F5 actor and F6 verifier restartable under one immutable
generation identity without reinterpreting an old package, relabeling support
as future evidence, or introducing a second owner of runtime truth.

```text
immutable operator artifacts
+ compiled structural dispatch
+ actor / renderer / verifier / capability / budget roots
-> CanonicalGenerationId
-> restart bundle
-> fresh support/future ledger
-> generation-bound F6 receipts
-> shadow integration
-> STOP-F7
```

F7 does not grant admission, mutate the production registry, or enable local
accept. Those are F8 boundaries.

## One Generation Identity

The kernel owns exactly one identity:

```text
CanonicalGenerationId = H(
  schema,
  sequence,
  parent generation,
  artifact set,
  dispatch index,
  actor program,
  renderer program,
  verifier contract,
  capability contract,
  resource budget
)
```

Every runtime bundle, evidence partition, and verifier receipt envelope must
carry this byte-identical ID. A downstream layer may validate it, but must not
derive a competing generation key.

Changing any committed component creates a new generation. Sequence one has no
parent. Every later sequence must name the previous immutable generation.

## Owner Slices

```text
F7-A  kernel manifest + runtime restart bundle
F7-B  generation-owned support/future ledger
F7-C  F6 receipt envelope bound to generation and lineage
F7-D  atomic IO adapter + restart recovery
F7-E  controlled shadow integration + STOP-F7
```

Owner boundaries:

```text
nando-operator-kernel
  owns generation identity and canonical bytes

nando-operator-runtime
  owns artifact decode, dispatch rebuild, pin and swap

nando-operator-proof
  owns generation-bound independent verifier receipts

nando-operator-learning
  owns support/future partition state and censored outcomes

nando-operator-persistence
  owns checkpoint composition, atomic file IO and crash recovery

nando-response-actor
  may own only startup and shadow orchestration adapters

nando-operator-admission
  is read-only during F7
```

## F7-A Contract

The restart bundle stores only canonical operator artifacts and the generation
manifest. On decode it must:

1. reject an oversized or non-canonical envelope;
2. validate every artifact and its canonical bytes;
3. reject duplicate artifact roots;
4. rebuild the structural dispatch index from artifacts;
5. recompute the F6 artifact-set root;
6. require both roots to match the manifest;
7. reproduce byte-identical bundle bytes;
8. expose `execution_authority=false`.

No raw request, response, teacher label, episodic payload, or actor-selected
binding is persisted.

F7-A derives and checks the artifact-set and dispatch-index roots. The actor,
renderer, verifier, capability, and resource-budget roots are immutable
commitments at this boundary; independent producer convergence for those roots
belongs to F7-E and is not claimed by F7-A.

Budgets:

```text
artifacts per generation       <= 32
restart bundle bytes           <= 512 KiB
manifest bytes                 <= 8 KiB
raw payload bytes persisted    0
authority                      false
```

## F7-B Evidence Ledger

Support and future rows are separate immutable partitions under the same
generation ID. Each row commits to a privacy-safe lineage root, event root,
request root, verifier receipt root, outcome class, and capture watermark.

Required outcome classes:

```text
VERIFIED_PASS
APPLICABILITY_NEGATIVE
HARD_CONTRADICTION
CENSORED
```

Only verified semantic outcomes enter positive or negative Wave evidence.
Timeout, unavailable environment, missing payload, and budget exhaustion are
censored and cannot become anti-centers.

Old support may remain support when its provenance still validates. It can
never be copied into the post-freeze future partition.

F7-B result:

```text
support-open -> freeze -> future-open state machine PASS
support append after freeze                         BLOCK
future append before freeze                         BLOCK
future row before watermark                         BLOCK
support lineage reused as future                    BLOCK
duplicate event/request/receipt roots               BLOCK
future growth changes evidence root                 PASS
future growth changes generation ID                 NO
censored semantic update                            NONE
canonical restart                                   BYTE IDENTICAL
execution authority                                 false
```

Canonical receipt:
`STOP_F7_B_GENERATION_EVIDENCE_LEDGER.json`.

## F7-C Receipt Binding

An F6 receipt becomes generation evidence only through a new opaque envelope:

```text
CanonicalGenerationId
+ partition kind
+ lineage root
+ event root
+ post-freeze watermark
+ F6 receipt SHA-256
+ F6 request SHA-256
-> GenerationVerifierReceiptV3
```

The envelope does not upgrade a F6 `ABSTAIN` or `REJECT` to PASS. It stores no
raw payload and grants no authority.

The envelope commits the capture sequence, lineage, event, partition and exact
support-freeze boundary, but it does not claim that arbitrary caller-supplied
roots are trusted capture evidence. F7-E must join these roots to the live
capture owner's immutable commitment before they can enter shadow evaluation.
That join remains outside the proof module so the verifier cannot become a
second capture owner.

F7-C result:

```text
F6 receipt canonical restart                         PASS
F6 artifact set equals generation artifact set       PASS
exact F6 verdict preserved                           PASS
REJECT relabelled as VERIFIED_PASS                   BLOCK
foreign generation manifest                          BLOCK
support receipt carrying freeze root                 BLOCK
future receipt before watermark                      BLOCK
future receipt without exact support freeze          BLOCK
receipt -> generation ledger bridge                  PASS
raw episodic payload persisted                       NO
production callers                                   0
execution authority                                  false
```

Canonical receipt:
`STOP_F7_C_GENERATION_VERIFIER_RECEIPT.json`.

## F7-D Persistence

`nando-operator-persistence` stores one self-validating checkpoint containing:

```text
canonical generation restart bundle
+ generation-owned evidence ledger
+ exact F6 receipt bytes
+ exact F7 receipt envelopes
+ receipt-set and checkpoint roots
```

Every ledger row must join one receipt pair exactly on generation, partition,
sequence, watermark, support freeze, lineage, event, request and positive
verdict. Missing, duplicate or extra receipts invalidate the checkpoint.

The store uses two alternating slots. It writes the inactive slot with
`create_new`, mode `0600`, file `fsync`, atomic rename and directory `fsync`.
The previous slot remains untouched until the new slot is fully durable.

Startup will eventually open HTTP/fallback before loading this store. Restore
happens off the request path and swaps the ready immutable generation only in
F7-E; F7-D has no serving caller.

Failure behavior:

```text
missing checkpoint       -> empty shadow registry
tampered checkpoint      -> quarantine + ABSTAIN
unknown schema           -> no migration in place
partial write            -> retain previous generation
non-monotonic generation -> reject swap
```

F7-D result:

```text
bundle + ledger + exact F6/F7 receipt join       PASS
canonical checkpoint restart                     BYTE IDENTICAL
two-slot publication                              PASS
file fsync -> rename -> directory fsync           SYSCALL VERIFIED
stale partial .new                                QUARANTINE
corrupt newest slot                               PREVIOUS RESTORED
both slots corrupt                                EMPTY SHADOW
same-generation evidence rollback                 BLOCK
publish sequence jump                             BLOCK
wrong next-generation parent                      BLOCK
symlink slot                                      QUARANTINE
broken temporary symlink                          QUARANTINE
raw runtime payload persisted                     NO
production callers                                0
execution authority                               false
```

Canonical receipt:
`STOP_F7_D_ATOMIC_GENERATION_STORE.json`.

### F7-D Live Boundary

F7-D has no live capture-owner join, startup loader, serving registry swap or
request-path caller. Those integrations belong only to F7-E shadow work.

### F7-D Authority Boundary

F7-D cannot call external admission, publish an ACTIVE package, enable local
accept or authorize execution. Those powers remain blocked until F8.

## STOP-F7

```text
new generation digest                     PASS
every committed component changes digest  PASS
fresh frozen future                       PASS
support/future lineage disjointness        PASS
restart byte identity                      PASS
artifact/index/F6 root convergence         PASS
tamper and truncation rejection            PASS
old generation bytes unchanged             PASS
ambient episodic memory needed             NO
censored semantic updates                  0
false accepts                              0
parity mismatches                          0
execution authority                        false
production callers                         0 until F7-E shadow
```

F8 remains blocked until STOP-F7 is complete and an external reader can
independently reconstruct the same generation, partition roots, and F6 receipt
set.

## F7-A Result

```text
single kernel-owned generation ID             PASS
all seven component roots committed           PASS
every committed component changes ID          PASS
F6/F7 artifact-set root convergence            PASS
dispatch rebuilt from canonical artifacts      PASS
artifact order invariance                      PASS
restart byte identity                          PASS
tamper/truncation/duplicate/oversize rejection PASS
old generation bytes unchanged                 PASS
raw episodic payload persisted                 NO
production callers                             0
execution authority                            false
```

Canonical receipt:
`STOP_F7_A_GENERATION_RESTART.json`.
