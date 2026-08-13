# K1 Exact Experiment Opportunity V2 Execution Plan

Status: PAPER GATED FOR LOCAL IMPLEMENTATION. The current structural `PASS`
receipts are coherence-only (`authority_ready = false`). Deployment remains
unauthorized until observed-source, evidence-bound, externally pinned authority
gates plus every implementation, replay, compatibility, and release gate in this
plan pass.

Date: 2026-08-13.

Scientific contract:
`K1_EXACT_EXPERIMENT_OPPORTUNITY_PREREGISTRATION_V1.md`.

Critique incorporated:
`K1_EXACT_EXPERIMENT_OPPORTUNITY_PLAN_CRITIQUE_V2.md`.

Rejected predecessor:
`K1_TERMINAL_FAILURE_QUOTIENT_PREREGISTRATION_V1.md`.

Historical execution plan:
`K1_EXACT_EXPERIMENT_OPPORTUNITY_EXECUTION_PLAN_V1.md`.

## 1. Decision In Plain Language

The scheduler is capable of trying many natural cohorts, but repeated
identifier failures are not causally identified or retained in enough detail.
The rejected patch tried to solve that by grouping failures into broad families
and demoting a whole family after four failures. That is unsafe: a broad group
can contain several genuinely different unknown laws.

V2 makes one narrower change:

```text
all immutable inputs that can change the initial identifier result
-> one OpportunityRoot
-> one deterministic attempt for that exact root
-> durable exact explanation of the result
```

Only byte-equivalent causal experiments are deduplicated. A changed support row,
topology, motif, relevant artifact, protocol-mode set, discovery basis, or
resource limit creates a new root and remains researchable. A changed timestamp,
generation number, queue position, score, or unrelated artifact does not.

This repairs the research machine. It does not claim Law #2.

## 2. Result We Are Buying

### Before

```text
readiness-PASS natural cohort
-> freeze with receipt-dependent identity
-> identifier drops individual program rejection causes
-> opaque ACQUISITION_FAIL
-> next wake may repeat equivalent work
-> broad family patch risks suppressing a different law
```

### After

```text
readiness-PASS natural cohort
-> authority-owned causal manifest
-> OpportunityRoot
-> first unseen exact root selected in ordinary rank order
-> immutable Freeze V8
-> identifier evaluates every seed exactly once
-> authority reruns and signs exact diagnostic
-> deterministic exact root enters append-only attempt projection
-> same exact root creates no second generation
```

The deployed mechanism must produce one of three useful outcomes:

1. A unique semantic class advances to the existing independent-future route.
2. A completed deterministic exact root is skipped without a new event.
3. A new exact root terminates with a specific retained blocker that names the
   next narrow repair target.

Repeated opaque failure is not an accepted outcome.

## 3. Non-Goals And Forbidden Shortcuts

This change does not:

- add FILTER, COUNT, BRANCH, renderer, or other operator hints to candidates;
- generate synthetic traffic or teacher outputs;
- manually map a candidate to a program family;
- change ordinary K1 scoring before exact-attempt observation;
- mutate phase memory or an ACTIVE generation;
- backfill V1-V7 terminals into the exact attempt index;
- grant `LawCertificate`, K1 OPEN, package activation, or CPU authority;
- change hot serving, Nginx, connector, MS8 portfolio, L2, or economics logic;
- promise that natural traffic contains Law #2.

Coarse consequence and semantic families may be counted for observation. They
must never rank, demote, exclude, suppress, authorize, or certify a candidate.

## 4. Authority Hierarchy

When documents disagree, use this order:

1. `/home/ubu/projects/nando-wave/ARCHITECTURE_CANON.md`;
2. exact-opportunity preregistration V1;
3. this V2 execution plan;
4. implementation-preflight manifest and receipt;
5. code-route design and result;
6. test and replay receipts;
7. dashboard prose.

The plan may make the contract executable. It cannot silently weaken it.

## 5. Current Starting State

```text
worktree         /home/ubu/projects/nando-wave-k1-transport-fix
branch           k1-topology-quotient-v2-20260810
paper baseline   ded3d18 (ahead of origin by one commit at plan freeze)
production       untouched by this planning stage
Law #2           NOT PROVED
K1               1 / 3 laws
```

The worktree contains an uncommitted implementation of the rejected coarse
family policy. It is evidence and a refactor starting point, not accepted code.
It must not be reverted destructively and must not be deployed.

Protected untracked paths:

```text
graphify-out/
plans/effect-law-unification-v1/evidence/S1C3*/
```

