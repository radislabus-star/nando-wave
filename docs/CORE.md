# Nando Operator Core

Status: canonical current-core map.

Current snapshot: 2026-07-21 Europe/Tallinn.

This document answers three questions in one place:

1. What machine is Nando building?
2. Which component owns each kind of truth?
3. What is the exact current blocker?

`ARCHITECTURE_CANON.md` remains the highest architectural authority. This file
is the current navigation and status view. If they disagree, stop and repair
the documentation before changing the core.

## 1. North Star

Nando is building a stable Rust machine that learns new bounded operator
programs from verified experience without recompiling the Rust core:

```text
real completed traces
-> structural relation fragments
-> Dynamic Operator Field
-> competing connected circuits
-> cross-plane phase locking
-> circuit-attractor
-> causal grokking proof
-> exact episodic authority cleanup
-> Operator Crystallizer / Compiler
-> versioned VM bytecode
-> immutable OperatorPackage
-> Dynamic Registry
-> Phase Router
-> Runtime Role Grounder
-> Operator VM
-> independent Verifier
-> EMIT | ABSTAIN
-> external Admission
-> verified CPU execution
-> VerifiedDeltaReceipt
-> BackwardWave generation g+1
```

The short formula is:

> Wave discovers and consolidates. The compiler crystallizes. The VM acts.
> The verifier establishes truth. Admission grants authority.

## 2. Where Grokking Happens

The VM does not grok. `OperatorPage32` does not grok. A threshold crossing does
not grok.

Grokking is the phase transition in the cold Dynamic Operator Field:

```text
partial evidence from independent surfaces
-> several competing whole-circuit hypotheses
-> constructive and destructive cross-plane interference
-> one connected circuit obtains a causal coherence margin
-> the same circuit transfers after exact episode authority is removed
```

The result is a `CircuitAttractor` containing a role graph, relation program,
applicability boundary, transform, composition DAG, renderer, and verifier
contract. The crystallizer then converts that proven attractor into immutable
VM data. The stored cube/page is the result of grokking, not the grokking
process itself.

## 3. Machine Of Truth

Each layer owns one kind of truth. No layer may manufacture the input
capability required by the next layer.

```text
[1] Evidence Kernel
    capture receipt + frozen pre-action graph + immutable provenance
    output: label-blind evidence
    forbidden: teacher label, selector, ProtocolMode, authority

[2] Semantic Law Kernel
    CanonicalEffectLawV3 + EffectLawIdV3
    output: physical-surface-neutral effect identity
    forbidden: runtime selector or concrete binding

[3] Binding Evidence Kernel
    bounded candidate relation graph + version space
    output: identifiable relation, ties, or insufficient evidence
    forbidden: choosing a selector

[4] Physical Trial Owner
    observed candidate execution + independent verifier receipt
    output: sealed physical trial receipts
    forbidden: deriving truth from intervention metadata or expected law

[5] Trusted Label Resolver
    externally pinned roots + frozen rows + physical trial receipts
    output: TrustedResolvedBindingRows
    forbidden: rebuilding or mutating frozen graphs

[6] Causal Adjudicator
    preregistered H0/H1 + trusted resolved rows
    output: AcceptedBindingLawEvidence | INSUFFICIENT
    forbidden: selector, ProtocolMode, actor, runtime authority

[7] F4 Protocol Compiler
    AcceptedBindingLawEvidence + EffectLawIdV3
    output: bounded competing ProtocolModes
    forbidden: compiling from a report, fixture, or unaccepted receipt

[8] Runtime Grounding
    ProtocolMode + current structural surface
    output: unique BoundRoleEnvironment | ABSTAIN
    forbidden: caller-provided binding authority

[9] Actor And Independent Verifier
    bound VM program -> candidate state -> independently verified result
    output: VerifiedDeltaReceipt | ABSTAIN

[10] External Admission
    proof denominators + future + negatives + parity + budgets
    output: immutable ACTIVE generation or BLOCK
```

The compiler/runtime boundary is therefore explicit:

```text
proof and evidence end at AcceptedBindingLawEvidence
compiler authority starts only from AcceptedBindingLawEvidence
runtime authority starts only after independent admission
```

## 4. Active Core Versus Target Core

The active product core is still the phase-center / streaming operator-memory
route. Stage2/Organ128 remains a legacy/control lineage.

The target core is the Attractor-to-VM machine above. Existing laboratory
operator synthesis, crystallization, role grounding, and admission components
are organs of that target, but laboratory availability does not make the full
machine live.

## 5. Current Status

