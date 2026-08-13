# K1 Exact Experiment Opportunity V1 Execution Plan

Status: SUPERSEDED by
`K1_EXACT_EXPERIMENT_OPPORTUNITY_EXECUTION_PLAN_V2.md`. Retained as historical
planning evidence; it MUST NOT independently authorize implementation or
deployment.

Date: 2026-08-13.

Authority contract:
`K1_EXACT_EXPERIMENT_OPPORTUNITY_PREREGISTRATION_V1.md`.

Rejected predecessor:
`K1_TERMINAL_FAILURE_QUOTIENT_PREREGISTRATION_V1.md`.

## Result We Are Buying

This is a bounded scheduler repair, not a promise that Law #2 will appear.

```text
Before
ready natural candidates
-> repeated opaque identifier failures
-> up to 16 transitions per tick
-> no exact causal identity
-> no retained rejection explanation

After
ready natural candidates
-> exact causal identity
-> one attempt per deterministic OpportunityRoot
-> one new freeze per tick + durable daily budget
-> exact rejection diagnostic
-> either a real identifier route or a named repair target
```

The project advances only when one of these useful outcomes is obtained:

1. A unique semantic class reaches independent future.
2. Exact duplicate work is measurably removed.
3. Rooted diagnostics identify a specific generator, binding, artifact, or
   representation defect that can be repaired under a new preregistration.

Repeated `ACQUISITION_FAIL` without a new exact root or diagnostic is forbidden.

## Work Tree

```text
S0 Paper authority                                      CURRENT
|- reject coarse family suppression
|- freeze exact OpportunityRoot contract
|- freeze terminal diagnostic contract
|- freeze research budget and rollback contract
`- structural + code-route + implementation preflight

S1 Pure causal model                                    BLOCKED BY S0
|- IdentifierCausalInputManifestV1
|- IdentifierResourceLimitsV1
|- RelevantIdentifierArtifactProjectionV1
|- OpportunityRoot
`- metamorphic tests

S2 Exact identifier diagnostics                        BLOCKED BY S1
|- prepared seed-program evaluator
|- per-program motif disposition
|- TerminalDiagnosticV1
`- privacy and conservation tests

S3 Durable authority inputs                             BLOCKED BY S2
|- read-only collection checkpoint projection
|- authority-owned support reconstruction
|- content-addressed artifact archive
`- learner/authority parity

S4 Scheduler policy                                    BLOCKED BY S3
|- ExactAttemptIndexV1
|- Queue V4
|- Freeze V8
|- WAITING_FOR_NOVEL_EVIDENCE
|- RESEARCH_BUDGET_COOLDOWN
`- one-new-freeze-per-tick

S5 Compatibility and crash safety                      BLOCKED BY S4
|- legacy byte fixtures
|- V8 reader with writer disabled
|- journal fault injection
|- wire and lane separation
`- rollback fence tests

S6 Production-copy replay and budgets                  BLOCKED BY S5
|- current snapshot
|- 10x replay
|- exact-repeat census
|- CPU/RSS/disk/wire report
`- value gate

S7 Full verification and implementation critique       BLOCKED BY S6
|- focused tests
|- crate suites
|- fmt + Clippy
|- structural/composite gates
`- independent diff critique

S8 Commit, Entire checkpoint, push                     BLOCKED BY S7

S9 Deployment Phase A: compatible readers              BLOCKED BY S8
`- writer OFF

S10 Deployment Phase B: exact writer                   BLOCKED BY S9
`- rollback target = Phase A

S11 Summary API and HTML                               BLOCKED BY S10
`- browser verification and close tabs

S12 Natural observation                               AUTONOMOUS
|- unique class -> existing independent-future route
|- deterministic repeat -> no work
|- new diagnosed failure -> bounded next decision
`- Law #2 remains NOT PROVED until certification
```

## Phase 0: Paper Authority

### Inputs

- architectural canon;
- current dirty implementation diff;
- preserved production-copy ledger and anchor;
- current queue/freeze/wire schemas;
- exact code routes for learner, authority, identifier, journal, and dashboard.

### Outputs

- V2 critique marking V1 policy rejected;
- exact-opportunity preregistration;
- this execution plan;
- structural worksheet and result;
- code-route V2 design and result;
- implementation-preflight manifest and receipt.

### Exit gate

All three must be true:

```text
structural verdict                 PASS
code-route design verdict          PASS
implementation preflight           READY_TO_IMPLEMENT
safe_to_implement                  true
```

`WATCH`, `VETO`, or `BLOCKED_BEFORE_CODE` stops coding and repairs paper only.

## Phase 1: Pure Causal Model

### Scope

Add authority-free, deterministic data types and builders under
`nando-operator-learning`.

