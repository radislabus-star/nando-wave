# Executable Operator Crystallization V1

## Goal

Turn a complete, order-independent blueprint winner into a real executable
operator using the existing actor and independent verifier implementations.

## Required route

```text
Complete FrozenOperatorBlueprintSet
-> canonical winner
-> CrystallizedOperator
-> bind new surface roles
-> ResponseProgram actor
-> independent VerifierProgram
-> VerifiedDeltaReceipt
-> BackwardWave candidate generation g+1
```

## Work order

1. Replace boolean completeness with explicit `Complete` / `Exhausted` and
   frontier accounting.
2. Sort support surfaces by full lineage and canonicalize role symmetry orbits.
3. Prove candidate-set SHA invariance under support and local-role permutation.
4. Compile a non-empty transform and executable renderer through existing
   response-actor program types.
5. Bind an independent verifier program, not a hash placeholder.
6. Build `CrystallizedOperator` and compile its compact `OperatorPage32`.
7. Prove execution and verification on a new symmetric surface.
8. Feed typed outcomes into immutable BackwardWave generation `g+1`.

## Completion vetoes

```text
search exhausted                         VETO
canonical orbit exhausted                VETO
empty transform                          VETO
cyclic composition                       VETO
renderer commitment without program      VETO
verifier commitment without program      VETO
support lineage reused in future          VETO
ambiguous pre-action role binding         ABSTAIN
```

## Current status

```text
explicit SearchCompletion                PASS
order-independent surfaces               PASS
local-role rename invariance              PASS
bounded symmetry overflow                 FAIL-CLOSED
fixed-point phase accumulation            PASS
CrystallizedOperator                     PASS
automatic project actor compilation       PASS
independent verifier binding              PASS
future parity receipt completeness        PASS
strong symmetric causal proof             PASS
BackwardWave g+1 bridge                    PASS
generic count/filter/compose compiler      WATCH
live admission                            DEFERRED
```
