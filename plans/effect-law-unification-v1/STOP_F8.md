# STOP-F8 External Admission And Live Shadow

Status: `PASS / CONTROLLED_LIVE_SHADOW / AUTHORITY_FALSE`

Date: `2026-07-22 Europe/Tallinn`

## Final Tree

```text
real Rust HTTP boundary                         PASS
  -> hash-only provider capture                 PASS
  -> atomic capture restart                     PASS
  -> pinned generation                          PASS
  -> structural role grounding                  PASS
  -> phase applicability                        PASS
  -> actor                                      PASS
  -> independent verifier                       PASS
  -> generation-owned shadow ledger             PASS
  -> restart-monotonic append                    PASS
  -> admission-owned causal controls            PASS
  -> immutable external reconstruction          PASS
  -> SHADOW_READY                               PASS
  -> local accept                               FALSE
  -> ACTIVE mutation                            0
  -> execution authority                        FALSE
```

## STOP Matrix

```text
provider capture owner                         PASS
request-path disk writes                       0
second request-body hash                       0
raw payload bytes persisted                    0
restart sequence reuse                         0
generation-owned live receipts                 3
live verified passes                           3
full phase gain                                3
wrong actions                                  0
false accepts                                  0
parity mismatches                              0
censored semantic updates                      0
support/future overlap                         0
external verdict                               SHADOW_READY
execution authority                            false
```

## Verification

```text
kernel tests                                   18 PASS
learning tests                                217 PASS
persistence tests                              16 PASS
admission tests                                  7 PASS + 1 explicit live audit PASS
runtime tests                                  47 PASS / 1 release-only ignored
transition-serving lib                         51 PASS / 3 unchanged baseline FAIL
gateway control                                20 PASS
Clippy touched packages -D warnings              PASS
release latency runs                            3 / 3 PASS
release hot RSS                                 PASS, 10,493,952 B
live service NRestarts                          0
NANDA structural routes                         3 / 3 PASS
NANDA Wave causal section                       PASS
NANDA composite                                 VETO by design: local accept false
```

The three transition-serving failures are reproduced with the same names and
assertions on clean base commit `f042241`; F8 does not expand that baseline.

## Proven Claim

F8 proves that one controlled operator can traverse the complete real service
boundary, survive durable restart, execute through the winner-owned actor, pass
an independent verifier, carry runtime-owned phase controls, and reconstruct a
`SHADOW_READY` external candidate without granting authority.

It does not prove natural grokking, phase-driven search reduction, broad
ordinary-traffic coverage, 50 percent CPU savings, ACTIVE admission or M3.
Those remain later product gates.

The composite gate is intentionally not `PASS`: its deployment contract
requires `local_accept_enabled=true`, while STOP-F8 requires the opposite.
Enabling local accept merely to turn that gate green would invalidate this
stage. The structural and Wave-causal sections pass independently; production
deployment remains a separate authority-bearing change.

Canonical evidence:

```text
STOP_F8_D_PHASE_CONTROL_RECEIPT.json
STOP_F8_E_EXTERNAL_ADMISSION_CANDIDATE.json
STOP_F8.json
```
