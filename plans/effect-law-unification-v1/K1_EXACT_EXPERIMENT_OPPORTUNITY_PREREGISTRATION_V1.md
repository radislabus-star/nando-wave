# K1 Exact Experiment Opportunity V1 Preregistration

Status: paper contract. Implementation and deployment remain blocked until the
structural, code-route, and implementation-preflight receipts all pass.

Date: 2026-08-13.

## Decision In One Tree

```text
Natural traffic
-> source-neutral motif catalog
-> ordinary K1 ranking
-> exact identifier input reconstruction
-> OpportunityRoot
   |-- unseen deterministic opportunity
   |     -> immutable Freeze V8
   |     -> identifier
   |     -> rooted TerminalDiagnosticV1
   |     -> terminal result or independent-future route
   |
   |-- exact deterministic opportunity already terminal
   |     -> WAITING_FOR_NOVEL_EVIDENCE
   |     -> no generation
   |     -> no ledger event
   |     -> no identifier work
   |
   `-- unseen opportunity but research budget closed
         -> RESEARCH_BUDGET_COOLDOWN
         -> no generation
         -> no scientific verdict
```

The scheduler remains operator-blind. It selects a natural evidence domain; it
does not name `FILTER`, `COUNT`, `BRANCH`, a renderer, or any other program.

## Objective

Stop paying repeatedly for the same deterministic identifier experiment while
preserving every genuinely new natural L1 opportunity.

The acceptance claim is deliberately narrow:

```text
same causal identifier input
-> same OpportunityRoot
-> no repeated completed deterministic experiment
```

## Non-Goals

This change does not:

- manufacture natural traffic;
- infer a law from a consequence type;
- suppress an entire scalar, collection, boolean, record, or renderer group;
- add opcodes, templates, teacher labels, or family mappings;
- issue a LawCertificate;
- open K1 or Natural L2;
- change phase memory;
- activate a package;
- claim answer quality or Wave causality.

## Current Measured Baseline

The preserved production-copy snapshot is:

```text
/tmp/k1-terminal-failure-quotient-v1-baseline-B2zz2fwF/
  k1-epistemic-scheduler-ledger-v1.json
  k1-epistemic-scheduler-anchor-v1.json

