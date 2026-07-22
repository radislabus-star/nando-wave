# STOP-F7-D Atomic Generation Store

Date: `2026-07-22`

Verdict: `COMPLETE_CONTROLLED_PROOF_PASS`

Authority: `false`

## Result

```text
generation restart bundle
+ generation evidence ledger
+ exact F6 receipts
+ exact F7 envelopes
        |
        v
self-validating checkpoint                  PASS
        +-- receipt set equals ledger       PASS
        +-- canonical restart               BYTE IDENTICAL
        +-- raw payloads                    0
        `-- authority                       false
        |
        v
inactive slot .new, O_EXCL, 0600
        |
        v
write -> fsync(file) -> rename -> fsync(dir) SYSCALL VERIFIED
        |
        v
two durable alternating slots               PASS
        +-- stale .new                       QUARANTINE
        +-- corrupt newest                   PREVIOUS RESTORED
        +-- both corrupt                     EMPTY SHADOW
        +-- evidence rollback                BLOCK
        +-- sequence jump                    BLOCK
        +-- wrong parent                     BLOCK
        +-- symlink slot                     QUARANTINE
        `-- broken temporary symlink         QUARANTINE
        |
        v
live capture-owner join                      F7-E / NOT STARTED
external admission                           F8 / BLOCKED
```

The store is a separate cold IO owner. Kernel, runtime, proof and learning do
not depend on it. `nando-response-actor` also has no F7-D dependency or caller;
its future F7-E role is orchestration only.

The checkpoint cannot be assembled from roots alone. It contains canonical
F6 and F7 receipt bytes and reopens each through its owning validator. The
ledger is accepted only when every row has one exact receipt pair and there
are no missing or extra pairs.

## Durability

The previous slot is never renamed away. A new snapshot replaces only the
inactive slot after its temporary file is fully synced. This avoids a window
where neither old nor new checkpoint exists.

Remote `strace` observed for both slots:

```text
openat(... .generation-slot-{a,b}.nwgc.new, O_EXCL, 0600)
fsync(file)
rename(.new, generation-slot-{a,b}.nwgc)
fsync(directory)
```

## Budgets

```text
checkpoint bytes       <= 16 MiB
receipt pairs          <= 4096
generation bundle      <= 512 KiB
evidence ledger        <= 2 MiB
source file maximum       177 lines
test file maximum         226 lines
raw payload bytes      = 0
production callers     = 0
authority              = false
```

## Verification

```text
F7-D atomic/recovery tests             8 / 8 PASS
F7-D syscall-order test                1 / 1 PASS
kernel/learning/proof/persistence    248 PASS / 1 ignored perf gate
runtime all-target check               PASS
four-crate Clippy -D warnings          PASS
changed-file rustfmt                   PASS
git diff --check                       PASS
services restarted                     NO
deployment changed                     NO
```

Next boundary: F7-E must join lineage/event roots to the actual live capture
owner, load this store after fallback is available, and atomically publish only
a shadow generation. It still may not call admission or enable local accept.
