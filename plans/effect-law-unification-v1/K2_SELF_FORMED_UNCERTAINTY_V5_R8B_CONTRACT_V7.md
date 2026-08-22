# K2 Self-Formed Uncertainty V5 R8B Contract V7

Status: `REPAIRED AFTER CRITIQUE V2 / PAPER ONLY / NO EXECUTION AUTHORITY`

Date: `2026-08-21`

V7 preserves the V6 scientific claim, private-truth boundary, restart state
machine, cleanup transaction, denominators and resource limits. It replaces
only the process-ledger ownership, suite receipt transport, aggregate
chronology and exact implementation scope named below.

## 1. Claim Boundary

R8B remains one cold, non-sealed DevelopmentRehearsal route. It may establish
only that the complete R7K Development route can be linked, measured, cleaned
and independently authorized without production mutation.

It does not establish K2 capability, natural-traffic validity, grokking,
runtime admission, a LawCertificate or deployment readiness.

```text
sealed attempts             0
authorization slots         0
production mutations        0
child external network      0
false accepts               0
```

## 2. Identities

The linked manifest remains exactly M01-M26 from V6. The suite manifest
remains exactly S01-S05. Manifest membership and process invocation counts
remain separate denominators.

M24 is the root orchestrator and parent observer. It is not its own child and
does not manufacture a `ChildStarted` row for an already-running process.
Its executable identity is bound by the linked manifest, parent-owned packet
entries and the final publication request.

## 3. Acyclic Chronology

The positive route is:

```text
P00 validate linked and suite manifests
-> P01 launch S01-S05 receipt-producing suite invocations
-> P02 read-only pre-production snapshot
-> P03 launch M24 child in one fresh delegated cgroup

child C01 Development owner
-> C02 generator
-> C03-C05 durable Development split and metadata
-> C06-C07 public coordinator and public descendants
-> C08 downstream invocation contract
-> C09-C14 private, oracle, controls and terminal
-> C15-C20 real cleanup and Development completion
-> C21 child candidate frozen
-> C22 child exit

parent P04 child exit and cgroup resources finalized
-> P05 post-production snapshot and survival
-> P06 closed aggregate packet frozen
-> P07 M25 authorizes that packet
-> P08 M26 publishes exact M25 bytes
-> P09 post-authorization audit freezes M25/M26 process outcomes
```

The M25 decision consumes only P06. P09 is audit evidence, never an input to
P07 or P08. No result authorizes its own producer or a future process exit.

## 4. Hierarchical Process Ledger

One append-only ledger covers every receipt-producing process completed before
P06, excluding only the already-running M24 root orchestrator.

The actual spawn owner writes each pair:

```text
M24 -> S01-S05, M24 child, M01, M10-M23
M01 -> M02
M10 -> every M03-M09 invocation
```

Before the owning process calls `spawn`, it acquires the ledger-directory lock,
reopens and validates the complete natural prefix, appends one immutable
`ChildStarted` row, fsyncs the file and directory, then releases the lock.

After normal exit and typed canonical receipt validation, the same owner
reopens the prefix and appends one immutable `ChildFinished` row. A failed
spawn, abnormal exit, malformed output, non-canonical output or validator
failure leaves a started-without-finished suffix and makes the fresh route
indeterminate. It is never replay authority.

M24 supplies only the route ID and absolute ledger root to M01 and M10. Those
values select the journal; they grant no experiment, cleanup, terminal or R8B
authority. Every writer verifies the existing route ID, sequence and previous
event root before append.

Every writer also binds its own executable before append and has a closed
child-role allowlist:

```text
M01 current executable -> M02 only
M10 current executable -> M03-M09 only
M24 root/child          -> its direct manifested children only
S02-S05 suite owner     -> direct Nando children named by that suite request
```

A foreign executable, foreign route, stale prefix, role outside the allowlist
or path not equal to the canonical ledger root is rejected before spawn.

Every finished row binds:

```text
route/stage/case/probe ordinal
role and executable SHA-256
request semantic root and stdin SHA-256
normal exit and exit code
actual stdout length and SHA-256
actual stderr length and SHA-256
bounded produced-receipt descriptors
start and finish monotonic values
```

Each produced-receipt descriptor binds:

```text
canonical final packet relative path or the reserved `stdout` route
byte length, Unix mode and content SHA-256
receipt schema and typed semantic root
```

