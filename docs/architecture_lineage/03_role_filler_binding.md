# Position 3: Role / Filler Binding

Anchor:

```text
docs/ARCHITECTURE.md:17
```

## Central Work

- Smolensky tensor product variable binding.
- Plate Holographic Reduced Representations / circular convolution.

Reference handles:

```text
Smolensky 1990:
  https://dl.acm.org/doi/10.1016/0004-3702%2890%2990007-M

Plate HRR:
  https://pubmed.ncbi.nlm.nih.gov/18263348/
```

Classical shape:

```text
role + filler -> bound representation
bound representation -> role-aware retrieval / unbinding
```

Examples:

```text
subject + Alice
object + Bob
slot_0 + token_A
source_role + output_slot
```

## Nando Wave Mapping

The current WavePredictor binding pressure is a role/filler problem:

```text
source slot -> output slot
role slot -> token identity
action -> target lane pressure
```

The current state-delta binding path is:

```text
state_before + rule_action_example
-> learned pressure over target_delta
-> role/action slot binding
-> flat parity check
-> ablation checks
```

In code, the important shape is:

```text
state_delta_role_binding_edges:
  (action_center, slot_id, sign_key) -> weight
```

The v2 ordered sequence gate showed that current binding can work under a
moderate pressure setting:

```text
lengths: 3..6
same-bag negatives
flat parity: exact
ablation without binding: collapse
```

## Stronger For Our Goal

The project's binding gate is stronger than a plain role/filler demo because it
rejects the easy paths:

```text
no target_id
no proof_rule_id authority
no concrete X lookup
no local_out_t runtime hack
same-bag correct/wrong candidates
flat runtime parity
binding ablation collapse
```

This is important because the model must learn:

```text
where a value goes
```

not just:

```text
which value is present
```

For ordered sequences, this is the first serious form of relation:

```text
token identity + output position + rule action
```

## Weak / Not Proven

The v3 pressure gate exposed the current limit:

```text
48 proof_rule_ids
lengths up to 8
8 output slots
dense rule/length/output-slot matrix
ordered_sequence_accuracy_milli: 269
flat_gap_parity_mismatches: 0
```

Interpretation:

```text
the runtime readout preserves the field;
the field does not separate action/rule/slot strongly enough.
```

Specific weak points:

```text
1. Action/operator motifs are not separable enough.
2. Role-binding form is fragile under dense matrix pressure.
3. Folded projection adds collision pressure.
4. There is no learned output phase center yet.
5. There is no full unbinding algebra like HRR/TPR.
```

## Next Proof / Debt

Take these into work:

```text
action separability proof:
  different rule actions must produce separable action centers.

role collision audit:
  measure folded and non-folded role/slot collisions.

output phase proof:
  only after evidence, test learned output phase centers.

binding basin:
  perturb source slots and measure whether correct output slots remain stable.

slot-ablation profile:
  remove action, role, slot, and binding channels separately.

operator consistency:
  prove that all output slots agree with one rule action, not independent slot guesses.
```

## Literature Update: Cleanup And Role Specificity

Date:

```text
2026-07-02
```

Why this was added:

```text
The v4 conditional paired-noise gate isolated a red strict decoder while
sequence energy stayed strong. The latest sign-aware diagnostic showed that
sign erasure is not enough to explain the failure: most wrong-role pressure
remains as same-sign folded collision.
```

Relevant classical answer from HRR / VSA:

```text
Plate HRR:
  bound structures decode into noisy approximations;
  the decoded item is cleaned up by an item/cleanup memory;
  frame-specific role vectors are stronger than generic role vectors;
  skipping intermediate cleanup is faster but increases errors;
  no single fixed threshold is reliable across different frame compositions.
```

Direct mapping to current Nando Wave:

```text
HRR noisy decoded vector
  ~= current strict-slot readout after role/filler binding

HRR cleanup memory
  ~= missing generic cleanup/readout stage for target lanes after binding

HRR frame-specific role vector
  ~= possible learned operator/surface-specific role disambiguation, but only
     if proved by ablation and not injected as target_id/proof_rule_id authority

HRR no-decision region
  ~= FIELD_UNSETTLED / insufficient gap, not a failure to be hidden
```

Relevant answer from recent role-filler binding work:

```text
Chen et al. role-filler binding:
  a model should recall arbitrary fillers for a role even when role/filler
  pairings violate training correlations;
  successful models use external memory-like components to store/retrieve
  role-filler pairs;
  generalization depends on sufficiently diverse fillers and architecture;
  correlation statistics may remain as bias even when nearest-neighbor accuracy
  looks correct.
```

Direct mapping to current v4 red gate:

```text
The current conditional failure should not be patched with a hardcoded output
slot. The classical direction is to test a generic cleanup / external-memory-like
readout for bound role/filler results, and to prove it still transfers arbitrary
fillers under same-bag negatives and heldout surfaces.
```

Proof debt created by this literature pass:

```text
1. Same-sign residual collision outcome:
   correlate residual collision with real gap/failure, not only static pressure.

2. Cleanup-memory candidate:
   test a generic learned cleanup stage for decoded lane pressure.
   It must not use target_id, proof_rule_id authority, concrete_x_lookup,
   fixed frame_id, or manual local_out_t.

3. Role-specificity candidate:
   test whether learned frame/operator-specific role vectors reduce collisions.
   It must be learned from rule_action/state structure and collapse under
   ablation, not be read from proof_rule_id.

4. No-decision margin:
   preserve FIELD_UNSETTLED when top-vs-runner-up gap is weak; do not turn
   weak strict readout into a fake green answer.
```

## Status

```text
relation to role/filler binding: YES
v2 ordered binding proof: PASSED
v3 dense binding proof: FAILED_CURRENT_ARCHITECTURE
v4 conditional paired-noise proof: RED_STRICT_DECODER / STRONG_SEQUENCE_ENERGY
manual local_out_t allowed: NO
next work: same-sign residual outcome / generic cleanup memory candidate
```
