# K2 Self-Formed Uncertainty V5 R8B Contract V8

Status: `REPAIRED AFTER LIVE PROBE / PAPER ONLY / NO EXECUTION AUTHORITY`

Date: `2026-08-21`

V8 supersedes V7 implementation readiness. It preserves the V6/V7 scientific
claim boundary and the nineteen evidence kinds. It repairs the launch,
process-ledger, producer-request, typed-validation, aggregate-root and resource
contracts exposed by review of the partial V7 implementation.

No V7 paper receipt or partial implementation byte is erased. The dirty
23-path implementation remains a measured donor only until a V8 preflight is
`READY_TO_IMPLEMENT`.

## 1. Claim Boundary

R8B remains one cold, non-sealed `DevelopmentRehearsal` route. A successful
R8B result may establish only that the complete R7K Development route was
linked, measured, cleaned and independently authorized under this contract.

It does not establish K2 capability, natural-traffic validity, grokking,
runtime admission, deployment readiness or a `LawCertificate`.

```text
sealed attempts             0
authorization slots         0
production mutations        0
child external network      0
false accepts               0
```

The following actions remain forbidden during paper, implementation and
ordinary tests:

```text
M24-M26 execution
P09 post-authorization execution
sealed attempt
private-truth content inspection by M24 or M25
deployment
dashboard mutation
production mutation
push
```

## 2. Frozen Identities And Inputs

The linked executable manifest remains exactly M01-M26. The suite producer
manifest remains exactly S01-S05. Manifest membership, Nando invocation count,
tool-process count and physical descendant count are separate denominators.

The V8 tool-dependency manifest binds these local executables by canonical
path, mode, byte length and SHA-256:

```text
/usr/bin/systemd-run
/usr/bin/systemctl
/usr/lib/systemd/systemd
/usr/bin/strace
/usr/bin/bwrap
/usr/bin/prlimit
/usr/lib/cargo/bin/sudo
/usr/lib/cargo/bin/coreutils/sha256sum
```

The machine snapshot is systemd `259.5-0ubuntu3.4`, sudo-rs
`0.2.13-0ubuntu1` and uutils coreutils `0.8.0`. A changed binary, path, mode,
version family or hash is pre-execution VETO. Diagnostic and observation tools
are not added to either Nando executable manifest and cannot produce evidence.
The two privileged tools are authorized only for the exact read-only
user-manager image probe in Section 5.

The partial implementation already has a direct `libc 0.2` edge. V8 does not
claim to add it. The only new direct dependency relative to that dirty donor is
`rustix 1.1.4`; its resolved lock additions are `rustix 1.1.4` and
`linux-raw-sys 0.12.1`. Rustix owns inheritable-fd and signal syscalls. The
restart suite must not invoke `/bin/kill` as an unbound tool.

Every positive producer request binds these inputs before its first write:

```text
Development seed artifact
R7K fixture root
linked executable manifest
suite executable manifest
open ledger path and route ID
exclusive output directory
producer executable and exact test selector
```

Each file binding includes canonical path, object kind, Unix mode, byte length,
content SHA-256 and typed semantic root when applicable. Each directory binding
includes canonical path, mode and deterministic tree root. Environment
variables may transport a request-bound value only; they are never authority.
An absent, extra or unequal environment value is rejected before mutation.

## 3. Bounded Invocation Cardinality

The frozen program permits `8 <= R <= 1792` representatives per case. For one
M10 case:

```text
t(R) = ceil(max(R - 8, 0) / 7) + 1
M03-M09 invocations = 6 + 4 * t(R)
```

At `R = 1792`, `t(R) = 256` and one case has at most 1,030 M03-M09
invocations. The exact worst-case pre-authorization Nando denominator is:

```text
five suite producers                                  5
M24 delegated child                                   1
M24-child direct invocations                        151
M01 nested M02                                        1
M10 descendants: 16 * 1,030                      16,480
suite-owned Nando children: 16 + 6 + 6 + 2           30
                                                   ------
maximum Nando invocations                         16,668
maximum ledger events: 2 per invocation           33,336
```

The manager-image observation adds exactly two `sudo` frontends and two
privileged `sha256sum` descendants, one pair before submission and one pair
after child termination. They are physical observation-tool processes, not
Nando invocations or ledger events, and do not change either denominator
above. Their complete outputs and process observations are bound into the
resource receipt.

