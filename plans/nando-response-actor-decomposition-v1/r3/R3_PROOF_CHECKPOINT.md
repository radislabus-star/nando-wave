# R3 Operator Proof Checkpoint

Status: `MOVE_ONLY_PASS_PRECOMMIT`

Date: 2026-07-21

Base HEAD: `e75ef2e707bd7c01015b1bf1a3771fd82577786b`

Authority: `false`

## Ownership Cut

```text
nando-operator-proof
|-- independent response verification
|-- proof-owned raw surface reconstruction
|-- source-neutral verifier compilation
|-- decidability and verified-delta contracts
`-- trusted V2 binding trial, resolver, and adjudication

nando-response-actor
|-- compatibility re-exports
|-- actor/verifier cross-owner parity and mutation tests
`-- legacy controlled V1 B1B proof fixture
```

The legacy V1 route remains proof/eval-only and has no production authority.
The modern V2 physical observation, independent trial verification, sealed
trial, trusted resolution, and accepted binding evidence now have one proof
owner.

## Dependency Boundary

```text
nando-operator-proof -> nando-core
nando-operator-proof -> nando-operator-kernel

nando-operator-proof -> nando-response-actor      0
nando-operator-proof -> runtime/learning/admission 0
proof IO or checkpoint state                       0
proof authority constructors                       0
```

The verifier reparses bounded raw payloads through proof-owned surface
functions. It does not consume actor-selected operands or expected values.

## Move Parity

```text
focused V2 physical-trial route                 PASS
proof compile/tests/Clippy                      PASS
proof clean-crate fingerprint                   PASS
response-actor compile                          PASS
historical response test fingerprint            PASS
historical response Clippy fingerprint          PASS
new background build processes                     0
```

Receipts:

- `R3_PROOF_REMOTE_STOP.json`
- `R3_FACADE_REMOTE_STOP.json`

The facade fingerprint still contains the frozen R0 debt: 26 known test
failures and 20 known Clippy diagnostics. The identity set did not grow.

## Size Result

```text
nando-response-actor/src before R0      103,389 Rust lines
nando-response-actor/src after R2        99,982 Rust lines
nando-response-actor/src at checkpoint   94,271 Rust lines
nando-operator-proof/src                  6,309 Rust lines
```

## Preserved Boundaries

- Public `nando_response_actor::*` paths remain available through re-exports.
- Actor/verifier parity and mutation-kill tests remain cross-owner integration
  tests in the facade.
- No runtime, learning, admission, checkpoint, deployment, or service behavior
  changed.
- F5-B remains paused.
- Unrelated dirty files are excluded from the checkpoint commit.

## Remaining STOP Work

```text
checkpoint commit and push
-> exact-HEAD proof and facade STOP
-> Graphify from the exact commit
-> owner-local structural gates
-> read-only live gate and service parity
-> STOP-R3
```