Planned modules:

```text
crates/nando-operator-learning/src/multi_source/
  identification/diagnostic.rs
  k1_natural_scheduler_v1/
    opportunity.rs
    attempt_index.rs
    model/queue.rs
    model/freeze.rs
```

### Required types

- `IdentifierResourceLimitsV1`;
- `IdentifierSupportRowV1`;
- `IdentifierCausalInputManifestV1`;
- `RelevantIdentifierArtifactProjectionV1`;
- `ProgramDispositionV1`;
- `IdentifierResultV1`;
- `TerminalDiagnosticV1`;
- `ExactAttemptIndexV1`.

### Refactor of current dirty work

Preserve:

- queue-root binding;
- schema plumbing;
- learner/authority parity fixture structure;
- terminal-history traversal helpers.

Remove from authority decisions:

- `terminal_failure_family_novelty_rank`;
- family threshold and exhausted-family state;
- family demotion ordering;
- any selection use of `semantic_novelty_signature`.

`terminal_failure.rs` is either renamed/refactored into exact attempt projection
or deleted after its reusable traversal logic moves. No unrelated user changes
or untracked evidence directories are modified.

### Exit tests

- all OpportunityRoot metamorphic tests;
- V8 Raw Phase receives OpportunityRoot, never the full Freeze V8 root;
- canonical ordering and duplicate rejection;
- legacy Queue V1-V3 and Freeze V1-V7 byte fixtures unchanged;
- coarse semantic family has no deduplication or suppression effect.

No runtime integration begins until these pass.

## Phase 2: Exact Identifier Diagnostics

### Mechanism

Split current seed filtering into a pure prepared evaluator:

```text
seed program set
-> bind each program to exact motif once
-> Accepted(bound program) | Rejected(stable reason)
-> conservation check
-> accepted set for existing identifier
-> rooted diagnostic projection
```

The evaluator reuses `bind_pre_action_t1_program_to_motif_v1`; it does not
duplicate binding semantics or add a new program language.

### Stable reason policy

Reason codes come from a closed enum owned by the binding layer. Free-form error
text may be logged but cannot enter a scientific root. Unknown errors map to
`internal_unclassified` and are operational, never deterministic exhaustion.

### Conservation invariants

```text
seed_count = accepted_count + rejected_count
each seed root appears exactly once
histogram total = rejected_count
accepted root = canonical accepted program roots
result root binds existing identifier report root
```

For V8, the pure evaluator receives `OpportunityRoot` as its frozen causal
domain. The outer Freeze V8 root is attached only by the diagnostic provenance
envelope after evaluation.

### Privacy gate

Serialize diagnostics and scan bytes for support prompts, provider payloads,
expected responses, and known test secrets. Only typed bounded structures,
stable codes, counts, and hashes are allowed.

### Exit tests

- mixed accept/reject set;
- all rejected;
- all accepted;
- malformed/tampered disposition;
- deterministic ordering under input permutation;
- exact old identifier result parity when accepted set is unchanged;
- no private payload retention.

## Phase 3: Durable Authority Inputs

### One builder, two source owners

The pure support/artifact manifest builder lives in
`nando-operator-learning`. It receives validated typed objects and has no file
access.

Cold learner source owner:

- current in-memory prepared motif archive;
- current validated collection miner snapshot.

Authority source owner:

- durable topology archive;
- durable frame archive;
- validated read-only online collection checkpoint;
- installed discovery basis and active protocol registry.

Both call the same pure builder. Exact byte parity is required.

The authority also replays the frozen topology/frame prefixes through the same
pure join, motif, catalog, and queue builders. It must not validate a queue
against a client-supplied but incomplete catalog.

### Configuration changes

Extend `CertificationAuthorityConfigV1` with explicit paths for:

- topology archive;
- frame archive;
- online collection checkpoint;
- identifier artifact archive;
- installed scheduler policy.

Every constructor and test fixture must set these paths explicitly. Deriving
them by parent-directory convention is forbidden for new authority inputs.

### Artifact source race

Authority reads checkpoint metadata/root before and after projection. A change
returns `STALE_BEFORE_FREEZE`; no archive manifest, freeze event, or attempt is
authorized. Content-addressed objects written during the failed attempt may
remain orphaned and are ignored by all projections.

An empty relevant artifact set is a rooted projection tied to exact support
identities. `None` is not a valid causal manifest.

### Exit tests

- learner/authority parity from independently restored sources;
- omitted or reordered catalog candidate is rejected;
- missing frame/topology/checkpoint fail closed;
- unrelated artifact does not change candidate OpportunityRoot;
- relevant program or prediction does;
- checkpoint race appends no event;
- restart reads frozen archive after mutable checkpoint changes;
- replacement/tamper is rejected.

