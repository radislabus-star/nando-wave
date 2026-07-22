# F6 Independent Verifier Convergence V1

Status: `COMPLETE / STOP-F6`

Authority: `false`

## Objective

Close the proof boundary left open by F5 without turning the verifier into a
second actor or a replay of actor-owned decisions.

```text
raw bounded request bytes
+ immutable ExecutableProtocolModeArtifactV3
+ opaque actor action and output claims
-> independent structural extraction
-> independent selector, role, and capability reconstruction
-> one physical action-equivalence class
-> independent reference execution and postconditions
-> F6 verifier receipt | REJECT | ABSTAIN
```

The verifier lives in `nando-operator-proof`. It must not depend on
`nando-operator-runtime`, call F5 extraction or binding functions, or consume
an actor-selected mapping as evidence. Shared immutable contracts and bounded
structural primitives from `nando-operator-kernel` are allowed.

Immutable artifacts are validated once into an opaque
`IndependentVerifierArtifactSetV3`. This cold proof step is digest-bound and
kept outside the per-request path. The hot verifier cannot construct or repair
that set.

## Owner Slices

```text
F6-A  contract, budgets, receipt schema, baseline
F6-B  raw-byte parser and independent structural surface
F6-C  selector, role, capability, and action-class reconstruction
F6-D  reference execution, effect postconditions, preserved frame
F6-E  adversarial proof, F5 handoff, remote/full gates, STOP-F6
```

F7 persistence, F8 admission, deployment, registry mutation, and local accept
are out of scope.

## Independence Contract

The verifier accepts no separate request-text hint. It derives the latest user
scene from the exact provider bytes before structural extraction. The verifier
also ignores these actor claims until final comparison:

- selected selector and source role;
- selected value and physical argument name;
- structural mapping and phase winner;
- capability symbol and capability identifier;
- expected output.

It independently derives a complete bounded candidate set. Distinct structural
paths may survive only when they collapse to one byte-identical physical action
class. Repeated physical capability paths, incomplete search, missing roles, or
multiple action classes produce `ABSTAIN`.

The verifier never emits an action and never grants authority.

## Budgets

```text
raw request bytes              <= 256 KiB
request text bytes             <= 16 KiB
JSON nodes                     <= 4096
structural role candidates     <= 64
structural relations           <= 256
advertised capabilities        <= 64
executable modes               <= 32
candidate path evaluations     <= 2048
actor output bytes             <= 16 KiB
```

Any exhausted bound is unknown evidence and therefore `ABSTAIN`, never a
negative semantic update.

## Preserved Frame

The current F5 actor is output-only. F6 binds the canonical raw pre-action
payload root before any interpretation and proves that reference execution
emits a separate action value without mutating that payload. The receipt joins
the immutable law's preserved-frame contract root, the pre-action root, and the
post-reference root. A future stateful opcode must provide an explicit typed
frame delta; it cannot inherit this output-only proof.

## STOP-F6 Gate

```text
valid F5 action and output                 VERIFIED
renamed physical surface                  VERIFIED
distinct paths, one physical action       VERIFIED
actor selector/value mutation             REJECT
role swap                                 REJECT
actor-selected semantic value mutation    REJECT
capability mutation                       REJECT
duplicate physical candidate paths        ABSTAIN
multiple physical action classes          ABSTAIN
missing expected role                     ABSTAIN
budget exhaustion                         ABSTAIN
unsupported effect opcode                 ABSTAIN
receipt canonical restart parity          PASS
false accepts                             0
parity mismatches                         0
execution authority                       false
```

## Frozen Baseline

Captured before F6 code changes on 2026-07-22:

```text
composite gate                            PASS
eligible_for_local_accept                 false
response ACTIVE packages                  0
response M3                               WATCH
response false accepts                    0
response runtime parity failures          0
transition active profiles                5
transition false accepts                  0
transition runtime parity mismatches       0
input token saving share                  0.7%
```

The gateway health contract was healthy. No production service was restarted
or reconfigured for this baseline.

## Result

Completed on 2026-07-22:

```text
proof unit tests                           5 PASS
F6 adversarial integration tests           8 PASS
release performance gate                   1 PASS
kernel + F5 runtime + F6 proof             72 PASS / 2 ignored
workspace cargo check                      PASS
kernel/runtime/proof Clippy -D warnings    PASS
gateway control receipt tests              17 PASS
gateway control Clippy -D warnings         PASS
normal proof -> runtime dependency         ABSENT
owner-local NANDA routes                  4/4 PASS
single-owner composite NANDA              VETO (expected owner split)
live composite gate                       PASS
eligible for local accept                 false
response ACTIVE packages                  0
response M3                               WATCH
matched verifier p99                       291773 ns
no-match verifier p99                       34659 ns
observed hard maximum                      354921 ns
performance samples per route              4096
controlled false accepts                   0
controlled parity mismatches               0
execution authority                        false
```

The admitted effect surface is intentionally narrow: one typed function
`CALL` with `COPY` relations and output-only preserved-frame semantics. Unknown
effect operations, missing request provenance, incomplete search, and
ambiguous physical actions remain fail-closed. Live receipt wiring,
persistence, generation ownership, and admission are F7/F8 work.
The control-panel source validates this STOP-F6 receipt and renders F7 as
locked; the running panel was not restarted during STOP-F6.
