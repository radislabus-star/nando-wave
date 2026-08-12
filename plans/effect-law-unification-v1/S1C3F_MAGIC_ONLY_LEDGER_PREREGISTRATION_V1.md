# S1C-3F Magic-Only Ledger Preregistration V1

Status: `PAPER FROZEN / CRITIQUE PASS / STRUCTURAL 4 OF 4 PASS / AUTHORITY FALSE`

Date: `2026-08-12 Europe/Tallinn`

Immutable parent:

```text
S1C-3E attempt root       442aefee66e7c04c561143b00d0a3fd6bcb01f65f5149571bd0f3d35f6b2a77c
S1C-3E receipt root       baedc988ac8664cde09946d2f8b780ce9221086dd8d6d68bb02280582214ecb0
S1C-3E final root         637d90ffece05f628d6ed27041dbe22706a3df4ef04240185096980b484fdcf3
S1C-3E journal root       6ab9cd4823f1d737ec731e9c96049c0492d1966d91b3408dd524ea23b1c8666c
```

## 1. Exact Question

Can the exact inherited S1C-3D candidate remain installed when the independent
verifier evaluates framed-ledger records rather than treating the mandatory
segment format header as a scientific row?

S1C-3F does not reinterpret S1C-3E. It inherits the positive writer-open
observation and repairs one false predicate under a new identity.

## 2. Frozen Empty-Ledger Definition

For each of the three frozen prefixes, an empty active segment is exactly:

```text
bytes                 4e 54 46 31 (`NTF1`)
size                  4
file SHA-256           4fc61a14f994e28249509ec2504e89df30497a2aa76b1d9c5f6c38e2acee6072
frame headers after 4  0
decoded CBOR records   0
recovered tail bytes   0
```

Required files remain exactly:

```text
decision-precommit-00000000000000000000.cbor
selected-action-binding-00000000000000000000.cbor
goal-satisfaction-00000000000000000000.cbor
```

Directory is `e:e 0700`; files are regular `e:e 0600`. Any extra file,
symlink, wrong magic, partial frame, trailing byte, malformed length, digest
mismatch, CBOR payload, or decoded record before cursor is a hard veto.

## 3. Record-Aware Independent Parser

The S1C-3F verifier implements a bounded parser independent of the Rust writer:

```text
read 4-byte magic
-> require NTF1
-> while bytes remain:
     read u32 little-endian payload length
     read u64 payload digest
     read exact payload
     reject truncation or payload above 16 MiB
     count one record
-> require record_count == 0 at opening cursor
```

It does not decode or generate a domain object for an empty segment. It never
writes to the journal.

## 4. Inherited Candidate And Production State

Before mutation the verifier binds:

- S1C-3D resource/parity roots and exact candidate binary/config;
- S1C-3E terminal, receipt, final-verification, and journal roots;
- current baseline binary/config restored by S1C-3E;
- exact three magic-only journal files and zero decoded records;
- false accepts and runtime parity failures equal zero;
- all services active, connector receipt failures zero.

S1C-3F does not rerun durability or parity measurements. The frozen 5 ms target
remains `OPTIMIZATION_WATCH`; the 20 ms safety boundary remains inherited PASS.

## 5. Allowed Transaction

After durable rollback arming:

```text
verify existing magic-only journal
-> install exact inherited candidate binary/config
-> intentional transition-serving restart
-> verify process env and clean boot-scoped log
-> parse journal again
-> freeze append cursor at record_count 0
-> survival and connector checks
```

The transaction may not create, truncate, rename, chmod, chown, or append any
journal file. It may not restart Nginx/connector, mutate packages/K1/phase
memory, generate traffic, train a model, or grant scientific authority.

## 6. Natural Concurrency

The opening cursor is frozen immediately after writer readiness is proven. If
a natural record arrives later during survival, it is allowed and preserved;
the verifier requires prefix preservation and valid framed parsing, not an
unchanged file size. Such a record belongs to post-cursor S1C-4.

A record present before the opening cursor is a hard veto because S1C-3F did
not preregister a retroactive denominator.

## 7. Rollback

Any failure restores the exact baseline binary/config. The existing journal is
never deleted by S1C-3F. Every prefix present before mutation must remain a
byte-identical prefix after rollback; natural suffix frames are preserved.

## 8. Result Boundary

```text
installation PASS
-> capture installed
-> S1C-4 COLLECTING from record_count-0 cursor
-> scientific_authority false

any correctness/safety failure
-> exact rollback
-> S1C-4 CLOSED
```

Installation does not prove a decision episode, grounded meaning, K2, model
training authority, or phase mutation.

## 9. Attempt Discipline

Paper and critique are committed/pushed before implementation. Implementation
and verifier are committed/pushed before one production attempt. S1C-3F is
never rerun or relabelled; any new defect requires another identity.
