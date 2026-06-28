# Interference Advantage Map

## Purpose

NANDA should not claim that wave interference beats LLMs in general.

The right question is narrower:

```text
where does an interference/composite-mode verifier beat ordinary text similarity,
lookup, and LLM-style judging?
```

This document defines the first places to test that claim.

## Non-Goals

NANDA is not expected to beat an LLM at:

```text
open-ended writing
world knowledge
large-scale storage
general conversation
exact database lookup
```

Traditional systems are simpler and stronger there:

```text
BM25
vector search
cross-encoder rerankers
knowledge graphs
LLM judges
ordinary tables
```

NANDA only matters if it wins on structural verification under noise and
conflict.

## Candidate Win Zones

### 1. Role Binding

The same tokens appear, but roles differ:

```text
cat bites dog
dog bites cat
```

Bag-of-words and cosine similarity can treat these as very close. A wave
verifier should reject the wrong role binding:

```text
subject(cat) + action(bites) + object(dog)
!=
subject(dog) + action(bites) + object(cat)
```

Required baselines:

```text
exact token overlap
bag cosine
role-aware symbolic baseline
small LLM judge
NANDA triad composite score
```

NANDA only wins if the composite mode rejects role swaps without memorizing each
sentence as a direct key.

### 2. Triad Shape

The values differ, but the relation is the same:

```text
1, 2, 1
4, 9, 4
low, high, low
```

The local tokens are not the whole pattern. The relation can be a shared
half-wave:

```text
rise -> fall
```

Expected NANDA advantage:

```text
recognize a shape even when exact tokens are unseen
reject examples with matching tokens but wrong contour
```

Required baselines:

```text
exact lookup
cosine over token ids
hand-coded shape rule
LLM judge
NANDA local modes + composite triad mode
```

If a hand-coded rule wins, that is fine. NANDA only matters if it learns or
stores the relation compactly and generalizes across many shapes.

### 3. Noisy Reconstruction

One block is missing or corrupted:

```text
A ? C
```

A verifier must select the candidate whose local blocks and composite mode agree:

```text
A B C -> accepted
A X C -> rejected
```

Expected NANDA advantage:

```text
partial cue -> stable structural peak
decoy with high local similarity -> vetoed by composite mode
```

Required baselines:

```text
nearest string edit distance
cosine similarity
BM25
cross-encoder reranker
NANDA composite-mode retrieval
```

### 4. Conflict Veto

Retrieved evidence contains two plausible candidates:

```text
subject=A, action=buys, object=B
subject=A, action=sells, object=B
```

Both share many tokens. NANDA must reject the candidate whose triad relation does
not match the query role/shape.

Expected NANDA advantage:

```text
not just top similarity
but a stable peak with cold/conflict rejection
```

Required baselines:

```text
top-k vector score
LLM judge
symbolic role parser
NANDA verifier
```

### 5. Cold Rejection

A cold, untrained wave state must not produce the same accepted answer:

```text
trained(query) -> accepts target
cold(query)    -> rejects target
```

This is the practical anti-lookup test. If cold can accept the same peak, NANDA
has not proven stored structure.

## First Eval: `SPARSE-TRIAD-0`

Target:

```text
2 MB dense clique
4096 candidate patterns
3 routed blocks per pattern
1 composite triad mode per pattern
route width: 16 / 32 cells
```

Metrics:

```text
patterns
active_bytes
route_width
cells_touched_per_pattern
mode_touches_per_pattern
bytes_per_pattern
noisy_hits
role_swap_rejections
shape_conflict_rejections
cold_rejections
composite_mode_hits
baseline_cosine_accuracy
baseline_lookup_accuracy
baseline_shape_rule_accuracy
nanda_accuracy
```

Pass condition:

```text
NANDA must beat token/cosine baselines on noisy role/shape conflicts.
NANDA must not merely tie a hand-coded symbolic rule.
NANDA must give a cold-ablation proof for accepted peaks.
```

## First LLM Comparison

Do not compare against a general chatbot first. Compare against a constrained LLM
judge on synthetic conflict cases:

```text
input: query triad + 4 retrieved candidates
task: choose structurally valid candidate or reject all
```

LLM failure modes to measure:

```text
role swap accepted
plausible decoy accepted
missing block hallucinated
conflicting evidence not rejected
```

NANDA wins only if it is more reliable on these narrow structural conflicts or
if it achieves similar reliability with much lower runtime/memory.

## Decision Rule

If NANDA does not beat traditional baselines on these tests, it should not be
sold as an LLM replacement or verifier.

Then the honest role is smaller:

```text
research toy
visual model of wave memory
or internal feature generator for another verifier
```

If NANDA wins:

```text
NANDA = cache-local structural verifier for RAG/LLM systems
```
