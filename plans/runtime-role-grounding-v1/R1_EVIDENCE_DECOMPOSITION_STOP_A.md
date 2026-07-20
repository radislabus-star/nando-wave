# R1 Evidence Decomposition: STOP-A

Date: 2026-07-20 Europe/Tallinn

Status: **STOP-A / BLOCK R2**

This receipt closes evidence decomposition only. It does not define or
implement `EffectLawIR`, change the law model, alter thresholds, rebuild a
generation, or grant authority.

## Source State

```text
HEAD                         23c04b728999716c53c988b0e67f03df034cefe5
branch                       main
commit created               NO
production changed           NO
services restarted           NO
execution_authority          false
self-training false accepts  0
runtime false accepts        0
```

The worktree already contained uncommitted semantic fail-closed changes in
`online_state.rs` and the diagnose binary. R1 adds only a read-only,
privacy-safe evidence export and selector-count diagnostics. Raw request text,
provider payloads, expected response text, and handle values are not serialized.

Full evidence artifact:

```text
/home/ubu/tmp/nando-r1/continuation-evidence-stop-a.json
sha256 e9e43513bca355a0ec77588d995c1a77c11188d59d8b1b5fc7dea8b9b1f9e9d0
size   674799 bytes
```

Diagnostic binary:

```text
/home/ubu/tmp/nando-r1/nando-online-response-diagnose-stop-a
sha256 701f5ebd8295f66cce1518286738456807a3140e0bd1b1273e58c74fb3c547de
```

## Denominator

```text
function:wait         96
function:write_stdin  33
total                129
unexplained receipts   0
```

Every actor matrix satisfies:

```text
EXACT + WRONG + ABSTAIN + VERIFY_FAILED = 129
```

The target physical `write_stdin` actor
`7f7f27d490bb09fb135b1a9b6de1c654b891113edb756560bfd6a55cd5334535`
across the 33 protocol-scoped `write_stdin` receipts gives:

```text
EXACT          24
WRONG           3
ABSTAIN         6
VERIFY_FAILED   0
```

F0 ownership reconciliation: these 33 rows contain 32 rows from the actor's
declared member signature plus one execution-budget-equivalent row from a
second physical signature. The full 129-cell actor matrix is
`24 EXACT / 3 WRONG / 102 ABSTAIN / 0 VERIFY_FAILED`. The `24/3/6/0` result is
therefore protocol-scoped, not a claim that one signature owns all 33 rows.

The complete six-actor by 129-receipt matrix is in `actors[].outcomes[]` in the
full evidence artifact. Each receipt row includes frame/evidence hashes,
physical member signature, effect-law hash, protocol, selector kind and digest,
source layout digest, observed/emitted types, argument schema digest, constants
digest, and capability class.

## Observed Structural Modes

All 33 `write_stdin` receipts have the same coarse structural layout,
observation type, emitted type set, and argument schema:

```text
layout       e9e5c1c9e4f6277339d1bcde0733a59bd42f8731f449da6dc13010a916930d48
selector     ContentLinePrefix("Process running with session ID ")
selector sha 203a1c5bc694351b2c996f6a8671668c371fc195446dc5ea4690dd03ed159455
observed     identifier
emitted      string + integer
arg schema   a809f39b0d0f7cb37f159809915854dac31cfde3a0d311f2cf4a3852536ec577
capability   function:write_stdin
```

Within that coarse layout, replay reveals three distinct runtime modes:

```text
PrefixPresentUniqueAligned      24 -> EXACT
PrefixPresentUniqueConflicting   3 -> WRONG
PrefixAbsent                     6 -> ABSTAIN
```

Execution-budget constants split into several digests across successful rows.
The digest shared by all three WRONG rows also occurs in five EXACT and four
ABSTAIN rows. Constants therefore do not explain the outcome boundary.

## Three WRONG Receipts

For every WRONG row, the physical selector finds exactly one candidate and
emits an action with the expected JSON shape. The selected argument digest does
not equal the independently expected argument digest. The canonical
`ContinuationHandle` actor repeats the same three mismatches, so broadening the
selector does not repair the binding.

| frame | evidence | candidates | actual arguments | expected arguments | explanation |
|---|---|---:|---|---|---|
| `2145879c6032005682066e0b3d9143c4567ad1085b0cc7d6765e7a00b1ea857e` | `a8c34ab71338328823df1abd42ad8bcf5dd2d47e1d1ef55dd9be882964247169` | 1 | `5b83c793bd12936ea383772e0c63e1c21a50037711681493bc44d099d1337bc2` | `8f26194eb05b6f1a58a11787a1096f68f4a3bce05d9f5f416208686c777973e0` | unique physical candidate binds the wrong continuation identity |
| `387e9962b6c11e6050d7c66058e42bb34351712eae298c2f12d81ade50abdb5f` | `7f4129145d41e387d3360fcdcf51b42c9e0ff13f0f091b4879114f143c5e2db3` | 1 | `5777f3db7320128535c51af69104bde6c7593825a29ca9c155ee4f6fa44dbad6` | `399dbbc36816a4ea9dfb535a02b2d2d039457bb014f63e5d38f8367559c7f7cf` | unique physical candidate binds the wrong continuation identity |
| `861276b216278b1cbefbf532742b05968810102fe5a722188bf7c5568ae0d246` | `24acd506083a2628390fec4c50937193813a1bc4d9052d55c30b55896ade964d` | 1 | `a3f60b0b2e0ccb52d7da5fcd9a9015d8917cf565c5c183e837e73519837995ca` | `a28a87db4c07571e3630d24c86fdc7defa11a3a0c9bbf548581ee6346ddb66c0` | unique physical candidate binds the wrong continuation identity |

