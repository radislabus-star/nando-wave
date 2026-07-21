# STOP-R4 Operator Runtime

Status: `PASS`

Date: 2026-07-21

Code HEAD: `931316decfa800d12bd68678be1d4ef37af70ca2`

Authority: `false`

## Result

```text
immutable RuntimeOperatorArtifact
-> bounded request context
-> structural selector candidates
-> complete role grounding
-> action-equivalence collapse
-> actor execution trace
-> OperatorPage VM parity
-> facade handoff
-> independent proof verifier
```

`nando-operator-runtime` now owns program execution, request parsing, routing,
role grounding, selector enumeration, Operator VM execution, and immutable
artifact restart. It imports neither proof, admission, learning, checkpoint IO,
filesystem, nor network owners.

The restart proof envelope is transport-only. Runtime validates page,
structure, schema, and byte integrity; the independent proof owner validates
the parity seal and proof commitments before a `VerifiedCrystallizedOperator`
can exist.

## STOP Gate

```text
runtime imports learning/proof/admission                  0
runtime imports response facade                           0
runtime filesystem/network/checkpoint IO                  0
runtime calls verifier directly                           0
actor output and ABSTAIN parity                         PASS
action-equivalence collapse parity                      PASS
runtime tests and Clippy -D warnings                    PASS
response historical test fingerprint                   PASS
response historical Clippy fingerprint                 PASS
restart page byte parity                                PASS
restart registry CBOR byte parity                       PASS
corrupt restart rejection                               PASS
F5-B symbols added                                         0
production caller behavior delta                           0
new background build processes                             0
authority                                              false
```

Golden restart SHA-256:

```text
OperatorPage32  982f2960d14552eab32702757f1a877c118989bbebe4a0a8ea5efab8f7d662a5
registry CBOR   73942962ee22ed1d95326d1f0dbb0f55e855d8b7f4e9b2a3928bf1a714897965
```

## Structural And Live Gates

```text
runtime execution owner route            PASS / authority_ready=false
runtime versus verifier route            PASS / authority_ready=false
facade dependency direction route        PASS / authority_ready=false
live composite gate                      PASS
eligible_for_local_accept                false
response M3                              WATCH
response ACTIVE packages                 0
false accepts                            0
runtime parity failures                  0
service restarts                         0
```

Service invocation IDs remain unchanged:

```text
nando-transition-serving  74ac3080f80b4fe387de2a94380e3657
nando-response-learning   8e59505eb1b943778601c9b3bacbd607
```

## Size Result

```text
nando-response-actor/src before R0       103,389 tracked Rust lines
nando-response-actor/src after R3         94,271 tracked Rust lines
nando-response-actor/src after R4         87,883 tracked Rust lines
nando-operator-runtime/src                 6,814 tracked Rust lines
```

The facade boundary lost 6,388 lines during R4 and 15,506 lines since R0.

## Receipts

- `STOP_R4_RUNTIME_EXACT_HEAD.json`
- `STOP_R4_FACADE_EXACT_HEAD.json`
- `STOP_R4_LIVE_GATE.json`
- `stop-r4-runtime-exec.trace.json`
- `stop-r4-runtime-proof-boundary.trace.json`
- `stop-r4-facade-direction.trace.json`

STOP-R4 unlocks only R5, `nando-operator-admission`. F5-B remains frozen until
STOP-R9.
