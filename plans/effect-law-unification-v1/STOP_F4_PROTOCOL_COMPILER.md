# STOP-F4R Protocol Compiler

Date: 2026-07-21

Base HEAD: 093c63c09e1df2f035cfe97478af21411a82b4ff

Verdict: F4R_CODE_COMPLETE_CONTROLLED_PASS_REAL_ADMISSION_BLOCKED

Authority: false

## Route

```text
PhysicalActorObservationV2
-> IndependentTrialVerifierReceiptV2
-> PhysicalTrialReceiptV2
-> TrustedResolvedBindingRowsV2
-> AcceptedBindingLawEvidenceV2
-> CanonicalEffectLawV3
-> protocol_mode::compile_protocol_modes_for_effect_law_v3()
-> internally generated mode candidates
-> derived mode x row matrix
-> bounded exact-cover search
-> ProtocolModeSetV2 | ABSTAIN
```

F4R closes the STOP-F4 review findings in controlled code. It does not connect
ProtocolModes to runtime, generation, checkpoint, admission, ACTIVE packages,
deploy, or restart.

Real independent evidence admission remains BLOCK because current physical
receipts are still controlled envelopes, not production physical truth.

## V1 Compatibility

Four historical B1B V1 golden JSON artifacts remained byte-identical:

```text
8e26fcafbbf723127cae096febde4d0a22e702b390ec98da2054a4acf73e0aeb  STOP_B1B_ADJUDICATION.json
ab166bd7f074e33be249846dcbb45f1c6aeb4ce4a5576656c2b52cc23d0cbfae  STOP_B1B_EXTERNAL_LABEL_TRUST.json
ebb4d19258cb61c50c4fa70c67107d648fae998e857d8e5a0f9c78f6e8ea15f7  STOP_B1B_LABEL_MANIFEST.json
91993bfff1296e741e314a5150ff6aabd68c1842417b53a73b92d84bd8985314  STOP_B1B_PHYSICAL_LABEL_RECEIPTS.json
```

Public V1 facade names and JSON fields are preserved.

## V2 Receipt Boundary

The V2 trial path remains tamper-evident and authority-free:

```text
actor program digest swap        -> REJECT
verifier program digest swap     -> REJECT
same actor/verifier program      -> REJECT
candidate action root mutation   -> REJECT
pre/post/delta root mutation     -> REJECT
graph/capture root mutation      -> REJECT
actor/verifier receipt mutation  -> REJECT
verifier disagreement            -> FAIL receipt, no authority
environment unavailable          -> CENSORED, no semantic evidence
V1 fixture truth dependency      -> forbidden by source scan
JSON restart                     -> byte-identical
```

No V2 physical trial code calls `support_scene()`, `future_scene()`,
`intervention_id`, teacher labels, row ordinal truth, or expected law.

## Capability

Only `adjudicate_binding_law_evidence_v2()` can create
`AcceptedBindingLawEvidenceV2`. The capability:

```text
has private fields
does not derive Deserialize
has no public constructor
contains proof roots and zero-error counters
contains no selector
contains no ProtocolMode
keeps execution_authority=false
```

`BindingAdjudicationReportV2` is a report only and cannot be converted into the
capability. Controlled fixture evidence can create a scoped capability for
compiler tests, but `production_admissible=false`.

## Compiler Repair

Owner:

```text
crates/nando-response-actor/src/protocol_mode.rs
```

Canonical F4R consumes only:

```text
AcceptedBindingLawEvidenceV2
typed CanonicalEffectLawV3
frozen graph views already present in trusted rows
compiler budget
```

The canonical entrypoint no longer accepts caller-supplied mode candidates or a
caller-supplied coverage matrix. It internally groups positive PASS rows by:

```text
relation_identity_sha256
protocol_facet_root_sha256
effect_invariant_root_sha256
```

It then derives the mode x row matrix from trusted row facets and physical
trial outcomes. The old manual candidate verifier is `#[cfg(test)]` only and is
used for adversarial controls.

Compile gates:

```text
positive coverage complete
wrong_actions = 0
verify_failed = 0
negative_accepts = 0
search not exhausted
all complete exact covers action-equivalent
```

Otherwise the compiler emits `ABSTAIN` and clears selected modes.

## Exact Cover

The compiler now searches bounded exact covers instead of requiring one mode to
cover every positive row.

Required control:

```text
mode A covers only positive A
mode B covers only positive B
A alone                         -> ABSTAIN
A union B                       -> ProtocolModeSet
two non-equivalent full covers  -> ABSTAIN
```

Caller claims are not authority:

```text
candidate claims both positives but matches only one row  -> coverage = 1
candidate claims zero positives but matches one row       -> coverage = 1
unknown claimed negative row                              -> ignored as claim, still ABSTAIN on incomplete coverage
known matched applicability-negative row                  -> negative_accepts = 1, ABSTAIN
```

## Controlled Output

Controlled end-to-end fixture result:

```text
verdict                         ProtocolModeSet
selected modes                  2
action_equivalence_classes      1
positive_rows                   2
positive_rows_covered           2
wrong_actions                   0
verify_failed                   0
negative_accepts                0
search_exhausted                false
production_admissible           false
execution_authority             false
```

## Checks

```text
CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor physical_trial_v2_tests --lib
  15/15 PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor binding_evidence_adjudication --lib
  24/24 PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor effect_law_v3 --lib
  40/40 PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor effect_law::tests:: --lib
  15/15 PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor binding_evidence --lib
  94/94 PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 check -p nando-response-actor --all-targets
  PASS

cargo +1.97.0 fmt --all -- --check
  PASS

git diff --check
  PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor --lib
  496 PASS / 26 known FAIL

CARGO_INCREMENTAL=0 cargo +1.97.0 clippy -p nando-response-actor --all-targets --message-format short
  BLOCKED by existing semantic_alias.rs unwrap errors; 17 legacy warnings; no new F4R-file diagnostics observed
```

Known full-baseline failures remain outside this slice.

## Structural Gates

Owner-local NANDA structural routes:

```text
f4r-compiler-owner          PASS authority_ready=false
f4r-proof-boundary-owner    PASS authority_ready=false
f4r-authority-boundary      PASS authority_ready=false
```

`nando-live-transition-gate --project-root /home/ubu/projects/nando-wave`:

```text
verdict                      PASS
eligible_for_local_accept    false
response active packages     0
response M3                  WATCH
response false accepts       0
runtime false accepts        0
runtime parity mismatches    0
```

This is a safety gate only. It does not enable local accept.

Graphify was updated after the code changes:

```text
24667 nodes / 56605 edges / 1066 communities
```

## Production Boundary

```text
production callers of canonical F4 API    0
runtime changed                           no
admission changed                         no
generation changed                        no
checkpoint changed                        no
deploy/restart/push                       no
execution_authority                       false
real independent B1B receipts             0 / NOT_EVALUATED
real evidence admission                   BLOCK
F5/runtime grounding                      NOT_STARTED
```

Final STOP-F4R status:

```text
F4R proof boundary / skeleton     PASS
Canonical F4 compiler             PASS on controlled evidence
real evidence admission           BLOCK
production authority              false
```
