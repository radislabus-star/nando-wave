# Effect Law Unification: STOP-F0

Date: 2026-07-20 Europe/Tallinn

Status: **F0 COMPLETE / STOP / F1 NOT STARTED**

This receipt closes evidence reconciliation and structural provenance only. It
does not define `CanonicalEffectLawV2`, choose a selector or binding rule,
change runtime behavior, rebuild a generation, deploy code, or grant authority.

## Source State

```text
HEAD                         23c04b728999716c53c988b0e67f03df034cefe5
branch                       main
commit created               NO
production changed           NO
services restarted           NO
execution authority          false
```

Canonical STOP-A artifact:

```text
/home/ubu/tmp/nando-r1/continuation-evidence-stop-a.json
sha256 e9e43513bca355a0ec77588d995c1a77c11188d59d8b1b5fc7dea8b9b1f9e9d0
rows   129
missing parity IDs 728
```

F0 provenance replay:

```text
/home/ubu/tmp/nando-r1/continuation-evidence-stop-f0.json
sha256 ec254a037ebf0e7bfb84af1d73ede142e28bf5d56ef74dbd54a212e26b63f08c
rows   129
missing parity IDs 714
```

The F0 artifact serializes hashes and structural coordinates only. It does not
serialize raw requests, provider text, payload fragments, expected responses,
surface prefixes, or continuation values.

## Count Reconciliation

The canonical STOP-A machine artifact has 728 unique missing parity frame IDs
and no duplicates. The 725 count in the earlier human report came from a live
bounded-pool snapshot taken before the final STOP-A replay. The later F0 replay
has 714 because the live pool moved again.

```text
725  earlier live snapshot; stale transcription
728  canonical frozen STOP-A artifact
714  later live F0 replay
129  fixed scored denominator in both artifacts
```

Missing parity rows are `CENSORED_UNKNOWN`. They are excluded from the scored
denominator and must never become positives, applicability negatives, or
anti-centers without an independent parity receipt.

## Fixed Denominator

```text
function:wait         96
function:write_stdin  33
total                129
unclassified rows      0
```

Across the 33 protocol-scoped `write_stdin` receipts, actor
`7f7f27d490bb09fb135b1a9b6de1c654b891113edb756560bfd6a55cd5334535`
still gives:

```text
EXACT          24
WRONG           3
ABSTAIN         6
VERIFY_FAILED   0
```

The 33 rows split into 32 rows from the actor's declared member signature and
one execution-budget-equivalent row from a second physical signature. The
actor's complete 129-cell matrix is:

```text
EXACT          24
WRONG           3
ABSTAIN       102
VERIFY_FAILED   0
```

Thus `24/3/6/0` is a protocol-class projection, not single-signature ownership.

Every actor by receipt cell retains an explicit outcome and reason in
`actors[].outcomes[]`. F0 adds structural value provenance without changing any
matrix decision.

## Structural Classification

The independently expected continuation identity is observable exactly once in
the immediate `tool_output` text tokens for all nine exceptional rows. Every
occurrence is at token ordinal 12, but F0 does not promote that observed ordinal
into a binding rule.

The three WRONG rows each contain one physical selector candidate. The selected
identity and independently expected identity both occur exactly once, at
different structural path hashes.

| frame | selected path SHA | expected path SHA | classification |
|---|---|---|---|
| `2145879c6032005682066e0b3d9143c4567ad1085b0cc7d6765e7a00b1ea857e` | `bfd938d28274157d0dfc3dc47c6895268e6b217c74dbf28dfb7e413013ef3006` | `3427852ad19aca172c10629c4d643272d4398a8e328f70a3dfd2119212523480` | one candidate selects a different observable identity |
| `387e9962b6c11e6050d7c66058e42bb34351712eae298c2f12d81ade50abdb5f` | `88e54f193a967a3856b9dcea594a7fd7a4ba111c7fe480e24aa00cebe9e91a0f` | `cacc48d398362251160334b5edff347b1e959750fa38ad3b35fcb5720fbc03ea` | one candidate selects a different observable identity |
| `861276b216278b1cbefbf532742b05968810102fe5a722188bf7c5568ae0d246` | `46443ecae0cadd15d34eed10b50e77101bbab4ec65fc04e7e788f1af2bcb6932` | `d2193be5910697d319bb82cc5493c40d1ff970c9222f0b32c2fb748154e7d106` | one candidate selects a different observable identity |

The six ABSTAIN rows have zero physical selector candidates, but each contains
one independently expected identity at a distinct structural path.

