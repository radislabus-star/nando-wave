# STOP-F4 Protocol Compiler

Date: 2026-07-21

Verdict: F4_CODE_COMPLETE_CONTROLLED_PASS_REAL_ADMISSION_BLOCKED

Authority: false

## Route

```text
PhysicalActorObservationV2
-> IndependentTrialVerifierReceiptV2
-> PhysicalTrialReceiptV2
-> TrustedResolvedBindingRowsV2
-> AcceptedBindingLawEvidenceV2
-> ProtocolModeCompilerV2
-> ProtocolModeSetV2 | ABSTAIN
```

F4 is code complete as a bounded compiler. It is not connected to runtime,
generation, checkpoint, admission, ACTIVE packages, deploy, or restart.

## V1 Compatibility

Four historical B1B V1 golden JSON artifacts remained byte-identical:

```text
8e26fcafbbf723127cae096febde4d0a22e702b390ec98da2054a4acf73e0aeb  STOP_B1B_ADJUDICATION.json
ab166bd7f074e33be249846dcbb45f1c6aeb4ce4a5576656c2b52cc23d0cbfae  STOP_B1B_EXTERNAL_LABEL_TRUST.json
ebb4d19258cb61c50c4fa70c67107d648fae998e857d8e5a0f9c78f6e8ea15f7  STOP_B1B_LABEL_MANIFEST.json
91993bfff1296e741e314a5150ff6aabd68c1842417b53a73b92d84bd8985314  STOP_B1B_PHYSICAL_LABEL_RECEIPTS.json
```

Public V1 facade names and JSON fields are preserved.

## V2 Receipt Security

New owners:

```text
physical_actor_observation_v2.rs
independent_trial_verifier_v2.rs
physical_trial_v2.rs
```

The V2 trial path separates actor observation from verifier receipt and seals
only exact joined roots. Actor and verifier program digests must differ.

Covered attacks:

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

## Resolver Integrity

New owner:

```text
trusted_resolver_v2.rs
```

The resolver consumes frozen rows, sealed physical trials, resolver program
digest, and an externally supplied manifest root. It does not build scenes,
does not execute the actor, and does not call V1 label-manifest construction.

The resolved row set is opaque: its wire form is private, has no `Default`, and
keeps `execution_authority=false`.

Tampered graph/row/trial/trust roots are blocked by the resolver manifest root.

## Capability

New owner:

```text
binding_law_evidence_v2.rs
```

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
capability.

Controlled fixture evidence can create a scoped capability for compiler tests,
but `production_admissible=false`. Real independent admission is still BLOCK.

## Compiler

New owner:

```text
protocol_mode_compiler_v2.rs
```

The F4 compiler consumes only:

```text
AcceptedBindingLawEvidenceV2
EffectLawIdV3 root
bounded ProtocolMode candidates
compiler budget
```

It emits `ProtocolModeSetV2` only when all gates pass:

```text
positive coverage complete
wrong_actions = 0
verify_failed = 0
negative_accepts = 0
search not exhausted
surviving covers share one action-equivalence class
```

Otherwise it emits `ABSTAIN` and clears surviving modes.

Additional fail-closed repair in this slice: any reported negative accept now
blocks the candidate, even if the row root is not part of the known negative
denominator.

## ProtocolMode Output

Controlled end-to-end fixture result:

```text
verdict                         ProtocolModeSet
surviving modes                 2
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

Adversarial compiler controls:

```text
known negative accept           -> ABSTAIN
unknown negative accept         -> ABSTAIN
competing action class          -> ABSTAIN
exhausted search                -> ABSTAIN
```

## Checks

```text
CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor physical_trial_v2 --lib
  13/13 PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor binding_evidence_adjudication --lib
  22/22 PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor effect_law_v3 --lib
  40/40 PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor effect_law:: --lib
  15/15 PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 check -p nando-response-actor --all-targets
  PASS

cargo +1.97.0 fmt --all -- --check
  PASS

git diff --check
  PASS

CARGO_INCREMENTAL=0 cargo +1.97.0 test -p nando-response-actor --lib
  494 PASS / 26 known FAIL

CARGO_INCREMENTAL=0 cargo +1.97.0 clippy -p nando-response-actor --all-targets --message-format short
  BLOCKED by existing legacy semantic_alias.rs unwrap errors; no new V2/F4 diagnostics observed
```

Known full-baseline failures are unchanged and remain outside this slice.

## Structural Gates

Owner-local NANDA structural routes:

```text
actor_observation_owner      PASS authority=false
verifier_owner               PASS authority=false
trial_join_owner             PASS authority=false
resolver_owner               PASS authority=false
capability_owner             PASS authority=false
compiler_owner               PASS authority=false
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

## Production Boundary

```text
production callers of V2/F4 API    0
runtime changed                    no
admission changed                  no
generation changed                 no
checkpoint changed                 no
deploy/restart/push                no
graphify update                    skipped to preserve dirty graphify-out/
execution_authority                false
real independent B1B receipts      0 / NOT_EVALUATED
real evidence admission            BLOCK
F5/runtime grounding               NOT_STARTED
```

Final STOP-F4 status:

```text
F4 compiler code              PASS
controlled compiler tests     PASS
real evidence admission       BLOCK
production authority          false
```