This proves a binding error, not a renderer, action-name, JSON-shape, constants,
or Wave-threshold error. R1 does not yet prove which path/turn ordinal should
win; that belongs to the architecture decision after STOP-A.

## Six ABSTAIN Receipts

All six fail before action construction. Candidate count is zero for both the
physical process-session prefix and canonical `ContinuationHandle` selector.

| frame | evidence | constants digest | candidates | explanation |
|---|---|---|---:|---|
| `03412e883591d47fdf87e91d8e3be433eaa3ae49ace1ea8144fe3726f91464b2` | `b89eeeda96ecdf6adcd2862fb05a44b22f79c59658a2377353e1ae91d9793589` | `62a4e6f2b03fac063e5c5193cdb7e44e361f684dde9ebbbede2b4ed5f657d534` | 0 | no recognized continuation handle on this runtime surface |
| `0edfa99564d56fa3a7c934c7824fbe4f17205918c1c4df84122907f4c339de79` | `5705c62c719126ce5b870f2c627970b386275e18d415e72b54dcdd92f6bab839` | `1f05bde6f4402bc5a752d4580dd5042fe2f344a04076e0b0606f4c6e0377032e` | 0 | no recognized continuation handle on this runtime surface |
| `68e587c6a8f1c4126911652b857b69d64ffc80f252b0d01b87aa7e8ad354a141` | `86c40e4cbee352b56964b623eef5c83f961d35de539ee6ac16452ac8e540805c` | `62a4e6f2b03fac063e5c5193cdb7e44e361f684dde9ebbbede2b4ed5f657d534` | 0 | no recognized continuation handle on this runtime surface |
| `871ddab018e787d8e4ce990d50d55b3266c1e9b482d3fef1891e108a85a0d740` | `87babd7e957195aaae7c7d5ed298a87b7b05b1aa54f49ffe0519cd6c74fc070b` | `be6bc94a77963b54501343eff1fb108a2a9ff8e57d57acb0ee885101a8a7782f` | 0 | no recognized continuation handle on this runtime surface |
| `c7822669da77831d881a90138f62e49e708e7ae3aca6ca72afba65f1464f3553` | `7a25adfc7fdf6a5a929f8ffd3e72500d9230398e10e176f82a6d0a1012eb250c` | `62a4e6f2b03fac063e5c5193cdb7e44e361f684dde9ebbbede2b4ed5f657d534` | 0 | no recognized continuation handle on this runtime surface |
| `cde622cba12074de95a8ffeb1abfd97d8978a71601b0273f746da03202ab92d2` | `34c90e64488c4158d89fe7e16f9d0fe4ee05d23b3b62c65a1017bfd0580d5304` | `62a4e6f2b03fac063e5c5193cdb7e44e361f684dde9ebbbede2b4ed5f657d534` | 0 | no recognized continuation handle on this runtime surface |

## Unresolved Cases

1. One of the 96 current wait receipts now has zero physical and canonical
   selector candidates. Current wait replay is therefore `95 EXACT / 1
   ABSTAIN`, whereas the earlier copied-checkpoint observation was `96 / 0`.
   It is outside the original `write_stdin` nine but must remain visible.
2. The canonical machine artifact contains 728 unique historical frame IDs
   without a runtime parity receipt. The earlier 725 count was transcribed from
   a stale live bounded-pool snapshot before the final replay. These frames are
   classified as censored evidence and excluded from the 129-receipt
   denominator.
3. The three WRONG rows share the same coarse layout as exact rows. R1 proves
   that a unique surface candidate can still bind the wrong identity, but does
   not choose the future path/turn-ordinal rule.
4. A broad law-wide inspection also reached a neighboring custom-tool polling
   adapter under the same effect-law hash. The scoped 129 matrix correctly uses
   generation-owned physical signatures and excludes it. Whether this is
   double semantic authority is intentionally left for the post-STOP-A review.

## Verification

```text
cargo check diagnostic binary                         PASS
focused semantic tests                                3/3 PASS
git diff --check                                      PASS
privacy scan for raw request/payload/response fields  PASS
NANDA write_stdin replay route                       PASS
NANDA denominator route                              PASS
NANDA authority/lifecycle route                      PASS
targeted replay                                       5.44 s
targeted replay max RSS                               442248 KiB
authority after replay                                false
production services                                   untouched
```

The first combined NANDA worksheet returned `VETO` because it collapsed the
wait denominator, write replay, authority, and lifecycle into one candidate
route. After splitting those ownership routes, all three local structural gates
returned `PASS`. The initial `VETO` trace remains at
`/tmp/nanda-structural-gate/r1-stop-a.trace.json`.

## Stop

R1 is complete as evidence decomposition. R2 is blocked pending review of:

```text
physical signature
effect identity
runtime binding
single-source Canonical EffectLawIR contract
```

No R2 implementation is authorized by this receipt.
