# K2 Self-Formed Uncertainty V5 R8B Contract V6

Status: `REPAIRED AFTER CRITIQUE / STRUCTURAL GATES PENDING / NO CODE AUTHORITY`

Date: `2026-08-21`

Supersedes for future implementation only:

```text
K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md
```

Preserves V5 byte-contract and Confirm-compatibility requirements unless this
document explicitly replaces one. The failed implementation remains preserved
at `af18cad60054a70eb9bdeb8f815e174575ca664e`.

Critique authority:
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6_CRITIQUE_V1.md`

## 1. Exact Claim Boundary

R8B V6 may prove only:

```text
one exact implementation commit
-> one complete non-sealed DevelopmentRehearsal process route
-> exact process, restart, cleanup, resource and production-survival receipts
-> one aggregate authorization over those actual receipts
-> R8B_FROZEN
```

It cannot prove the self-formed-uncertainty hypothesis, Natural K2,
natural-traffic transfer, Wave-causal grokking, answer quality, product value,
CPU savings or deployment readiness. It creates no nonce, authorization slot or
sealed attempt and grants no runtime authority.

R9B, R10B and R11B remain locked until a separately reviewed R8B result exists.

## 2. Chronology And Isolation

```text
V6 draft
-> adversarial critique
-> repaired V6 bytes
-> structural route gates
-> design code-route gate
-> implementation preflight
-> READY_TO_IMPLEMENT
-> exact implementation commit
-> observed-source code-route gate
-> clean mini-PC build with CARGO_BUILD_JOBS=20
-> non-sealed R8B run
-> immutable result commit
```

The implementation commit is a child of the final V6 paper commit. The prior
`af18cad` commit is evidence and a donor only; it is never represented as V6
PASS.

Production services, connector, dashboard, K1 registry, phase memory and
ordinary traffic remain untouched.

## 3. Exact 26-Identity Linked Manifest

The canonical linked-route executable manifest contains exactly one entry for
each of these identities:

```text
M01 Development owner
M02 deterministic generator
M03 learner
M04 probe
M05 selector
M06 frozen-baseline evaluator
M07 selection preverifier
M08 closure planner
M09 closure verifier
M10 public coordinator
M11 private resolver
M12 safety evaluator
M13 inquiry worker
M14 inquiry observer
M15 final verifier
M16 oracle-baseline evaluator
M17 control evaluator
M18 terminal evaluator
M19 fresh R7K control-case producer
M20 cleanup authorizer
M21 cleanup owner
M22 cleanup verifier
M23 Development result publisher
M24 linked R8B runner test binary
M25 R8B aggregate authorizer
M26 R8B evidence publisher
```

Every entry binds role, canonical executable path, byte length, Unix mode and
SHA-256. Paths and hashes are pairwise unique. Missing, duplicate, substituted
or extra entries fail before linked execution.

Manifest membership and invocation count are different denominators. The
manifest contains 26 identities exactly once; the process ledger records every
actual invocation, including per-case and per-probe repeats.

The fresh control-case producer is distinct from the control evaluator. The
Development result publisher is distinct from the cleanup verifier and R8B
evidence publisher. None may substitute for another.

Mandatory non-linked suites have a separate five-entry producer manifest:

```text
S01 crate unit-test binary: known answers, publication and no-follow controls
S02 restart integration-test binary: P01-P07
S03 mode-matrix integration-test binary: X01-X20
S04 cleanup-negative integration-test binary
S05 aggregate-authority and publication-fault integration-test binary
```

Suite identities and linked identities are never summed. M24 owns the canonical
linked route and is not duplicated in the suite manifest.

## 4. One Parent/Child Linked Receipt Chain

One fresh route ID and attempt ID bind this exact order. M24 runs once as a
parent observer and once as a child route process; both invocations bind the
same executable SHA-256 and have separate process-ledger rows.

```text
P00 executable and suite manifests validated
-> P01 read-only pre-production snapshot
-> P02 linked child launched in a fresh delegated cgroup

