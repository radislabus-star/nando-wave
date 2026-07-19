# Runtime Role Grounding And Sealed Crystallization V1

Status: canonical implementation sequence.

Implementation status:

```text
R1 Runtime Role Grounding                         PASS
R2 Sealed Winner Provenance                       PASS
R3 Executable Parity Seal And Full Backward Loop  PASS
R4 OperatorPage32 Restart Roundtrip                PASS
R5 Exact-Commit Architecture Receipt               NEXT
R6-R8                                             PENDING
```

This plan closes the gap between a phase-selected relation circuit and an
operator whose circuit actually determines its CPU computation. It does not
expand the opcode set.

## Invariant

```text
The RoleGraph and RelationProgram choose the runtime operands.
TransformOp8 computes from those bound operands.
The independent verifier repeats the structural derivation.
No diagnostic report, caller-provided hash, or pre-typed actor grants authority.
```

## R1: Runtime Role Grounding

Input:

```text
CandidateOperatorBlueprint
+ canonical RuntimeSurfaceEvidence
```

Implement a bounded role CSP that returns:

```text
Complete(action-equivalent mappings) -> BoundRoleEnvironment
Complete(multiple action classes)    -> ABSTAIN
Exhausted                             -> ABSTAIN
Invalid relations                    -> ABSTAIN
```

`BoundRoleEnvironment` commits to the surface, mapping, and binder-derived
action equivalence. Only `BoundCrystallizedOperator` exposes execution.

Gate:

```text
local role rename                       PASS
multiple mappings, same action          PASS
multiple mappings, different actions    ABSTAIN
semantic source-role swap               ABSTAIN or verifier reject
invalid relation cell                   ABSTAIN
search exhaustion                       ABSTAIN
```

## R2: Sealed Winner Provenance

Replace crystallization authority from public `BlueprintFutureReport` with a
private `SealedBlueprintWinnerReceipt` created only by the evaluator. Commit to
support, future evidence, candidate set, fixed-point scores, winner, runner-up,
margin, configuration, and versions.

Gate:

```text
manually constructed report              REJECT
same lineage, different surface           REJECT
same surface, different raw input         REJECT
mutated bundle after evaluation           REJECT
no/shuffled/magnitude/random phase        no winner seal
```

## R3: Executable Parity Seal And Full Backward Loop

Compile actor and independent verifier only after R1 and R2. Execute every
admitted future case, seal binding and execution receipts, then construct a
`VerifiedCrystallizedOperator`.

Feed its real `VerifiedDeltaReceipt` through `BackwardWave` into candidate
generation `g+1`.

Gate:

```text
actor mutation                            verifier reject
missing future receipt                    REJECT
wrong accepts                             0
ACTIVE page generation g                  byte-identical after feedback
censored outcome                          no semantic Wave update
verified applicability negative           bounded counter-wave in g+1
hard contradiction                        split, repair, or revoke path
```

## R4: OperatorPage32 Restart Roundtrip

Make the compact page plus digest-bound registry sufficient to restore the
role graph, relation program, actor, verifier, renderer, and proof commitments.

Gate: serialize, restart, restore, bind, execute, and verify with a
byte-identical decision and no ambient support memory.

## R5: Exact-Commit Architecture Receipt

On the remote build host, check out the exact source commit, run
`graphify update .`, and copy only `graphify-out/` back. The graph metadata must
name the same commit as the source tree and must remain untracked.

## R6: Generic Scalar Live Shadow

Connect completed live traces to the new blueprint path without production
authority. Measure real binding outcomes and explicit blockers. No fabricated
support, future evidence, or receipts.

Gate: real renamed surfaces bind, ambiguous surfaces abstain, actor and verifier
agree, and resource state remains bounded.

## R7: External Admission

Submit only a `VerifiedCrystallizedOperator` with both seals to the existing
external admission controller. Run `nando-live-transition-gate` before any
local CPU authority.

Gate: immutable ACTIVE generation, fallback intact, false accepts zero, runtime
parity mismatches zero, and every local accept independently verified.

## R8: Extend Operator Capacity

Only after R1-R7 pass, add count, filter, compose, and other transforms in the
order of verified live token opportunity. Each opcode must use bound roles and
have an independently implemented verifier path.

## Forbidden Shortcuts

- Do not attach a ready `ResponseProgram` to a circuit and call it induction.
- Do not use phase to override failed structural constraints.
- Do not trust caller-provided action-equivalence or proof hashes.
- Do not give a public diagnostic report authority.
- Do not mutate an ACTIVE generation in place.
- Do not expand opcodes before the circuit causally controls the scalar actor.
- Do not call a seal external admission authority.
- Do not claim grokking from threshold crossing, anti-center creation, or
  package promotion alone.

## Canonical Claim Boundary

Until R1-R3 pass:

```text
Nando Wave can synthesize and phase-select a new relation circuit and attach a
generic scalar primitive. It has not yet proven that the learned circuit causes
the complete executable operator.
```

After R1-R3 pass, but before R6-R7:

```text
The circuit controls a sealed, independently verified laboratory operator.
Live product authority remains blocked.
```
