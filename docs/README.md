# Nando Wave Documentation Map

This directory contains current contracts, active roadmaps, proof receipts,
research notes, and historical snapshots. They do not carry equal authority.

## Authority Order

```text
1. ARCHITECTURE_CANON.md
   architectural identity, invariants, forbidden shortcuts

2. docs/CORE.md
   current end-to-end core, ownership, status, blocker

3. docs/CURRENT_CORE_DECISION.md
   active product-core decision

4. docs/GOAL.md
   product completion and economics gates

5. active implementation roadmaps
   mechanism-specific work order and STOP contracts

6. STOP reports and machine receipts
   immutable evidence at a particular checkpoint

7. research notes, old roadmaps, and archived documents
   context only; never current authority by filename alone
```

If two active documents disagree, stop. Resolve the contradiction in
`docs/CORE.md` and the owning roadmap before editing code.

## Current Core Route

```text
docs/CORE.md
plans/operator-grokking-core-v1/OPERATOR_GROKKING_CORE_V1.md
plans/effect-law-unification-v1/README.md
plans/effect-law-unification-v1/EFFECT_LAW_UNIFICATION_REFACTOR_V1.md
plans/runtime-role-grounding-v1/RUNTIME_ROLE_GROUNDING_V1.md
plans/nando-attractor-to-vm-machine-v1/NANDO_ATTRACTOR_TO_VM_ROADMAP_V1.md
```

## Current Status

```text
B1B controlled fixture          PASS
independent physical truth      BLOCK
AcceptedBindingLawEvidence      BLOCK
F4 Protocol Compiler            BLOCKED
production authority            false
M3                               WATCH
```

See `docs/CORE.md` for the reason and unlock contract.

Revision receipt:
[`CORE_DOCUMENTATION_AUDIT_2026-07-21.md`](CORE_DOCUMENTATION_AUDIT_2026-07-21.md).

## Historical Status Rule

Files named `STOP_*` are append-only scientific checkpoints. Statements such
as `F4 BLOCKED`, `H1 UNPROVEN`, or `future NOT OPENED` inside them describe the
world at that STOP. They must not be bulk-rewritten when a later stage advances.

The current state is maintained only in:

```text
docs/CORE.md
docs/CURRENT_CORE_DECISION.md
the status header of the owning active roadmap
```

## Supporting Documents

Useful but subordinate references include:

```text
docs/NORTH_STAR.md
docs/NANDA_WAVE_THEOREM.md
docs/NANDO_WAVE_SIGNAL_PATH_L1_TO_OPERATOR.md
docs/L3_SEMANTIC_GROKKING.md
docs/SYMBOL_CELL8_ARCHITECTURE.md
docs/OPERATOR_PRODUCT_LINES_AND_CAPACITY.md
docs/RISKS.md
docs/PARKING_LOT.md
docs/architecture_lineage/README.md
```

Date-bearing progress trees and benchmark reports are snapshots. Refresh their
measurements before quoting them as current product performance.