child C01 Development owner request
-> C02 one generator dispatch and pipe receipt
-> C03 immutable 34-artifact Development split
-> C04 durable Development owner receipt
-> C05 metadata/public loader receipt
-> C06 public coordinator and ALL_CASES_PRECOMMITTED
-> C07 public coordinator process exit observed
-> C08 downstream invocation contract frozen from immutable closure plans
-> C09 private resolver/safety/worker/observer/final-verifier receipts
-> C10 oracle case receipts and oracle batch
-> C11 twelve fresh R7K control-case process receipts
-> C12 four separate control-evaluator receipts
-> C13 unchanged terminal evaluator receipt
-> C14 DEVELOPMENT_REHEARSAL_PASS
-> C15 complete before-cleanup census and registry
-> C16 cleanup authorization receipt
-> C17 cleanup owner receipt and deletion journal
-> C18 independent cleanup verification receipt
-> C19 Development result-publisher receipt
-> C20 DEVELOPMENT_REHEARSAL_COMPLETE
-> C21 immutable child candidate packet fsynced
-> C22 linked child exits

parent P03 child cgroup and process-exit resource receipt finalized
-> P04 read-only post-production snapshot and survival receipt
-> P05 closed aggregate evidence packet frozen
-> P06 R8B aggregate authorizer over that packet
-> P07 immutable R8B_FROZEN publication
```

Each transition consumes canonical bytes from the preceding receipt or a
manifested set of preceding receipts and binds their semantic roots. A route
receipt cannot be replaced by a listed executable hash, an expected count, a
fixture root or an in-process constructor result.

The canonical linked parent/child test launches every process required by
C01-C22 and P06-P07. Component
tests may use synthetic negative inputs, but their roots cannot satisfy the
positive linked-route conjunct.

The child cannot authorize or publish `R8B_FROZEN`. The parent cannot issue a
terminal, cleanup, completion or aggregate PASS. Only M25 can authorize the
closed aggregate and only M26 can publish it.

## 5. Development Owner And Restart State Machine

V5 Development byte types, 34 payloads, split root, owner root, immutable
publication rules, one nonblocking lab-root lock and one generator dispatch are
retained.

The seven process-level restart cases are mandatory and separate from the 72
pure publication boundaries:

```text
P01 incomplete attempt initialization
    -> real owner process returns indeterminate
    -> zero generator dispatch

P02 ArtifactsFrozen, no GeneratorDispatched
    -> real owner process performs exactly one first dispatch
    -> complete owner receipt

P03 GeneratorDispatched, no complete split
    -> real owner process records/returns indeterminate
    -> no redispatch

P04 complete split, journal before CasesGenerated
    -> real owner reconstructs split
    -> appends CasesGenerated and publishes owner receipt

P05 CasesGenerated, no owner receipt
    -> real owner publishes byte-identical owner receipt

P06 owner receipt durable, stdout absent
    -> real owner returns byte-identical receipt
    -> no dispatch and no durable mutation

P07 traced first real owner holds the exact lab-root lock
    -> second real owner returns busy
    -> contender causes zero tree mutation
    -> first owner remains authoritative and completes
```

Every case starts from a fresh attempt ID, launches the actual owner executable,
records process exit, stdout, stderr hash, before/after tree census and generator
dispatch count, and has one expected terminal disposition. A library-only state
projection is not process restart evidence.

The owner uses one nonblocking `flock(LOCK_EX | LOCK_NB)` on the opened canonical
lab-root directory. P07 uses parent-child `ptrace` with syscall-stop and
fork/exec tracing. The harness stops the first real owner immediately after the
successful `flock` syscall, verifies the stopped PID and lab-root inode in `/proc/locks`,
launches the second real owner, then resumes and detaches the first. The process
trace also records every generator `execve`. If tracing or `/proc/locks`
identity is unavailable, P07 is indeterminate and R8B cannot pass. It is never
skipped and a foreign test-process lock holder is not accepted as P07.

## 6. Private Truth And Oracle Mount

The linked runner may read:

```text
public payload contents
typed split and owner receipts
private artifact relative path
private artifact kind, mode, byte length, content SHA-256 and semantic root
filesystem custody metadata without content bytes
```

The runner may not read, decode, copy, hash or serialize resolver-table or
final-truth contents. It opens each private source only with
`O_PATH | O_NOFOLLOW`, verifies inode, regular-file type, link count, mode and
length through `fstat`, keeps that descriptor open through child startup and
uses the inherited descriptor path as the bind source. A pathname-only mount is
forbidden.

For every oracle case, the runner constructs the public evidence entries and an
oracle manifest. The private-truth manifest entry is constructed only from the
immutable Development split descriptor. The actual private-truth file is
mounted read-only directly into the oracle child at the manifest path. The
oracle child alone reads the bytes, recomputes length and content SHA-256,
decodes the typed truth, checks the semantic root and emits the oracle receipt.

The existing sandbox transport gains exactly two new guest roles without
changing evaluator logic:

```text
Oracle
  read-only /oracle evidence root
  read-only /oracle/private-truth.json descriptor overlay
  cwd /oracle

