# R4 Operator Runtime Core Checkpoint

Status: `MOVE_ONLY_PASS / R4_IN_PROGRESS`

Date: 2026-07-21

Base HEAD: `eff2fbe52f59509441d2125959ac33f60f246926`

Authority: `false`

## Ownership Cut

```text
nando-operator-runtime
|-- response program execution and bounded surface parsing
|-- runtime routing atom construction
|-- pre-action context and capability projection
`-- OperatorPage VM execution

nando-response-actor
|-- compatibility entrypoints and public paths
|-- renderer bytecode compiler
|-- actor/verifier application orchestration
`-- cross-owner parity and mutation tests
```

There is one implementation of the routing algorithms. The response facade
delegates to the runtime owner and retains no second copy of request,
capability, observation, selector, or pre-action routing logic.

Nested consensus execution still receives verification through an external
validator callback. The runtime does not import the proof crate or create
authority.

## Dependency Boundary

```text
nando-operator-runtime -> nando-core
nando-operator-runtime -> nando-operator-kernel

nando-operator-runtime -> nando-response-actor                 0
nando-operator-runtime -> nando-operator-proof                 0
nando-operator-runtime -> learning/admission/checkpoint owners 0
runtime filesystem or network IO                               0
```

VM instruction constants are immutable kernel contracts. VM execution is
runtime-owned, while renderer encoding remains compiler-owned in the facade
until its later owner move.

## Move Parity

```text
runtime compile/tests/Clippy                       PASS
runtime clean-crate fingerprint                    PASS
response compile                                  PASS
historical response test fingerprint               PASS
historical response Clippy fingerprint             PASS
new routing diagnostics                               0
new background build processes                        0
```

Receipts:

- `R4_RUNTIME_CORE_REMOTE_STOP.json`
- `R4_FACADE_CORE_REMOTE_STOP.json`

The facade fingerprint retains the exact 26 known test failures. Three old
Clippy diagnostics disappeared because their implementations moved and were
fixed in the runtime owner. Their exact immutable R0 rows are retired through
`RETIRED_CLIPPY_DIAGNOSTICS.tsv`; unknown retirements and new diagnostics still
fail the gate.

## Size Result

```text
nando-response-actor/src before R0       103,389 Rust lines
nando-response-actor/src after R3         94,271 Rust lines
nando-response-actor/src at checkpoint    89,787 Rust lines
nando-operator-runtime/src                 4,691 Rust lines
nando-operator-kernel/src                  3,960 Rust lines
nando-operator-proof/src                   6,309 Rust lines
```

## Preserved Boundaries

- Existing `nando_response_actor::*` public paths remain available.
- Existing output, ABSTAIN, selector, routing, and nested-consensus behavior is
  unchanged.
- No runtime, learning, admission, deployment, checkpoint, or service state was
  changed.
- F5-B remains paused.
- Unrelated dirty files are excluded from the checkpoint commit.

## Remaining R4 Work

```text
mixed crystallized_operator owner
-> split bind, restore, and execute from compiler/proof orchestration
-> runtime execution trace handed to independent verifier
-> exact-HEAD runtime and facade STOP
-> structural and live read-only gates
-> STOP-R4
```

This checkpoint does not satisfy STOP-R4 and does not unlock R5.