ledger revision       1174
ledger bytes          2,246,130
freezes               586
terminals             585
empty V6 pairs        211
distinct motif roots  175
```

This snapshot is diagnostic and may be stale relative to live production. Old
V6 pairs lack the exact causal manifest required by this contract. Their
authoritative attempted count is therefore zero.

## Five Separate Identities

The implementation must not merge these roots.

### 1. AuthorityBindingManifestV1

Answers: was this selection authorized in the current system state?

It binds:

- scheduler ledger revision and root;
- Epistemic Registry revision and root;
- K1 deficit snapshot root;
- fixture exclusion root;
- catalog and queue roots;
- selected candidate root;
- active protocol-mode set root;
- installed minimum queue/freeze/wire schemas;
- durable evidence-source snapshot roots;
- artifact source checkpoint root;
- authority policy root.

These fields authorize a freeze but do not define causal experiment equality.

### 2. IdentifierCausalInputManifestV1

Answers: what exact immutable information can change the initial identifier
result?

It binds only:

```text
schema
candidate structural root
ordered exact support join roots
ordered exact completed-frame roots
ordered exact topology roots
ordered exact motif roots
ordered exact embedding roots
RelevantIdentifierArtifactProjectionV1 root
candidate generator schema
discovery basis root
active protocol-mode set root
IdentifierResourceLimitsV1 root
```

The manifest is valid only if all lists are canonical, non-empty where required,
within frozen limits, and independently reconstructable by the authority.

For V8, every identifier-scoped domain input that currently receives the full
`freeze_root_sha256` must instead receive `OpportunityRoot`: both
`FrozenRawPhaseT1ContractV1.frozen_domain_root_sha256` and the frozen
identification evidence-domain root. The complete Freeze V8 root remains
provenance, but it cannot perturb candidate scores, Raw Phase envelopes,
executable blueprint formation, or the causal identifier result merely because
a timestamp, queue root, or generation number changed.

### 3. OpportunityRoot

Answers: is this the same deterministic initial identifier experiment?

```text
OpportunityRoot = SHA256(canonical IdentifierCausalInputManifestV1)
```

It explicitly excludes:

- generation sequence;
- selected or terminal timestamps;
- queue and catalog roots;
- K1 score and token opportunity;
- bounded economic cost score;
- registry revision except through the active protocol-mode set actually used
  by the identifier;
- future minimum sequence and deadline;
- irrelevant candidate artifacts;
- unread overflow rows;
- dashboard fields;
- coarse semantic or consequence families.

Changing an excluded field must preserve the OpportunityRoot. Changing any
causal field must change it.

### 4. IdentifierResultRoot

Answers: what exactly happened when the identifier consumed that opportunity?

It binds the exact seed set, program dispositions, accepted set, semantic class
set, identifier state, and stable blocker. It excludes candidate freeze root,
generation, timestamps, and other receipt metadata. It never participates in
pre-experiment ranking.

### 5. TerminalDiagnosticRoot

Answers: which authorized generation produced this causal result?

It binds `OpportunityRoot`, `IdentifierResultRoot`, the candidate Freeze V8
root, support/archive roots, and diagnostic disposition. This is the signed
provenance envelope. Two hypothetical receipts for the same causal experiment
may have different diagnostic roots, but they must have the same
`OpportunityRoot` and `IdentifierResultRoot`; the attempt policy prevents the
second receipt from being created.

## Exact Support Selection

The support manifest is built before freeze from the same bounded source-neutral
motif route used by the identifier.

For every support row it commits:

```text
join root
completed frame root
topology commitment root
capture generation root
motif root
ordered embedding roots
capture sequence
session lineage root
```

Ordering is canonical by `(capture_sequence, join_root, motif_root)`. Duplicate
rows are rejected. Support rows above the selected support watermark are not
read and do not enter the manifest. Overflow counts may be reported in the
authority manifest, but unread overflow cannot perturb OpportunityRoot.

### Evidence source snapshot

Ranking freezes an `EvidenceSourceSnapshotV1` before causal manifests are built:

```text
topology archive consumed prefix count and root
frame archive consumed prefix count and root
join-builder schema
motif archive/config roots
collection checkpoint byte root
active protocol-mode set root
```

The cold learner builds the catalog and queue from exactly those prefixes. The
authority independently replays the same bounded prefixes through the existing
pure join, motif, catalog, and queue builders. A changed prefix or checkpoint
returns `STALE_BEFORE_FREEZE`; a client-provided catalog cannot omit a higher
ranked candidate and still pass authority reconstruction.

## Relevant Candidate Artifact Projection

Collection artifacts are causal only when the identifier can consume them.
The full mutable online collection checkpoint is not a causal identity.

`RelevantIdentifierArtifactProjectionV1` is derived by one shared pure builder:

```text
validated durable collection checkpoint
+ exact frozen support identities
-> matching (turn_intent, session) artifacts only
-> ComposeCollection programs only
-> exact canonical program roots and bytes
-> matching predicted typed consequence roots
-> canonical relevant projection root
```

The projection excludes unrelated sessions, unrelated artifact fields, and
non-consumed program kinds. Adding an unrelated artifact may wake the scheduler
but must not change an existing candidate's OpportunityRoot.

Missing relevant artifacts are represented by a valid canonical empty
projection that binds builder schema plus the exact requested support
identities. It is not represented by an absent optional field. An unrelated
checkpoint append preserves that empty projection; the first relevant artifact
changes it.

### Authority ownership

The cold learner may propose a projection root. It cannot authorize it.

The certification authority must:

1. read the durable online collection checkpoint through the existing validated
   read-only checkpoint decoder;
2. independently restore exact support identities from configured durable
   topology and frame archives;
3. call the same pure relevant-projection builder as the learner;
4. compare exact projection and OpportunityRoot;
5. reject a checkpoint race as `STALE_BEFORE_FREEZE` without appending an event;
6. archive the exact bounded relevant projection content-addressably before
   appending Freeze V8.

The shared builder prevents a second identifier preparation language. The two
processes still own independent source restoration and must produce byte-equal
manifests.

### Immutable artifact archive

Freeze V8 binds an append-only manifest under the authority root:

```text
k1-identifier-artifact-archive-v1/
  objects/<projection-object-root>.cbor
  manifests/<relevant-projection-root>.cbor
