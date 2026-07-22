# STOP-F8-D Causal Phase Controls

Status: `PASS / APPLICABILITY_GAIN / AUTHORITY_FALSE`

Date: `2026-07-22 Europe/Tallinn`

## Result

The runtime now exports proof-owned phase observations from the exact traffic
execution that produced the actor and verifier receipts. The generation-owned
shadow ledger commits those observations, and external admission recomputes the
aggregate from the ledger instead of accepting caller-provided counters.

```text
one immutable traffic receipt set
-> full phase
-> no phase
-> shuffled phase
-> magnitude only
-> matched random center
-> no Wave
-> generation-owned phase evidence
-> admission-owned aggregate
```

Final controlled live set:

```text
live receipts                       3
full phase correct / selected       3 / 3
no phase correct / selected         0 / 0
shuffled correct / selected         0 / 0
magnitude correct / selected        0 / 0
random center correct / selected    0 / 0
no Wave correct / selected          0 / 0
wrong actions                       0
false accepts                       0
parity mismatches                   0
censored semantic updates           0
support/future overlap              0
```

The measured gain is an applicability gain, not a search gain. Structural role
grounding already produces one action class for this controlled scalar
operator. Full phase crosses the frozen coherence floor; every phase ablation
abstains. F8-D therefore proves that the runtime phase field gates this
operator's applicability. It does not prove natural circuit discovery, lower
search cost, broad traffic coverage, or a product-level advantage over the
structural baseline.

## Ownership Repair

The earlier F8-C API allowed a caller to seal aggregate control counts. That
route is removed. The only accepted route is now:

```text
RuntimePhaseControlEvidenceV3
-> exact traffic report and dispatch-index join
-> GenerationShadowReceiptV3
-> immutable shadow ledger
-> derive_external_phase_control_receipt_v3
-> byte-identical external reconstruction
```

The canonical receipt is
`STOP_F8_D_PHASE_CONTROL_RECEIPT.json`. Its file SHA-256 is
`1207700a779bc6e1abf15aaa2dbabce7f69dbce4d51ed2db722e820420a1ecf9`.

## Performance Gate

Three isolated runs on CPU 4 of `e-MEGA-MINI-M1-13th` used 4,096 matched and
4,096 no-match samples per run:

```text
matched p99 ns      645301 / 646618 / 648010
no-match p99 ns     194403 / 194290 / 195340
hard max ns         690741 / 659750 / 660128
budgets              1000000 / 250000 / 2000000
```

The production allocator resource gate also passed:

```text
matched p99 ns                  368874
no-match p99 ns                 168661
hard max ns                     489076
hot RSS peak delta bytes      10493952
hot RSS budget bytes          16777216
```

## Boundary

```text
causal applicability                 PASS
causal search reduction              NOT_PROVEN
natural operator discovery           NOT_EVALUATED
execution authority                  false
local accept                         false
```