## Phase 4: Scheduler Policy

### Queue V4 construction

1. Build ordinary operator-blind ranking.
2. Build exact causal manifest for each retained readiness-PASS row, bounded to
   256 rows.
3. Project exact attempt index from signed epistemic history.
4. Annotate rows as `unseen` or `attempted_deterministic`.
5. Select the first ordinary-ranked unseen row.

No attempt state changes the underlying score or the order of rows.

### Authority minimum schema

Server policy, not `proposed.schema`, selects Queue V4 and Freeze V8. Any lower
client schema receives a downgrade error. A higher unknown schema also fails.

### Terminal attempt authority

The generic append route rejects new V8 diagnostics and deterministic terminal
verdicts. A dedicated request makes the authority:

1. restore the active Freeze V8 and archived causal inputs;
2. rerun the shared pure initial identifier evaluator;
3. reconstruct `IdentifierResultRoot` and `TerminalDiagnosticV1`;
4. classify deterministic versus operational/future-contingent disposition
   from a closed authority policy;
5. append diagnostic and matching verdict idempotently.

Client-provided result roots, reason histograms, dispositions, and timestamps
have no suppression authority.

### New runtime states

```text
WAITING_FOR_EVIDENCE
  no readiness-PASS row

WAITING_FOR_NOVEL_EVIDENCE
  readiness-PASS rows exist, all exact roots attempted

RESEARCH_BUDGET_COOLDOWN
  unseen root exists, budget currently closed
```

These are observation states. They append no event and consume no generation.

### Loop bound

Replace the effective multi-generation loop with:

- at most one new Freeze V8 per scheduler wake;
- additional transitions for the already-active generation may proceed;
- a terminal result cannot immediately create another freeze in the same wake;
- next candidate waits for the preregistered cooldown.

### Exit tests

- exact repeat creates no generation/event;
- new root creates exactly one freeze;
- no ready row, no novel row, and cooldown are distinct;
- budget reconstructs from signed events after restart;
- mechanism lane cannot affect epistemic attempt index;
- schema downgrade rejected;
- authority owns final timestamp and freeze bytes.
- forged diagnostic, disposition, blocker, or terminal timestamp is rejected.

## Phase 5: Compatibility And Crash Safety

### Reader matrix

```text
reader       V1-V7 suffix       V8 suffix       writer
legacy       PASS               expected FAIL   legacy only
Phase A      byte-exact PASS     PASS             V8 forced OFF
Phase B      byte-exact PASS     PASS             V8 ON
```

### Fault points

Inject failure after:

1. artifact object create;
2. artifact manifest create;
3. freeze journal file `fsync`;
4. freeze directory `fsync`;
5. anchor write;
6. diagnostic journal append;
7. terminal journal append;
8. cache publication.

Every restart must yield either the old valid prefix or the new complete valid
prefix. No partial state may mark an opportunity attempted.

### Exit tests

- legacy fixture roundtrip;
- all fault points;
- journal/anchor monotonicity;
- signed diagnostic parity;
- wire compression and maximum request budget;
- rollback fixture proving Phase A is the only safe post-V8 target.

## Phase 6: Replay, Measurement, And Value Gate

### Disposable inputs

- preserved production-copy snapshot;
- refreshed read-only production copy if obtainable without pausing services;
- deterministic 10x concatenated replay fixture;
- no connection from replay process to production sockets or state paths.

### Measurements

Record in one machine-readable receipt:

- exact roots considered, unseen, attempted, and legacy-unbound;
- repeated exact deterministic roots;
- diagnostic blocker and rejection histograms;
- wall and CPU time;
- peak RSS;
- queue, wire, journal, and archive bytes;
- current-to-10x scaling ratio.

### Value decision

```text
exact repeats found
-> exact index has immediate savings value

no exact repeats, but one rejection reason dominates new attempts
-> budget + diagnostic have value
-> next work is a separately preregistered code repair

no exact repeats and diagnostics remain unclassified
-> VETO deployment
-> improve diagnostic boundary first

superlinear replay, unbounded archive, or wire overflow
-> VETO deployment
```

The old `211 -> 4 families` number is forbidden as an acceptance metric.

## Phase 7: Final Verification And Critique

Run in this order, batching independent suites:

1. focused opportunity/diagnostic/authority tests;
2. full `nando-operator-learning` tests;
3. full `nando-transition-serving` tests;
4. legacy response-actor tests touched by artifact projection;
5. `cargo fmt --check`;
6. strict Clippy on changed crates;
7. structural gate on actual code routes;
8. code-route gate with observed source locations;
9. composite deployment gate;
10. independent implementation critique against this preregistration.