They are read-only for this route and must not enter commits.

## 6. End-To-End Work Tree

```text
P0 Paper authority                                           COMPLETE
|- critique V1 and reject broad family suppression
|- freeze exact scientific contract
|- write executor-grade V2 plan
|- Gate A: exact identity + Raw Phase
|- Gate B: selection/terminal authority + persistence
|- Gate C: compatibility + budget + dashboard + claims
|- code-route PASS
`- implementation preflight READY

P1 Remove rejected authority                                 NEXT
|- inventory dirty coarse-family code
|- preserve reusable schema/parity scaffolding
|- remove family rank/demotion/exhaustion decisions
`- prove coarse fields have zero authority

P2 Pure exact identity                                       BLOCKED BY P1
|- causal input manifest
|- relevant artifact projection
|- OpportunityRoot
|- Queue V4 / Freeze V8 contracts
|- V8 Raw Phase uses OpportunityRoot
`- metamorphic tests

P3 Exact diagnostic evaluator                                BLOCKED BY P2
|- evaluate every seed once
|- accepted/rejected dispositions
|- stable closed reason codes
|- IdentifierResultRoot
|- TerminalDiagnosticV1
`- conservation + privacy tests

P4 Durable authority and persistence                         BLOCKED BY P3
|- independent source restoration
|- learner/authority parity
|- content-addressed relevant artifact archive
|- authority-owned Freeze V8
|- authority-owned terminal transaction
|- ExactAttemptIndexV1
`- crash/restart tests

P5 Scheduler state machine and research budget               BLOCKED BY P4
|- ordinary rank preserved
|- first unseen exact root
|- one new freeze per wake
|- five-minute / 48-per-day / 256-row bounds
|- distinct waiting and cooldown states
`- idempotence tests

P6 Compatibility and rollback                                BLOCKED BY P5
|- legacy byte fixtures
|- Phase A reader with writer OFF
|- V8 fixture replay
|- Phase B writer policy
`- post-V8 rollback fence

P7 Production-copy replay and value gate                     BLOCKED BY P6
|- preserved current copy
|- deterministic 10x copy
|- exact-repeat and diagnostic census
|- CPU/RSS/disk/wire measurements
`- deployment GO or VETO

P8 Full verification and independent critique                BLOCKED BY P7
|- focused + crate suites
|- fmt + strict Clippy
|- observed code-route gate
|- structural/composite gates
`- implementation critique

P9 Focused commits, Entire checkpoint, push                  BLOCKED BY P8

P10 Deployment Phase A: compatible readers, writer OFF       BLOCKED BY P9

P11 Deployment Phase B: exact writer ON                      BLOCKED BY P10

P12 Backend summary then factual HTML                        BLOCKED BY P11

P13 Natural autonomous observation                           BLOCKED BY P12
|- unique class -> independent future route
|- exact repeat -> no work
`- diagnosed new defect -> one separately preregistered repair
```

## 7. Five Identities That Must Never Be Merged

| Identity | Question answered | Includes | Excludes | Owner |
|---|---|---|---|---|
| `AuthorityBindingManifestV1` | Was selection authorized in this current system state? | ledger, registry, deficit, fixture, catalog, queue, policy, source snapshot roots | causal equality claim | certification authority |
| `IdentifierCausalInputManifestV1` | What immutable information can change the initial identifier result? | exact support, topology, motif, embedding, relevant artifact, generator, basis, modes, limits | receipt metadata and irrelevant artifacts | shared pure builder over independently restored inputs |
| `OpportunityRoot` | Is this the same deterministic initial experiment? | canonical causal manifest only | generation, time, queue, score, economics, future deadline, coarse family | canonical root function |
| `IdentifierResultRoot` | What causal result did the identifier produce? | seed set, dispositions, accepted set, semantic classes, state, blocker | freeze receipt metadata | pure prepared evaluator |
| `TerminalDiagnosticRoot` | Which authorized generation produced that result? | opportunity, result, freeze, support/archive roots, disposition | promotion authority | certification authority |

Required equality rule:

```text
same IdentifierCausalInputManifestV1 bytes
<=> same OpportunityRoot
<=> same initial deterministic identifier domain
```

The full Freeze V8 root is provenance. It is not an identifier domain root.

## 8. Authority And Process Matrix

