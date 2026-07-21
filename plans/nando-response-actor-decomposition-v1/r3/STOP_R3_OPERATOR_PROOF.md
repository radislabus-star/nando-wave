# STOP-R3 Operator Proof

Status: `PASS`

Date: 2026-07-21

Code HEAD: `4138a15ffdbb90ab8f8bf38b2551be9c5eb0b9e7`

Authority: `false`

## Result

```text
immutable kernel IR
-> proof-owned raw surface reconstruction
-> source-neutral verifier compilation
-> independent expected consequence derivation
-> proof receipt or fail-closed rejection
```

Moved to `nando-operator-proof`:

- independent response verifier implementation;
- proof-owned bounded payload/surface reconstruction;
- source-neutral verifier compiler;
- decidability and verified-delta contracts;
- V2 physical actor observation and independent trial receipt;
- trusted V2 row resolver and binding-law adjudication.

The response crate preserves the previous public paths through compatibility
re-exports. Actor/verifier parity and mutation tests remain cross-owner
integration tests there. The older V1 B1B controlled fixture remains
proof/eval-only in the facade and grants no authority.

## STOP Gate

```text
proof imports runtime/learning/admission                 0
proof imports response facade                            0
proof reads actor-selected expected truth                0
proof mutable global/checkpoint state                    0
proof filesystem/network/process IO                      0
proof fixture authority                                  0
mutation/parity focused routes                        PASS
trusted V2 binding proof route                        PASS
proof tests and Clippy                                PASS
response historical test fingerprint                 PASS
response historical Clippy fingerprint               PASS
historical failure-set delta                             0
exact-HEAD Graphify                                   PASS
NANDA proof owner route                               PASS
NANDA authority_ready                                false
live transition composite gate                        PASS
eligible_for_local_accept                            false
response ACTIVE packages                                 0
response M3                                          WATCH
false accepts                                            0
runtime parity mismatches                                0
new background build processes                           0
```

The facade's non-zero test and Clippy exits are the exact frozen R0 debt: 26
known test failures and 20 known diagnostics. The fingerprint comparison is
PASS; no new failure or diagnostic was introduced.

## Size Result

```text
nando-response-actor/src before R0      103,389 Rust lines
nando-response-actor/src after R2        99,982 Rust lines
nando-response-actor/src after R3        94,271 Rust lines
nando-operator-proof/src                  6,309 Rust lines
```

This stage removed another 5,711 lines from the facade ownership boundary.

## Exact Receipts

- `R3_PROOF_EXACT_HEAD_STOP.json`
  SHA-256 `8dccbd9818df861dc802a5a361c065c097a2d4f8c455e120ae57d308bc3740e7`
- `R3_FACADE_EXACT_HEAD_STOP.json`
  SHA-256 `b7ee4a9d8ed759656f3201cb95f785f3e2779ff27aa86a93b06a6f4f3c42ccb8`
- `R3_MUTATION_PARITY_FAST.json`
  SHA-256 `d6e2014d9de06f32433005e09c06d07c6b0c61b1cc841ee29796c3aa0c19fcc1`

The exact proof STOP took 5.15 seconds to compile and 23.68 seconds for
Graphify. The exact facade STOP preserved its baseline fingerprint.

## Service Parity

```text
nando-transition-serving InvocationID  74ac3080f80b4fe387de2a94380e3657
nando-transition-serving NRestarts      0
nando-response-learning InvocationID    8e59505eb1b943778601c9b3bacbd607
nando-response-learning NRestarts       0
deploy/restart                           0
```

F5-B remains paused. The next owner boundary is R4,
`nando-operator-runtime`, and may move only existing execution, grounding,
VM, renderer, and immutable actor-trace behavior.