For ordinary one-shot protocol processes the list has exactly one descriptor,
uses the reserved `stdout` route and its bytes equal stdout. Suite producers
and M24 child use the closed channel in Section 5. Descriptor paths and roots
are unique inside one event and the list is bounded by the packet limits.

S02-S05 use the same shared test-support appender around every direct Nando
subprocess. M01 and M10 still own their nested descendants, so one actual spawn
has one writer. Diagnostic host tools are not receipt producers and are bound
as tool dependencies rather than inserted into either executable manifest.

M25 and M26 outcomes are written to a separate post-authorization audit log.
They are not members of the P06 process ledger and are not packet conjuncts.

## 5. Canonical Multi-Receipt Channel

S01-S05 remain the producers of their own evidence. The M24 child remains the
producer of its linked-route aggregates. M24 parent does not construct either
class of PASS from an exit code or expected count.

Before each suite launch M24 creates one empty private output directory and
freezes a suite request binding:

```text
route ID
suite role and executable SHA-256
exact test selector
allowed evidence kinds and exact denominators
exclusive output directory
request root
```

The suite binary runs the named aggregate test. It performs the underlying
checks, constructs its typed `K2UncertaintyR8BMeasuredReceiptV2` values, writes
each through create-new + fsync + no-clobber publication, fsyncs the directory
and exits normally. Libtest stdout/stderr remain ordinary process diagnostics.

Each closed-channel file is created at the same relative path it will occupy
inside P06. P06 copies the bytes to that path without changing content or
logical path; hard links are forbidden on both sides.

After exit M24 reopens the exclusive directory, requires the exact
request-declared path set and rejects missing, extra, symlinked, hard-linked,
writable or non-canonical objects. Every receipt must be regular `0400` with
`nlink=1`. M24 validates every typed receipt, freezes the directory to `0500`
and only then writes `ChildFinished`. The process event records actual
stdout/stderr separately from the produced-receipt descriptors.

The exact suite ownership is:

```text
S01 core aggregate
  Confirm canonical bytes
  Development known answers 3/3
  immutable publication boundaries 72/72

S02 restart aggregate
  P01-P07 7/7
  tool dependency: /usr/bin/strace path/mode/length/SHA-256

S03 mode aggregate
  X01-X20 20/20

S04 cleanup negative aggregate
  interrupted cleanup 1/1

S05 authority aggregate
  aggregate publication faults 2/2
```

The M24 child receives a separate empty output directory before P03. It emits
exactly these three immutable objects after C20 and before C22:

```text
linked-route measured receipt
Oracle batch built from the sixteen actual M16 receipt roots
four-scope coverage census built from four distinct M17 receipt roots
```

The coverage census uses `K2UncertaintyR8BMeasuredReceiptV2`, observes `4`,
binds all four control receipt roots and is explicitly derived coverage. It is
not counted as a fifth control result and cannot replace the Legacy, V3, V4 or
fresh-control denominators.

`FrozenControlScopes` therefore uses the measured-receipt schema in V7. M25 no
longer decodes it as an ordinary single-scope M17 receipt.

The positive cleanup transaction and Development result remain actual M22 and
M23 linked-route receipts. They are not suite summaries.

M25 verifies suite and M24-child packet bytes against exactly one matching
produced-receipt descriptor from the named producer event, not against libtest
framing bytes. One descriptor cannot satisfy two entries. Substitution by the
M24 parent, another role or an unmanifested executable is VETO.

## 6. Public Coordinator And Generator Instrumentation

The M01 and M10 decision logic remains unchanged. V7 adds observation around
their existing process boundaries:

```text
M01: journal M02 start -> existing generator dispatch -> validate response
     -> journal M02 finish

M10: journal each M03-M09 start -> existing sandbox dispatch -> decode and
     validate canonical output -> journal finish
```

No prepared output, replay cache, in-process substitute, wrapper executable,
synthetic row or second generator dispatch is allowed.

`confirm_sandbox.rs` may expose its already-measured outcome to M10 without
changing guest bytes, mount authority, limits or default callers. The existing
one-shot API must remain byte-identical for non-R8B callers.

## 7. Private Truth, Cleanup And Resources

