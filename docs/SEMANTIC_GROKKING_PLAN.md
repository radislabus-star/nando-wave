# Semantic Grokking Plan

Goal: make Nando Wave learn semantic atoms, not just recognize surface
patterns.

## Layer Status

L1 Center Memory is the base layer.

See:

```text
docs/L1_CENTER_MEMORY.md
docs/L2_CENTER_MEMORY.md
docs/L3_SEMANTIC_GROKKING.md
```

Current L1 status:

```text
240k Russian train / 60k heldout
surface 4-gram centers + residual refs
promotion_ready_for_l2 = true
```

Current L2 status:

```text
240k Russian train / 60k heldout
L1 center-id sequence motifs + residual refs
promotion_ready_for_l3 = true
```

L2 is still surface/motif grokking, not semantic grokking.

Current L3 status:

```text
hard bounded Linux semantic profile
L2 motif field -> learned CueField -> learned contrastive semantic field -> L3 center convergence -> EquationForm
heldout semantic role binding
4 relation families
16 paraphrase surfaces
manual_cue_rules_used = false
cue_field_learned = true
cue_contrastive_training_used = true
cue_extractor_learned = true
cue_edge_count = 124189
cue_accuracy = 1.0
cue_ablation_drop = 3.4282615
semantic_compiler_ready = true
manual_weight_table_used = false
field_weights_learned = true
contrastive_training_used = true
interference_edge_count = 53
interference_gap_lift = 3.4282615
nearest_wrong_center_suppressed = true
anti_field_ablation_drop = 0.3125
role-swap / route-splice / missing-evidence / negative-route traps
false_promotion_rate = 0.0
semantic_field_ready = true
semantic_grokking_ready = true
```

This is bounded semantic grokking, not open-domain language understanding.

## Claim Boundary

This is not general chat and not broad LLM readiness.

Target claim:

```text
semantic atom grokking for bounded profiles
```

## Required Moves

1. Scale the hard L3 semantic-field corpus.

   The dataset must hide a reusable rule behind role-complete facts:

   ```text
   subject_role + relation + route -> object_role/object
   ```

   Current hard L3 already has multiple relation families, paraphrases,
   learned contrastive field interference, nearest-wrong suppression,
   anti-trap lanes, role-swap traps, route-splice traps, missing-evidence
   blocks, negative-route blocks, heldout fillers, and ablation. The next
   corpus must add a larger interference matrix, withheld paraphrase families,
   cross-domain ambiguous anchors, evidence-specific no-answer states, and
   larger false-promotion stress.

2. Scale learned L2 -> L3 promotion beyond the current bounded profile.

   The current promotion layer already learns:

   ```text
   L2 motif field -> frame candidate
   L2 motif field -> role slot
   L2 motif field -> relation operator
   L2 motif field -> object anchor
   generic surface residual cues -> anti-wave shortcut signals
   ```

   Handwritten cue rules are no longer used at runtime. Bootstrap labels are
   still used to create the training target, so the next gap is not "does the
   CueField exist"; it is whether learned cue induction survives larger
   domains, withheld paraphrase families, stronger collisions, and
   evidence-specific no-answer states.

3. Strengthen the semantic grokking proof under scale.

   Pass criteria:

   ```text
   exact_lookup = 0
   heldout improves after training
   center_gap grows
   Fourier modes are causal
   Fourier ablation drops heldout
   role-swap is rejected
   route-splice is rejected
   random baseline is beaten
   ```

## Surface To Semantic Promotion Contract

Do not treat `SurfaceWave -> SemanticAtom` as parsing.

Treat it as promotion:

```text
SurfaceWave
-> reusable surface motifs
-> Frame candidate
-> Role slot candidates
-> Binding wave
-> Grounding/evidence
-> Heldout + ablation proof
-> SemanticAtom promotion
```

Mandatory rule:

```text
No SemanticAtom without frame + binding + grounding + heldout/ablation proof.
```

Meaning of each stage:

```text
Frame      = bounded relation context, e.g. linux_command_provider
Role slots = typed slots inside the frame, e.g. package, command
Binding    = wave operation tying roles, relation, route, and fillers together
Grounding  = evidence, task feedback, execution result, or source artifact
Heldout    = transfer to facts not seen as exact training rows
Ablation   = causal proof that the learned modes carry the behavior
```

Promotion gates:

```text
surface-only match         -> no authority
frame candidate only       -> no authority
role slot candidate only   -> no authority
binding without grounding  -> no authority
grounding without heldout  -> no authority
heldout without ablation   -> no authority
```

Accepted SemanticAtom requires:

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

## Non-Goals

Do not call the current WavePattern Compiler grokking.

Current state:

```text
wave-pattern recognition + gated EquationForm
```

Future target:

```text
learned surface/phase/operator compiler + heldout semantic transfer
```

## Size Target

Use sparse ternary waves for semantic atoms and operators.

Working estimate:

```text
1 semantic atom     ~= 64 bytes
1 semantic operator ~= 64 bytes
```

Small proof model:

```text
100k atoms + 100k operators ~= 10-20 MB
```

Useful domain model:

```text
1M atoms + 1M operators ~= 100-150 MB
```

Large multi-domain cache:

```text
10M atoms + 10M operators ~= 1.3-1.7 GB with indexes/metadata
```

Runtime rule:

```text
cold/warm memory may be GB-scale
hot active wave front should stay near 8-32 MB
```

Claim boundary:

```text
GB-scale storage is acceptable only if active reasoning uses a small hot window.
```
