# STOP-B1B Causal Binding Adjudication

Date: 2026-07-21 Europe/Tallinn

## Post-Review Status

```text
B1B controlled fixture                CONTROLLED_FIXTURE_PASS
independent physical truth            BLOCK
AcceptedBindingLawEvidence            BLOCK
F4 ProtocolMode compiler              BLOCKED
execution authority                   false
```

This addendum changes no frozen evidence. The machine JSON remains an immutable
receipt of the original controlled run, including its historical
`f4_status=UNLOCKED_NOT_STARTED` field. Architectural review found that the
physical scene reconstruction, proof actor, proof verifier, label production,
trust orchestration, and adjudication still share one proof module. Therefore
that field is not current compiler authorization. Current status is owned by
[`../../docs/CORE.md`](../../docs/CORE.md).

## Original Controlled-Run Verdict

The following block is preserved as the verdict emitted at execution time. Its
F4 line is superseded by the post-review status above.

```text
B1B-S label-blind support              12 / 12 FROZEN
B1B-F label-blind future               12 / 12 FROZEN
physical replay                        24 / 24 BYTE-EXACT
support labels                         6 positive / 6 applicability-negative
future labels                          6 positive / 6 applicability-negative
candidate execution parity             0 failures
wrong H1 bindings                       0
negative accepts                        0
I1-I6 predictions                       6 / 6 PASS
H0 relation_not_observable              REJECTED
H1 parent_action_to_capability_instance SUPPORTED
F4 ProtocolMode compiler                UNLOCKED / NOT STARTED
selector / ProtocolMode / authority     NOT CREATED / false
```

This is the closed controlled B1B result. It identifies the missing causal
pre-action relation. It does not yet prove a production selector, a compiled
`ProtocolMode`, a live Rich Operator, or execution authority.

The current physical observer is a bounded controlled proof fixture. Its
reconstructed scenes, candidate executor, and proof verifier are not production
runtime components and must not become a second `RuntimeRoleBinder`. Splitting
those proof owners and deriving privately validated
`AcceptedBindingLawEvidence` is a prerequisite for F4, not additional authority
carried by this receipt.

## Signal Route

```text
frozen support + frozen future
-> deterministic physical scene replay
-> byte-identical capture records and candidate graphs
-> candidate actions executed against the physical capability state
-> independent verifier checks every observed delta
-> hashed PhysicalLabelReceipt set
-> untrusted label manifest
-> separate external TrustOwner pins exact manifest bytes
-> trusted label resolver
-> H0/H1 adjudicator
-> STOP-B1B
```

The physical observer has no trust-root constructor. The trust owner does not
generate labels. The adjudicator cannot capture evidence, compile a selector,
or grant authority.

## What Was Actually Learned

The accepted relation is:

```text
active parent action instance
-> advertised capability instance
-> unique candidate action
```

The six interventions separate this relation from candidate order and local
surface layout:

```text
I1 reorder candidates, preserve linkage       unique binding preserved
I2 change parent linkage, preserve order       selected parent changes
I3 add same-type decoy                         unique binding preserved
I4 complete parent                             not applicable
I5 expose two active parents                   ambiguous
I6 remove matching parent                      not applicable
```

Support and future use disjoint session lineages. Future uses unseen field
names and four layouts. Raw strings are used only inside bounded replay and are
not persisted in the receipt set; persisted evidence contains structural roles,
action-equivalence hashes, execution outcomes, and proof roots.

## Shortcut Rejections

```text
label from intervention ID                    rejected by frozen-row join
recomputed expected action + envelope digest rejected by external manifest pin
recomputed relation + physical receipt       rejected by external receipt pin
changed preregistration or B1A denominator    rejected by owner challenge
teacher action or post-action label source    absent
candidate order as authority                  falsified by I1
negative row accepted                         0
parity mismatch                               0
```

## Machine Artifacts

```text
STOP_B1B_PHYSICAL_LABEL_RECEIPTS.json
  file sha256 91993bfff1296e741e314a5150ff6aabd68c1842417b53a73b92d84bd8985314
  root        aa323d3fb44ba68c3a0bcdf2571d6359e66dcfd83e67cba837c911168cddae21

STOP_B1B_LABEL_MANIFEST.json
  file sha256 ebb4d19258cb61c50c4fa70c67107d648fae998e857d8e5a0f9c78f6e8ea15f7

STOP_B1B_EXTERNAL_LABEL_TRUST.json
  file sha256 ab166bd7f074e33be249846dcbb45f1c6aeb4ce4a5576656c2b52cc23d0cbfae
  receipt     d895a1581971178758c5fa923351ef2d8c737ff5c697bfc09c1ac9e383c6e5a6

STOP_B1B_ADJUDICATION.json
  file sha256 8e26fcafbbf723127cae096febde4d0a22e702b390ec98da2054a4acf73e0aeb
  report      1e68a3aa066a3cc94a4f3653a3d8d39ac0da493c617a8e4c6e7d0f574e5cbdb0
  relation    8230edf11ece28d5b4e4fdf22e47c985323c8a9e995eecefbdd1dc4618c212b6
```

## Post-Review Boundary

F4 may not begin from this receipt. First split physical trial ownership,
trusted label resolution, causal adjudication, and report serialization. The
physical trial owner must consume observed execution plus independently
committed verifier results and must not reconstruct truth from intervention
metadata. Only the adjudicator may create a private, validated
`AcceptedBindingLawEvidence` capability.

After that capability exists, F4 may compile competing bounded structural
modes, run complete search, and retain only modes with `WRONG=0`,
`VERIFY_FAILED=0`, and negative accepts `=0`.

No service was restarted or deployed. `execution_authority=false` remains
mandatory.

## Verification Receipt

```text
focused B1B adjudication tests       9 / 9 PASS
response-actor full baseline      481 PASS / 26 known FAIL
previous frozen baseline          472 PASS / 26 known FAIL
new B1B failures                     0
format / bin compilation             PASS
release binaries build               PASS
golden artifact byte parity        4 / 4 PASS
strict Clippy B1B diagnostics          0
legacy Clippy diagnostics         12 lib + 8 test-only

physical observer route              PASS, authority_ready=false
external trust-owner route           PASS, authority_ready=false
trusted label resolver route         PASS, authority_ready=false
causal adjudicator route             PASS, authority_ready=false
live composite gate                  PASS, eligible_for_local_accept=false

response ACTIVE packages                0
response false accepts                  0
response runtime parity failures        0
M3                                  WATCH
```

The 26 full-library failures are the same named legacy baseline recorded at
STOP-B1B-F; the nine newly added tests account exactly for the
increase from 472 to 481 passes. Strict Clippy still stops on the same legacy
library and test-only diagnostics, with no diagnostic in a B1B adjudication or
owner binary.

```text
full baseline wall time              214.92 s
release build wall time              211.90 s
release build peak RSS              2676228 KiB
Graphify wall time                    34.70 s
Graphify graph                   24418 nodes / 55919 edges / 1043 communities
```

Read-only service verification preserved the existing processes:

```text
nando-response-learning  InvocationID 8e59505eb1b943778601c9b3bacbd607
nando-transition-serving InvocationID 74ac3080f80b4fe387de2a94380e3657
```