```

Writes use create-new, file `fsync`, atomic publication, and directory `fsync`.
An existing path is accepted only when bytes are identical. An orphan object
created before a failed freeze append is harmless and never grants authority.

An active generation reads artifacts only from this archive, never from the
later mutable checkpoint.

## TerminalDiagnosticV1

Every pre-future identifier terminal must retain enough information to explain
the result without retaining raw private traffic.

```text
TerminalDiagnosticV1
|- opportunity root
|- identifier result root
|- candidate freeze root
|- support manifest root and row count
|- relevant artifact projection root and object count
|- seed program count and canonical root
|- ProgramDispositionV1[]
|  |- program root
|  |- accepted | rejected
|  `- stable rejection reason code
|- accepted motif-bound program count and root
|- rejection reason histogram and root
|- semantic class count and root
|- identifier report root
|- exact result state and blocker
|- terminal disposition
|  |- deterministic_pre_future
|  |- future_contingent
|  `- operational_retryable
`- authority_ready = false
```

Motif binding must evaluate each seed exactly once and record its disposition.
The current `.filter(...is_ok())` path is forbidden because it destroys causal
diagnostics.

Raw prompts, provider payloads, rendered answers, and private values do not
enter this diagnostic. Only already bounded typed objects, counts, stable reason
codes, and SHA-256 roots are retained.

The learner cannot append this receipt through the generic scheduler-event
route. A dedicated terminal-attempt authority request restores Freeze V8,
support and archived artifacts, runs the same pure initial identifier evaluator,
reconstructs `IdentifierResultRoot` and `TerminalDiagnosticV1`, owns the terminal
timestamp, then appends the diagnostic and matching verdict. Generic append
rejects both a V8 `TerminalDiagnosticV1` and a deterministic V8 terminal verdict.
This is independent execution over independent durable source restoration, not
a second identifier implementation.

## ExactAttemptIndexV1

The index is a pure projection of signed, append-only events. It is not a
mutable blacklist file.

An opportunity becomes `attempted_deterministic` only when the ledger contains:

```text
Freeze V8 with OpportunityRoot
+ matching TerminalDiagnosticV1
+ matching terminal verdict
+ disposition == deterministic_pre_future
+ authority-recomputed IdentifierResultRoot
+ exact roots and generation all agree
```

V1 deterministic allow-list:

- `motif_program_candidates_empty`;
- `natural_collection_candidate_artifact_missing`;
- `natural_collection_candidate_generation_empty`;
- `all_supported_t1_protocol_modes_already_active` when the active-mode root is
  bound by the opportunity.

Invalid evidence, archive I/O, authority timeout, signature failure, checkpoint
race, persistence failure, panic, or decode error is operational. It never marks
an OpportunityRoot attempted.

`PROBE_EXHAUSTED`, `INDEPENDENT_FUTURE_NOT_OBSERVED`, and post-freeze
contradictions are future-contingent in V1. They remain visible but do not create
a pre-freeze suppression entry. Extending exact deduplication to those stages
requires a separate preregistration that binds their later causal inputs.

Legacy V1-V7 events have no `IdentifierCausalInputManifestV1`; they contribute
only diagnostic counters and never authoritative attempted roots.

## Queue V4

Queue V4 preserves the existing operator-blind rank order:

```text
safety and leakage veto
-> fixture provenance veto
-> expected K1 gain
-> readiness
-> bounded discovery cost
-> expected verified tokens
-> stable candidate hash
```

It adds exact opportunity observation, not a coarse family rank:

```text
queue root
attempt index root
artifact source snapshot root
per readiness row:
  candidate root
  score
  causal manifest root
  OpportunityRoot
  exact attempt state: unseen | attempted_deterministic
```

Rows remain in their ordinary rank order. Selection scans readiness-PASS rows
and chooses the first `unseen` OpportunityRoot. Attempted rows remain visible
for audit but cannot create another generation.

If no readiness-PASS row exists, state is `WAITING_FOR_EVIDENCE`.
If readiness-PASS rows exist but every exact root was attempted, state is
`WAITING_FOR_NOVEL_EVIDENCE`. These states must never be conflated.

