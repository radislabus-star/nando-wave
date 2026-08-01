# Canonical IR And Crystal Bundle V4

Status: implemented; canonical for newly crystallized packages, live authority
unchanged until a separately gated generation rollout.

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
                         ├─ RoutingImage: roles + signed phase relations
                         ├─ ExecutionImage: Page32 + bounded canonical program pages
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

Certificates issued after sealing are also absent from BundleV4. They bind to
its immutable `bundle_id` through an append-only external ledger, so a later
Wave verdict cannot rewrite content identity or silently revoke execution.
See `THREE_CERTIFICATE_REGISTRIES_AND_K1_V1.md`.

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
IR + frozen compiler inputs rebuild exact bundled Page32 bytes
valid resealed Page32 from different compiler inputs is rejected
wrong externally expected bundle_id is rejected
tampered image is rejected
serialized bundle contains no authority or lease field
legacy restart golden hashes remain unchanged
```

Newly crystallized packages serialize V4 bytes and `bundle_id`. Their legacy
`page_bytes` and `registry_cbor` remain compatibility aliases and must restore
an execution-equivalent operator with the same parity seal. Old packages decode
without V4, but new external admission reconstruction requires V4 and rejects a
missing or mismatched ID. Existing live authority is not rewritten by this
source migration; a new immutable generation still requires the normal
composite deployment gate.

The V4 compiler owns the image split. `RoutingImage` cannot contain the actor
template or entry page. `ExecutionImage` contains the exact 4,032-byte entry
page plus one to eight canonical program pages; restart concatenates those
pages, reconstructs the IR with the routing image, and rejects non-canonical,
missing, or oversized page layouts. This keeps `OperatorPage32` hot while the
complete VM program remains portable data.