| Decision or mutation | Cold learner | Certification authority | Journal/anchor | Control plane |
|---|---:|---:|---:|---:|
| Observe natural prefixes | yes | independently restores | no | no |
| Build proposed catalog/queue | yes | independently rebuilds | no | observes only |
| Rank ordinary candidates | proposes | recomputes exact rank | no | observes only |
| Build causal manifest | proposes via pure builder | recomputes via same pure builder | stores in freeze | observes root |
| Select and seal Freeze V8 | no | sole authority | append + anchor | no |
| Execute active identifier | yes | independently reruns for terminal authority | no | observes state |
| Classify deterministic terminal | proposes only | sole authority from closed policy | diagnostic then verdict | observes code/count |
| Mark exact root attempted | no mutable state | no mutable blacklist | pure signed-event projection | observes count |
| Grant LawCertificate | no | existing independent certificate route only | certificate ledger | observes result |
| Activate package | no | external admission only | admitted registry | observes result |

The generic append route cannot authorize a V8 deterministic diagnostic or
matching terminal verdict.

## 9. Scheduler Decision Order

Every wake uses this exact order:

```text
1. restore and freeze EvidenceSourceSnapshotV1
2. rebuild operator-blind catalog and ordinary rank
3. retain at most 256 readiness-PASS rows
4. build exact causal manifest and OpportunityRoot for each retained row
5. project ExactAttemptIndexV1 from signed V8 history
6. annotate each row unseen | attempted_deterministic
7. if no readiness-PASS row -> WAITING_FOR_EVIDENCE
8. if every ready exact root attempted -> WAITING_FOR_NOVEL_EVIDENCE
9. if unseen root exists but budget closed -> RESEARCH_BUDGET_COOLDOWN
10. otherwise authority selects first ordinary-ranked unseen root
11. append at most one new Freeze V8 during this wake
12. never create another freeze in the same wake after a terminal
```

Attempt state cannot modify the score tuple or reorder queue rows. It only
prevents a second generation for the same completed deterministic exact root.

## 10. Runtime State Machine

| Current condition | State | Event appended | Generation consumed | Next trigger |
|---|---|---:|---:|---|
| No readiness-PASS row | `WAITING_FOR_EVIDENCE` | no | no | source snapshot changes |
| Ready rows, all exact roots attempted | `WAITING_FOR_NOVEL_EVIDENCE` | no | no | causal input changes |
| Unseen root, rate limit closed | `RESEARCH_BUDGET_COOLDOWN` | no | no | next eligible authority time |
| Unseen root, budget open, no active generation | `FREEZE_PENDING_AUTHORITY` | only on authority PASS | yes | authority transaction |
| Freeze durable | `IDENTIFYING` | freeze already exists | existing | identifier result |
| Multiple viable hypotheses remain | existing `PROBE_PENDING` | existing contract only | existing | independent distinguishing evidence |
| Pre-future deterministic terminal | `TERMINAL_AUTHORITY_PENDING` | diagnostic then verdict | existing | authority rerun |
| Operational error | retryable operational state | no attempted-root entry | existing or none | source/operation repair |
| Unique semantic class | existing independent-future route | existing protocol events | existing | post-freeze natural future |

No waiting or cooldown state is scientific evidence. No operational failure is
negative knowledge.

## 11. Research Budget

The frozen V1 limits are independent controls:

```text
new V8 freezes per wake                    <= 1
seconds between authority-sealed freezes  >= 300
new V8 freezes in trailing 24 hours       <= 48
readiness rows considered per wake        <= 256
```

Budget is reconstructed from authority-owned signed Freeze V8 timestamps. There
is no mutable counter with suppression authority.

Lower limits are an operational restriction. Raising any limit changes the
policy root and requires new preregistration.

## 12. Executor Resume Protocol

At the beginning of every phase:

1. Read the latest user instruction and stop if it no longer authorizes this
   route.
2. Run `entire status`, `git status --short --branch`, and `git log -1`.
3. Verify protected untracked paths remain unmodified and unstaged.
4. Read the preceding phase receipt instead of rerunning completed discovery.
5. Re-run a gate only when one of its bound inputs changed.
6. Record one short progress tree with the current node and blocker.

Never reset or discard the dirty V1 patch. Refactor it in place after P0 passes.
If a preflight baseline under `/tmp` no longer exists, or any baseline source
hash changed before P1 starts, the old preflight receipt is stale. Create a new
read-only baseline copy, update the manifest deliberately, and require a new
`READY_TO_IMPLEMENT` receipt before editing code.

## 13. P0: Paper Authority

### Inputs

- architecture canon;
- exact-opportunity preregistration;
- V2 critique;
- current dirty implementation inventory;
- current queue, freeze, journal, wire, identifier, and dashboard routes;
- preserved production-copy ledger and anchor hashes.

