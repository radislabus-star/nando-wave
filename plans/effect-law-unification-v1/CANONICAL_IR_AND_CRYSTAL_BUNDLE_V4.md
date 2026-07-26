# Canonical IR And Crystal Bundle V4

Status: implemented candidate, authority unchanged.

## Route

```text
ResponseProgram ---------------------\
                                       +-> CanonicalOperatorIrV1
CandidateOperatorBlueprint ----------/
                                           |
                                           v
                                  one page compiler
                                           |
                         RuntimeOperatorArtifact
                                           |
                                           v
                         CrystallizedOperatorBundleV4
                         ├─ RoutingImage
                         ├─ ExecutionImage
                         ├─ VerifierImage
                         └─ ProofEnvelope
                                           |
                                           v
                              external Admission only
```

## Ownership

```text
nando-operator-kernel
└─ authority-free CanonicalOperatorIrV1 contract and identities

nando-operator-runtime
└─ route adapters, IR compiler, RuntimeOperatorArtifact

nando-operator-persistence
└─ bounded content-addressed V4 bundle codec and digest validation

nando-response-actor
└─ proof/verifier assembly and compatibility restart facade

nando-operator-admission
└─ external expected bundle ID, lease, rollout, authority
```

The bundle cannot authorize itself. `AuthorityLease`, rollout state, live
metrics, raw evidence, and learner state are absent from its serialized bytes.

## Identity

```text
law_id      = H(executable CanonicalOperatorIR without routing phase)
routing_id  = H(full routing image)
artifact_id = H(compiler || VM ABI || law_id || execution image)
verifier_id = H(verifier ABI || verifier image)
proof_id    = H(proof envelope)
bundle_id   = H(law_id || routing_id || artifact_id || verifier_id || proof_id)
```

Routing phase remains part of the full IR and hot page. It is excluded only
from executable identity so independently induced routes can converge on one
VM law without claiming byte-identical routing evidence.

## Proof Boundary

The focused proof requires:

```text
adaptive IR executable identity == blueprint IR executable identity
full artifact identity differs when routing phase differs
bundle encode -> decode -> restore -> encode is byte-identical
wrong externally expected bundle_id is rejected
tampered image is rejected
serialized bundle contains no authority or lease field
legacy restart golden hashes remain unchanged
```

Production package serialization and admission ownership are deliberately not
switched by this change. The V4 bundle remains a sealed candidate until a
separate compatibility migration proves old checkpoint decode, package byte
parity, and external admission reconstruction.
