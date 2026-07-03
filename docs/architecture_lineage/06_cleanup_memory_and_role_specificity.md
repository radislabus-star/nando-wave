# Position 6: Cleanup Memory / Role Specificity

Anchor:

```text
data/rule_logic_operator_battery_v4/NEXT_MECHANISM_CONTRACT.md
```

## Why This Card Exists

The current v4 conditional paired-noise red gate is not a generic failure:

```text
sequence energy: strong
strict slot readout: weak
sign-aware role matching: safe but too weak
same-sign residual collision: still high
```

This shape matches a known classical problem:

```text
bound distributed structures decode into noisy approximations;
raw readout is not enough;
cleanup / role-specific disambiguation is required.
```

## Central Works / Known Ideas

Smolensky tensor product variable binding:

```text
role/filler binding is represented by explicit value-variable products.
Interference can be analyzed; binding is not just coactivation.
```

Reference:

```text
https://www.microsoft.com/en-us/research/publication/tensor-product-variable-binding-representation-symbolic-structures-connectionist-systems/
```

Plate Holographic Reduced Representations:

```text
distributed role/filler structures are decoded noisily;
the decoded result should be cleaned up by associative memory;
frame-specific roles reduce ambiguity compared with generic roles;
skipping cleanup is faster but less accurate;
fixed thresholds are unreliable across different frame compositions.
```

Reference:

```text
https://redwood.berkeley.edu/wp-content/uploads/2020/08/Plate-HRR-IEEE-TransNN.pdf
```

Chen et al. role-filler binding with schematic knowledge:

```text
a model performs real role/filler binding only if it recalls arbitrary fillers
for a role even when the pairing violates training correlations;
successful systems use external-memory-like storage/retrieval;
training diversity and architecture both matter.
```

Reference:

```text
https://www.dpmlab.org/papers/peerj-11046.pdf
```

Hummel / Holyoak compositional connectionism:

```text
conjunctive coding alone can break role/filler independence;
dynamic binding alone is capacity-limited for storage;
true compositionality needs binding, role/filler independence, and integration
of multiple bindings.
```

Reference:

```text
https://reasoninglab.psych.ucla.edu/wp-content/uploads/sites/273/2021/04/Hummel_Et_Al_2004_AAI04.pdf
```

## Nando Wave Mapping

Current readout:

```text
state_t + rule_action
-> active role/action centers
-> state_delta_role_binding_edges
-> target lane pressure
```

Observed failure:

```text
correct whole sequence often has better sequence energy,
but individual output slots can still have negative gap.
```

Classical translation:

```text
The operator attractor is present,
but decoded role/filler lane pressure needs cleanup.
```

## What Not To Do

Do not patch the red gate with:

```text
target_id
proof_rule_id authority
concrete_x_lookup
manual local_out_t
fixed frame_id
hand-coded bind(X)
surface-family special case
```

Those would be substitutions, not a compact transferable operator.

## Candidate Directions Allowed By Literature

### 1. Generic Cleanup Memory

Meaning:

```text
After role/filler readout produces noisy lane pressure, apply a learned
associative cleanup that sharpens the decoded filler without knowing the answer.
```

Required gates:

```text
cleanup ablation collapses strict slot readout;
flat/runtime parity remains exact;
same-bag negatives remain hard;
heldout fillers/surfaces still transfer;
forbidden flags remain false.
```

### 2. Learned Role Specificity

Meaning:

```text
Generic role vectors may be too ambiguous.
The model may need learned operator/frame-specific role variants.
```

Allowed only if:

```text
the role variant is induced from rule_action/state structure;
it is not proof_rule_id authority;
it is not target_id;
it collapses under ablation;
it improves same-sign residual collision.
```

### 3. No-Decision Margin

Meaning:

```text
If top-vs-runner-up gap is weak, emit FIELD_UNSETTLED instead of a fake answer.
```

This preserves proof discipline:

```text
red gate stays red until strict readout is genuinely repaired.
```

## Next Proof Debt

The next runnable diagnostic should answer:

```text
Does same-sign residual collision predict the actual negative strict-slot gap?
```

If yes:

```text
test generic cleanup memory or learned role specificity.
```

If no:

```text
do not build cleanup yet; inspect another failure source.
```

## Residual Collision Outcome Result

Date:

```text
2026-07-02
```

Artifact:

```text
data/rule_logic_operator_battery_v4/diagnostics/conditional_paired_noise/seed_001/train_per_cell_2/conditional_residual_collision_outcome_cleanup8.log
```

Result:

```text
strict_row_accuracy_milli: 615
sequence_energy_accuracy_milli: 984
energy_pass_slot_fail: 284

train_slot_accuracy_milli after cleanup: 1000
train candidate min gap positive by epoch 3/8
```

Residual bucket accuracy:

```text
high_same_sign_residual: 859
mid_same_sign_residual: 890
low_same_sign_residual: 927
no_same_sign_residual: 923
```

Surface accuracy:

```text
business: 1000
symbols: 1000
network: 787
ru_words: 808
```

Interpretation:

```text
Same-sign residual collision is a real pressure but not the sole cause.
The decisive shape is energy/readout mismatch under surface pressure:
the correct operator is usually selected by sequence energy, but decoded
slot-level lane pressure remains noisy for network/ru_words.
```

Updated proof debt:

```text
Test cleanup memory as a generic readout stabilizer.
The cleanup candidate must improve strict slot readout when sequence energy is
already correct, and must collapse under cleanup ablation.

Do not test cleanup as an answer table.
Do not treat cleanup as proof_rule_id, target_id, concrete_x, local_out_t, or a
surface-family special case.
```

## Operator Compiler Update

Date:

```text
2026-07-02
```

Artifact:

```text
data/rule_logic_operator_battery_v4/diagnostics/OPERATOR_GROKKING_PROBE.md
data/rule_logic_operator_battery_v4/diagnostics/operator_grokking_probe_report.json
```

Result:

```text
train transitions -> compact operator program -> heldout application
train_rows: 5312
heldout_rows: 5312
compiled_operator_programs: 380
operator_program_conflicts: 0
heldout_accuracy_milli: 1000
```

Interpretation:

```text
Cleanup/readout remains a valid classical repair path, but it should not be the
primary philosophy if a one-pass operator compiler can recover the transition
program. The next architecture step is compiler-first:

one-pass induced operator program
-> compile into Wave weights / sequence energy / cleanup
-> optional epoch repair only if the compiled Wave gate remains red.
```

Boundary:

```text
This is a diagnostic stand, not a Wave-runtime proof. It uses normalized
rule_action_example as an operator key. The next gate must prove the induced
program inside the Wave field/readout path and keep forbidden substitutions
false.
```

Compiler/runtime boundary:

```text
Training epochs belong to table induction and proof diagnostics.
Runtime inference must use compiled tables and must not run training epochs.
```

## Current Status

```text
literature answer found: YES
implemented mechanism: NO
residual-collision outcome diagnostic: COMPLETE
next code step: compile one-pass induced operator programs into Wave path
repair path if red: generic cleanup/readout candidate with ablation
claim allowed: sequence energy strong, strict decoder still red
claim allowed: v4 corpus supports compact one-pass operator induction
claim forbidden: v4 conditional solved
claim forbidden: Wave runtime operator proof complete
```
