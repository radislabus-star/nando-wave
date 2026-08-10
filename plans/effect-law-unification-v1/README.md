# Effect Law Unification Evidence Index

Current core status: [`../../docs/CORE.md`](../../docs/CORE.md).

Owning roadmap:
[`EFFECT_LAW_UNIFICATION_REFACTOR_V1.md`](EFFECT_LAW_UNIFICATION_REFACTOR_V1.md).

Forward recursive-meaning architecture:
[`GROUNDED_MEANING_ARCHITECTURE_V1.md`](GROUNDED_MEANING_ARCHITECTURE_V1.md).

The forward contract keeps K1 operational laws, K2 grounded meanings, and the
historical W1/W2/W3 Wave representation layers in separate namespaces. It does
not rewrite or promote any frozen evidence receipt in this directory. K2 meaning
is goal-conditioned decision evidence, not a latent alias for one verified
transition.

This directory contains one migration ladder and its immutable checkpoint
evidence. Read the ladder in order; do not infer current authority from the
newest-looking filename.

## Current Boundary

```text
CanonicalEffectLawV3              COMPLETE
V1/V3 dual classification         COMPLETE
B1A version space                 INSUFFICIENT
B1B controlled fixture            CONTROLLED_FIXTURE_PASS
independent physical truth        BLOCK
AcceptedBindingLawEvidence        BLOCK
F4 Protocol Compiler              BLOCKED
production authority              false
```

## Evidence Ladder

```text
STOP-F0
  complete the frozen evidence denominator
  -> STOP_F0_EVIDENCE_COMPLETION.md

STOP-F1
  separate read-only diagnostics from state/runtime ownership
  -> STOP_F1_DIAGNOSTIC_OWNERSHIP.md

STOP-F2 / F2R / F2R2 / F2R3 / F2R4 / F2-V3
  converge from an over-closed law enum to evidence-bound,
  provenance-sealed CanonicalEffectLawV3
  -> STOP_F2_CANONICAL_EFFECT_LAW.md
  -> STOP_F2R_OPEN_EFFECT_LAW_CANDIDATE.md
  -> STOP_F2R2_EVIDENCE_BOUND_QUOTIENT_CANDIDATE.md
  -> STOP_F2R3_SEALED_EFFECT_LAW_CANDIDATE.md
  -> STOP_F2R4_TRUSTED_EFFECT_LAW_CANDIDATE.md
  -> STOP_F2_CANONICAL_EFFECT_LAW_V3.md

STOP-F3 / F3R
  compare V1 and V3 classifications and require pairwise discrepancy witnesses
  -> STOP_F3_DUAL_CLASSIFICATION_V1_V3.md
  -> STOP_F3R_PAIRWISE_DISCREPANCY_REPAIR.md

STOP-B1A
  build bounded binding-relation version space; retain ties as insufficient
  -> STOP_B1A_BINDING_EVIDENCE.md

STOP-B1B0 / B1B0R
  preregister trusted acquisition, interventions, lineage split, and budgets
  -> STOP_B1B0_PREREGISTRATION.md
  -> STOP_B1B0R_TRUSTED_ACQUISITION_BOUNDARY.md

STOP-B1B-S
  freeze label-blind support and the physical watermark
  -> STOP_B1B_S_SUPPORT_FREEZE.md

STOP-B1B-F0
  freeze future acquisition protocol before future evidence exists
  -> STOP_B1B_F0_FUTURE_ACQUISITION_FREEZE.md

STOP-B1B-F
  freeze disjoint label-blind future graphs
  -> STOP_B1B_F_FUTURE_FREEZE.md

STOP-B1B
  controlled causal adjudication; useful fixture evidence, not compiler authority
  -> STOP_B1B_CAUSAL_ADJUDICATION.md

NEXT
  split physical trial, trusted resolver, causal adjudicator, and report owners
  -> create private validated AcceptedBindingLawEvidence or INSUFFICIENT
  -> only then start F4
```

## Receipt Rule

JSON artifacts are frozen machine receipts. Do not edit them to reflect a later
architectural review. In particular,
`STOP_B1B_ADJUDICATION.json:f4_status=UNLOCKED_NOT_STARTED` records the original
controlled-run verdict; it is not current F4 authority.

Human STOP reports describe what was known at that checkpoint. A later review
may add an explicit addendum, but it must not rewrite denominators, hashes, or
observed outcomes. Current status is owned only by `docs/CORE.md`,
`docs/CURRENT_CORE_DECISION.md`, and the active roadmap header.

## Forbidden Shortcuts

```text
fixture label -> AcceptedBindingLawEvidence          forbidden
intervention id -> physical truth                    forbidden
report or raw JSON -> F4 input                       forbidden
phase/coherence score -> compiler authority          forbidden
runtime binder -> self-verification                  forbidden
controlled PASS -> production authority              forbidden
```
