# North Star Proof V1 Implementation Checkpoint

Date: 2026-07-27

## Verdict

```text
MS3 generation registry                  IMPLEMENTED
MS3 generation 1                         CONTRADICTION
cellular phase-receipt support bridge     SHADOW TEST PASS
five-seed proof contract                  IMPLEMENTED
five-seed experiment                      NOT EVALUATED
North Star scientific claim               NOT PROVEN
authority                                 false
phase mutation                            false
```

## Implemented Route

```text
FrozenVersionSpaceEnvelopeV1
-> NorthStarProofContractV1
-> five fixed seeds
-> seven equal-budget arms
-> independently rooted arm artifacts
-> delayed transition and cleanup checks
-> support/future disjointness
-> snapshot and remote restore checks
-> NorthStarProofReportV1
```

The cellular arm has a narrower input contract:

```text
independently verified TypedExecutionStage receipts
-> VerifiedDeltaReceipt
-> BackwardWave
-> source-neutral relation fragments
-> CircuitSynthesizer
-> FrozenSynthesizedCircuitSet
```

It does not accept `ResponseProgram`, teacher output, or exact MS3 candidate
programs. Exact structural search remains a separate control arm.

## Fixed Arms And Thresholds

```text
cellular_wave_ensemble
equal_budget_monolith
exact_structural_search
no_phase
shuffled_phase
magnitude_only
random_center

required seeds                         5
minimum passing seeds                  4
minimum primary gain                  30 milli
minimum key ablation drop             50 milli
minimum key/non-key ratio           2000 milli
wrong accepts                          0
runtime parity failures                0
verifier coverage                   1000 milli
support/future overlap                 0
```

Every arm binds its numbers to an experiment-report root, a frozen-future root,
and a snapshot root. Contract, seed receipts, reports, and the MS3 generation
registry are canonical, bounded, restart-checked artifacts.

## Durable Lifecycle

`nando-transition-serving` now owns `generation-registry-v1.cbor` beside the
frozen contract, applicability ledger, prediction ledger, and independent
future. On restore it bootstraps a missing registry from a valid legacy
generation, rejects cross-generation root mismatch, and binds the terminal
future back to the exact generation. A different second terminal is rejected.

Read-only status:

```text
GET /v2/multi-source/ms3-generation-registry
```

No endpoint or report grants authority.

## Verification

```text
cargo +1.97.0 test -p nando-operator-learning --lib
  307 PASS

cargo +1.97.0 test -p nando-transition-serving --lib
  140 PASS / 2 ignored

cargo +1.97.0 clippy \
  -p nando-operator-learning \
  -p nando-transition-serving \
  --all-targets -- -D warnings
  PASS
```

The scientific experiment has not run. The implementation checkpoint must not
be quoted as ensemble-mode, ablation, seed-stability, snapshot-memory, or
remote-restore proof.