```text
CanonicalEffectLawV3                         COMPLETE / STOP-F2
V1/V3 dual classification                    COMPLETE / STOP-F3R
B1A bounded binding version space             COMPLETE / INSUFFICIENT
B1B label-blind support and future             FROZEN
B1B controlled causal fixture                  CONTROLLED_FIXTURE_PASS
independent physical-truth ownership            BLOCK
AcceptedBindingLawEvidence                      BLOCK
F4 Protocol Compiler                            BLOCKED
runtime role grounding                          LAB PASS / LIVE WATCH
general Operator VM                             BLOCK
response ACTIVE packages                        0
verified input-token saving share               0.8%
false accepts                                    0
runtime parity failures                          0
M3                                               WATCH
production authority                            false
```

The live numbers are the read-only composite-gate snapshot taken on
2026-07-21. They are not timeless claims and must be refreshed before a product
or economics statement.

## 6. Exact Current Blocker

The controlled B1B experiment found a plausible relation:

```text
parent_action_to_capability_instance
```

It passed its bounded synthetic interventions with zero wrong bindings,
negative accepts, or parity failures. That is useful controlled evidence, but
the post-implementation review found an ownership leak:

```text
binding_evidence_adjudication.rs
  contains synthetic scene reconstruction
  + physical candidate execution
  + proof verification
  + label-manifest production
  + trust orchestration
  + H0/H1 adjudication
  + report hashing
```

In particular, the proof observer reconstructs a scene from frozen
intervention metadata, while the local proof actor and proof verifier inspect
the same synthetic scene model. Therefore the existing receipt supports H1
inside the controlled fixture. It does not yet create independently owned
physical truth for F4.

The machine artifact `STOP_B1B_ADJUDICATION.json` is retained as an immutable
controlled receipt. Its historical `f4_status=UNLOCKED_NOT_STARTED` field is
not current compiler authorization after this review.

## 7. Required Repair Before F4

Do one ownership refactor, not another learning experiment:

```text
frozen support/future graphs
-> PhysicalTrialReceipt owner
     consumes observed execution and independent verifier receipts
     cannot infer a label from intervention id
-> TrustedLabelResolver
     validates roots and joins immutable rows
-> CausalAdjudicator
     emits AcceptedBindingLawEvidence or INSUFFICIENT
-> deterministic report serializer
```

Required gates:

```text
move-only module split preserves current golden bytes
synthetic scene renderer remains proof-fixture-only
physical actor and verifier have separate inputs/program commitments
intervention metadata cannot produce or alter a label
forged trial, label, graph, or trust root is rejected
adjudicator has no selector/ProtocolMode constructor
AcceptedBindingLawEvidence has one private validated constructor
F4 accepts that capability, never a report or raw JSON
execution_authority remains false
```

Only after these gates may F4 start.

## 8. F4 Definition

F4 is not another classifier and not a table of physical actors. It compiles a
proven relation law into bounded structural programs:

```text
AcceptedBindingLawEvidence
+ EffectLawIdV3
-> competing structural ProtocolModes
-> complete bounded search
-> guard/execution matrix
-> exact cover over already safe modes
-> action-equivalence check across all admissible covers
-> ProtocolModeSet | ABSTAIN
```

Every admitted mode must have:

```text
positive coverage complete
WRONG = 0
VERIFY_FAILED = 0
applicability-negative accepts = 0
all surviving covers action-equivalent
```

## 9. Claim Boundary

Allowed now:

> Nando has a controlled causal B1B fixture that supports one binding relation
> and preserves zero-error fail-closed behavior.

Not allowed now:

- independently proven natural binding law;
- completed F4 compiler;
- general learned VM program;
- live Rich Operator authority;
- broad autonomous execution;
- M3 or 50% verified savings.

## 10. Canonical Documents

Read in this order:

```text
ARCHITECTURE_CANON.md
  immutable architectural meaning and forbidden shortcuts

docs/CORE.md
  current machine, ownership map, status, and exact blocker

docs/CURRENT_CORE_DECISION.md
  active product-core decision versus target machine

docs/CORE_DOCUMENTATION_AUDIT_2026-07-21.md
  receipt for the current documentation and ownership revision

docs/GOAL.md
  product completion definition

plans/operator-grokking-core-v1/OPERATOR_GROKKING_CORE_V1.md
  circuit birth and causal grokking mechanism

plans/effect-law-unification-v1/EFFECT_LAW_UNIFICATION_REFACTOR_V1.md
  EffectLaw -> binding evidence -> F4 migration

plans/effect-law-unification-v1/README.md
  ordered EffectLaw evidence ladder and receipt interpretation

plans/runtime-role-grounding-v1/RUNTIME_ROLE_GROUNDING_V1.md
  compiler output -> role-bound execution -> verifier

plans/nando-attractor-to-vm-machine-v1/NANDO_ATTRACTOR_TO_VM_ROADMAP_V1.md
  end-to-end implementation ladder
```

Every STOP report is an immutable historical checkpoint. Its status describes
that checkpoint, not necessarily the current project state. Use
`docs/README.md` for the documentation authority map.