The 151 M24-child direct invocations are frozen as:

```text
M01 + M10                                              2
M11/M12 over 24 planned probes                        48
M13/M14 over 24 executions                            48
M15 over 16 cases                                     16
M16 over 16 cases                                     16
M19 fresh controls                                    12
M17 control evaluators                                 4
M18 terminal                                           1
M20-M23 cleanup                                        4
```

S01 has no Nando child. S02, S03, S04 and S05 own exactly 16, 6, 6 and 2
total Nando invocations. S02 is partitioned further:

```text
S02 request owner -> ten M01 owner invocations
S02 request owner -> three direct setup M02 invocations
three M01 owners  -> one nested M02 invocation each
```

The producer request binds total and per-writer counts plus the outcome policy
of each invocation. A direct setup M02 uses the S02 writer partition; an M02
dispatched inside M01 uses that exact M01 invocation as request owner.

The M24-child request does not embed 16,668 rows. It binds the formula version,
sixteen case IDs, per-case limits and the semantic root of a deterministic
schedule grammar. M25 reconstructs the concrete schedule from role-specific
M04 facts and the ledger. Any count outside the formula is VETO.

## 4. Acyclic Chronology

The positive route is:

```text
P00 validate source inventory, input bindings and executable/tool manifests
-> P01 launch S01-S05 producers and close their receipt channels
-> P02 read-only pre-production snapshot
-> P03A bind user-manager identity and privileged live-image hash
-> P03B freeze delegated launch request and submit one transient user service

child C00 validate systemd credential and producer request
-> C01 Development owner
-> C02 generator
-> C03-C05 durable Development split and metadata
-> C06-C07 public coordinator and public descendants
-> C08 downstream invocation contract
-> C09-C14 private, oracle, controls and terminal
-> C15-C20 real cleanup and Development completion
-> C21 child candidate frozen
-> C22 child exits

parent P04A observe terminal unit state and freeze cgroup/process resources
-> P04B repeat and compare user-manager identity and privileged live-image hash
-> P05 post-production snapshot and survival
-> P06 close invocation ledger and freeze packet directory
-> P07 M25 authorizes only the closed P06 packet
-> P08 M26 publishes exact M25 bytes
-> P09 optional post-authorization diagnostic audit
```

M25 consumes only P06. P09 is never an input to P07 or P08. No outcome can
authorize its own invocation or a later process.

## 5. Delegated M24 Child Launch

The current cgroup cannot safely delegate memory control. M24 therefore does
not create a child cgroup through direct cgroupfs writes and does not claim to
fork the M24 child.

Before invoking `systemd-run`, M24 freezes a durable delegated-launch request
that binds:

```text
route ID and unique unit name derived from it
M24-child executable path, mode, length and SHA-256
exact libtest selector and argv
producer-request credential path, bytes and SHA-256
exclusive stdout, stderr and output paths
exact systemd-run argv and property set
expected user-manager process and executable identity
20-minute parent deadline
```

The physical ownership chain is recorded exactly as:

```text
M24 request owner
-> /usr/bin/systemd-run submission tool
-> user systemd manager launch owner
-> M24 child service main process
```

V8 never rewrites that chain as `M24 directly spawned M24 child`.

User-manager identity has two independent observation channels. The
unprivileged channel binds the authenticated user-bus peer PID and unique bus
name, a pidfd for that peer, boot ID, `/proc/<pid>/stat` start time, command
line, UID, cgroup, manager-reported version and the system-manager-owned
`user@<uid>.service` `InvocationID`, `MainPID`, `ExecStart`, fragment and
control group. Every PID and ownership projection must identify the same
process and `/usr/lib/systemd/systemd --user` route.

The current user cannot dereference `/proc/<pid>/exe`; version and path claims
therefore cannot substitute for live image bytes. Immediately before
submission and again after child termination, M24 runs exactly:

```text
/usr/lib/cargo/bin/sudo
  --non-interactive
  --user=root
  --
  /usr/lib/cargo/bin/coreutils/sha256sum
  --binary
  --zero
  /proc/<validated-decimal-manager-pid>/exe
```

