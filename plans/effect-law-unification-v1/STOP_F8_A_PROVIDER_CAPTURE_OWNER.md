# STOP-F8-A Provider Capture Owner

Status: `CONTROLLED_PASS / LIVE_NOT_RUN / AUTHORITY_FALSE`

Date: `2026-07-22 Europe/Tallinn`

## Result

F8-A closes the code and controlled-proof boundary for provider capture without
changing the running service:

```text
provider request bytes
-> one existing request SHA-256
-> hash-only ProviderRequestCaptureReceiptV3
-> bounded nonblocking queue
-> atomic two-slot capture index
-> exact no-authority F7 telemetry handoff
```

The feature defaults to disabled. No deploy, restart, live enable, package
promotion, local accept or authority change was performed.

## Ownership

```text
nando-transition-serving/provider_capture
  request-path sequence allocation, receipt sealing, try_send, telemetry

nando-operator-learning/provider_capture_v3
  canonical receipt, durable lease commitment, bounded index

nando-operator-persistence/provider_capture_store_v3
  private two-slot publication, restore and corruption policy

nando-transition-serving/generation_shadow
  exact receipt handoff to existing F7 -> F5 -> independent F6 telemetry
```

The capture owner cannot select an operator, execute an actor, relabel verifier
outcomes, grant admission or mutate semantic memory.

## STOP Matrix

```text
request-path blocking writes          0      PASS
second request-body hash              0      PASS
raw payload bytes persisted           0      PASS
restart sequence reuse                0      PASS
duplicate event/request roots         BLOCK  PASS
queue overload semantic updates       0      PASS
```

Important terminal distinction:

```text
request path: ENQUEUED
writer after fsync publication: CAPTURED
```

F7 may evaluate an enqueued receipt as telemetry. F8-B must join that receipt
against the durable capture index; otherwise it remains censored and cannot
become future evidence.

## Budgets

```text
capture queue                  <= 48
canonical index records        <= 16,384
canonical index bytes          <= 8 MiB
receipt bytes                  <= 1,024
root directory mode            0700
slot file mode                 0600
raw payload fields             0
```

## Verification

Remote worker: `e@192.168.3.94`, isolated worktree
`/home/e/projects/nando-wave-f8a`, incremental compilation disabled.

Code manifest SHA-256:
`cc621a7d5b513f6609765bd8496467e064ffd0b7155e6311db3eba17ac41473f`.

```text
SHA-256 commitment tests                 2/2 PASS
capture receipt/index tests              5/5 PASS
atomic persistence/restart tests         3/3 PASS
serving capture/overload tests           3/3 PASS
exact F7 receipt handoff                  1/1 PASS
F7 integration                           5/5 PASS, 1 release perf ignored
gateway signal-map tests                 3/3 PASS
kernel full suite                       18/18 PASS
learning full suites                    213/213 PASS
persistence full suites                 13/13 PASS
gateway full suites                     18/18 PASS
Clippy five touched packages            PASS, -D warnings
rustfmt / git diff --check               PASS
```

The transition-serving library baseline remains unchanged apart from the two
new F8-A tests:

```text
untouched baseline      49 PASS / 3 known FAIL
F8-A branch             51 PASS / 3 same known FAIL
new failure classes      0
```

The three pre-existing failures are in `session_stream`/`session_backfill` and
were reproduced independently in the untouched baseline worktree.

## Structural Gates

```text
HTTP ingress owner               PASS / authority_ready=false
capture kernel owner             PASS / authority_ready=false
persistence owner                PASS / authority_ready=false
F7 handoff owner                 PASS / authority_ready=false
nando-live-transition-gate       PASS
eligible_for_local_accept        false
response ACTIVE packages         0
M3                               WATCH
```

Graphify on the final code surface: `27,690 nodes / 61,972 edges / 1,289
communities`.

## Stop Boundary

```text
F8-A implementation              CONTROLLED_PASS
F8-A live provider traffic       NOT_RUN
F8-B durable shadow ledger       NOT_STARTED
F8-C external reconstruction     NOT_STARTED
F8-D causal/performance gates    NOT_STARTED
F8-E SHADOW rollout              NOT_STARTED
execution authority              false
```

The next permitted action is a separate predeployment review followed by an
explicit SHADOW-only enable of F8-A. F8-B must not start from in-memory telemetry
or from an uncommitted receipt.
