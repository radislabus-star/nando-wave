# Nando CPU Coverage To 50% Plan V1

Status: `PREREGISTERED_IMPLEMENTATION_PLAN`

Date: 2026-07-24 Europe/Tallinn.

Scope: live Nando response-operator path.

This plan changes no runtime threshold, package authority, registry generation,
or provider fallback. It defines the measured route from the current two
ACTIVE operators to sustained verified CPU processing of at least 50% of
ordinary input tokens.

## 1. Product Gate

The target is not a momentary dashboard value.

```text
three consecutive mature M3 windows
each window:
  verified CPU input-token share >= 50%
  ordinary intents              >= 10,000
  duration                      >= 24 hours
  false accepts                  = 0
  runtime parity failures        = 0
  bridge loss                    = 0
  provider fallback              preserved
```

Only independently admitted CPU accepts count. Shadow executions, candidate
tokens, gross positive token mass, cache hits, fixture runs, and optimistic
upper bounds do not count.

## 2. Frozen Starting Point

### 2.1 Live product snapshot

Source: read-only local `/control/<key>/tokens` API.

```text
total input tokens                 6,438,226,495
miner-visible tokens                 914,188,530   14.199%
CPU-processed tokens                 224,434,743    3.486% lifetime

current epoch input tokens            476,445,771
current epoch CPU tokens              181,919,446   38.183%
current epoch gap to static 50%        56,303,440

ACTIVE packages                                2
crystallized input                             3
crystallized admissible                        2
HELD                                           1
generation delta                               0
bridge loss                                    0
active false accepts                           0
active parity mismatches                       0
```

The current-epoch denominator is moving. The 56.3 million-token gap is useful
for orientation but is not an implementation target and cannot prove M3.

### 2.2 Opportunity-board planning snapshot

Source: read-only reconstruction of the live miner checkpoint.

```text
ordinary intents                    5,916
ordinary tokens               905,738,609
CPU_VERIFIED                  136,983,932   15.1%
target at 50%                 452,869,305
verified-token gap            315,885,373
```

The opportunity board and live current epoch have different window ownership.
They must not be combined into one percentage. The board is used to rank work;
the live M3 windows determine product completion.

### 2.3 Disjoint opportunity classes

| Class | Intents | Unique tokens | Share of board | Meaning |
|---|---:|---:|---:|---|
| `CPU_VERIFIED` | 994 | 136,983,932 | 15.1% | Already independently verified |
| `EXECUTABLE_CANDIDATE` | 17 | 2,180,959 | 0.24% | Existing expression and verifier route may suffice |
| `MISSING_DSL_PRIMITIVE` | 15 | 1,472,649 | 0.16% | Law needs a missing generic VM/IR primitive |
| `MISSING_EXTERNAL_VERIFIER` | 1,137 | 181,805,159 | 20.1% | Action is observed but no independent verifier owns it |
| `UNEXPLORED_MULTI_SOURCE` | 3,557 | 557,668,032 | 61.6% | Multiple values/roles/steps are not yet induced |
| `NON_DETERMINISTIC_OR_CREATIVE` | 196 | 25,627,878 | 2.8% | Proven irreducible for the current deterministic VM |

The disjoint accounting identity holds:

```text
sum(class tokens) = ordinary tokens
```

The optimistic executable upper bound is 880,110,731 tokens, or 97.1%, but it
is not a coverage claim.

## 3. Why The Current Panel Is Insufficient

The dashboard exposes only aggregate counts:

```text
admission-ready cohorts  29
candidate input           3
admissible                2
HELD                      1
ACTIVE                    2
```

It does not expose:

- which effect law owns each cohort;
- unique marginal input tokens;
- overlap with ACTIVE packages;
- semantic version-space size;
- support/future proof basis;
- the exact blocker;
- the next distinguishing probe;
- projected verified gain.

The detailed information currently exists only inside the cold learner
checkpoint and a manual read-only diagnostic. That is an observability defect.
The gateway must not parse the 150+ MiB checkpoint.