There is no shell, PATH lookup, environment-derived argument, alternate target
or other privileged command. The PID path is constructed only after all
unprivileged identity projections agree. Each probe must exit zero, emit empty
stderr and emit exactly one NUL-terminated GNU-format SHA-256 row for that
exact proc path. The pre-hash, post-hash and pinned on-disk systemd SHA-256 must
be equal. The two user-bus identities, pidfd liveness, PID/start time, unit
identity, version and command line must also remain equal. The resource receipt
retains both normalized argv arrays, tool identities, statuses, raw-output
hashes, parsed live-image hashes and monotonic observation bounds.

An unavailable noninteractive privilege route, policy prompt, extra output,
tool drift, PID disagreement, pidfd exit, manager re-exec/reconnect signal or
hash inequality is pre-P06 VETO. The probe grants no general root authority and
is never an R8B evidence producer. This direct live-byte check is required
because an already-running manager may predate an on-disk package replacement.

The transient user service has a fresh route-derived name and exactly these
resource/security properties:

```text
Type=exec
MemoryMax=536870912
MemorySwapMax=0
TasksMax=256
RuntimeMaxSec=1200
KillMode=control-group
PrivateNetwork=yes
RestrictAddressFamilies=AF_UNIX
RemainAfterExit=yes
```

The normalized submission command contains exactly `--user`,
`--no-ask-password`, `--expand-environment=no`, the route-derived `--unit`,
`--service-type=exec`, `--remain-after-exit`, the property set above,
`LoadCredential=r8b-producer-request:<bound-path>`, and bound
`StandardOutput=file:<bound-path>` / `StandardError=file:<bound-path>`
properties followed by the exact executable and libtest argv. It contains no
`--pipe`, `--wait`, `--collect`, shell, PTY or environment-derived argv.

The credential source is regular `0400`, `nlink=1` inside a frozen `0500`
test-owned directory. The child reads the fixed credential basename once,
validates exact canonical bytes and current executable identity, then starts
C01. Service stdout/stderr are diagnostics files, never receipt bytes.

M24 polls only the exact unit and binds `InvocationID`, `MainPID`,
`ExecMainCode`, `ExecMainStatus`, `ActiveState`, `SubState`, `MemoryPeak`,
`MemorySwapPeak`, `OOMPolicy`, `OOMKills`, `TasksCurrent` and monotonic
timestamps. After C22, M24 freezes the resource receipt while the unit remains
loaded. It then runs `systemctl --user stop` on only that exact unit. It never
stops a slice, user manager, service collection or production unit.

The stop command must exit successfully and the same unit must become inactive
or disappear only after its prior `InvocationID` and terminal metrics were
frozen. Its exact cgroup must contain no descendant. Unknown stop status,
continued activity or residue makes P06 ineligible.

Timeout, unit-name collision, property drift, missing credential, premature
unit unload, unknown metrics, nonzero swap, OOM kill or an unexplained process
leaves the route indeterminate and grants no retry authority.

## 6. Streaming Process Ledger

C08 remains the immutable expected downstream invocation contract frozen before
C09. `process-ledger.json` is observed invocation provenance. The ledger cannot
define its own expected cardinality. M25 compares three disjoint projections:

```text
S01-S05 and M24-child requests
  -> their exact producer invocation plans

M10 descendants
  -> schedule grammar + exact case IDs + typed M04 RepresentativeCount facts

C09-C20 downstream only
  -> frozen C08 expected invocation contract
```

Each expected projection must equal its observed ledger projection exactly.
C08 never absorbs P01, M01/M02 or M03-M10 authority. Neither C08 nor the ledger
is evidence kind 20.

The open journal is append-only canonical JSON Lines. Its open file and lock
live in a request-bound sibling staging directory on the same device as P06,
never inside the packet member set. It contains one header, up to 33,336 event
lines and one terminal seal. Before every append the writer
opens the canonical ledger and lock with no-follow semantics, takes the
exclusive ledger lock, validates the immutable header and complete terminal
tail against the expected prior root, appends exactly one bounded canonical
line, fsyncs the file and directory, then unlocks. A stale tail or foreign lock
object is rejected before write.

P06 first proves that every requested invocation has one allowed completion and
that no writer remains alive. It then takes the same lock, validates the
complete natural prefix, appends the terminal seal, fsyncs, renames without
replacement to `process-ledger.json`, chmods it to `0400`, fsyncs the directory
and never rewrites it. The lock and staging directory remain retained
diagnostics outside P06 and cannot satisfy a packet descriptor.

Frozen limits are:

```text
general protocol or receipt object     <=   1,048,576 bytes
captured stdout                         <=   1,048,576 bytes
captured stderr                         <=      65,536 bytes
one ledger event line                   <=       4,096 bytes
one invocation event pair               <=       7,168 bytes
ledger event count                      <=      33,336
complete process-ledger.json            <= 134,217,728 bytes
```

Writers validate limits before append. M25 validates the ledger as a stream;
it must not deserialize the complete file into one in-memory protocol object.

Each logical Nando invocation has exactly one `InvocationRequested` event and,
on any observed terminal outcome, exactly one completion event. An incomplete
natural suffix is retained but makes P06 ineligible.

Completion kinds are:

```text
AuthoritySuccess
DiagnosticExpectedFailure
UnexpectedFailure
LaunchFailure
```

Only `AuthoritySuccess` may contain a validated typed output or authority
output descriptor. `DiagnosticExpectedFailure` is legal only when the producer
request preregisters its exact exit predicate; it records actual exit,
stdout/stderr length and hashes and has no P06 authority descriptor.
`UnexpectedFailure`, `LaunchFailure` and an incomplete pair make the route
indeterminate. After the first such state no later invocation may be requested
or appended. This fail-stop rule is part of the ledger byte bound.

Every request event binds:

```text
sequence, prior event root and route ID
request-owner role and executable SHA-256
target Nando role and executable SHA-256
launch kind: direct, strace-mediated, bwrap-prlimit-mediated or user-systemd
ordered tool chain with exact tool hashes
stage, case ID and probe ordinal
request semantic root and stdin/credential SHA-256
expected outcome policy and role-specific validator ID
```

Producer-request validation occurs before the first event and freezes these
serialization limits:

```text
relative or absolute path field        <= 240 UTF-8 bytes
schema or validator identifier         <= 128 ASCII bytes
role, stage, launch and fact tags       = fixed enums
cryptographic root                     = exactly 64 lowercase hex bytes
validated facts                        = closed role-specific enum
authority-output descriptors           <= 4
```

An oversized or unknown field rejects the request before a durable start.

Every completion repeats the immutable invocation identity and binds actual
status, diagnostics, monotonic values and physical observations. A successful
completion additionally binds one role-specific validated-output record:

```text
stdout byte length and SHA-256
receipt schema and semantic root
validator ID and validator executable SHA-256
closed role-specific facts required for schedule reconstruction
zero to four immutable authority-output descriptors
```

Validated facts are a closed enum keyed by validator ID, not a free-form map.
Only the exact M04 decoder may emit `RepresentativeCount`, and it does so only
after concrete-type validation plus canonical decode/reserialize equality.
M25 uses that writer-attested fact to reconstruct process provenance. It does
not report the fact as an independent semantic revalidation of ephemeral M04
stdout. M25 independently reopens and validates every retained P06 object.

An authority-output descriptor binds final packet relative path, closed
`object_role`, optional evidence kind, receipt schema, required denominator or
exact known-answer/schedule root, byte length, mode, content SHA-256 and
semantic root. Evidence roles require exactly one evidence kind. The C08 role
is `downstream_invocation_contract` and requires no evidence kind. A descriptor
path is already its final P06 path; P06 may copy bytes but may not relabel or
move it.

## 7. Writer Partitions And Tool-Mediated Ownership

One invocation request has one request owner. Physical launch ownership is
recorded separately and may be mediated by a bound tool.

```text
M24 root       -> S01-S05 producer requests and delegated M24-child request
S02-S05       -> their exact direct Nando invocation plans
M24 child      -> M01, M10-M23 direct logical requests
M01            -> M02 only
M10            -> M03-M09 only
```

S02 records `S02 -> strace -> M01`, not `S02 directly forked M01`. M24 and M10
sandbox routes record `request owner -> bwrap -> prlimit -> Nando target`.
The user-service route records the chain in Section 5. Bound tools have no
receipt, decision or writer authority.

The ledger appender rejects a foreign writer, target role, tool chain, route,
stage, case, probe, request root, stale prefix or schedule position before
launch. The authorizer reconstructs all five writer partitions and rejects
missing, extra, duplicate or cross-partition invocations.

## 8. Producer Request And Typed Validators

Each S01-S05 request and the M24-child request binds a closed expected-output
table. One row contains:

```text
final relative path
closed object role and optional evidence kind
exact receipt schema
required denominator or known-answer root set
producer role and executable hash
role-specific validator ID
```