### Required structural packets

```text
Gate A  exact identity and V8 Raw Phase
Gate B  selection/terminal authority and persistence
Gate C  compatibility, budget, dashboard, and claim boundary
```

Each packet must have closely paired source and plan triads, one coherent owner
vocabulary, no foreign route, and verdict `PASS`. `WATCH` and `VETO` block code.
These design-paper receipts are coherence gates. They do not authorize a
production mutation, scientific claim, or signed runtime decision.

### Other paper gates

- code-route design: `PASS`;
- implementation preflight: `READY_TO_IMPLEMENT`;
- `safe_to_implement = true`;
- baseline file hashes still match the preflight or are intentionally refreshed
  before code under a new receipt.

### Artifacts

```text
K1_EXACT_EXPERIMENT_OPPORTUNITY_PLAN_CRITIQUE_V2.md
K1_EXACT_EXPERIMENT_OPPORTUNITY_EXECUTION_PLAN_V2.md
evidence/K1_EXACT_EXPERIMENT_OPPORTUNITY_V1/structure-v2/
  identity-and-raw-phase.worksheet.md
  identity-and-raw-phase.result.json
  authority-and-persistence.worksheet.md
  authority-and-persistence.result.json
  compatibility-budget-claim.worksheet.md
  compatibility-budget-claim.result.json
  gate-summary.json
```

### Exit gate

All required verdicts pass, paper inputs have stable hashes, and production is
untouched. The paper gate receipt records this condition; P1 is now permitted.

## 14. P1: Remove Rejected Coarse-Family Authority

### Objective

Turn the current uncommitted V1 patch into a safe starting point without losing
unrelated user work or useful plumbing.

### Inventory before edits

Record every dirty addition related to:

- `K1TerminalFailureQuotientV1`;
- family keys, thresholds, summaries, and exhausted-family state;
- `terminal_failure_family_novelty_rank`;
- queue/freeze V3/V7 family fields;
- selection demotion ordering;
- authority parity for family quotient;
- dashboard family counters;
- related tests.

### Preserve only if semantically reusable

- queue-root binding;
- terminal-history traversal helpers with no family policy;
- schema fixture/parity scaffolding;
- authority recomputation test structure.

### Remove from every authority path

- family threshold and exhaustion;
- family novelty rank;
- family demotion or candidate exclusion;
- selection use of consequence type or semantic novelty signature;
- V1/V7 fields whose only purpose is coarse suppression.

### Required tests

1. Mutating consequence type or coarse semantic signature alone has no attempt
   or rank authority.
2. Four failures in one broad family do not demote a causally new row.
3. Legacy terminals yield zero exact attempted roots.
4. Existing ordinary ranking is byte-equal when exact attempt index is empty.

### Receipt and exit

Store an inventory showing every rejected field as `removed`,
`observation_only`, or `reused_without_authority`. P1 passes only when no
coarse-family value reaches ranking, selection, freeze authorization, terminal
authorization, or attempted-root projection.

## 15. P2: Pure Exact Identity

### Objective

Implement deterministic, file-free contracts in
`nando-operator-learning` before runtime integration.

### Planned modules

```text
crates/nando-operator-learning/src/multi_source/
  identification/diagnostic.rs
  k1_natural_scheduler_v1/
    opportunity.rs
    attempt_index.rs
    model/queue.rs
    model/freeze.rs
```

### Types

- `IdentifierResourceLimitsV1`;
- `IdentifierSupportRowV1`;
- `EvidenceSourceSnapshotV1`;
- `IdentifierCausalInputManifestV1`;
- `RelevantIdentifierArtifactProjectionV1`;
- `IdentifierResultV1`;
- `ProgramDispositionV1`;
- `TerminalDiagnosticV1`;
- `ExactAttemptIndexV1`;
- Queue V4 and Freeze V8 fields.

### Build sequence

1. Canonicalize support rows by `(capture_sequence, join_root, motif_root)`.
2. Reject duplicate, out-of-watermark, invalid, or non-reconstructable rows.
3. Build relevant artifact projection from validated typed artifacts and exact
   support identities.
4. Represent missing relevant artifacts as a rooted empty projection, never
   `None`.
5. Seal `IdentifierCausalInputManifestV1` with only causal fields.
6. Derive `OpportunityRoot` from canonical manifest bytes.
7. Add Queue V4 observation fields without changing ordinary score/order.
8. Add Freeze V8 with full causal manifest and separate authority provenance.
9. Change V8 Raw Phase and identification evidence domain to
   `OpportunityRoot`; retain full freeze root only in provenance envelopes.

