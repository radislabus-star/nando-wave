# K1 Terminal Failure Quotient V2 Critique

Status: final rejection of the V1 selection policy. No implementation or
deployment authority.

Date: 2026-08-13.

## Verdict

The V1 terminal-failure quotient must not be deployed.

It solves a real operational symptom, repeated empty identifier runs, by
grouping failures into broad pre-identifier families and demoting a family
after four failures. That policy is not causally justified. In the current
implementation, one family can contain many different unknown L1 laws. A
family-level negative would therefore turn failure to identify one program
into authority to suppress other programs that have never been tested.

The safe replacement is exact causal experiment deduplication:

```text
same exact identifier inputs
-> same OpportunityRoot
-> one deterministic terminal attempt is enough
-> no second generation for that exact root

different identifier inputs
-> different OpportunityRoot
-> old failures have no suppression authority
```

Coarse semantic or consequence groups may remain dashboard diagnostics. They
must never rank, demote, exclude, freeze, or certify a candidate.

## Why The V1 Proof Was Invalid

### P0: the semantic signature does not identify action meaning

`semantic_novelty_signature_root_sha256` is currently sealed from the
`consequence_type` alone in
`crates/nando-transition-serving/src/k1_natural_scheduler_runtime/evidence/motif.rs`.

Consequently, the observed collapse

```text
211 empty terminals -> 4 families
```

mostly means that terminals were bucketed into broad output kinds such as
`scalar`, `record`, `collection`, or `rendered_sequence`. It does not show four
causal laws, four program families, or four exhausted scientific hypotheses.

### P0: the threshold four has no valid interpretation

For identical deterministic inputs, one completed terminal result is enough to
avoid paying for the same computation again. For different causal inputs, four,
forty, or four hundred failures do not authorize suppression. The V1 threshold
is therefore simultaneously too large for exact duplicates and unsafe for
non-identical experiments.

### P0: the identifier discards the evidence needed to explain failure

Motif binding currently filters seed programs with `.is_ok()` in
`crates/nando-operator-learning/src/multi_source/identification.rs`. Rejection
reasons and per-program dispositions disappear. The final blocker
`motif_program_candidates_empty` proves only that the surviving set is empty;
it does not prove why it is empty or whether the same causal input was tested.

Without a rooted terminal diagnostic, historical V6 terminals are useful
observations but cannot become an authoritative blacklist.

### P0: client schema currently selects authority strictness

`selection_authority.rs` branches on `proposed.schema`. A client can therefore
choose an older queue path and avoid the new policy. The authority must select
the minimum accepted schema from installed server policy and reject downgrade,
not infer strictness from client bytes.

### P1: candidate artifacts can change without waking a waiting scheduler

`service.rs` evaluates waiting-tick reuse before reading current collection
artifacts. A newly durable collection program can therefore exist while the
scheduler keeps reusing `WAITING_FOR_EVIDENCE`. Artifact-source identity must be
sampled before reuse and included in the reuse key.

### P1: candidate artifacts are not durably frozen for replay

The active identifier receives artifacts from the current mutable online
collection checkpoint. The existing freeze does not retain the exact
identifier-relevant artifact projection. A later checkpoint update can change
restart behavior for an already frozen generation.

The repair must archive the exact bounded relevant projection before appending
the freeze and make active-generation replay read that immutable archive.

### P1: old freezes do not bind the full causal input

Legacy V6 freezes bind motif and evidence roots, but not every identifier input,
especially the relevant collection artifact projection and exact rejection
semantics. Old terminal history is therefore diagnostic-only. It must not be
backfilled into an exact attempt index by guesswork.

### P1: the runtime can spend sixteen transitions per tick

`service.rs` permits up to sixteen state transitions in one scheduler tick.
Exact deduplication removes exact repeats but does not bound a stream of distinct
opportunities. A separate durable research budget and a one-new-freeze-per-tick
rule are required. Budget cooldown is an operational state, not evidence
waiting and not a scientific terminal result.

### P1: a new freeze schema creates a rollback trap

The existing wire structs use `deny_unknown_fields`. Once a V8 event is appended,
an older binary can fail to replay the natural ledger suffix. A deployment that
writes V8 immediately cannot safely roll back to the current production binary.

Deployment must therefore have two phases:

```text
Phase A: V8-capable readers, V8 writer disabled
Phase B: same compatible readers, V8 writer enabled
rollback target after first V8 write: Phase A only
```

## What Can Be Salvaged

The uncommitted V1 implementation contains useful mechanical work, but none of
its coarse-family policy has authority.

Safe to preserve after refactoring:

- explicit queue-root binding in the freeze;
- learner/authority queue reconstruction tests;
- append-only terminal-history projection patterns;
- schema-version plumbing and legacy decode tests;
- diagnostic test fixtures that do not assert family suppression.

Must be removed or converted to observation-only:

- `terminal_failure_family_novelty_rank` as a ranking input;
- `terminal_failure_exhausted_families` as an authority decision;
- the threshold of four;
- family demotion or fallback ordering;
- any claim that `consequence_type` is a semantic law identity.

## Accepted Replacement

```text
durable natural evidence
-> authority-verifiable exact support selection
-> exact relevant artifact projection
-> IdentifierCausalInputManifestV1
-> OpportunityRoot
-> ExactAttemptIndexV1
   |-- unseen deterministic root -> one freeze and identifier run
   `-- completed deterministic root -> WAITING_FOR_NOVEL_EVIDENCE

identifier
-> TerminalDiagnosticV1
   |-- seed program set
   |-- accepted motif-bound set
   |-- per-reason rejection histogram
   `-- exact result root
-> signed terminal event
-> append-only exact attempt memory
```

The complete contract is
`K1_EXACT_EXPERIMENT_OPPORTUNITY_PREREGISTRATION_V1.md`.

## Claim Boundary

Passing the replacement proves only:

```text
one exact deterministic identifier input
-> at most one paid terminal experiment
```

It does not prove Law #2, K1 OPEN, Natural L2, answer quality, Wave causality,
or that any coarse family is scientifically exhausted.