Evidence output rows require exactly one of the nineteen evidence kinds. The
C08 row requires object role `downstream_invocation_contract`, its exact schema
and no evidence kind. No generic or untyped object role is allowed.

Paths alone are not an output contract. Generic recursive extraction of a
`schema` field and any field ending in `_root_sha256` is forbidden.

Every validator decodes one concrete Rust type, invokes that type's `validate`,
checks its exact evidence kind, denominator, source roots and denied-authority
projection, reserializes canonical bytes and requires byte equality. A schema
or root from the wrong role is VETO even when its JSON shape is otherwise
valid.

The suite producer owns its aggregate PASS. Its child invocations may be
successful or preregistered diagnostic failures, but cannot themselves satisfy
a P06 suite evidence entry. The producer emits its immutable measured receipt
only after its exact invocation plan and local assertions are complete.

S02 journals its ten direct M01 and three direct setup M02 invocations. It also
passes the request-bound ledger transport and exact invocation ID to every M01.
Each of the three M01 routes that dispatches a generator journals its own nested
M02 request and completion. Any unjournaled direct setup M02 or M01-nested M02
is forbidden.

## 9. Canonical Multi-Receipt Channel

S01-S05 and the M24 child write only into their exclusive request-bound output
directories. Every expected file is created through create-new, fsync,
no-clobber publication at its final P06 relative path. The producer fsyncs the
directory before exit.

The request owner then requires the exact expected path set and rejects a
missing, extra, symlinked, hard-linked, writable, moved or non-canonical file.
Each evidence file is regular `0400` with `nlink=1`; the closed directory is
`0500`. Stdout/stderr remain diagnostics and cannot substitute for this
channel.

The suite-owned evidence remains:

```text
S01 Confirm canonical bytes
S01 Development known answers                         3/3
S01 immutable publication boundaries                72/72
S02 process restart                                  7/7
S03 mode matrix                                     20/20
S04 cleanup interruption                             1/1
S05 aggregate publication faults                     2/2
```

The M24 child emits exactly:

```text
frozen C08 downstream invocation contract
linked-route measured receipt
Oracle batch
four-scope control census
```

Positive M22 cleanup and M23 Development result receipts remain their actual
linked-route outputs, not suite summaries.

## 10. Exact M16 And M17 Root Equality

For every `AuthoritySuccess` M16 completion, the ledger retains both the
completion event root and the validated M16 receipt semantic root. The Oracle
batch contains two sorted unique sets:

```text
sixteen M16 completion-event roots
sixteen M16 receipt semantic roots
```

M24 child constructs the batch from those exact sets. M25 independently
reconstructs both sets from the streamed ledger and requires set equality, not
subset membership, cardinality alone or roots supplied only by the batch.

The four-scope census has the same dual-set contract for exactly four M17
completions and four M17 receipt roots. Its evidence kind remains
`FrozenControlScopes`; its schema is the derived measured-receipt schema. It is
coverage evidence, not a fifth control result.

Any duplicate, missing, extra, relabelled, non-M16/M17 or writer-partition
mismatched root is VETO.

## 11. Private Truth, Cleanup And Resources

The V6 descriptor boundary remains exact. Resolver, final-truth and Oracle
private files are opened with `O_PATH | O_NOFOLLOW`, validated by descriptor
metadata and mounted from the inherited descriptor path. M24 and M25 never
read, hash, decode or serialize private contents.

M20-M23 remain separate cleanup authorization, mutation, verification and
result owners. Cleanup remains intent-first and deletes only registry-classified
test-owned private paths. No V8 ledger or packet file is cleanup authority.

The transient M24-child service is the resource denominator. It includes all
of its bwrap, prlimit and Nando descendants and excludes S01-S05, M24 parent,
M25, M26 and P09.

```text
MemoryPeak                 <= 512 MiB
MemorySwapPeak              = 0
OOMKills                    = 0
complete child route       <= 20 min
each sandbox               <= 60 s
external network calls      = 0
```

Private network isolation and `AF_UNIX` restriction are enforcement controls;
the zero-network result is still reported separately from service-resource
metrics.

## 12. Closed P06 Packet Directory

P06 is a closed immutable directory, not one giant protocol value. It contains:

```text
19 evidence objects, including linked and suite manifests
 1 frozen C08 downstream invocation contract
 1 parent resource receipt
 1 process-ledger.json
 1 packet-manifest.json
--
23 exact files
```