### Metamorphic test matrix

| Mutation | OpportunityRoot | Initial Raw Phase result |
|---|---|---|
| timestamp | same | same |
| generation sequence | same | same |
| queue/catalog root | same | same |
| score/cost/token estimate | same | same |
| future deadline | same | same |
| unrelated artifact | same | same |
| unread overflow | same | same |
| support join/frame/topology/motif/embedding | different | allowed different |
| relevant program/prediction | different | allowed different |
| active protocol-mode set | different | allowed different |
| discovery basis | different | allowed different |
| causal resource limit | different | allowed different |

Only provenance envelope roots may differ for excluded receipt mutations.

### Exit gate

- all metamorphic tests pass;
- V1-V7 byte fixtures remain exact;
- no file or socket access exists in pure builders;
- no runtime integration is active;
- P2 receipt records schema roots and test names.

## 16. P3: Exact Diagnostic Evaluator

### Objective

Stop deleting the reason every seed program failed.

### Pure evaluation route

```text
canonical seed program set
-> bind each seed to exact motif exactly once
-> Accepted(bound program) | Rejected(stable code)
-> conservation validation
-> existing identifier consumes accepted set
-> IdentifierResultRoot
-> diagnostic projection
```

Reuse `bind_pre_action_t1_program_to_motif_v1`. Do not add another binder or
program language.

### Stable reason contract

- closed enum owned by the binding/evaluator boundary;
- free-form text may exist in local logs but never in scientific roots;
- unknown errors map to `internal_unclassified`;
- `internal_unclassified` is operational and never creates an attempted root;
- changing a reason code is a schema change, not a silent edit.

### Conservation invariants

```text
seed_count = accepted_count + rejected_count
every seed root appears exactly once
histogram total = rejected_count
accepted_set_root = canonical accepted program roots
IdentifierResultRoot binds existing identifier report root
```

### Privacy tests

Serialize every diagnostic fixture and scan bytes for prompts, provider payloads,
expected responses, rendered answers, raw values, and known test secrets. Only
typed bounded metadata, reason codes, counts, and hashes may remain.

### Required cases

- mixed accepted/rejected;
- all rejected;
- all accepted;
- empty seed set;
- malformed or duplicated disposition;
- input permutation;
- unchanged accepted set preserves old identifier behavior;
- forged count, reason, root, result, or disposition fails.

### Exit gate

Conservation, parity, determinism, and privacy tests pass. A compact receipt
records the disposition schema root and exact blocker histogram for fixtures.

## 17. P4: Durable Authority And Persistence

### Objective

Make client proposals non-authoritative and every completed deterministic
attempt restart-safe.

### Independent source ownership

Cold learner reads its current validated in-memory archives. Certification
authority independently reads configured durable topology, frame, collection,
registry, basis, and journal sources. Both call the same pure builders.

Exact parity covers every intermediate root:

```text
EvidenceSourceSnapshot
-> support manifest
-> relevant artifact projection
-> catalog
-> Queue V4
-> causal manifest
-> OpportunityRoot
-> Freeze V8
```

Comparing only the final freeze root is insufficient.

### Explicit configuration

`CertificationAuthorityConfigV1` receives explicit paths for topology archive,
frame archive, collection checkpoint, identifier artifact archive, and installed
scheduler policy. Parent-directory inference is forbidden for new authority
inputs.

### Checkpoint race

Authority reads checkpoint metadata/root before and after projection. A change
returns `STALE_BEFORE_FREEZE`, appends no event, and consumes no generation or
attempt. Orphan content-addressed objects have no authority.

### Artifact publication

```text
objects/<object-root>.cbor
manifests/<projection-root>.cbor
```

Use create-new, file `fsync`, atomic publication, and directory `fsync`.
Existing paths are accepted only for byte-identical content. Active generations
read only the archived immutable projection.

### Freeze authority transaction

1. Restore signed ledger and current registry/deficit roots.
2. Restore exact frozen source prefixes.
3. Rebuild catalog, rank, causal manifests, and exact attempt index.
4. Recheck registry and checkpoint CAS.
5. Enforce budget and installed minimum schema.
6. Publish relevant artifact archive.
7. Reseal authority-owned timestamp and Freeze V8 bytes.
8. Append and anchor exactly one freeze event.

Any mismatch appends nothing.

### Terminal authority transaction