## 4. P0: Coverage Opportunity Snapshot

The first implementation is a compact learner-owned snapshot.

```text
OpportunityBoard + candidate/generation state
-> CoverageOpportunitySnapshotV1
-> atomic cold publication
-> read-only gateway API
-> dashboard technical detail
```

### 4.1 Ownership

```text
nando-operator-learning
  owns classification, marginal accounting, blocker and next-probe data

nando-operator-persistence
  owns atomic snapshot publication and restart parity

nando-gateway-control
  reads the compact snapshot; it never opens the learner checkpoint

nando-transition-serving
  is unchanged
```

### 4.2 Per-cohort row

```text
CoverageOpportunityRowV1 {
    cohort_id
    effect_law_id
    action_class
    semantic_class
    operator_state
    unique_intents
    unique_marginal_input_tokens
    overlap_with_active_tokens
    support_basis
    future_basis
    semantic_version_space_size
    physical_adapter_count
    verifier_available
    missing_vm_primitive
    blocker
    next_distinguishing_probe
    projected_verified_tokens
    evidence_updated_at
}
```

No raw payload, user text, teacher response, exact answer, or physical secret
may enter this snapshot.

### 4.3 Required identities

```text
one ordinary intent belongs to exactly one opportunity class
sum class tokens = ordinary tokens
CPU_VERIFIED tokens = tokens attributed to admitted package receipts
sum marginal portfolio tokens <= unresolved tokens
ACTIVE overlap is subtracted before projected gain
same intent cannot buy coverage for two candidate packages
```

### 4.4 Snapshot budgets

```text
maximum rows                    256
maximum serialized bytes       1 MiB
publication interval           <= 60 s
gateway parse target           <= 5 ms
hot serving overhead           0
learner checkpoint scan        forbidden in gateway
atomic write + restart parity  required
```

### 4.5 STOP-C0

P0 passes only when:

```text
snapshot accounting identity       PASS
opportunity-board byte parity      PASS
ACTIVE attribution                 PASS
restart byte parity                PASS
raw-payload privacy scan           PASS
gateway renders top cohorts        PASS
authority files byte-identical     PASS
```

## 5. What The Existing Cohorts Actually Say

### 5.1 Admission-ready path

The read-only reconstruction currently reports 29 admission-ready cohorts.
All 29 are blocked by legacy receipt partition accounting:

```text
receipt_backed_partition_below_32
support: 0..32
future:  0
```

This does not mean all 29 need more examples. It means the old cohort route
cannot express the adaptive proof basis already used by the two ACTIVE scalar
operators.

Do not wait for 32 future rows. Do not lower 32 to another global number.
Each high-value cohort must be re-evaluated through:

```text
complete bounded search
-> one executable semantic class
-> sealed freeze
-> one or more genuinely distinguishing independent future observations
-> actor/verifier parity
-> AdaptiveProofBasis
```

### 5.2 CEGIS state

```text
61 CEGIS pools
├─ 19 winners
├─ 40 semantic_version_space_ambiguous
└─  2 negative_unseparable_at_current_representation
```

Routing:

```text
winner
  -> compute unique marginal tokens
  -> adaptive proof-basis audit
  -> crystallization/admission

ambiguous
  -> choose probe by guaranteed version-space split / verifier cost
  -> collect only that distinguishing evidence

negative unseparable
  -> improve pre-action structural representation
  -> or permanently ABSTAIN
```

### 5.3 Legacy generation state

```text
58 generation records
├─ 55 support_rows_below_32
├─  2 future_rows_below_32
└─  1 legacy-ready
```

These counts are diagnostic debt, not the new readiness rule. Migration is
allowed only for a cohort with a valid adaptive proof capability.

## 6. Gross Action-Family Ranking

The frame-level numbers below are deduplicated by relation-frame ID but remain
gross historical masses. Different surfaces and backfills can still overlap
with the opportunity-board window. They rank investigation; they cannot be
added to predict product coverage.

