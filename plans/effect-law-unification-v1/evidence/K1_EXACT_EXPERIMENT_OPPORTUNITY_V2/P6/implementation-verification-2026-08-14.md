# P6 Compatibility And Rollback Verification

Status: `PASS`

## Scope

P6 installed Freeze V8 as a reader-compatible extension of the exact connected
motif domain already used by Freeze V6 and V7. It did not change discovery,
ranking, identifiers, evidence, scientific authority, or production state.

```text
Freeze V1-V5                    legacy reader domains unchanged
Freeze V6-V7                    exact connected motif domain unchanged
Freeze V8                       same topology, binding, and basis domain
production writer               OFF / untouched
Law #2                          NOT PROVED
K1                              1 / 3
```

## Real Compatibility Denominator

The preregistered frozen production copy was read from:

```text
/tmp/k1-terminal-failure-quotient-v1-baseline-B2zz2fwF/
  k1-epistemic-scheduler-ledger-v1.json
```

```text
bytes                           2,246,130
SHA-256                         00e44ee2c9127c71231bb2b413500fbe1a4693e1c834c5cf061f60c8df8cd362
ledger revision                1,174
candidate freezes              586
terminal verdicts              585
Freeze V1/V2/V3/V4/V5/V6       40 / 37 / 8 / 37 / 32 / 432
active legacy generation       V6 generation 586
active freeze root             685ead18cde7fa40330a743e474758b5ee0436115730418903d7c6fde94afadd
legacy unbound terminals       585
exact deterministic attempts  0
```

The current reader decoded, validated, and re-encoded all bytes identically.
An isolated completion appended one legacy terminal while preserving the exact
1,174-event prefix; it did not rewrite or backfill any legacy event as V8.

The copied anchor was intentionally not asserted as current: its revision is
321 while the copied cache and signed journal reach revision 1,174. Claiming
anchored parity across those two stale capture instants would be false.

## Runtime And Failure Gates

Remote mini-PC checks used `~/.cargo/bin/cargo 1.97.1`, `CARGO_BUILD_JOBS=20`,
and no production socket or mutable production path.

```text
k1_natural_scheduler focused suite       60 PASS / 0 FAIL
archive fault suite                       2 PASS / 0 FAIL
cargo fmt --all -- --check               PASS
git diff --check                         PASS
```

Covered failures include orphan object, orphan manifest, tampered object,
noncanonical manifest, stale journal temp, signed tail with lagging anchor,
diagnostic without verdict, durable verdict with lagging anchor, cache restart
parity, and duplicate terminal retry.

## Rollback Fence

Every exact authority wake now validates the durable ledger against the
installed minimum Freeze reader schema, including writer-OFF wakes. Before a V8
suffix a legacy reader remains valid. After the first V8 candidate freeze only
a policy declaring the V8 reader is accepted; V7 or earlier is rejected with
`k1_post_v8_rollback_reader_forbidden` or the stronger policy downgrade veto.

## Claim Boundary

P6 proves compatibility and recovery behavior only. Fixtures and the frozen
production copy are not discovery evidence, independent future, package
execution, cleanup evidence, or a LawCertificate. Production services, the
dashboard, phase memory, and K1 membership were not changed.

The next authorized stage is P7: bounded replay on the frozen copy and an exact
10x denominator, producing one machine-readable value/resource receipt before
any deployment.
