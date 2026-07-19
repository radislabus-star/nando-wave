# Nando Attractor-to-VM Roadmap Structural Receipt

Date: 2026-07-19

Scope: documentation architecture only. No production code, service, registry,
admission state, checkpoint, or runtime route was changed.

## Gate Method

The roadmap was expressed as source/candidate triads and checked with the Rust
`nanda-check sparse-triad-v6.0-rust` structural gate.

The first combined packet returned `VETO` because it intentionally contained
eight different ownership routes and exceeded the role limit. The repair was
to preserve those ownership boundaries and split the check by route, not merge
the system behind one false owner.

## Route Results

```text
learning       PASS  mandatory
proof          PASS
compile        PASS
runtime        PASS  mandatory
verification   PASS
admission      PASS
feedback       PASS  mandatory
product        PASS
```

All route checks reported:

```text
Candidate structure is coherent with source triads.
```

## Verified Ownership Boundaries

```text
completed trace -> structural evidence              learning
competing circuits -> circuit-attractor             Wave learning
causal controls -> proven grokking                  proof
sealed winner -> versioned bytecode                 crystallization
observation -> route -> grounding -> VM             hot runtime
raw surface + output -> independent receipt         verification
two seals -> ACTIVE                                 external admission
verified residual -> candidate generation g+1       feedback
three verified windows -> M3                        product economics
```

The structural receipt validates route coherence only. It does not prove the
scientific mechanism, code implementation, live authority, CPU coverage, or
Product M3.