The evidence enum remains exactly nineteen kinds. C08 is expected downstream
cardinality authority; `process-ledger.json` is observed provenance; the
resource receipt has its own denominator. None becomes evidence kind 20.

`packet-manifest.json` is below the general 1 MiB limit. It binds every member
by final relative path, object role, evidence kind when applicable, byte length,
Unix mode, content SHA-256 and typed semantic root. Its C08 descriptor binds
the expected schedule grammar and downstream counts; its resource descriptor
binds the exact transient unit and service denominator; its ledger descriptor
binds event count and final event-chain root.

All members are regular `0400`, `nlink=1`; all directories are `0500`; path
sets are exact; symlinks and hard links are forbidden. The packet manifest is
sealed last and the packet root is fsynced before P07.

Parent-owned evidence remains limited to linked manifest, suite manifest and
production survival. The parent separately owns the non-evidence resource
receipt and packet assembly. C08 and every other child-owned packet object must
match exactly one authority-output descriptor from its declared producer
completion. One descriptor cannot satisfy two entries.

## 13. M25 Streaming Authorization

M25 receives only the exact P06 packet path and expected packet-manifest root.
It opens every component with no-follow semantics and performs, in order:

```text
1. packet path, mode, link and exact-member census
2. manifest byte/root validation
3. streaming ledger syntax, limits, event chain and terminal seal
4. C08 expected schedule and writer-partition reconstruction
5. expected diagnostic-failure and successful-output validation
6. nineteen-kind exact census and role-specific typed validation
7. descriptor-to-entry bijection
8. exact M16 and M17 dual-root set equality
9. resource, production-survival, cleanup and denied-authority checks
10. authorization receipt emission
```

M25 does not read private-truth content, execute a child, repair the packet,
infer a missing event or trust a producer exit code as evidence. Any failed
step emits no positive authorization. M25 independently validates all 23
retained packet files. It treats non-retained internal output facts as
writer-attested provenance with role-specific parity, not independent semantic
truth.

M26 publishes exact M25 authorization bytes and a concrete publication
receipt. P09, if separately authorized later, may append diagnostics only and
cannot change M25 or M26 bytes or dispositions.

## 14. Exact Source Scope And Spectral Split

The dirty 23-path donor is pinned by current worktree bytes, modes and hashes
before V8 code begins. Its porcelain status and index blob IDs are recorded
separately without staging, unstaging or changing the index. V8 may edit only
that measured donor scope plus these eight ownership modules:

```text
crates/nando-operator-learning/src/k2_goal_environment/learned_composition/self_formed_uncertainty/r8b_process_model.rs
  producer requests, invocation events, ledger and packet descriptors

crates/nando-operator-learning/src/k2_goal_environment/learned_composition/self_formed_uncertainty/r8b_process_ledger.rs
  bounded JSONL append, fsync, natural-prefix and terminal-seal logic

crates/nando-operator-learning/src/k2_goal_environment/learned_composition/self_formed_uncertainty/r8b_process_authorizer.rs
  streaming chain, writer-partition and schedule reconstruction

crates/nando-operator-learning/tests/k2_self_formed_uncertainty_confirm_r8b_support/process_runtime.rs
  tool-bound direct, sandbox and user-systemd launch adapters

crates/nando-operator-learning/tests/k2_self_formed_uncertainty_confirm_r8b_linked_v1/parent.rs
  P00-P09 parent route; the existing linked file retains child-route ownership

crates/nando-operator-learning/tests/k2_self_formed_uncertainty_confirm_r8b_linked_v1/parent/preproduction.rs
  P00-P02 source, producer-channel and pre-production snapshot capabilities

crates/nando-operator-learning/tests/k2_self_formed_uncertainty_confirm_r8b_linked_v1/parent/resource.rs
  P04 loaded-unit, stop, residue and post-observation capabilities

crates/nando-operator-learning/tests/k2_self_formed_uncertainty_confirm_r8b_linked_v1/parent/postproduction.rs
  P06 closed-packet custody and P09 diagnostic-process capabilities
```

The eight-module scope buys three measured properties: bounded streaming
memory, truthful launch ownership and separation of proof logic from test
adapters. It does not authorize another decision owner or broader product
scope.

Frozen budgets are:

