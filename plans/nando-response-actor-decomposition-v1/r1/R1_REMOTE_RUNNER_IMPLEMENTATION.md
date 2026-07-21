# R1 Remote Development Runner

Status: `IMPLEMENTED_AWAITING_EXACT_COMMIT_STOP`

Base HEAD: `2fdfa3a19bdb14b6254b6fab3683cc5c9acbd92c`

Authority: `false`

F5-B: `NOT_STARTED`

## Route

```text
local exact HEAD
  -> dedicated remote clone /home/e/projects/nando-wave-dev
  -> optional explicit path-scoped dirty overlay
  -> fast | stop | release target
  -> one compiled test binary
  -> direct owner-filter executions
  -> exact baseline fingerprint comparison
  -> machine-readable receipt
  -> SSH master closed, Cargo/Rustc children 0
```

Unrelated dirty paths are absent unless named with `--scope`. The runner never
rsyncs the whole working tree and therefore does not silently import the local
dirty `nando-core/src/wave.rs`, journal, Graphify output, or diagnostic file.

## Measured Results

```text
cold fast compile                         26.30 s
warm unchanged compile                     0.05 s
real comment-only owner edit compile       5.02 s
owner test runtime                         0.37 s
clean STOP compile                        22.45 s
full 528-test runtime                     21.72 s
release test build                        64.28 s
dev target after owner edit                1.04 GiB
proof target                               198.5 MiB
release target                             114.3 MiB
new background build processes                  0
```

The temporary timing comment was removed immediately after measurement; the
source file has no diff.

## Proof Baseline

```text
full lib                            502 PASS / 26 known FAIL
test failure fingerprint            PASS
Clippy                              12 library + 8 test-only
Clippy fingerprint                  PASS
scope patch overlay                 PASS
scope untracked overlay             PASS
warm path without GitHub network    PASS
target hard budget                  12 GiB
```

The first warm attempt exposed an unnecessary GitHub fetch and failed during a
TrustTunnel DNS reinitialization. The runner now checks the local remote object
database first; warm runs perform no network fetch. A missing commit uses a
bounded retry only until that commit is present.

## Live Boundary

```text
nando-transition-serving InvocationID   74ac3080f80b4fe387de2a94380e3657
nando-response-learning InvocationID    8e59505eb1b943778601c9b3bacbd607
service restarts                        0
authority                              false
deployment                             not touched
```

## Remaining STOP Action

Commit the tooling, then run the `stop --graphify` profile from that exact
commit. Only that second receipt may mark STOP-R1 complete.
