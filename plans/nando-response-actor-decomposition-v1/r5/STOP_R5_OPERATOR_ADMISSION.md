# STOP-R5 Operator Admission

Status: `PASS`

Date: 2026-07-21

Code HEAD: `d22fb7c97ffb9a9f1970c5f7d2e1ec6726a16d5f`

Authority: `false`

## Result

```text
immutable candidate commitments
-> independent proof reconstruction
-> digest-bound package record
-> ordered fail-closed admission policy
-> unique composite authority binding
-> ValidatedResponseAuthority | BLOCK
```

`nando-operator-admission` now owns admission wire contracts, authority lease
validation, package policy, runtime parity receipt sealing, package lifecycle
wire types, and the only composite authority constructor. It depends on the
kernel and proof owners, but imports neither runtime nor learning.

The proof owner independently reconstructs the candidate from bounded support
and future rows. Admission compares package, support, future evidence, future
lineage, winner, and executable parity commitments before authority can exist.
Caller counters and `ResponsePackageState::Active` remain compatibility inputs;
neither is authority. Authority exists only as a validated, digest-bound
`ValidatedResponseAuthority`.

`lifecycle.rs` was deliberately not moved wholesale. Its substantive work is
candidate synthesis, support freeze, and package compilation, so it belongs to
the R6 learning cut rather than the R5 authority owner.

## Code Commits

```text
b87ce75  refactor: extract admission authority core
1771a96  refactor: centralize operator admission policy
d22fb7c  refactor: bind admission to independent proof reconstruction
```

## STOP Gate

```text
admission imports runtime/learning                         0
actor as verifier oracle                                   0
caller proof counters as authority                         0
admission report/schema drift                              0
ACTIVE/authority state change                              0
tamper and restart controls                             PASS
full historical failure-set delta                          0
authority                                              false
```

The facade public export surface did not change between STOP-R4 and the R5
code HEAD. Existing report and serde compatibility checks remain in the full
historical fingerprint.

## Exact Remote Gates

All exact gates used commit `d22fb7c` with an empty overlay.

```text
nando-operator-admission tests                           PASS
nando-operator-admission Clippy -D warnings             PASS
nando-operator-proof tests                               PASS
nando-operator-proof Clippy -D warnings                 PASS
nando-response-actor historical fingerprint             PASS
retired test failures                                       0
retired Clippy diagnostics                                  4
new background build processes                              0
exact-HEAD Graphify                                      PASS
```

The response facade still exits nonzero on its frozen historical test and
Clippy debts. The exact fingerprint proves that R5 introduced no new failures;
the four retired diagnostics are recorded by the R4 retirement manifest.

## Structural And Live Gates

```text
admission owner isolation route             PASS / authority_ready=false
proof reconstruction route                  PASS / authority_ready=false
authority truth route                       PASS / authority_ready=false
live composite gate                         PASS
eligible_for_local_accept                   false
response M3                                 WATCH
response ACTIVE packages                    0
false accepts                               0
runtime parity failures                     0
```

The live gate remains fail-closed. Its blockers are the absence of an ACTIVE
response package and insufficient M3 windows and coverage; none is relaxed by
this ownership move.

No R5 command deployed or restarted production. At documentation close both
`nando-transition-serving.service` and `nando-response-learning.service` are
`inactive/dead`; they are not started for decomposition verification.

## Size Result

```text
nando-response-actor/src before R0       103,389 tracked Rust lines
nando-response-actor/src after R4         87,883 tracked Rust lines
nando-response-actor/src after R5         87,184 tracked Rust lines
nando-operator-admission/src               1,003 tracked Rust lines
```

The facade boundary lost 699 lines during R5 and 16,205 lines since R0.

## Receipts

- `STOP_R5_ADMISSION_EXACT.json`
- `STOP_R5_PROOF_EXACT.json`
- `STOP_R5_FACADE_EXACT.json`
- `STOP_R5_LIVE_GATE.json`
- `stop-r5-admission-isolation.trace.json`
- `stop-r5-proof-reconstruction.trace.json`
- `stop-r5-authority-truth.trace.json`

STOP-R5 unlocks only R6, `nando-operator-learning`. F5-B remains frozen until
STOP-R9.
