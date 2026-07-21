# R4 Runtime Artifact Checkpoint

Status: `MOVE_ONLY_PASS / R4_READY_FOR_EXACT_STOP`

Date: 2026-07-21

Authority: `false`

## Result

```text
RuntimeOperatorArtifact
|-- OperatorPage32
|-- RoleGraph
|-- OperatorCircuit
|-- TransformProgram
|-- renderer
`-- actor template
        |
        v
runtime-owned bounded restart codec
        |
        +-- structural and page integrity validation
        `-- non-authoritative proof envelope transport
                |
                v
facade/proof validates parity seal and proof roots
```

The runtime codec does not validate or grant proof authority. It reconstructs
only the immutable executable artifact and transports proof commitments for the
independent proof owner.

## Byte Parity

Golden values were measured from commit `14bef3020805c78e1a0f76712184f7cfe6c21bc1`
before the codec move and are asserted by the existing crystallization test:

```text
OperatorPage32 SHA-256
982f2960d14552eab32702757f1a877c118989bbebe4a0a8ea5efab8f7d662a5

registry CBOR SHA-256
73942962ee22ed1d95326d1f0dbb0f55e855d8b7f4e9b2a3928bf1a714897965
```

Both values remain byte-identical after extraction.

## Evidence

```text
runtime tests and Clippy -D warnings       PASS
response historical test fingerprint     PASS
response historical Clippy fingerprint   PASS
restart roundtrip                         PASS
corrupt registry rejection               PASS
page and registry golden SHA              PASS
new diagnostics                              0
authority                                false
deploy/restart                               0
```

Receipts:

- `R4_RUNTIME_ARTIFACT_REMOTE_STOP.json`
- `R4_FACADE_ARTIFACT_REMOTE_STOP.json`

The remaining work is the exact-HEAD structural and live read-only STOP-R4.
F5-B remains frozen.
