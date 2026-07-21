# R2 Executable Artifact Checkpoint

Status: `PASS_ON_SCOPED_OVERLAY`

Date: 2026-07-21

Base HEAD: `f46487d`

Authority: `false`

## Ownership Cut

The F5-A immutable artifact now belongs to `nando-operator-kernel`:

```text
nando-operator-kernel
|-- ExecutableProtocolModeArtifactV3
|-- ExecutableProtocolModeV3
|-- ProtocolFacetPayloadV3
|-- ProtocolCapabilityArgumentV3
|-- canonical digest and restart validation
`-- closed payload/mode/artifact finalizers

nando-response-actor
|-- ProtocolFacetEvidenceInputV3
|-- physical facet parser
|-- evidence-to-artifact compiler orchestration
`-- compatibility re-exports
```

The compiler cannot construct or mutate artifact wire fields. It passes
evidence-derived capability information into kernel finalizers, which set the
schema/version, derive every immutable root, force both authority flags off,
and validate the completed object.

## VM Boundary

No second VM language was introduced.

```text
ResponseProgram and typed immutable contracts -> nando-operator-kernel
OperatorPage32 / TransformOp8                 -> canonical nando-core owner
operator_vm execution and renderer            -> remains runtime work for R4
```

`operator_vm.rs` contains private execution functions and renderer opcodes, not
a separate public immutable contract suitable for R2.

## Measured Shape

```text
nando-response-actor/src Rust lines     99,982
nando-operator-kernel/src Rust lines     3,935
old executable artifact module lines       509
new response compiler/facade lines          214
kernel executable artifact owner lines      674
duplicate key type definitions                0
kernel forbidden owner dependencies           0
kernel filesystem/network/process effects     0
```

## Proof

```text
kernel compile/test/Clippy                 PASS
kernel fingerprint                        PASS
F5-A focused tests                        PASS
F5-A restart parity                       PASS
F5-A tamper rejection                     PASS
F5-A constant rejection                   PASS
response R0 failure fingerprint           PASS
response R0 Clippy fingerprint            PASS
new background build processes               0
```

Machine receipts:

- `R2_EXECUTABLE_KERNEL_REMOTE_STOP.json`
- `R2_EXECUTABLE_FACADE_REMOTE_STOP.json`

## Unchanged Boundaries

```text
F5-B                                  PAUSED
production callers                        0
execution authority                   false
service deploy/restart                     0
Wave thresholds/evidence/admission changes 0
```

Exact-HEAD Graphify, owner routes, live gate, and service parity remain for the
final STOP-R2 receipt after this checkpoint is committed.
