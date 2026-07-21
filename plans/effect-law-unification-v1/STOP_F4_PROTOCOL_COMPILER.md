# STOP-F4R2 Executable Structural Protocol Compiler

Date: 2026-07-21

Base HEAD: f87dc6018995b00eb0880966130d0ad36b7d1f54

Verdict: F4R2_CONTROLLED_STRUCTURAL_PASS_REAL_ADMISSION_BLOCKED

Authority: false

## Route

```text
PhysicalActorObservationV2
-> IndependentTrialVerifierReceiptV2
-> PhysicalTrialReceiptV2
-> TrustedResolvedBindingRowsV2
-> AcceptedBindingLawEvidenceV2
       + CanonicalEffectLawV3
       + sealed FrozenCandidateRelationGraphV1 payloads
-> bounded structural selector induction
-> execute each selector over each frozen graph
-> labels score executed outcomes
-> individually safe ProtocolModeV2 candidates
-> bounded exact cover
-> ProtocolModeSetV2 | ABSTAIN
```

F4R2 closes the two remaining F4 defects:

1. `ProtocolModeV2` now carries executable typed role/selector/capability
   payload, not only SHA-256 commitments.
2. The compiler derives its mode/row matrix by executing selectors over sealed
   graph payloads. Caller coverage claims and row-facet equality are not the
   canonical matrix.

The new path is still shadow-only. It has no production caller and does not
connect generation, checkpoint, runtime grounding, actor, verifier, admission,
deployment, or service restart.

## Executable Mode Payload

```text
ProtocolModeProgramV2
├─ ProtocolSourceRoleSchemaV2
├─ ProtocolSelectorProgramV2
├─ ProtocolValueContractV2
├─ ProtocolCapabilityContractV2
├─ ProtocolArgumentRoleSchemaV2
├─ ProtocolConstantContractV2
├─ ProtocolStructuralGuardV2
└─ ProtocolTemporalCardinalityContractV2
```

The selector reuses `BindingPredicateV1` and
`FrozenCandidateRelationGraphV1`; F4R2 does not introduce a second graph or
runtime language. Current scope is one uniquely bound scalar action class.
Multi-role composition remains F5+ work.

## Non-Circular Matrix

Positive PASS rows supply bounded hypothesis seeds. Their target digest is used
only to locate the seed node during cold compilation; it is absent from the
emitted selector. For every candidate and every trusted row:

```text
frozen graph
-> source role type filter
-> selector predicates
-> selected action-equivalence classes
-> only then trusted row label/outcome scoring
```

The canonical entrypoint accepts no caller-supplied candidates or coverage
matrix. The historical manual candidate compiler remains `#[cfg(test)]` for
adversarial controls only.

Unsafe hypotheses are expected during bounded induction. They are rejected
before exact cover. `wrong_actions`, `verify_failed`, and `negative_accepts`
describe the admitted result; when no safe cover exists they retain diagnostic
counts from rejected hypotheses.

## Controlled Causal Result

```text
physical protocol facets                 2
positive rows                             4
  support / future per facet              1 / 1
applicability-negative rows               4
  support / future per facet              1 / 1
direct and renamed/wrapped layouts        PASS
selected modes                            2
selector law                              request_relation = mentioned
positive rows covered                     4 / 4
wrong actions                             0
verify failed                             0
negative accepts                          0
search exhausted                          false
action-equivalence classes                1
production admissible                     false
execution authority                       false
```

The selected selector contains no field name, function name, row ordinal,
target value, or raw request text.

## Fail-Closed Controls

```text
missing graph payload                     InvalidGraphView
extra graph payload                       InvalidGraphView
mutated graph with stale root             InvalidGraphView
symmetric roles with two action classes   ABSTAIN
incompatible EffectLawV3                  ABSTAIN
candidate budget exhaustion               ABSTAIN
exact-cover search exhaustion             ABSTAIN
wrong action                              rejected before cover
verifier failure                          rejected before cover
applicability-negative acceptance         rejected before cover
non-equivalent complete covers            ABSTAIN
tampered restart selector                 REJECT
canonical restart                         byte-identical
```