The authority chooses Queue V4 as the installed minimum schema. A client V1-V3
proposal is rejected with `K1_AUTHORITY_SCHEMA_DOWNGRADE`; it cannot select a
weaker derivation path.

## Freeze V8

Freeze V8 retains existing immutable K1 fields and additionally commits:

```text
authority_binding_manifest_root_sha256
identifier_causal_input_manifest: IdentifierCausalInputManifestV1
opportunity_root_sha256
relevant_artifact_archive_manifest_root_sha256
exact_attempt_index_root_sha256
queue_root_sha256
```

The full causal manifest is stored in the freeze, not only its root, so restart
validation can recompute it without consulting mutable scheduler state.

`selected_at_unix` remains receipt metadata but is authority-owned and excluded
from OpportunityRoot. The authority reseals the final V8 bytes after all CAS
checks; client timestamps do not determine authoritative bytes.

## Waiting Wake Contract

Before deciding that a waiting tick is reusable, the cold process samples:

```text
topology archive cursor/root
frame archive cursor/root
active protocol-mode set root
Epistemic Registry root
artifact source checkpoint root
relevant artifact snapshot root
scheduler projection root
```

Any change forces a bounded re-evaluation. An unrelated artifact can wake the
scheduler but metamorphic tests must prove that it does not alter unrelated
OpportunityRoots or create a generation.

## Research Budget

Exact deduplication and research rate are separate controls.

Preregistered conservative V1 policy:

```text
new exact freezes per scheduler tick                 <= 1
minimum time between authority-sealed V8 freezes    300 seconds
new exact freezes in trailing 24 hours              <= 48
readiness rows considered per wake                   <= 256
```

The authority derives budget use from signed Freeze V8 timestamps that it owns.
No separate mutable counter has authority.

When an unseen opportunity exists but the budget is closed, runtime state is
`RESEARCH_BUDGET_COOLDOWN` with the limiting rule and next eligible time. It
does not append a ledger event, consume the opportunity, or claim missing
evidence. Lowering these limits is operationally allowed; raising them requires
a versioned policy-root change and a new preregistration receipt.

## Event And Crash Contract

New signed event order:

```text
artifact objects + artifact manifest durably published
-> CandidateFreeze V8 durably appended and anchored
-> identifier executes
-> TerminalDiagnosticV1 durably appended and anchored
-> matching TerminalVerdict durably appended and anchored
```

Failure transitions:

- crash before Freeze V8: no generation; orphan content-addressed objects grant
  no authority;
- crash after Freeze V8: replay restores exact support and artifact projection,
  then resumes the same generation;
- crash after diagnostic before verdict: replay reuses the byte-identical
  diagnostic and appends the matching verdict once;
- crash after verdict: `ExactAttemptIndexV1` reconstructs the attempted root;
- any root mismatch: fail closed, append nothing further, retain diagnostics.

`TerminalDiagnosticV1` is a separate event variant. This preserves exact bytes
and avoids overloading legacy terminal verdict semantics.

Diagnostic and matching verdict are handled by one idempotent authority
transaction. If the process stops after the diagnostic event, the same request
must reconstruct byte-identical authority-owned bytes and append only the
missing verdict. A conflicting retry fails closed.

## Reader And Rollback Compatibility

Because legacy readers deny unknown fields and variants, deployment is
transactional in two phases.

### Phase A: compatible readers

- Queue V4, Freeze V8, and TerminalDiagnosticV1 decoders installed;
- writer feature gate forced OFF;
- legacy V1-V7 suffix replay byte-identical;
- old runtime behavior continues;
- rollback target is the pre-Phase-A deployment.

### Phase B: V8 writer

- verify every Phase A service reads a synthetic V8 suffix in an isolated copy;
- enable V8 writer for cold learner and authority;
- allow an already active V1-V7 generation to finish unchanged; V8 selection
  begins only when no legacy generation is active;
- first natural V8 append creates a rollback fence;
- rollback target becomes Phase A, never a pre-V8 binary;
- naturally appended suffix and external anchor are always preserved.

No migration rewrites old events. No old terminal is converted into V8.

## Required Tests

### Root metamorphism

