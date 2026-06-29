# L3 Self-Induced Semantic Grokking

This proof is the bounded grokking step after supervised L3 semantic grokking.

The older L3 path trains from explicit semantic facts. This path trains only
from:

```text
surface query -> answer label
```

It does not receive role labels, schema labels, route labels, or hidden frame
ids as training authority. Hidden operators are used only by the evaluator.

## Mechanism

```text
surface query + answer label
-> observed answer family
-> observed object slot
-> learned modular delta
-> learned surface field center
-> learned required center contract
-> heldout answer
```

The hidden task is operator-like:

```text
answer_slot = object_slot + learned_delta mod m
```

The learner sees only the text and answer label. It induces the latent operator
centers from repeated surface/answer regularities.

## What Is Proven

Current default proof:

```text
verdict: Proven
train_examples: 1536
heldout_examples: 384
hidden_operator_count: 4
induced_operator_count: 4
modulus: 251
train_surface_answer_only: true
hidden_frame_labels_used_for_training: false
schema_labels_used_for_training: false
manual_role_labels_used_for_training: false
hand_written_cue_rules_used: false
field_weights_learned: true
operator_delta_learned: true
center_grokking_trace_observed: true
train_accuracy_early: 1.0
heldout_accuracy_early: 0.25
train_accuracy_final: 1.0
heldout_frame_accuracy: 1.0
heldout_answer_accuracy: 1.0
average_center_gap: 19.553698
min_center_gap: 4.734352
exact_query_lookup_hits: 0
exact_answer_lookup_hits: 0
answer_binding_ablation_accuracy: 0.0
frame_field_ablation_accuracy: 0.25
frame_ablation_drop: 0.75
binding_ablation_drop: 1.0
role_swap_rejected: true
route_splice_rejected: true
surface_shuffle_rejected: true
false_accept_rate: 0.0
model_hot_bytes: 15336
naive_observation_bytes: 15728640
model_to_naive_ratio: 0.000975
```

## Meaning

This is not open-domain semantic parsing.

It is a bounded grokking proof:

```text
without semantic labels:
  surface examples induce latent operators
  operators carry modular answer binding
  heldout slots solve without lookup
  corrupted surfaces do not promote
```

The key difference from the supervised L3 proof:

```text
supervised L3:
  text -> learned cues -> known semantic frame

self-induced L3:
  text + answer signal -> induced operator center -> answer
```

## Current Boundary

This still does not prove:

```text
open-domain language understanding
general chat
world knowledge
unbounded semantic atom extraction
Nanda-style transformer Fourier circuit discovery inside a transformer
```

It does prove a small but important step:

```text
Wave can learn relation operators from surface/answer experience instead of
being handed semantic roles as labels.
```