R8BAggregateAuthorizer
  read-only /evidence aggregate packet
  cwd /evidence
```

The host oracle evidence root contains one classified, non-authoritative
mountpoint file which is overlaid before the child starts. Manifest closure is
validated inside the child namespace. The oracle process runs in a networkless
bubblewrap namespace with a read-only public evidence root and exactly one
read-only private-truth mount. A wrong
path, mode, length, content hash, semantic root, case ID or extra private mount
must fail.

The public coordinator exits before the first private resolver, final verifier
or oracle process starts. Private results never return to the public
coordinator.

## 7. Downstream Process Ledger

The linked runner writes an append-only process ledger before accepting any
child receipt. Every row binds:

```text
route ID and stage ID
case ID and optional probe ordinal
manifest role and executable SHA-256
request semantic root and stdin SHA-256
normal exit and exit code
stdout byte length, SHA-256 and decoded receipt root
stderr byte length and SHA-256
start/end monotonic timestamps
```

Public coordinator cardinalities are frozen from the 16-case Development batch
before C06. Closure-plan lengths do not exist yet and are not guessed. After
`ALL_CASES_PRECOMMITTED` and public coordinator exit, the child reopens the
immutable public artifacts, derives exact private, worker, observer, final and
oracle cardinalities, publishes C08, fsyncs it and only then starts C09.
Observed cardinality must equal C08. No missing or extra process is tolerated.

Before every spawn the child fsyncs a `ChildStarted` journal row. After normal
exit and canonical receipt validation it fsyncs one `ChildFinished` row. A
started-without-finished row is indeterminate and is never permission to replay
the child automatically. A fresh route and attempt ID are required.

The generator cardinality is always exactly one for the canonical route.

## 8. Real Cleanup Transaction

Cleanup and completion remain a four-process route with separate authority,
mutation, proof and result-publication owners:

```text
complete before-census + terminal root + observer fsync root
-> cleanup authorizer process
-> authorization receipt
-> cleanup owner process
-> intent-first deletion journal
-> owner receipt
-> cleanup verifier process
-> retained parity + required absence + no residue
-> Development result-publisher process
-> DEVELOPMENT_REHEARSAL_COMPLETE
```

The runner may construct the closed cleanup registry, but cannot authorize a
deletion, perform a deletion or issue the completion receipt.

Every actual linked-attempt path is classified exactly once as:

```text
RetainAlways
DeleteAfterTerminalAndObserverFsync
SupersededNeverUse
```

Unclassified residue, duplicate path ownership, symlink, foreign hard link,
wrong retained bytes, missing retained evidence, undeleted disposable data or
interrupted deletion blocks C20 and therefore blocks `R8B_FROZEN`.

The positive cleanup denominator must launch all three existing cleanup
executables and the distinct existing Development result publisher. A census
plus manual file removal in the runner is a negative fixture only.

## 9. Aggregate Evidence Authority

The parent freezes one closed, read-only aggregate evidence directory. It
contains a manifest, paged entries and every canonical receipt needed for the
claim. The R8B authorizer is launched through the
`R8BAggregateAuthorizer` sandbox role. Its stdin contains only the route ID,
manifest root and M25 executable SHA-256. It reopens the complete packet and
rejects any missing, extra, symlinked, hard-linked, writable or mismatched path.

Each manifest entry binds:

```text
conjunct kind
relative path
byte length and mode
content SHA-256
typed semantic root
producer executable role and SHA-256
route ID
observed denominator
PASS / FAIL / INDETERMINATE
```

For a positive request, every entry contains canonical bytes produced by the
actual linked route or a separately named mandatory suite. The authorizer reads
the bytes, recomputes content SHA-256, decodes the kind-specific receipt and
checks its semantic root, producer, route and denominator. Ad hoc roots,
expected-only counts and synthetic positive conjunct constructors are rejected.
It validates denied authority and `false_accepts = 0` before emitting one
authorization receipt.

The publisher accepts only the exact authorization bytes, writes one immutable
`R8B_RECEIPT_V2.json` through temp, fsync, no-clobber publication and directory
fsync, and returns a publication receipt. It cannot reinterpret evidence.

## 10. Exact Denominators

```text
Confirm canonical byte fixtures                   observed exact count
Development known-answer roots                    3 / 3
mode and legacy negatives                         20 / 20
immutable publication boundaries                  72 / 72
process restart states                             7 / 7
linked Development route                           1 / 1
oracle cases                                      16 / 16
frozen control scopes                               4 / 4
legacy static regression rows                      32 / 32
V3 static regression rows                           4 / 4
V4 static regression rows                          16 / 16
cleanup transaction                                1 / 1
cleanup interruption negative                      1 / 1
Development result publication                     1 / 1
linked manifest identities                        26 / 26
suite producer identities                           5 / 5
fresh R7K control-case processes                  12 / 12
production survival                                1 / 1
aggregate publication faults                       2 / 2
false accepts                                           0
sealed attempts                                         0
production mutations                                    0
```

No denominator is summed into another. Historical R7J/R7K receipts are donor
and regression evidence, not execution at the V6 implementation commit.

## 11. Resources

The linked child route from C01 through C22 runs alone in a fresh delegated
cgroup after compilation:

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
```