| frame | expected path SHA | classification |
|---|---|---|
| `03412e883591d47fdf87e91d8e3be433eaa3ae49ace1ea8144fe3726f91464b2` | `88e54f193a967a3856b9dcea594a7fd7a4ba111c7fe480e24aa00cebe9e91a0f` | expected identity observable; physical selector misses it |
| `0edfa99564d56fa3a7c934c7824fbe4f17205918c1c4df84122907f4c339de79` | `a3d6eac74e5c7c13e56497aac23166a1dd04fae606083bee6ef41421538f523e` | expected identity observable; physical selector misses it |
| `68e587c6a8f1c4126911652b857b69d64ffc80f252b0d01b87aa7e8ad354a141` | `fff412007549def40ec3702cc9e6613bdac66e57c4dde6021ab35c00a3be738f` | expected identity observable; physical selector misses it |
| `871ddab018e787d8e4ce990d50d55b3266c1e9b482d3fef1891e108a85a0d740` | `cd3e819e923aa1d88ccf1d744c4dee4e9b128e1afa7186ed34b2e0770f2703ae` | expected identity observable; physical selector misses it |
| `c7822669da77831d881a90138f62e49e708e7ae3aca6ca72afba65f1464f3553` | `e1ce9193c147475ed0cbbce6eade015268d3d9a70d8ef6463756e68e951c98c1` | expected identity observable; physical selector misses it |
| `cde622cba12074de95a8ffeb1abfd97d8978a71601b0273f746da03202ab92d2` | `5e885508729e0fe8e22b1ae9b8c9c5d5834f5675ac1db225b6719f6117e3c3de` | expected identity observable; physical selector misses it |

## F0 Conclusion

F0 proves:

```text
correct role value observable             YES, all 9 exceptional rows
current physical selection causally valid NO
all exceptions structurally classified    YES
binding rule identifiable without labels  NO
```

The expected identity was obtained from the independent parity/teacher outcome.
Using that identity, its token ordinal, or its path hash to design the runtime
selector would leak the label. The available corpus does not yet identify a
causal structural relation that distinguishes the correct occurrence before
the action is known.

Final F0 verdict:

```text
INSUFFICIENT_BINDING_EVIDENCE
```

This is a complete evidence result, not permission to guess a selector. The
diagnostic two-prefix candidate counter remains read-only and must not become
law identity or runtime authority.

## Unresolved Cases

1. A causal, label-free source-role relation remains unidentified.
2. One current `wait` receipt remains a separate physical/canonical ABSTAIN and
   must stay visible in the next architecture review.
3. The neighboring custom-tool polling adapter under the same legacy effect-law
   hash remains evidence of possible double semantic authority; F0 does not
   resolve grouping.
4. The live missing-parity inventory will continue to drift. Only an artifact
   SHA gives a count provenance.

## Authority Boundary

```text
diagnostic export -> evidence report only
diagnostic export -X-> grouping authority
diagnostic export -X-> selector authority
diagnostic export -X-> runtime execution
diagnostic export -X-> admission
```

Production authority remains false. No service, checkpoint, generation,
package, threshold, or live route was changed.

## Verification

```text
cargo +1.97.0 check diagnostic binary                 PASS
focused online_state semantic tests                    3/3 PASS
129 unique rows                                        PASS
six actor matrices, 129 reasoned cells each            PASS
protocol-scoped write_stdin projection                 24/3/6/0 PASS
nine exceptional provenance invariants                 PASS
STOP-A missing IDs                                     728/728 unique
privacy scan for raw request/payload/response fields   PASS
git diff --check                                       PASS
NANDA count reconciliation                             PASS
NANDA exceptional provenance                           PASS
NANDA authority/lifecycle                              PASS
Graphify update                                        PASS
live execution_authority                               false
top-level miner false_accepts                          0
production service restart by F0                       NO
```

NANDA traces:

```text
/tmp/nanda-structural-gate/f0-count-reconciliation.trace.json
/tmp/nanda-structural-gate/f0-exceptional-provenance.trace.json
/tmp/nanda-structural-gate/f0-authority-lifecycle.trace.json
```

Graphify rebuilt 23,278 nodes, 52,390 edges, and 1,003 communities. The F0
artifact was not regenerated during finalization, so its resource metrics are
not invented; the last measured STOP-A targeted replay remains 5.44 s and
442,248 KiB max RSS.

Residual test debt, outside the F0 provenance path: the broad `semantic_` test
filter produced 22 PASS and three pre-existing `online_collection` failures:

```text
semantic_program_pool_survives_field_renames_and_collects_future
semantic_count_inside_teacher_prose_reaches_external_admission
multi_output_semantic_program_reaches_external_admission
```

These failures do not change the F0 matrix or authority state, but they remain
unresolved and must not be reported as a full-suite PASS.

## Stop

F0 is complete. Work stops here. F1 and all EffectLaw, ProtocolMode, selector,
runtime, verifier, generation, and admission changes require the next explicit
authorization.
