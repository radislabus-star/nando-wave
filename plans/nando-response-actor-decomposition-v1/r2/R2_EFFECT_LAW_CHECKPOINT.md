# R2 EffectLaw Checkpoint

Status: `PASS / R2_IN_PROGRESS`

Base HEAD: `3b7e71f4645f7fba93c4ba2698ff98cdb1218d80`

Authority: `false`

F5-B: `NOT_STARTED`

## Ownership Cut

```text
EffectSource + canonical EffectLaw language/constants
  -> nando-operator-kernel::effect_law

CanonicalEffectLawV3 bytes, ID, validation, read-only access
  -> nando-operator-kernel::effect_law

observation, trust, quotient search, dual classification
  -> nando-response-actor::effect_law_v3
```

`CanonicalEffectLawV3` retains private fields. The response compiler constructs
it through canonical wire bytes; trust and classification use read-only
accessors. No public-field or unchecked-constructor bypass was introduced.

## Verification

```text
kernel EffectLaw tests                              1 / 1 PASS
all kernel tests                                  12 / 12 PASS
EffectLaw V3 quotient/trust/classifier tests      40 / 40 PASS
checked-in F3 golden report parity                      PASS
kernel Clippy -D warnings                              PASS
response denominator                                    520
response result                    494 PASS / 26 known FAIL
combined kernel + response result  506 PASS / 26 known FAIL
test failure fingerprint                               PASS
Clippy fingerprint                            12 + 8 / PASS
kernel forbidden side-effect imports                      0
duplicate canonical-law definitions                        0
new background build processes                             0
authority                                               false
```

The STOP runner receipt is `R2_EFFECT_LAW_REMOTE_STOP.json`. Both live services
retained their original invocation IDs with `NRestarts=0`.

This checkpoint permits the remaining immutable artifact and VM-contract moves
inside R2. It does not satisfy STOP-R2 and does not unlock R3.
