# STOP-F8-0 Resource Truth

Date: `2026-07-22`

Verdict: `RESOURCE_PASS_LATENCY_WATCH`

Authority: `false`

## Result

The old F5 measurement mixed generation compilation, dropped fixture
artifacts and the retained hot registry under an allocator policy that is not
used by the deployed service.

```text
2048-mode generation
-> compiler scratch dropped
-> 128 matched + 128 no-match warmup requests
-> 4096 matched + 4096 no-match measured requests
-> peak RSS across load / warmup / benchmark
```

Measured on `e-MEGA-MINI-M1-13th`:

```text
default mimalloc peak RSS delta             62,099,456 B  WATCH
deployed purge-policy maximum               10,723,328 B  PASS
target                                      16,777,216 B
production-policy resource observations          12 / 12 PASS
```

The repository unit and the currently loaded systemd unit both already set:

```text
MIMALLOC_PURGE_DELAY=0
```

No service restart, deployment change or budget change was required.

## Canonical Gate

The release-only runtime test now uses the production allocator and refuses to
run as a proof unless `MIMALLOC_PURGE_DELAY=0` is present. It measures RSS after
generation load, after warmup and after the full request loop, then enforces
the maximum of those three values against 16 MiB.

Canonical command:

```text
MIMALLOC_PURGE_DELAY=0 CARGO_INCREMENTAL=0 \
cargo test -p nando-operator-runtime --release \
  traffic_shadow_v3::tests::performance::production_allocator_hot_registry_resource_measurement \
  -- --ignored --exact --nocapture
```

## Latency Boundary

F8-0 closes only the resource blocker. It does not manufacture a latency PASS.
Concurrent and affinity-pinned observations were:

```text
no-match p99 range      160,882 .. 284,901 ns
target                  250,000 ns
matched p99 maximum     619,241 ns
matched target        1,000,000 ns
hard maximum          1,236,284 ns
hard ceiling          2,000,000 ns
```

One no-match observation exceeded its p99 target. F8-D must freeze an isolated
traffic measurement protocol and either restore the margin or return WATCH.

## Verification

```text
canonical release resource test       PASS
runtime unit tests                     47 PASS / 1 release ignored
gateway-control                        19 / 19 PASS
Clippy -D warnings                     PASS
Graphify nodes / edges / communities   27402 / 61309 / 1279
services restarted                     NO
deployment changed                     NO
```

## Boundary

```text
F8-0 hot RSS blocker       PASS
F8-A capture owner         READY / NOT STARTED
F8-D latency               WATCH
local accepts              0
execution authority        false
```
