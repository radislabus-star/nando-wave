# S1C-3E Journal Ownership Terminal Report

Status: `TERMINAL ROLLBACK PASS / PRODUCTION RESTORED / S1C-4 CLOSED`

Attempt: `20260812T153838Z-25c0f1168fa4-s1c3e-v1`

## Result

```text
ownership repair                    PASS
runtime writer open                 PASS
three segment files                 PASS
segment owner/mode                  e:e 0600
decoded records                     0
verifier empty-byte predicate       FAIL
transaction verdict                 S1C3E_ROLLBACK_PASS
production restored                 true
S1C-4                               CLOSED
scientific authority                false
```

Roots:

```text
deployment receipt  baedc988ac8664cde09946d2f8b780ce9221086dd8d6d68bb02280582214ecb0
final verification  637d90ffece05f628d6ed27041dbe22706a3df4ef04240185096980b484fdcf3
terminal state       442aefee66e7c04c561143b00d0a3fd6bcb01f65f5149571bd0f3d35f6b2a77c
forward journal      6ab9cd4823f1d737ec731e9c96049c0492d1966d91b3408dd524ea23b1c8666c
```

## Exact Failure

Every `FramedCborLedger` segment begins with the required four-byte magic
`NTF1`. The three files were therefore each four bytes long with SHA-256
`4fc61a14f994e28249509ec2504e89df30497a2aa76b1d9c5f6c38e2acee6072`.
No frame header or CBOR payload followed the magic, so the exact decoded record
count was zero.

S1C-3E incorrectly required file size zero and emitted:

```text
s1c3e_journal_segment_nonempty:decision-precommit-00000000000000000000.cbor
```

The fail-closed rollback restored baseline binary/config and left the three
magic-only files in place because deletion was forbidden whenever the
transaction could not prove that bytes were operational rather than natural.
This conservative preservation is correct.

## Preserved Production

```text
baseline binary SHA-256  6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58
baseline config SHA-256  cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5
Nginx PID                682430 unchanged
connector PID            2919 unchanged
route receipt failures   0
false accepts             0
runtime parity failures   0
```

S1C-3E is immutable and will not be rerun or relabelled. A record-aware repair
requires a new S1C-3F paper, implementation root, and transaction identity.
