# STOP-B1B0: Binding Evidence Preregistration

Date: 2026-07-21 Europe/Tallinn

```text
B1A claim corrected                 PASS
candidate hypotheses frozen         PASS
trusted label provenance            PASS
intervention matrix frozen          PASS
support/future lineage contract     SEALED
acquisition                         NOT RUN
F4                                  BLOCKED / NOT STARTED
execution authority                 false
```

## Scope

STOP-B1B0 freezes the scientific contract for a later B1B acquisition. It
does not collect traces, score H0 or H1, select a binding relation, compile a
selector, or create a `ProtocolMode`.

The corrected B1A result is:

```text
missing discriminator exists                    PROVEN
which causal relation resolves it               UNKNOWN
parent_action_to_capability_instance             HYPOTHESIS H1 / UNPROVEN
relation_not_observable                          HYPOTHESIS H0 / UNPROVEN
applicability-negative evidence in B1A           0 rows
```

The 86 machine probes now request only the distinction between the expected
action-equivalence class and its competing classes. They do not name H1 or any
other relation as the answer.

## Machine Artifacts

```text
plans/effect-law-unification-v1/STOP_B1B0_PREREGISTRATION.json
schema        nando.binding-evidence-preregistration.v1
file sha256   b9322ed7e5413b58d8c8e31cd89ef15105037b9c0ec41e5fde9ebe94403a5ab6

plans/effect-law-unification-v1/STOP_B1A_BINDING_EVIDENCE.json
schema        nando.binding-version-space-report.v1.r1
file sha256   e2b887fbb8569afc1e702b9132b6b45c6e77c0d60353c1586d1fd8ef09783b73
report sha256 cd9a6bdddd64cf8ad75f7be9e9c9c149030f7ff599716abd81fb63836bfed64b
```

The B1A report was regenerated from the same 129 frozen rows. After removing
the expected schema, report digest, and probe wording changes, its complete
JSON is structurally identical to the accepted B1A artifact.

## Trust Boundary

```text
frozen label-blind candidate graph
-> UntrustedBindingLabelEnvelopeV1
   capture receipt root
   parity receipt root
   verifier root
   external manifest root
   pre-action wire root
   session lineage
   support/future partition
   intervention ID
   evaluation label
-> canonical manifest bytes
-> externally pinned opaque TrustedBindingLabelManifestRootV1
-> TrustedBindingLabelSetV1
```

The envelope checksum establishes integrity only. Trust requires an exact
match to externally pinned manifest bytes before deserialization and a match
to the external manifest root. Replacing the expected action digest and
recomputing both the envelope checksum and manifest bytes is rejected as
`InvalidTrustRoot` against the original pin.

`TrustedBindingLabelManifestRootV1` has private fields and no production
constructor in STOP-B1B0. The only pin helper is compiled under `cfg(test)`.
This is deliberate: the external manifest owner is not invented by the
diagnostic module.

The older `ExpectedBindingReceiptV1::positive()` remains available only for
the accepted B1A diagnostic replay. Its checksum is not treated as B1B trust
and it cannot satisfy this preregistration.

## Frozen Hypotheses

```text
H0  relation_not_observable
    status              UNPROVEN
    observation source  pre_action_wire
    teacher action      forbidden

H1  parent_action_to_capability_instance
    status              UNPROVEN
    observation source  pre_action_wire
    teacher action      forbidden
```

Neither hypothesis receives priority, selector status, runtime authority, or
an implicit tie-break from the preregistration order.

## Frozen Interventions

```text
I1  change candidate order; hold parent linkage
    H1 predicts binding preserved

I2  change parent linkage; hold candidate order
    H1 predicts binding changed

I3  add same-type decoy; hold parent linkage
    H1 predicts binding preserved

I4  complete the parent; hold order and values
    H1 predicts not applicable

I5  expose two active parents; hold order and values
    H1 predicts ambiguous binding

I6  expose output without matching parent; hold order and values
    H1 predicts not applicable
```

H0 predicts `INSUFFICIENT_EVIDENCE` for every intervention. These predictions
are frozen before acquisition and serialized in the golden JSON.

## Denominator And Lineage Contract

The later acquisition manifest is admissible only with:

```text
support positive rows minimum                 6
support applicability negatives minimum      6
future positive rows minimum                  6
future applicability negatives minimum       6
rows per intervention per partition minimum  1
support/future session-lineage overlap        0
historical rows promoted to future            0
future rows captured post-freeze              required
censored rows used as negatives               forbidden
```

The trusted resolver enforces the positive, applicability-negative,
intervention, lineage, and post-freeze requirements. A 24-row synthetic
manifest exercises the contract in tests: one positive and one applicability
negative for every intervention in both support and future. Those fixtures
are not acquired B1B evidence and do not adjudicate H0 or H1.