Any failure stops before commit/push/deployment. Baseline failures must be
reproduced on the pinned baseline and reported separately; they cannot be
silently waived.

## Phase 8: Git And Entire

### Commit boundaries

Prefer three reviewable commits:

1. pure causal contracts and diagnostics;
2. authority/scheduler integration and compatibility;
3. summary API/dashboard only after backend truth.

Before each commit:

- inspect exact diff;
- ensure `graphify-out/` and all existing `S1C3*` evidence remain untouched;
- ensure no private production data entered Git;
- create/review an Entire checkpoint;
- push only after tests for that boundary pass.

## Phase 9: Deployment Phase A

Install reader-compatible cold learner, authority, and control binaries with V8
writer forced OFF.

Required live proof:

- legacy ledger and anchor replay exactly;
- scheduler continues old behavior;
- writer-disabled state visible in health;
- cold/authority/control PIDs survive observation;
- hot serving, Nginx, and connector PIDs unchanged;
- false accepts and parity failures remain zero;
- no ledger rewrite and no V8 event.

Rollback on any failure to the pre-Phase-A deployment.

## Phase 10: Deployment Phase B

Enable the V8 policy only after Phase A passes and record Phase A as the rollback
target.

Required live proof before waiting for natural traffic:

- authority minimum schema is V4/V8;
- cold learner and authority policy roots match;
- isolated authority parity probe passes without appending production history;
- budget state is visible;
- no service outside cold/authority/control restarts.

If a V1-V7 generation is active, it completes under its original immutable
contract before any V8 candidate is selected.

After the first natural V8 append, rollback may restore Phase A binaries only.
The natural suffix and newest anchor must be preserved.

## Phase 11: Summary API And HTML

Backend summary first, HTML second.

Display only decision-relevant facts:

```text
K1 basis                         1 / 3
Law #2                           NOT PROVED
ready natural candidates         N
new exact opportunities          N
attempted deterministic roots    N
legacy unbound terminals         N
current state                    evidence | novel evidence | cooldown | active
current exact blocker            code + count
next eligible research time      timestamp when cooldown
false accepts / parity failures  N / N
```

Do not show a coarse-family count as laws, semantics, or exhausted hypotheses.

Verify with the existing authenticated browser on desktop and mobile widths:

- no overflow;
- no JS errors;
- values equal summary API;
- refresh survives service restart;
- all tabs opened for verification are closed at the end.

## Phase 12: Natural Result And Next Decision

### Success route

```text
unique semantic class
-> independent future
-> BundleV4
-> verified CPU and economics
-> cleanup
-> LawCertificate #2
```

Only then may K1 move from `1/3` to `2/3`.

### Diagnosed failure route

If terminal diagnostics identify a dominant stable defect, freeze a new,
narrow preregistration for exactly that component. Examples:

- generator emitted no seed program;
- source-neutral program cannot bind a required motif role;
- collection artifact is absent for an otherwise complete support row;
- protocol-mode quotient maps a new program onto an active law;
- resource limit truncates every viable seed.

Do not change scheduler ranking to hide such a defect.

### Stop route

Pause further K1 engineering when any condition holds:

- no useful exact deduplication and no classified diagnostic;
- the next fix requires synthetic evidence or manual program mapping;
- false accepts or parity failures become nonzero;
- replay/resource budget fails;
- the full Law #2 route would require weakening independent future or cleanup.

## Efficient Execution Rules

To avoid another long retry loop:

1. One active route: exact opportunity only. No L2, Wave-mechanism, economics,
   or unrelated dashboard work enters this change.
2. One paper contract, one implementation preflight, one production-copy
   baseline. Reuse fresh results until an input changes.
3. Focused tests during coding; crate-wide suites once per completed phase; full
   gate once before commit and once on release binaries.
4. Batch independent reads and tests. Keep raw logs on disk and retain only
   counts, roots, failures, and receipts in conversation.
5. Two identical blockers in consecutive attempts stop retries and trigger
   direct diagnosis against the frozen manifest.
6. No browser work until backend summary is deployed.
7. No deployment retries. A failed transaction rolls back, records one receipt,
   and returns to code/paper diagnosis.

## Final Definition Of Done

```text
paper gates                              PASS
coarse-family authority                  REMOVED
exact causal manifest                    PASS
terminal diagnostic conservation         PASS
learner-authority parity                 PASS
exact repeated deterministic attempts    0
research budget                          ENFORCED
legacy byte/restart parity               PASS
Phase A rollback target                  RECORDED
Phase B writer                           DEPLOYED
hot/Nginx/connector interruption         0
false accepts / parity failures          0 / 0
dashboard/API parity                     PASS
Law #2                                   NOT PROVED unless full old route passes
```