The V6 descriptor contract remains exact. Resolver, final-truth and Oracle
private files are opened with `O_PATH | O_NOFOLLOW`, verified by descriptor
metadata and mounted from the inherited descriptor path. M24 never reads,
hashes, decodes or serializes private contents.

M20-M23 remain separate authorization, mutation, verification and result
owners. The real intent-first cleanup path and all V6 retention classes remain
unchanged.

The child C01-C22 route alone is measured in the delegated cgroup:

```text
MemoryPeak descendant-inclusive <= 512 MiB
MemorySwapPeak                   = 0
OOMKills                         = 0
complete child route             <= 20 min
each sandbox                     <= 60 s
child external network calls     = 0
```

Suite processes, M24 parent, M25 and M26 are separate resource denominators.

## 8. Aggregate Packet

P06 contains exactly the nineteen V6 evidence kinds, actual canonical receipt
bytes, both identity manifests and the complete pre-authorization ledger.

Parent-owned entries remain limited to linked manifest, suite manifest and
production survival. Every other entry must bind a completed producer event.

For a normal protocol producer:

```text
entry bytes == producer `stdout` descriptor bytes == actual stdout bytes
```

For S01-S05 and M24 child only:

```text
entry bytes == one unique closed-channel produced-receipt descriptor
```

Actual stdout/stderr hashes remain in the event and cannot be replaced by the
receipt-channel hashes.

The Oracle batch packet entry names the M24 child producer event and binds all
sixteen M16 case roots through its own typed validator. The four-scope census
does not duplicate any control receipt bytes.

## 9. Exact Source Scope

V7 implementation begins from the V7 gate commit. Its source scope is:

```text
7 modified predecessor paths
  the six V6 paths
  + confirm_public_coordinator.rs

16 new V6 paths

23 total implementation paths
```

No other predecessor decision owner may change. In particular oracle,
control, terminal, cleanup and Development result decision logic remains exact
predecessor bytes.

The existing V6 partial implementation may be replayed only after source-scope
parity confirms that every changed path belongs to this V7 inventory.

The revised ownership budgets are:

```text
immutable_publication.rs     <= 650 lines  shared locked ledger appender
r8b_model.rs                 <= 800 lines  produced-receipt vector and schemas
r8b_authorizer.rs            <= 600 lines  descriptor-to-entry matching
r8b_support/mod.rs           <= 1200 lines shared process/suite orchestration
r8b_linked_v1.rs             <= 1600 lines complete parent/child route
```

All other V6 file budgets remain unchanged. These increases buy named process
ownership and independent byte matching; they do not authorize helper-junk
drawers, duplicated decision logic or a larger source scope.

## 10. Required Negative Tests

V7 adds these mandatory failures to the V6 suite:

```text
N01 M01 cannot spawn M02 before durable start
N02 M10 cannot spawn M03-M09 before durable start
N03 nested writer with foreign route/root is rejected
N04 nested writer with stale ledger prefix is rejected
N05 typed validation failure leaves no ChildFinished
N06 suite stdout substituted for receipt channel is rejected
N07 M24-created suite receipt is rejected
N08 suite missing/extra/symlink/hard-link/writable receipt is rejected
N09 M25/M26 future outcome inserted into P06 is rejected
N10 post-authorization audit used as M25 input is rejected
N11 one produced descriptor reused by two packet entries is rejected
N12 Oracle batch without sixteen distinct M16 roots is rejected
N13 four-scope census without four distinct M17 roots is rejected
N14 suite direct Nando subprocess missing its ledger pair is rejected
N15 S02 strace tool identity missing or substituted is rejected
N16 produced path changed between child output and P06 is rejected
```

## 11. Gates And Execution Boundary

```text
V6 discrepancy
-> V7 contract
-> adversarial critique
-> repaired V7 if required
-> structural route gates
-> design code-route gate
-> exact implementation preflight
-> READY_TO_IMPLEMENT
-> implementation
-> observed-source code-route
-> source-scope parity
-> static build and tests
-> separate explicit R8B execution boundary
```

Paper PASS, compilation or component tests do not authorize an R8B attempt,
deployment, dashboard change, production write, push or scientific claim.

P09 is append-only diagnostics only. It cannot change M25 authorization bytes,
M26 publication bytes or either disposition. The final authority artifact is
the exact M25 authorization published by M26 together with M26's concrete
publication receipt.
