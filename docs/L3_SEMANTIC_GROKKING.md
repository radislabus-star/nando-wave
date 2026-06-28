# L3 Semantic Grokking

L3 is the first layer where Nando Wave may promote semantic atoms.

It does not skip L1/L2.
It consumes L2 motif fields, selects a semantic frame, then lets a semantic
relation operator solve a heldout role binding.

## Layer Chain

```text
raw surface
-> L1 4-gram/position centers
-> L2 motif centers over L1 center-id sequences
-> L3 frame center + role binding + semantic operator
```

## Current Bounded Profile

The current proof is a bounded Linux-style command-provider profile with two
competing frames:

```text
package provides_command command   route=linux.command.provider
service executes_command command   route=linux.service.runtime
```

Training examples teach both:

```text
which package provides command cmd00042
which service executes command cmd00042
```

Heldout slots are disjoint from training slots.

## What Is Learned

L3 learns:

```text
L2 motif field -> frame center
frame center   -> unknown role
frame center   -> object anchor
semantic facts -> relation operator
```

The object label is copied from the bounded surface slot after the learned
object anchor. That copy span is not itself semantic authority. Authority comes
from:

```text
frame selected from L2 motif center
semantic operator solves the role binding
heldout slot was not an exact training fact
role-swap and route-splice traps are rejected
frame ablation damages the result
```

## Proof Result

Current unit proof:

```text
verdict: L3SemanticGrokkingVerdict::Proven
train_examples: 16,000
heldout_examples: 4,000
frame_count: 2
l2_center_count: 937
operator_count: 2
frame_accuracy: 1.0
answer_accuracy: 1.0
average_frame_gap: 0.625
frame_ablation_drop: 0.625
role_swap_rejected: true
route_splice_rejected: true
exact_lookup_heldout_hits: 0
model_hot_bytes: 1,102,000
naive_semantic_fact_bytes: 163,840,000
model_to_naive_ratio: 0.006726074
semantic_grokking_ready: true
```

Run:

```bash
cargo test -p nando-core \
  l3_semantic_grokking_learns_frame_from_l2_and_solves_heldout \
  -- --nocapture
```

## Meaning

This proves:

```text
L2 motif fields can promote a bounded semantic frame.
Heldout role bindings solve without exact fact lookup.
Frame selection is causal under ablation.
Role-swap and route-splice traps stay rejected.
The semantic layer is much smaller than naive per-fact wave storage.
```

It does not prove:

```text
open-domain language understanding
general chat
arbitrary query parsing
world knowledge
free-form semantic extraction
```

## Next Honest Step

The next step is not a bigger cache.

The next step is a harder L3 corpus:

```text
multiple relation families
surface paraphrases
ambiguous anchors
negative evidence routes
grounding/evidence requirements
heldout across route families
```

Semantic promotion remains forbidden unless all of these pass:

```text
frame_pass
role_slot_pass
binding_pass
grounding_pass
heldout_pass
ablation_pass
role_swap_reject
route_splice_reject
false_promotion_rate bounded
```