- timestamp change preserves OpportunityRoot;
- generation sequence change preserves OpportunityRoot;
- score, cost, token opportunity, queue root, and catalog root changes preserve
  OpportunityRoot;
- relevant support join/frame/topology/motif/embedding change alters it;
- relevant artifact program or prediction change alters it;
- unrelated artifact addition preserves it;
- active protocol-mode set or discovery basis change alters it;
- causal resource-limit change alters it;
- unread overflow change preserves it.
- V8 Raw Phase and initial identifier semantic outputs remain byte-equal under
  excluded receipt-field mutations; only provenance envelope roots may differ.

### Attempt behavior

- one completed deterministic root creates one attempted index entry;
- repeated same root creates no generation and no event;
- a new root permits exactly one generation;
- legacy V6 terminal creates no attempted entry;
- future-contingent terminal creates no V1 pre-freeze suppression entry;
- operational error creates no attempted entry;
- coarse consequence family never affects ranking or selection.

### Diagnostic behavior

- every seed has exactly one accepted/rejected disposition;
- accepted plus rejected equals seed count;
- histogram equals rejected dispositions;
- tampered reason, count, program root, or result root fails validation;
- zero accepted programs produces the exact deterministic blocker;
- raw/private payload bytes are absent.

### Authority and restart

- learner and authority independently derive byte-equal support, artifact,
  catalog, queue, causal, opportunity, and freeze roots;
- authority independently reconstructs byte-equal IdentifierResultRoot and
  TerminalDiagnosticV1 before append;
- client schema downgrade is rejected;
- client artifact/timestamp/attempt-index tamper is rejected;
- checkpoint race returns `STALE_BEFORE_FREEZE` with no event;
- frozen artifact archive survives mutable checkpoint change;
- signed journal replay reproduces the exact attempt index;
- crash at every event boundary preserves monotonic replay;
- lane separation prevents mechanism history from suppressing epistemic work.

### Compatibility

- all preserved legacy event fixture bytes decode and re-encode exactly;
- Phase A readers consume a V8 fixture with writer disabled;
- pre-Phase-A reader is proven unable to serve as a post-V8 rollback target;
- bounded authority wire remains under its byte budget.

## Replay And Resource Acceptance

Replay the preserved production copy and a deterministic 10x concatenated copy.

Required scientific result:

```text
repeated exact deterministic OpportunityRoot  0
coarse-family suppression                      0
legacy authoritative attempted roots           0
```

Required resource report:

- wall time and CPU time;
- peak RSS;
- queue and authority wire bytes;
- artifact archive bytes;
- attempt-index reconstruction time;
- current-copy and 10x scaling ratio;
- cache growth per genuinely new opportunity.

No fixed performance claim is made before measurement. Any unbounded growth,
authority wire overflow, or worse-than-linear 10x replay is a deployment VETO.

## Production Acceptance

Production deployment may proceed only when all paper, implementation, replay,
and compatibility gates pass and a deployment receipt freezes:

- Git commit and release binary hashes;
- Phase A rollback commit and binary hashes;
- writer policy root and feature-gate state;
- scheduler ledger/anchor prefix before deployment;
- service PIDs and restart counts;
- false accepts and parity failures;
- exact attempt and opportunity summary roots.

Protected runtime:

- hot serving stays running;
- Nginx stays running;
- connector stays running;
- no generated traffic is injected;
- naturally appended ledger suffix is preserved.

The control page is updated only after backend truth is deployed. It must show:

```text
K1 laws                        1 / 3 until genuinely changed
new exact opportunities       count
attempted deterministic       count
legacy diagnostic terminals   count
current blocker               exact reason
WAITING_FOR_EVIDENCE           only when no ready candidate exists
WAITING_FOR_NOVEL_EVIDENCE     only when ready roots are exact repeats
RESEARCH_BUDGET_COOLDOWN        separately
Law #2                         NOT PROVED until the existing full route passes
```

## Claim Boundary

A complete PASS proves only exact deterministic identifier deduplication and
diagnostic preservation. Law #2 still requires:

```text
unique semantic class
-> immutable freeze
-> independent post-freeze future
-> BundleV4
-> external admission
-> verified ordinary CPU execution
-> exact economics
-> cleanup receipt
-> LawCertificate PASS
```
