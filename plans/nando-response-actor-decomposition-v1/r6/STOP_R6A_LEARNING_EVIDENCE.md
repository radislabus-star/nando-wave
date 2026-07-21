# STOP-R6A Learning Evidence

Status: `PASS`

Date: 2026-07-22

Code HEAD: `dae7ec36ba42b4fdef5ca324da51ac49182fc953`

Authority: `false`

## Result

```text
raw evidence envelope
-> deterministic ledger and evidence graph
-> capture commitments
-> label-blind binding graph and version space
-> support/future capture owners
-> compressed runtime parity evidence
-> teacher join
-> EffectGraph and EffectLaw V2 quotient
```

The new `nando-operator-learning` crate owns cold evidence and acquisition.
The response crate keeps compatibility facades and candidate-specific
integration only. `RuntimeParityCase` moved out of the admission bundle because
it is learning evidence, not authority.

The learning crate imports neither `nando-operator-runtime` nor
`nando-operator-admission`. Capture validation consumes immutable commitments;
the response facade alone extracts receipts from its concrete admission
candidate type.

## Code Commit

```text
dae7ec3  refactor: extract operator learning evidence
```

## Exact Gates

```text
nando-operator-learning tests                       PASS
nando-operator-learning Clippy -D warnings         PASS
nando-response-actor historical fingerprint        PASS
response known tests                         368 PASS / 26 known FAIL
response effective Clippy diagnostics                 14 known
new response test failures                              0
new response Clippy diagnostics                         0
overlay files in exact gates                            0
new background build processes                          0
learning imports runtime/admission                       0
authority                                            false
```

Two old binding-evidence test lints left the facade and were fixed in the new
owner. The four R4 retirements remain unchanged; all six facade retirements are
listed in `R6A_RETIRED_CLIPPY_DIAGNOSTICS.tsv`.

## Size Result

```text
nando-response-actor/src before R0       103,389 tracked Rust lines
nando-response-actor/src after R5         87,184 tracked Rust lines
nando-response-actor/src after R6-A       73,815 tracked Rust lines
nando-operator-learning/src               13,483 tracked Rust lines
```

No deploy, restart, registry mutation, F5-B work, or authority change occurred.

## Receipts

- `R6A_LEARNING_EXACT.json`
- `R6A_FACADE_EXACT.json`
- `R6A_RETIRED_CLIPPY_DIAGNOSTICS.tsv`

STOP-R6A unlocks only R6-B. It does not unlock F5-B.