The zero-network denominator applies to the child route and all its descendants.
M25 and M26 run afterward with separate process outcomes, each under 60 seconds,
under 512 MiB and with no network calls. Measurement failure is indeterminate,
never PASS.

## 12. Production Survival

Read-only pre/post snapshots compare only preregistered stable fields: service
unit bytes, executable bytes, MainPID, restart count, active state, registry
revision, authority packages, local-accept mode and revocation state. Ordinary
traffic counters, token counters, uptime and lease age are expected to change
and are reported separately.

Any service restart, executable replacement, authority mutation, dashboard
change, K1 mutation, phase-memory mutation or production write is VETO.

M24 parent owns this observation event and performs only direct read-only
service/proc inspection and bounded health GETs. Those GETs are counted as
production-observation traffic and are not hidden inside the child
zero-network denominator. No shell-generated receipt is accepted.

## 13. Implementation Scope

The V6 implementation begins from the final paper commit, not from a dirty
cherry-pick. Its exact source scope is:

```text
6 modified predecessor paths
  the five V5 paths
  + confirm_sandbox.rs transport-only Oracle/aggregate roles

16 new paths
  the fifteen V5 paths
  + k2_self_formed_uncertainty_confirm_r8b_restart_v1.rs

22 total implementation paths
```

The dedicated restart integration test owns P01-P07 and actual process tracing.
The module-private Development tests retain known-answer, publication and
filesystem controls only. Exact paths, baseline hashes and line budgets are
frozen by the post-critique implementation preflight.

Control evaluator, terminal evaluator, oracle evaluator, oracle model, cleanup
authorizer, cleanup owner, cleanup verifier, fresh control-case producer and
Development result-publisher decision logic remain exact predecessor bytes.

## 14. Required Gates

```text
adversarial critique
-> repaired V6 contract
-> owner-bounded structural routes
-> design code-route gate with explicit process nodes
-> implementation preflight over exact source bytes
-> READY_TO_IMPLEMENT
-> implementation only
-> postimplementation observed-source code-route gate
-> source-scope parity
-> remote build and execution
```

A structural PASS has `authority_ready=false` and cannot authorize code by
itself. `READY_TO_IMPLEMENT` authorizes only the scoped cold non-sealed
implementation. It grants no execution, scientific, deployment or runtime
authority.

## 15. Successor Boundary

Only a real `R8B_FROZEN` result commit may unlock a separate R9B freeze paper.
R10B remains an exact-root authorization stop. R11B alone may own one sealed
scientific attempt.
