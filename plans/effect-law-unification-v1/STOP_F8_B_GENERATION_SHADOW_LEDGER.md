# STOP-F8-B Generation Shadow Receipt Ledger

Status: `PASS / AUTHORITY_FALSE`

Date: `2026-07-22 Europe/Tallinn`

## Result

```text
durable ProviderRequestCaptureReceiptV3
+ pinned F7 generation checkpoint
+ F5 TrafficShadowReceiptV3
+ hash-only F6 IndependentVerifierReceiptV3
-> generation-owned GenerationShadowReceiptV3
-> atomic two-slot ledger
-> byte-identical restart
```

The provider capture sequence domain remains separate from the frozen F7
generation capture domain. A live receipt enters this ledger only after an
exact join to the durable provider capture index. Missing capture durability is
censored and cannot update semantic memory.

## Ownership

```text
nando-operator-learning/generation_shadow_v3
  receipt schema, terminal outcomes, hash chain, bounded ledger

nando-operator-persistence/generation_shadow_store_v3
  private two-slot publication, restart, rollback and corruption policy

nando-transition-serving/generation_shadow
  off-path F5/F6 evaluation and exact durable-capture join
```

Serving cannot relabel an unverified result as `VERIFIED_PASS`. The ledger
requires the complete canonical F6 receipt and independently validates its
request, action, output, verdict and receipt roots.

## STOP Matrix

```text
generation/request/capture join        EXACT      PASS
F5 traffic receipt binding             EXACT      PASS
F6 actor/output/verifier binding        EXACT      PASS
receipt append after restart           MONOTONIC  PASS
duplicate receipt                      BLOCK      PASS
foreign generation                     BLOCK      PASS
unverified PASS relabel                 BLOCK      PASS
censored semantic updates              0          PASS
raw payload bytes persisted            0          PASS
local accepts                           0          PASS
execution authority                    false      PASS
```

## Budgets

```text
ledger records                         <= 4,096
canonical ledger bytes                 <= 16 MiB
capture durability retry               <= 25 * 2 ms, worker only
request-path filesystem waits          0
root directory mode                    0700
slot file mode                         0600
```

## Verification

Remote worker: `e@192.168.3.94`, worktree
`/home/e/projects/nando-wave-f8-full`, incremental compilation disabled.

```text
generation shadow learning             4/4 PASS
atomic shadow persistence              2/2 PASS
serving F7/F8 integration              6/6 PASS, 1 release perf ignored
learning full library                216/216 PASS
learning receipt bridge                1/1 PASS
persistence full integration          15/15 PASS
Clippy touched packages -D warnings       PASS
rustfmt / git diff --check                PASS
```

Code manifest SHA-256:
`6a333770d1951bf4b29ec0a151a772e9d478a102753ff6d401b2fa6a6785d8b8`.

## Boundary

F8-B creates durable evidence, not admission or production authority. Its only
consumer in this stage is the independent F8-C reconstructor.
