# STOP-R6C Online Learning

Status: `PASS`

Date: 2026-07-22

Code HEAD: `716ae73edd28c188bf3f9e767175a09119d9750d`

Authority: `false`

## Result

```text
verified transition evidence
-> bounded online learning state
-> relation routing and structural role grounding
-> competing version space and family discovery
-> bounded synthesis and CEGIS
-> generation rollover
-> learned Wave route and adapter-wave fitting
-> immutable checkpoint/report contracts
```

`nando-operator-learning` now owns the cold online-learning mechanisms,
CEGIS, generation rollover, structural grounding, family/version search,
response-program synthesis, phase-route fitting, and immutable learning report
contracts. Generic phase and coherence algorithms remain in `nando-core`.

The response facade still owns cross-owner application orchestration in
`online_state.rs` and `online_collection.rs`. Those files invoke runtime,
proof, and admission owners and therefore cannot move wholesale into the
learning crate. Their route split is explicitly deferred to R7/R8; this STOP
does not claim that either monolith is already decomposed.

## Code Commits

```text
90447a1  refactor: extract online learning state leaves
5dea47b  refactor: extract structural role grounding
e7cc2a0  refactor: centralize relation routing atoms
5728db8  refactor: extract cross-surface family discovery
e577e7b  refactor: extract learning version space
375797a  refactor: extract response operator synthesis
9d78f97  refactor: extract cegis learning cluster
a884ed9  refactor: centralize adapter wave learning
1be936b  refactor: extract self training report contracts
40c29e6  refactor: extract collection learning contracts
716ae73  refactor: extract online collection report contracts
```

These are bounded owner-local moves inside the reviewed R6-C substage. They do
not change runtime semantics, admission policy, thresholds, or authority.

## Exact Remote Gates

Both final gates ran from exact HEAD with an empty overlay.

```text
nando-operator-learning compile/tests/Clippy       PASS / PASS / PASS
learning compile time                              12.55 s
learning tests time                                 2.17 s
nando-response-actor historical fingerprint       PASS
response historical failures                  26 known
response retired Clippy diagnostics                 17
response active known Clippy diagnostics             3
new response failures                                0
new response Clippy diagnostics                      0
new background build processes                       0
learning imports runtime/admission                    0
```

The response test and Clippy process exits remain `101` only because the exact
historical debt is still present. The remote runner compared the fingerprint
and returned `worker_verdict=PASS`; no new diagnostic was accepted as debt.

## Structural And Live Gates

```text
R6-C online-learning structural route       PASS
structural complexity score                   20
trusted production proof               NOT REQUESTED
authority_ready                             false

live structural routes                       PASS 3/3
live Wave causal proof                        PASS
live deployment health                        PASS
response runtime                              VETO
response M3                                  WATCH
response ACTIVE packages                         0
response false accepts                           0
response runtime parity mismatches               0
eligible_for_local_accept                     false
```

The live `VETO/WATCH` is the expected fail-closed product state: no ACTIVE
response package, no completed M3 windows, and only 0.7% measured input-token
savings. It is not converted into a decomposition PASS and it did not mutate
the registry or service.

## Size Result

```text
nando-response-actor/src before R0       103,389 tracked Rust lines
nando-response-actor/src after R6-C        56,255 tracked Rust lines
nando-operator-learning/src                30,734 tracked Rust lines
lines removed from facade boundary         47,134
facade reduction                            45.6%
```

R6 has established the owner boundary, but R7/R8 still have substantial work:
the facade root is 3,068 lines, the miner binary is 5,127 lines, and the two
largest mixed orchestration files remain above the hard file budget.

## STOP-R6 Verdict

```text
learning imports runtime/admission                         0
duplicate Wave/coherence implementation                    0
checkpoint and report compatibility                     PASS
support/future and censored-outcome compatibility        PASS
historical failure-set delta                                0
new Clippy diagnostic delta                                 0
false accepts                                               0
runtime parity mismatches                                   0
authority                                               false
STOP-R6                                                  PASS
```

Receipts:

- `R6C_FINAL_LEARNING_EXACT.json`
- `R6C_FINAL_FACADE_EXACT.json`
- `R6C_RETIRED_CLIPPY_DIAGNOSTICS.tsv`
- `NANDA_R6C_ONLINE_LEARNING.md`
- `stop-r6c-online-learning.trace.json`
- `STOP_R6C_LIVE_GATE.json`

STOP-R6 unlocks only R7. F5-B remains frozen until STOP-R9.