The persisted mode set revalidates schema, roots, typed program payload,
sorted unique modes, disjoint sorted row coverage, exact denominator, action
class count, digest, and `execution_authority=false`.

## Compatibility

The four historical B1B V1 golden artifacts remain byte-identical:

```text
8e26fcafbbf723127cae096febde4d0a22e702b390ec98da2054a4acf73e0aeb  STOP_B1B_ADJUDICATION.json
ab166bd7f074e33be249846dcbb45f1c6aeb4ce4a5576656c2b52cc23d0cbfae  STOP_B1B_EXTERNAL_LABEL_TRUST.json
ebb4d19258cb61c50c4fa70c67107d648fae998e857d8e5a0f9c78f6e8ea15f7  STOP_B1B_LABEL_MANIFEST.json
91993bfff1296e741e314a5150ff6aabd68c1842417b53a73b92d84bd8985314  STOP_B1B_PHYSICAL_LABEL_RECEIPTS.json
```

The public V1 facade and checked-in V1 JSON fields are unchanged.

## Checks

Heavy Rust checks ran on the dedicated build host:

```text
e@192.168.3.94:/home/e/projects/nando-wave-f4r2-build
```

```text
physical_trial_v2_tests                  18 / 18 PASS
binding_evidence suite                   97 / 97 PASS
effect_law_v3                            40 / 40 PASS
historical effect_law                    15 / 15 PASS
cargo check --all-targets                PASS
rustfmt --check                          PASS
git diff --check                         PASS
full lib baseline                        499 PASS / 26 known FAIL
```

The 26 full-baseline failures are the same historical failure set outside this
slice. F4R2 adds three passing tests and no failure.

Crate-wide `clippy -D warnings` remains blocked by 12 existing library and 8
test-only diagnostics outside the F4R2 files. No diagnostic names
`protocol_mode.rs`, `protocol_mode/selector.rs`, or the new F4R2 test code.

## Structural And Live Gates

Owner-local NANDA routes:

```text
f4r2 compiler owner       PASS / authority_ready=false
f4r2 matrix proof owner   PASS / authority_ready=false
f4r2 admission boundary   PASS / authority_ready=false
```

The first broad worksheet correctly returned `VETO` because it mixed compiler,
proof, runtime, verifier, and admission owners. Splitting by decision owner
removed the conflict without granting authority.

`nando-live-transition-gate`:

```text
composite verdict             VETO
structural                    PASS
wave causal                   PASS
runtime admission             PASS
deployment                    PASS
response runtime              VETO
eligible_for_local_accept     false
response ACTIVE packages      0
response M3                   WATCH
response false accepts        0
runtime false accepts         0
runtime parity mismatches     0
```

The response-runtime veto is caused by `no_active_response_package`,
`m3_windows_below_required`, `m3_window_coverage_below_threshold`, and
`response_baseline_safety_veto`. It does not invalidate the controlled F4R2
compiler proof, but it blocks live authority exactly as intended. This
read-only safety gate does not promote F4R2.

## Graphify

The exact-commit graph is stored untracked, as required by the architecture
canon. `graphify-out/SOURCE_RECEIPT.md` records the final source commit, command,
host, and graph counts. The generated graph is not production authority.

## Production Boundary

```text
production callers of F4R2 API       0
real independent B1B receipts         0 / NOT_EVALUATED
F5 runtime grounding                  NOT_STARTED
actor/verifier from F4R2              NOT_CONNECTED
admission from F4R2                   BLOCK
runtime changed                       no
generation/checkpoint changed         no
deploy/restart                         no
execution_authority                   false
```

Final STOP-F4R2 status:

```text
proof-owner boundary                  PASS
executable structural compiler        CONTROLLED PASS
real natural binding law              NOT PROVEN
F5 runtime convergence                NEXT
production authority                  false
```
