# Effect Law Unification: STOP-F2R2 Candidate

Date: 2026-07-21 Europe/Tallinn

Status: **SUPERSEDED BY STOP-F2R3 / F3 NOT STARTED**

F2R2 remains as historical evidence. Its checksum-only observation and partial
effect quotient are not canonical authority. The replacement candidate is
recorded in `STOP_F2R3_SEALED_EFFECT_LAW_CANDIDATE.md`.

This is an uncommitted repair candidate. It does not create a verified law,
switch grouping, or change runtime authority.

## Git And Authority

```text
HEAD                         32ce298799b331db32a311654c070ad5c393a00e
origin/main                  23c04b728999716c53c988b0e67f03df034cefe5
F2R2 commit created          NO
push                         NO
production deployed          NO
services restarted           NO
F3 started                   NO
live controller verdict      BLOCK
live active packages         0
execution_authority          false
```

The local HEAD is the rejected historical F2 commit. F2R2 exists only in the
working tree.

Source ownership:

```text
effect_law.rs        1312 lines  0678a3c3ec54affd182509505b8e7f71c1023aee0d16d32e8b0746004d114374
effect_law_tests.rs   516 lines  cde9b71595a241c45f359cb785875b8838519358b343021edb69c7c6863bf069
```

## Repaired Boundary

```text
one accepted structured transition
-> EvidenceBoundEffectObservationV2
   |- exact physical EffectGraph
   |- exact operation classes
   |- exact physical value types
   |- all integer/string/boolean constants
   |- argument ownership commitment
   `- parity/verifier evidence references

multiple independent observations
-> bounded exact quotient search
   |- exact invariant bytes -> unverified CanonicalEffectLawV2 candidate
   `- physical difference -> ProtocolModeDifferenceV2 + BLOCK
```

A single observation cannot create a law candidate. Distinct structured
`wait` and `write_stdin` fixtures are not preemptively merged. The current
search deliberately performs only an exact invariant quotient; it does not
guess which operation, type, budget, empty string, or boolean is adapter
surface.

There is no `VerifiedCanonicalEffectLawV2`, verified receipt constructor, or
verified status in the F2R2 public API. A future sealed independent parity
boundary must be designed before any verified law can exist.

## Identity And Provenance

Canonical bytes include:

```text
IR version
opcode_dictionary_root
value_type_dictionary_root
exact canonical topology
bounded relation program
preserved-frame contract
```

`from_unverified_program` returns `CanonicalizedEffectLawV2`, including the
physical-node to canonical-node mapping. Role nodes are remapped through this
mapping before program canonicalization. Unknown numeric opcodes remain
representable only as unverified data.

Constants are not classified as semantic or protocol surface in F2R2. Empty
strings, non-empty strings, `false`, `true`, and integer budgets are retained
with exact typed commitments and `argument_key_sha256` ownership. That later
classification belongs to a protocol contract supported by multi-observation
evidence.

## Focused Proof

```text
cargo +1.97.0 test -p nando-response-actor --lib effect_law::tests
15 PASS / 0 FAIL
```

Covered invariants:

```text
single observation cannot create candidate                 PASS
independent identical observations can create exact candidate PASS
same lineage is insufficient                               PASS
tampered observation is rejected before quotient search    PASS
wait/write structured fixtures are not preemptively merged PASS
Call != Project                                            PASS
Integer != String                                          PASS
Identifier != Integer                                      PASS
false constant retained                                    PASS
empty string retained                                      PASS
integer budget retained                                    PASS
constant owner changes observation                         PASS
dictionary roots change law ID                             PASS
unknown opcode remains unverified data                     PASS
physical-to-canonical mapping returned                     PASS
restart bytes byte-identical                               PASS
repeated typed roles and multi-output accepted             PASS
incomplete topology blocks                                 PASS
```

The transition inputs are distinct structured fixtures, not real production
receipts and not an independent parity proof.

## Compatibility Baseline

With an isolated `TMPDIR` because the host `/tmp` tmpfs was nearly full:

```text
semantic_ tests  22 PASS / 3 known FAIL
```

The unchanged failures are:

```text
semantic_program_pool_survives_field_renames_and_collects_future
semantic_count_inside_teacher_prose_reaches_external_admission
multi_output_semantic_program_reaches_external_admission
```

`cargo check -p nando-response-actor --lib` passes. Full package Clippy remains
blocked by 12 pre-existing diagnostics in other owners. The F2R2 files produce
zero Clippy diagnostics.

## NANDA Gates

The mixed-owner packet is preserved as VETO because one large source file was
used as evidence for several owners. After splitting by owner:

```text
observation-only evidence ownership PASS
constant ownership                   PASS
exact multi-observation quotient     PASS
dictionary binding                   PASS
canonical node provenance            PASS
no verified authority                PASS
```

Packets, JSON results, and traces are under:

```text
target/f2r2-nanda*.md
target/f2r2-nanda*-result.json
target/f2r2-tmp/nanda-structural-gate/
```

## Unresolved Boundary

F2R2 does not yet provide the sealed independent parity receipt required to
create a verified law. It also does not infer a cross-surface quotient between
`wait` and `write_stdin`; that requires multiple independent observations and
an explicit bounded quotient hypothesis with anti-merge evidence.

Repeated same-typed action slots currently fail closed if the existing
`EffectGraph` cannot uniquely map a relation slot back to a physical graph
node. The mapping is never guessed by ordinal, prefix, latest value, or raw
surface name.

## Stop

F2R2 stops here for architecture review. No commit, push, F3 dual run, B1,
ProtocolMode compiler, runtime, verifier, generation, admission, deployment,
service restart, or authority change is authorized by this candidate.
