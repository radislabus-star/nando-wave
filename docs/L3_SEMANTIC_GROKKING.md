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

## Current Hard Profile

The baseline proof is a bounded Linux-style command-provider profile. The
current hard proof keeps the same L1 -> L2 -> L3 mechanism and adds more
semantic pressure:

```text
package provides_command command   route=linux.command.provider
service executes_command command   route=linux.service.runtime
config  enables_service  service   route=linux.service.config
package installs_file    file      route=linux.package.file
```

Training examples use multiple paraphrases per frame:

```text
which package provides command cmd00042
find package for command cmd00042
command cmd00042 belongs to which package
package provider for command cmd00042
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
role-swap, route-splice, missing-evidence, and negative shortcut traps are rejected
frame ablation damages the result
```

## Proof Result

Current hard unit proof:

```text
verdict: L3SemanticGrokkingVerdict::Proven
train_examples: 16,000
heldout_examples: 4,000
relation_family_count: 4
paraphrase_template_count: 16
frame_count: 4
l2_center_count: 2,032
operator_count: 4
frame_accuracy: 1.0
answer_accuracy: 1.0
average_frame_gap: 0.1699056
frame_ablation_drop: 0.16986167
object_anchor_pass: true
evidence_requirement_pass: true
missing_evidence_blocked: true
role_swap_rejected: true
route_splice_rejected: true
negative_route_rejected: true
false_promotion_rate: 0.0
exact_lookup_heldout_hits: 0
model_hot_bytes: 1,092,216
naive_semantic_fact_bytes: 163,840,000
model_to_naive_ratio: 0.0066663576
semantic_grokking_ready: true
hard_profile_ready: true
```

Run:

```bash
cargo test -p nando-core \
  l3_hard_semantic_grokking_rejects_role_route_and_evidence_traps \
  -- --nocapture
```

## Meaning

This proves:

```text
L2 motif fields can promote a bounded semantic frame.
Heldout role bindings solve without exact fact lookup.
Frame selection is causal under ablation.
Role-swap and route-splice traps stay rejected.
Missing-evidence and negative-shortcut surfaces do not promote to EquationForm.
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

The next step is not L4/L5/L6 yet.

The next step is a larger hard L3 corpus:

```text
more relation families
withheld paraphrase families, not only heldout fillers
ambiguous anchors across domains
evidence-specific no-answer states
negative evidence routes with anti-wave scoring
measured false promotion under scale
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