| Rank | Observed action surface | Gross tokens | Rows | Sessions |
|---:|---|---:|---:|---:|
| 1 | `function:write_stdin` | 1,743,364,113 | 12,209 | 10 |
| 2 | `custom_tool:exec/write_stdin` | 967,311,396 | 5,823 | 10 |
| 3 | `function:wait` | 608,527,171 | 9,223 | 262 |
| 4 | `custom_tool:exec/exec_command` | 412,453,705 | 2,544 | 9 |
| 5 | `custom_tool:exec/write_stdin(empty)` | 168,615,371 | 941 | 11 |
| 6 | `custom_tool:exec/update_plan` | 8,851,606 | 52 | 2 |

The observed Ctrl-C surfaces contain only 280,324 gross tokens. The HELD
Ctrl-C candidate is therefore a safety boundary, not the first coverage
priority.

## 7. Portfolio Arithmetic

Projected gain for one candidate package:

```text
marginal_tokens
  = unique unresolved intent tokens accepted by this package
  - overlap with every ACTIVE or higher-priority candidate

projected_verified_tokens
  = marginal_tokens
  * measured safe-accept rate
  * verifier availability
  * frozen-future transfer rate
```

Every factor is derived from immutable evidence. No manual optimism factor is
allowed.

Portfolio ordering:

```text
projected_verified_tokens
-----------------------------------------
learner cost + verifier cost + hot bytes
```

Safety vetoes dominate this score.

## 8. Quantitative Route To 50%

The opportunity-board target is 452,869,305 verified tokens.

### 8.1 Fast pipeline control

Convert the 17 `EXECUTABLE_CANDIDATE` intents:

```text
maximum additional tokens        2,180,959
maximum board share afterward       15.365%
```

Purpose: prove automatic cohort → proof basis → generation delta → CPU
without manual registry work. This is a pipeline control, not the coverage
strategy. Spend at most one bounded implementation slice before moving to the
larger classes.

### 8.2 External verifier portfolio

`MISSING_EXTERNAL_VERIFIER` is the largest near-term class:

```text
unique tokens                   181,805,159
maximum board share after
EXECUTABLE + verifier class         35.437%
```

Required work:

1. Split this class by source-neutral effect law and capability protocol.
2. Rank by unique marginal tokens.
3. Implement generic verifier contracts, not function-name allowlists.
4. Independently reconstruct roles, arguments, constants and preserved frame.
5. Admit each law as a separate immutable package.

Priority verifier families:

```text
typed capability invocation
scalar continuation
bounded custom-tool wrapper
status/result projection
deterministic command completion state
```

Even perfect conversion of this entire class does not reach 50%.

### 8.3 Missing VM/DSL primitives

Current direct mass is only 1,472,649 tokens:

```text
maximum board share after executable
+ verifier + current DSL class              35.600%
```

This class is low direct value but may unlock multi-source laws. Add a
primitive only when at least one high-marginal frozen candidate requires it.

Expected generic families:

```text
COMPARE
FILTER
MAP
COUNT
ASSERT_GUARD
BRANCH
CALL_OPERATOR
FORMAT
```

Each primitive needs:

```text
bounded bytecode semantics
independent reference verifier
malformed/exhausted fail-closed tests
hot-page or extension-page budget
restart parity
```

### 8.4 Multi-source operator portfolio

This is the required strategic step:

```text
UNEXPLORED_MULTI_SOURCE tokens   557,668,032
```

The detailed implementation route is preregistered in:

```text
plans/multi-source-discovery-v1/MULTI_SOURCE_DISCOVERY_PLAN_V1.md
```

After ideal conversion of executable, verifier and current DSL classes, the
remaining gap is:

```text
130,426,606 unique verified tokens
= 23.388% of the multi-source class
```

The first rich families should be selected by the P0 marginal ledger, not by
names. Expected structural shapes include:

```text
two or more values from tool output
-> bind each value to a request role
-> compute relation/transform
-> preserve unrelated frame
-> render typed output

status + value
-> choose branch
-> emit structurally bound response

collection + predicate
-> filter/map/count
-> render result

operator A -> operator B
-> bounded CompositionDag
-> independently unfold and verify both
```

Natural rich-operator route:

```text
independent partial surfaces
-> source-neutral relation fragments
-> competing circuit blueprints
-> cross-plane phase coherence
-> unique connected circuit
-> exact-memory cleanup
-> crystallized OperatorPage/extension
-> runtime role grounding
-> independent verifier
-> external admission
```

No single training surface may contain or authorize the entire rich law.

## 9. Implementation Sequence

### R0. Compact opportunity observability

```text
CoverageOpportunitySnapshotV1
-> gateway API
-> dashboard top-20 opportunities
-> token conservation and overlap accounting
```

Exit: `STOP-C0`.

### R1. One automatic low-risk package

Select the highest-value `EXECUTABLE_CANDIDATE` row that already has:

```text
unique semantic class
external verifier
independent future
zero applicability negatives accepted
```

Run the entire automatic path:

```text
candidate
-> reconstructed proof
-> generation delta > 0
-> atomic registry generation
-> hot reload
-> real CPU receipt
```

No manual registry replacement.

Exit: `STOP-C1-FIRST-DELTA`.

### R2. External verifier expansion

Process `MISSING_EXTERNAL_VERIFIER` by descending unique marginal token value.
One generic verifier family at a time.

Exit for every package:

```text
wrong accepts                 0
parity failures               0
cross-family negative accepts 0
matched shadow p99            <= 1 ms
hard ceiling                  <= 2 ms
```

Portfolio checkpoint: projected frozen-board share >= 35%, then confirm it on
live traffic rather than claiming the projection.

Exit: `STOP-C2-VERIFIER-PORTFOLIO`.

### R3. Adaptive migration of existing winners

For the 19 CEGIS winners and 29 legacy-ready cohorts:

1. Join to the P0 marginal ledger.
2. Remove ACTIVE overlap.
3. Discard zero-value or stale cohorts.
4. Re-run complete semantic candidate search.
5. Migrate only cohorts with one executable class.
6. Request the smallest distinguishing future basis.
7. Produce `AdaptiveProofBasis`.
8. Submit through external admission.

Do not reinterpret old `32 + 0` receipts as adaptive proof.

Exit: `STOP-C3-ADAPTIVE-MIGRATION`.

### R4. Representation repair for ambiguous cohorts

For 40 ambiguous pools:

```text
formal version space
-> candidate distinguishing probes
-> maximum guaranteed split / probe cost
-> stable hash tie-break
-> new evidence
```

For the two unseparable pools:

```text
missing pre-action relation identified
-> source-neutral structural atom added
-> old and new representation ablation
-> otherwise permanent ABSTAIN
```

No teacher-only or post-action atom may break a runtime tie.

Exit: `STOP-C4-REPRESENTATION`.

### R5. Rich multi-source operators

Select enough non-overlapping portfolios to produce at least 130.4 million
projected verified tokens after R1-R3, then implement in descending
value/cost order.

Every rich law must prove:

```text
circuit causes role selection and computation
renderer reads VM output
independent verifier repeats derivation
phase ablations abstain
exact episode memory removed
new topology transfers
```

Exit: `STOP-C5-RICH-PORTFOLIO`.

### R6. M3 closure

Start a new immutable evaluation epoch. Historical false accepts remain in the
audit history but cannot be silently erased or counted as active errors.

```text
window 1  >= 50%, >= 10k intents, >= 24h, safety zero
window 2  >= 50%, >= 10k intents, >= 24h, safety zero
window 3  >= 50%, >= 10k intents, >= 24h, safety zero
```

Any active false accept, parity mismatch, authority drift, or denominator
failure vetoes the current window and triggers package revocation/repair.

Exit: `M3 = YES`.

## 10. Automatic Generation Contract

Manual publication is not part of the finished system.