```text
immutable_publication.rs       <= 700 lines  generic immutable files only
r8b_model.rs                   <= 650 lines  non-process R8B evidence models
r8b_process_model.rs           <= 850 lines  process and packet schemas
r8b_process_ledger.rs          <= 700 lines  journal persistence
r8b_authorizer.rs              <= 450 lines  nineteen-kind authorization
r8b_process_authorizer.rs      <= 750 lines  streamed provenance proof
r8b_support/mod.rs             <= 700 lines  thin shared router
r8b_support/process_runtime.rs <= 900 lines  process adapters
r8b_linked_v1.rs              <= 1200 lines  child route
r8b_linked_v1/parent.rs       <= 2200 lines  parent route
r8b_linked_v1/parent/preproduction.rs <= 900 lines  P00-P02 capabilities
r8b_linked_v1/parent/resource.rs      <= 1200 lines  P04 resource capabilities
r8b_linked_v1/parent/postproduction.rs <= 1300 lines  P06/P09 capabilities
```

Oracle, control, terminal, cleanup and Development result decision logic stays
byte-identical to the measured donor. Move-only extraction and V8 behavior
changes must be separate commits and separate test checkpoints.

## 15. Mandatory Negative And Parity Tests

V8 requires all V6/V7 tests plus:

```text
N17 direct-cgroupfs or false direct-M24 child ownership rejected
N18 systemd unit collision, tool substitution or property drift rejected
N19 missing, changed or extra producer input binding rejected before write
N20 request path matches but evidence kind/schema/denominator differs rejected
N21 child-role or expected-outcome plan substitution rejected
N22 expected-failure child with authority descriptor rejected
N23 successful child decoded only by generic root extraction rejected
N24 S02 setup M02 without S02 pair or nested M02 without its M01 pair rejected
N25 strace, bwrap or prlimit chain omitted or reordered rejected
N26 incomplete, oversized or over-cardinality ledger rejected
N27 whole-ledger in-memory protocol path rejected by bounded-memory test
N28 packet manifest embeds ledger or creates evidence kind 20 rejected
N29 M16 batch subset, superset, duplicate or receipt/event root swap rejected
N30 M17 census subset, superset, duplicate or receipt/event root swap rejected
N31 writer-partition crossing or schedule count drift rejected
N32 unbound environment seed, fixture, manifest or ledger input rejected
N33 resource metrics read after unit disappearance rejected
N34 stop operation targeting anything except exact test unit rejected
N35 rustix signal/fd path parity and external `/bin/kill` regression rejected
N36 C08 omitted, relabelled, self-derived from ledger or unequal to ledger rejected
N37 running user-manager PID/start/image changes across delegated route rejected
N38 `--pipe`, `--wait`, `--collect`, shell, PTY or credential substitution rejected
N39 P06 staging lock, open journal or cross-device ledger freeze rejected
N40 oversized path/schema/fact request rejected before first ledger event
N41 C08 compared outside the C09-C20 projection rejected
N42 version/path/unit-only manager identity without both live-image probes rejected
N43 privileged probe command drift, prompt, extra output or unequal hash rejected
```

Positive parity includes:

```text
frozen predecessor scientific payload bytes and exact denominators unchanged
V8 wrapper/process/packet schemas versioned and semantic projections checked
Confirm and Development mode parity unchanged
direct, strace, bwrap-prlimit and systemd launch-route identity parity
pre/post privileged live-image probe and unprivileged manager-identity parity
role-specific decoder and producer-request expected-output parity
streamed ledger root equals reference in-memory root on bounded fixtures
M16/M17 event-root and receipt-root exact-set parity
zero production and private-truth authority
non-R8B transient-unit capability and normalized-command parity before execution
```

## 16. Gates And Execution Boundary

```text
V7 readiness revocation
-> V8 contract
-> adversarial critique V1
-> repaired V8
-> critique V2 if material findings remain
-> structural worksheets
-> V8 design code-route gate
-> exact dirty-donor implementation preflight
-> READY_TO_IMPLEMENT
-> stop and switch max -> high
-> scoped implementation
-> observed-source route gate
-> source-scope and budget parity
-> build and non-attempt tests
-> separate explicit R8B execution authorization
```

`READY_TO_IMPLEMENT` authorizes only edits in the frozen V8 scope. It does not
authorize an R8B suite, M24-M26, P09, deployment, dashboard change, push or any
scientific claim.