1. Restore active Freeze V8 and immutable archived inputs.
2. Rerun the shared initial evaluator.
3. Reconstruct exact `IdentifierResultRoot` and diagnostic.
4. Classify disposition through the closed authority policy.
5. Append and anchor diagnostic.
6. Append and anchor matching verdict.
7. On retry, reuse byte-identical diagnostic and append only the missing verdict.

Client result roots, histograms, dispositions, blockers, and timestamps have no
authority.

### Deterministic V1 allow-list

- `motif_program_candidates_empty`;
- `natural_collection_candidate_artifact_missing`;
- `natural_collection_candidate_generation_empty`;
- `all_supported_t1_protocol_modes_already_active` only with the exact active
  protocol-mode set bound into the opportunity.

I/O, timeout, signature, decode, race, panic, persistence, and unclassified
errors are operational. Future exhaustion and post-freeze contradiction are
future-contingent. Neither category enters the V1 pre-freeze attempt index.

### Crash table

| Fault point | Durable truth after restart | Attempted root? | Recovery |
|---|---|---:|---|
| object written, no manifest | old ledger; orphan object ignored | no | rebuild or reuse object |
| manifest written, no freeze | old ledger; orphan manifest ignored | no | rebuild and authorize |
| freeze file fsync, no directory fsync/anchor | old or complete valid prefix only | no unless full event valid | fail closed/replay |
| freeze anchored | active exact generation restored | no | resume same generation |
| identifier completed, no diagnostic | active freeze only | no | authority reruns evaluator |
| diagnostic appended, no verdict | byte-identical diagnostic restored | no | append only matching verdict |
| verdict appended | complete event triple restored | yes if deterministic | project index |
| any root conflict | prior valid prefix retained | no new entry | stop and diagnose |

### Exit gate

All authority tamper, race, fault-injection, idempotence, lane-separation, and
restart-parity tests pass. The exact index is always a pure signed-event
projection, never a mutable blacklist.

## 18. P5: Scheduler Policy And Bounded Work

### Queue V4

Queue V4 preserves ordinary operator-blind order and adds only:

- attempt-index root;
- artifact-source snapshot root;
- causal-manifest and OpportunityRoot per retained readiness row;
- `unseen | attempted_deterministic` observation.

The server-installed policy selects Queue V4 and Freeze V8. Client downgrade or
unknown higher schemas fail closed.

### Wake and terminal limits

- at most one new freeze per wake;
- already-active generation may make bounded progress;
- terminal completion cannot open another generation in the same wake;
- exact repeat appends no event and consumes no budget;
- cooldown appends no event and consumes no generation;
- mechanism lane history cannot affect epistemic attempt state.

### Required tests

1. Same completed deterministic OpportunityRoot creates no generation/event.
2. One causal input change creates one unseen root and at most one freeze.
3. No evidence, no novel evidence, cooldown, and active generation are distinct.
4. Restart reconstructs daily budget and next eligible time from signed events.
5. Forged client queue, attempt index, policy, timestamp, or blocker is rejected.
6. Generic append rejects V8 deterministic diagnostic and verdict.
7. Lower client schema returns `K1_AUTHORITY_SCHEMA_DOWNGRADE`.

### Exit gate

State-machine transition coverage is complete, all no-event states leave journal
and generation unchanged, and a scheduler simulation cannot exceed any frozen
research limit.

## 19. P6: Compatibility And Rollback

### Reader matrix

| Reader | V1-V7 suffix | V8 fixture | Writer | Valid rollback after V8? |
|---|---:|---:|---:|---:|
| pre-Phase-A | pass | fail expected | legacy only | no |
| Phase A | byte-exact pass | pass | forced off | yes |
| Phase B | byte-exact pass | pass | on | current |

### Required legacy fixtures

- Queue V1-V3;
- Freeze V1-V7;
- all existing scheduler event variants;
- signed journal and anchor projection;
- authority wire request/response versions;
- active legacy generation completion.

No migration rewrites an old event. No legacy terminal becomes V8.

### Fault injection

Inject after artifact object, artifact manifest, freeze file `fsync`, freeze
directory `fsync`, anchor write, diagnostic append, verdict append, and cache
publication. Every restart yields old valid prefix or new complete valid prefix.

### Exit gate

Legacy bytes remain exact, Phase A reads isolated V8 fixtures with writer off,
wire limits pass, and a rollback test proves pre-Phase-A binaries are forbidden
after the first V8 append.

## 20. P7: Production-Copy Replay And Value Gate

### Isolation

Use a preserved production-state copy and deterministic 10x concatenated copy.
Replay processes receive no production socket or state path. Natural production
services continue running untouched.

### Freeze before measurement

