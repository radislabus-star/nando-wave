# Effect Law Unification: STOP-F2R (REJECTED)

> **Historical receipt only. F2R was rejected by architecture review on
> 2026-07-21.** Its self-signed verified type, single-transition semantic
> quotient, constant classification, and missing dictionary/mapping contracts
> are not canonical. The replacement candidate is
> `STOP_F2R2_EVIDENCE_BOUND_QUOTIENT_CANDIDATE.md`. F3 remains forbidden.

Date: 2026-07-21 Europe/Tallinn

Status: **F2R REJECTED / HISTORICAL ONLY / F3 NOT STARTED**

This is an uncommitted repair candidate for the rejected F2. It removes the
closed effect and role ontology before any grouping migration begins.

## Git And Authority

```text
HEAD                         32ce298799b331db32a311654c070ad5c393a00e
origin/main                  23c04b728999716c53c988b0e67f03df034cefe5
F2R commit created           NO
push                         NO
production deployed          NO
services restarted           NO
execution_authority          false
service active since         2026-07-20 05:38:30 EEST
```

F3, B1, F4, runtime, verifier, generation, admission, and authority were not
changed.

## P0 Repair

Rejected route:

```text
EffectKindV2 closed enum
+ EffectRoleKindV2 closed enum
+ one role per kind
-> new law requires Rust variant
-> Rich Operator cannot express repeated roles
```

F2R route:

```text
verified TeacherTransition
-> complete physical EffectGraph
-> transport-neutral canonical topology
-> alpha-canonical RoleRef program
-> bounded EffectClauseV2[]
-> deterministic bytes
-> EffectLawId
```

There is no `EffectKindV2`, `EffectRoleKindV2`, predicate enum, or
postcondition enum in the candidate.

## Open Relation IR

```text
EffectRoleV2
|- role_id: RoleRefV2
|- canonical topology node
|- open EffectValueTypeV2(u16)
`- min/max cardinality

EffectClauseV2
|- open EffectOpcodeV2(u16)
|- lhs RoleRefV2
|- optional rhs RoleRefV2
`- optional TypedConstantCommitmentV2

CanonicalEffectLawV2
|- transport-neutral topology
|- roles[]
|- clauses[]
`- preserved frame roles[]
```

`EffectOpcodeV2` accepts any nonzero numeric opcode. The identity layer can
store and fingerprint a data-defined relation program without a Rust enum
change. Runtime capability binding and external admission remain responsible
for rejecting unsupported execution opcodes.

Repeated value types are legal. Role IDs are alpha-renamed from canonical
topology node, value type, and cardinality. One role per topology node remains
the structural invariant; there is no one-role-per-semantic-kind restriction.

## Evidence Boundary

Manual construction is explicitly named:

```text
CanonicalEffectLawV2::from_unverified_program(...)
-> EffectLawVerificationStatusV2::Unverified
```

Verified status has a separate type and route:

```text
accepted TeacherTransition
+ runtime parity evidence receipt
+ complete physical EffectGraph
+ valid SHA-256 proof commitments
-> derive_verified_effect_law_v2(...)
-> VerifiedCanonicalEffectLawV2
-> EffectLawEvidenceReceiptV2
```

The manual constructor cannot emit `VerifiedCanonicalEffectLawV2` or an
evidence receipt. Rejected and parityless transitions fail closed.

## Physical Quotient

The law topology removes operation nodes owned by physical transport adapters,
then re-canonicalizes the remaining typed relation graph under a bounded
permutation budget. Physical Identifier/Integer/String scalar handles become
the open semantic class `EFFECT_VALUE_OPAQUE_SCALAR`; collections remain a
separate structural class.

Therefore:

```text
wait(function, identifier handle)
write_stdin(custom wrapper, integer handle, chars="")
-> same transport-neutral topology and relation program
```

Non-empty action constants produce a typed SHA-256 commitment clause and
therefore a different law ID. Raw function names, argument names, transport
wrappers, payload values, and prefixes never enter canonical bytes.

## Source Ownership

```text
effect_law.rs         831 lines  domain IR, canonicalization, evidence derivation
effect_law_tests.rs   424 lines  proof corpus only
lib.rs                           crate exports only
```

Source SHA-256:

```text
effect_law.rs        b30a11bf026a59129e95fc3506d3aa0290dc532c56c716808f583551ea7c8cae
effect_law_tests.rs  7865f30c2723daa0d53a69c4226f79e9f3f2511cd2fd3ea94f8b4cc5f9f5c56a
```

No diagnostic, runtime, verifier, semantic alias, generation, admission, or
authority module imports the F2R path.

## Mandatory Tests

```text
distinct real wait/write_stdin transitions -> one ID       PASS
direct/wrapped transitions -> one ID                       PASS
non-empty input -> different ID                            PASS
two same-typed source roles admissible and alpha-canonical PASS
multi-output law admissible                                PASS
new composition law assembled as clauses                  PASS
open data-defined opcode representable                     PASS
manual program remains unverified                          PASS
rejected/parityless transition cannot verify               PASS
changed preserved frame changes ID                         PASS
restart bytes and ID byte-identical                        PASS
incomplete topology has no law                             PASS
```

Focused result:

```text
cargo +1.97.0 test -p nando-response-actor --lib effect_law::tests
12 PASS / 0 FAIL
```

Broad compatibility baseline:

```text
semantic_ tests  22 PASS / 3 known FAIL
```

The same three accepted `online_collection` failures remain unchanged.

Clippy `-D warnings` still reports the pre-existing global debt. No diagnostic
points to `effect_law.rs` or `effect_law_tests.rs`.

## NANDA Gates

The four routes reported as VETO by review were rerun independently:

```text
generic identity       PASS mandatory complexity 16
multi-role             PASS mandatory complexity 14
extensibility          PASS mandatory complexity 14
evidence derivation    PASS mandatory complexity 14
opcode authority       PASS owner-local boundary
```

The first mixed extensibility/admission worksheet remains preserved as VETO;
it was split by decision owner rather than relabeled.

Trace directory:

```text
/home/ubu/tmp/nando-f2r-gate-tmp/nanda-structural-gate/
```

## Remaining Review Questions

1. Whether the transport-neutral topology quotient is the accepted canonical
   boundary between `EffectGraph` and future ProtocolModes.
2. Whether open numeric opcode identity should additionally bind a future
   versioned opcode dictionary root before F4.
3. Whether proof receipt derivation should remain public or become crate-local
   until F3 diagnostics consumes it.

## Stop

F2R stops here for architecture review. No commit, push, F3 dual run, grouping
switch, selector, ProtocolMode, runtime, verifier, generation, admission, or
authority change is authorized by this candidate.