```text
cold learner
  owns immutable candidate bundle publication
  cannot issue authority or write the ACTIVE registry
        |
        v
external admission controller
  detects bundle or proof change
  independently reconstructs package proof
  computes additive package merge and generation delta
  issues a bounded authority lease only for an accepted package
  cannot mutate the hot generation in place
        |
        v
registry publisher
  writes the inactive registry generation
  fsyncs bytes and directory metadata
  atomically switches the generation pointer
  retains the previous generation for rollback
  cannot create or widen an authority lease
        |
        v
hot serving
  validates generation root and lease
  reloads the immutable generation
  executes only the admitted package scope
  cannot promote a candidate
```

`generation delta = 0` means no new proven package and must not rewrite the
registry.

No owner in this route may both propose a package and authorize it. No owner
may infer missing proof from successful publication or successful loading.

## 11. Dashboard Contract

The main view remains small:

```text
through Nando
seen by miner
processed on CPU
current epoch CPU share
ACTIVE / HELD / generation delta
```

The technical detail must expose:

```text
top cohort opportunities by unique marginal tokens
effect/action class
version-space size
support/future proof basis
blocker
next distinguishing probe
projected safe gain
ACTIVE overlap
```

Auto-refresh must preserve the selected technical section.

## 12. Forbidden Shortcuts

- Summing overlapping `positive_tokens` across buckets.
- Ranking by row count instead of unique input tokens.
- Reintroducing universal `32 support + 32 future`.
- Treating `admission_ready_cohorts` as admitted packages.
- Adding function names as generic law authority.
- Letting a candidate or report authorize itself.
- Using actor-selected values as verifier truth.
- Counting shadow tokens as CPU savings.
- Widening an ACTIVE package instead of admitting a new immutable package.
- Mutating an ACTIVE generation in place.
- Hiding historical false accepts or changing the denominator between windows.
- Optimizing Ctrl-C first merely because it is currently visible as HELD.

## 13. Verification Matrix

| Layer | Required checks |
|---|---|
| Snapshot | conservation, overlap, privacy, restart parity |
| Discovery | complete search, version-space report, deterministic tie handling |
| Grokking | whole-circuit coherence, no/shuffled/magnitude ablations |
| Compiler | bounded bytecode, unknown opcode rejection |
| Runtime | role grounding, action equivalence, exhausted → ABSTAIN |
| Verifier | independent derivation, preserved frame, negative controls |
| Admission | reconstruction, future, negatives, parity, revocation |
| Deployment | atomic generation, rollback, lease reload |
| Economics | unique denominator, actual CPU receipt, three mature M3 windows |

## 14. Current Priority

```text
P0  expose compact per-cohort opportunity truth
P1  prove one automatic generation delta without manual publication
P2  convert high-value MISSING_EXTERNAL_VERIFIER families
P3  migrate high-value existing winners to AdaptiveProofBasis
P4  induce rich multi-source operators for at least 130.4M unique tokens
P5  complete three verified >=50% M3 windows
```

The central engineering conclusion is:

> Two ACTIVE operators prove the route. Reaching 50% now requires a
> token-ranked portfolio of new independently verified laws, not weaker gates
> and not more evidence for every cohort indiscriminately.

## 15. Evidence Used For This Plan

```text
live API:
  http://127.0.0.1:8787/control/<redacted>/tokens

live checkpoint:
  /var/lib/nando-wave/transition/response-online-miner.checkpoint
  SHA-256 28de8882a54e0a8e27d0fd5d633061f5158478ebf6b3bb74ea2e7d469f007dec

verified relation frames:
  /var/lib/nando-wave/transition/response-relation-frames-v4-verified.jsonl
  SHA-256 015efbc76d5b92e2d2a8e114a26c4a209081edb91eb323c06cec9d7b45453e83

read-only diagnostic snapshot:
  SHA-256 3a9db473bba5c118a7326d9fba473953e9b7e0bda974c79202e15739db0c14f3
```

The live files continue to change. These hashes identify only the snapshot
used to preregister this plan.