Record input paths, byte counts, roots, event counts, policy root, build commit,
command, and environment limits before replay. The 10x denominator is exactly
ten copies of the frozen current input, not a separately sampled dataset.

### One machine-readable receipt records

- ready rows considered;
- exact roots considered, unseen, completed, and legacy-unbound;
- exact repeats avoided;
- deterministic, future-contingent, operational, and unclassified outcomes;
- stable blocker and reason histograms;
- wall time and CPU time;
- peak RSS;
- queue, wire, journal, cache, and archive bytes;
- attempt-index reconstruction time;
- current-to-10x time, memory, and byte ratios.

### Decision

```text
exact repeats > 0
-> immediate deduplication value demonstrated

exact repeats = 0 AND classified diagnostics dominate
-> bounded diagnostic value demonstrated

unclassified diagnostics dominate
-> deployment VETO

wire overflow OR unbounded archive/cache OR worse-than-linear 10x behavior
-> deployment VETO

false accepts > 0 OR parity failures > 0
-> hard VETO
```

The old `211 failures -> 4 families` count is forbidden as an acceptance metric.

## 21. P8: Full Verification And Independent Critique

Run dependency-ordered groups; batch independent suites inside each group:

1. focused identity, diagnostic, authority, attempt-index, state, and fault tests;
2. full `nando-operator-learning` suite;
3. full `nando-transition-serving` suite;
4. touched response-actor artifact/checkpoint tests;
5. `cargo fmt --check`;
6. strict Clippy on changed crates;
7. structural gate on observed implementation routes;
8. code-route gate with observed source locations;
9. evidence-bound structural packets with a proof manifest whose exact root is
   pinned by the external trust owner; require `authority_ready = true`;
10. mandatory live-transition/composite gate on isolated release artifacts;
11. independent diff critique against preregistration and this plan.

Baseline failures must reproduce on the pinned baseline and remain separately
named. No failure is silently waived.

The implementation critique must answer:

- Can any coarse group suppress a new root?
- Can any receipt-only field perturb Raw Phase?
- Can a client forge a diagnostic or terminal disposition?
- Can an operational failure mark an attempt complete?
- Can a crash lose diagnosis or duplicate a terminal?
- Can a pre-V8 binary be selected after a V8 suffix exists?
- Can the dashboard imply Law #2?

Any unresolved answer blocks commit/push/deployment.

## 22. P9: Commit And Entire Boundaries

Required focused commits:

1. remove rejected family authority; add pure exact identity contracts;
2. add diagnostic evaluator and authority/persistence integration;
3. add scheduler state, compatibility, replay receipts, and tests;
4. add backend summary and HTML only after backend truth exists.

Before every commit:

- inspect `git diff --check` and exact diff;
- confirm protected untracked paths are unstaged and untouched;
- scan for private production data;
- run tests mapped to that boundary;
- create and inspect an Entire checkpoint;
- record commit hash in the phase receipt.

Push only after P8 passes for the backend commits. Do not mix deployment receipts
into source commits unless the repository's existing convention requires it.

## 23. P10: Deployment Phase A

Install compatible cold learner, authority, and control readers with the V8
writer forced OFF.

### Transaction receipt freezes

- commit and release/installed binary hashes;
- pre-Phase-A rollback commit and binary hashes;
- writer flag `OFF` and policy root;
- ledger prefix root/revision and anchor revision;
- cold, authority, control, hot, Nginx, and connector PIDs/restart counts;
- false accepts and parity failures;
- reader fixture result.

### Live acceptance

- legacy ledger/anchor replay exactly;
- no V8 event appended;
- old scheduler behavior continues;
- writer-disabled state visible in health;
- changed services survive a bounded observation;
- hot, Nginx, and connector PIDs remain unchanged;
- false accepts and parity failures remain zero.

Any failure restores pre-Phase-A binaries because no V8 suffix exists yet.

## 24. P11: Deployment Phase B

Enable Queue V4/Freeze V8 writer only after Phase A passes.

### Before enable

- Phase A is recorded as rollback target;
- isolated authority parity probe passes without production append;
- cold and authority policy roots match;
- installed minimum schemas are visible;
- budget state and next eligible time are reconstructable;
- any active V1-V7 generation remains under its immutable original contract.

### After enable

- only cold/authority/control services may restart;
- protected service PIDs remain unchanged;
- no generated traffic is injected;
- natural suffix and external anchor are preserved;
- first natural V8 event permanently moves rollback target to Phase A.

A failed Phase B transaction rolls back once to Phase A and records one receipt.
No deployment retry loop is allowed.

