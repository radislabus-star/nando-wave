# L3 Semantic Grokking

L3 is the first layer where Nando Wave may promote semantic atoms.

It does not skip L1/L2.
It consumes L2 motif fields, excites a learned contrastive L3 semantic field,
settles onto a semantic center, then lets a semantic relation operator solve a
heldout role binding.

## Layer Chain

```text
raw surface
-> L1 4-gram/position centers
-> L2 motif centers over L1 center-id sequences
-> L3 semantic field excitation
-> L3 center convergence
-> EquationForm
-> learned answer binding operator
-> semantic operator fallback
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
L2 motif field               -> weak frame activation
L2 motif + generic surface cues -> learned role/relation/anchor/binding cues
learned attraction lane      -> compatible semantic centers
learned repulsion lane       -> nearest wrong centers
learned anti-trap lane       -> forbidden overclaim centers
settled center               -> EquationForm
semantic facts               -> answer binding operator
semantic facts               -> relation operator fallback
```

Bootstrap role/relation labels are used only to create the training target.
Runtime inference does not call the manual cue rules. It reads the learned
CueField edges produced from L2 motifs plus generic normalized word/bigram cue
tokens.

The surface residual cues are not allowed to be the only authority path. The
compiler now also requires an independent L2 structural support pass with the
surface word/bigram residual cues removed. Surface residuals may help fill the
bounded EquationForm, but they cannot by themselves grant answer authority.

The object label is copied from the bounded surface slot after the learned
object anchor. That copy span is not itself semantic authority. Authority comes
from:

```text
surface motifs excite several centers
learned contrastive field raises the compatible center gap
learned repulsion suppresses the nearest wrong center
learned anti-trap lane blocks complete-but-forbidden shortcuts
semantic field settles to one center
learned answer binding operator solves the heldout answer
heldout slot was not an exact training fact
role-swap, route-splice, missing-evidence, and negative shortcut traps are rejected
attraction/repulsion/anti-field ablations damage the result
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
answer_binding_operator_count: 4
frame_accuracy: 1.0
answer_accuracy: 1.0
average_raw_field_gap: 0.1378287
average_settled_field_gap: 3.56609
interference_gap_lift: 3.4282615
average_interference_energy: 3.6666133
cue_edge_count: 124,189
manual_cue_rules_used: false
cue_field_learned: true
cue_contrastive_training_used: true
cue_extractor_learned: true
cue_accuracy: 1.0
cue_margin_min: 3.59375
cue_ablation_drop: 3.4282615
wrong_cue_suppressed: true
shortcut_stress_examples: 256
shortcut_frame_accuracy: 1.0
shortcut_answer_accuracy: 1.0
shortcut_answer_binding_ablation_accuracy: 0.0
answer_binding_learned: true
answer_lookup_only: false
role_binding_ablation_drop: 1.0
structural_without_residual_rate: 0.75
lexical_overlap_split: true
surface_shortcut_rejected: true
residual_cue_ablation_drop: 1.5911419
motif_pair_ablation_drop: 0.0
no_exact_bigram_lookup: true
same_words_role_swap_rejected: true
semantic_compiler_ready: true
interference_edge_count: 53
manual_weight_table_used: false
field_weights_learned: true
contrastive_training_used: true
heldout_margin_min: 2.4270833
nearest_wrong_center_suppressed: true
attraction_ablation_drop: 3.2829638
repulsion_ablation_drop: 0.17827344
anti_field_ablation_drop: 0.25
frame_ablation_drop: 3.4282615
object_anchor_pass: true
evidence_requirement_pass: true
missing_evidence_blocked: true
role_swap_rejected: true
route_splice_rejected: true
negative_route_rejected: true
false_promotion_rate: 0.0
exact_lookup_heldout_hits: 0
heldout_answer_exact_lookup_hits: 0
model_hot_bytes: 2,583,460
naive_semantic_fact_bytes: 163,840,000
model_to_naive_ratio: 0.015767407
semantic_field_ready: true
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
L2 motif fields plus learned contrastive interference can settle onto a bounded semantic center.
Learned CueField can induce role/relation/anchor/binding cues from L2 motifs and generic surface residual cues.
Heldout role bindings solve without exact fact lookup.
Heldout shortcut answers solve through a learned answer binding operator.
Removing role/slot binding drops shortcut answer accuracy from 1.0 to 0.0.
Semantic field convergence is causal under interference ablation.
Cue induction is causal under cue ablation.
Shortcut stress uses heldout surfaces with no exact normalized bigram overlap.
Surface residual cues are measured and cannot be the sole authority path.
Nearest wrong centers are actively suppressed by learned repulsion.
Complete-but-forbidden shortcuts are blocked by learned anti-trap lanes.
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
full answer solving for every shortcut-stress alpha label
```

## Next Honest Step

The next step is not L4/L5/L6 yet.

The next step is not cue induction anymore. It is making shortcut stress harsher
without mixing it with a new semantic-operator claim:

```text
more relation families
more withheld paraphrase families with no exact normalized bigram overlap
raise structural_without_residual_rate above 0.75
separate EquationForm transfer from answer-solving transfer
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