## Observability

Every accepted envelope declares `pre_action_wire` as its observation source
and commits the corresponding wire root. `teacher_action` and
`post_action_state` are rejected before the label set becomes trusted.

No candidate value, expected value, path hash, ordinal, prefix, teacher
action, or post-state is introduced into the label-blind candidate graph by
STOP-B1B0.

## Verification

```text
B1B0 focused tests                       10 / 10 PASS
B1A focused tests                         11 / 11 PASS
F3R dual-classifier tests                 12 / 12 PASS
Canonical F2 V3 tests                     28 / 28 PASS
Historical F2 tests                       15 / 15 PASS
golden preregistration JSON parity                PASS
recomputed expected-digest forgery                REJECTED
support/future lineage overlap                    REJECTED
missing applicability-negative denominator       REJECTED
missing positive denominator                     REJECTED
missing intervention denominator                 REJECTED
teacher/post-action observability                 REJECTED
pre-freeze future row                             REJECTED
cargo check nando-response-actor                  PASS
Clippy warnings in B1B0 files                        0
accepted legacy Clippy warnings                     12
git diff --check                                  PASS
privacy-safe machine artifacts                    PASS
graphify update                                   PASS
production callers                                      0
execution authority                                 false
```

Production-copy B1A replay:

```text
elapsed       2:14.31
max RSS       455100 KiB
exit          0
rows          129 / 129
hypotheses    2441
ties          86
```

The full lib Clippy run exits successfully and reproduces the same 12 accepted
legacy warnings outside the B1B0 files. No new warning points to
`binding_evidence.rs`, `binding_evidence_preregistration.rs`, its tests, or
the new exports.

The privacy scan confirms that machine artifacts contain only hashes,
structural feature names, bounded counts, enum values, and generic probe text.
They contain no absolute paths, raw requests, provider payloads, expected
responses, UUIDs, or physical handle values.

## Structural Gates

```text
trust provenance owner                    PASS
lineage and negative denominator owner    PASS
authority lifecycle owner                 PASS
H0 owner-local route                      PASS
H1 owner-local route                      PASS
I1-I6 owner-local routes                   6 / 6 PASS
```

The first broad worksheets returned `VETO`: shared line references created
incompatible fillers, owner groups mixed manifest ownership with resolver
enforcement, and the combined hypothesis/intervention sheet had composite
interference despite exact pair matches. The packets were repaired by using
exact source lines, separating real owners, and applying the prescribed
`linked-group` split. Requirements and candidate claims were not weakened.
All original VETO traces remain preserved beside the passing traces:

```text
/home/ubu/tmp/nando-b1b0/nanda-tmp/nanda-structural-gate/
```

## Runtime State

Read-only systemd inspection after all checks:

```text
nando-response-learning.service
  state         active/running
  InvocationID  8e59505eb1b943778601c9b3bacbd607

nando-transition-serving.service
  state         active/running
  InvocationID  74ac3080f80b4fe387de2a94380e3657
```

Both IDs match the pre-STOP-B1B0 baseline. No `daemon-reload`, restart,
deployment, or service mutation was performed.

## Diff Ownership

```text
binding_evidence.rs
    relation-neutral distinguishing probes and diagnostic-only label warning

binding_evidence_preregistration.rs
    frozen hypotheses/interventions, prospective trusted-label resolver,
    lineage and denominator contracts

binding_evidence_preregistration_tests.rs
    trust attacks, observability, lineage, denominator, and golden controls

lib.rs
    diagnostic/preregistration exports only

STOP_B1A_BINDING_EVIDENCE.{json,md}
    corrected B1A machine and human claims

STOP_B1B0_PREREGISTRATION.{json,md}
    frozen machine and human preregistration

EFFECT_LAW_UNIFICATION_REFACTOR_V1.md
    lifecycle and hypothesis status
```

No STOP-B1B0 module imports or mutates runtime execution, admission, Wave,
generation, thresholds, selectors, checkpoints, systemd, registry authority,
or ACTIVE packages.

## STOP-B1B0

```text
B1A claim corrected                 PASS
candidate H0/H1 status              FROZEN / UNPROVEN
trusted label contract              PASS
intervention matrix                 FROZEN
support/future lineage contract     SEALED
applicability-negative contract     SEALED / acquisition pending
acquisition                         NOT RUN
binding relation selected           NO
selector / ProtocolMode             NOT CREATED
F4                                  BLOCKED / NOT STARTED
runtime / admission / authority     UNCHANGED
```

Work stops here. B1B acquisition and F4 are not started by this receipt.