## 25. P12: Backend Summary And HTML

Backend truth is implemented and verified before HTML.

### Required fields

```text
K1 laws                              1 / 3 until certificate changes
Law #2                               NOT PROVED
readiness-PASS rows                  N
new exact opportunities              N
attempted deterministic roots        N
legacy unbound terminals             N
current state                        evidence | novel evidence | cooldown | active
current exact blocker                stable code + count or NONE
next eligible research time          timestamp or NOT APPLICABLE
false accepts / parity failures      N / N
quality                              UNKNOWN unless independently verified
```

### Display rules

- zero, unknown, absent, and not applicable are separate values;
- `WAITING_FOR_EVIDENCE` is used only for zero readiness-PASS rows;
- `WAITING_FOR_NOVEL_EVIDENCE` is used only when all ready exact roots are
  completed deterministic repeats;
- cooldown never appears as missing evidence;
- coarse family counts never appear as laws or exhausted hypotheses;
- routing/local completion does not imply answer quality;
- scheduler PASS does not imply Law #2.

### Browser verification

Use the existing authenticated browser. Check desktop and mobile widths, no
overflow, no JS errors, exact API/HTML parity, refresh after service restart,
and close every tab opened for verification.

## 26. P13: Natural Autonomous Result

### Success route

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

Only the final line moves K1 from `1/3` to `2/3`.

### Diagnosed failure route

A dominant stable blocker permits one new narrow preregistration for its actual
owner, for example generator, motif binder, collection artifact capture,
protocol-mode quotient, or resource bound. Do not change scheduler ranking to
hide the defect.

### Exact repeat route

The scheduler records no new generation/event. It waits for a causal source
change and reports `WAITING_FOR_NOVEL_EVIDENCE`.

## 27. Stop Conditions

Stop this implementation route when any condition is true:

- a paper packet is `WATCH` or `VETO`;
- implementation preflight is not safe;
- coarse-family authority remains reachable;
- learner/authority parity differs at any intermediate root;
- private traffic enters diagnostics or Git;
- exact attempt projection can be forged or mutated independently of history;
- replay is unbounded, superlinear, or exceeds wire/resource limits;
- false accepts or parity failures become nonzero;
- deployment would restart hot serving, Nginx, or connector;
- rollback would require discarding a natural suffix;
- the next step requires synthetic evidence, teacher output, or manual program
  mapping;
- the full Law #2 route would weaken independent future, admission, economics,
  cleanup, or certificate authority.

After a successful deployment, stop changing this scheduler merely because a
natural law has not appeared yet. The mechanism has one job: bounded exact
search with truthful diagnostics. Any new code repair requires its own measured
blocker and preregistration.

## 28. Efficiency Rules

1. Keep one active route: exact opportunity only.
2. Reuse fresh verified results until a bound input changes.
3. Run focused tests while editing; crate suites once at phase boundaries.
4. Batch independent reads and tests; keep raw logs on disk.
5. Admit only roots, counts, errors, durations, and verdicts to conversation.
6. Two identical unexpected blockers stop retries and trigger direct diagnosis.
7. No browser work before backend deployment truth.
8. No deployment retries after transactional rollback.
9. Do not regenerate `graphify-out/` during this paper/implementation route.
10. Report progress as the work tree node, completed receipt, and exact blocker.

## 29. Definition Of Done

### Engineering completion

```text
three structural packets                       PASS
paper structural authority_ready               false (coherence-only)
code-route design and observed route           PASS
implementation preflight                       READY_TO_IMPLEMENT
coarse-family suppression authority             0
OpportunityRoot metamorphism                    PASS
V8 Raw Phase causal-domain parity               PASS
diagnostic conservation/privacy                 PASS
learner/authority intermediate-root parity      PASS
exact repeat new generations/events              0
operational/future failures in attempt index      0
legacy authoritative attempted roots             0
research limits                                 ENFORCED
legacy bytes and restart parity                  PASS
production-copy replay/resource gate             PASS
release structural authority_ready              true (externally pinned)
Phase A rollback target                         RECORDED
Phase B writer                                  DEPLOYED
protected service interruptions                   0
false accepts / parity failures                   0 / 0
summary API / HTML parity                       PASS
```

### Scientific status after engineering completion

```text
exact-opportunity scheduler repair   PASS
Law #2                               NOT PROVED unless full natural route passed
K1                                   1 / 3 unless certificate ledger changed
Natural L2                           BLOCKED until K1 vocabulary opens
```

That distinction is mandatory. A better experiment machine is valuable, but it
is not itself the discovered law.
