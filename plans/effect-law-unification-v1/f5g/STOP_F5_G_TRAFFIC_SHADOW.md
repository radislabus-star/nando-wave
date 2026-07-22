# STOP-F5-G Incoming Traffic Shadow

Status: `COMPLETE / PERFORMANCE WATCH / F5 COMPLETE`

Implementation commit:

```text
98cee36bf9edc2333facaea836df5b837e2cbbe9
```

Authority: `false`

## Result

```text
immutable StructuralDispatchIndexV3
-> TrafficShadowGenerationV3
-> short-lock Arc pin
-> one borrowed request snapshot
-> F5-B context extraction
-> F5-C dispatch and complete binding
-> F5-D capability/action grounding
-> F5-F phase ranking
-> F5-E actor/VM shadow
-> one hash-only terminal verdict
```

F5-G adds orchestration, not another runtime owner. It calls the already frozen
F5-B through F5-F boundaries and cannot compile an operator, verify its result,
persist a package, or grant authority.

## Frozen Ordinary Window

The immutable economics source contained 222 rows, of which 25 were marked as
ordinary traffic. The frozen artifact commits the source SHA-256 and keeps only
bounded metadata:

```text
source rows                                      222
ordinary denominator                              25
accounted terminal verdicts                       25 / 25
CENSORED_PAYLOAD_UNAVAILABLE                      25
raw request text available                         0
raw provider payload available                     0
invented replay rows                                0
```

Source SHA-256:

```text
cf6de0789fc363957d79ffc207f93a7f5c542edd2892dd87a86c705f8af07e60
```

Frozen manifest SHA-256:

```text
cc2eec8e389ab4b22c8a5f6ba0262733a303d59bba9c33a5c737cbd64febb280
```

The ordinary denominator is fully accounted, but organic actor/VM matching is
`WATCH_PAYLOAD_UNAVAILABLE`. Metadata was not upgraded into synthetic runtime
evidence.

## Traffic Controls

```text
Responses non-streaming                         PASS
Responses streaming                             PASS
Chat Completions non-streaming                  PASS
Chat Completions streaming                      PASS
unsupported Transition API                      ABSTAIN
oversized request text                          ABSTAIN
100 requests pinned across generation swap      PASS
mixed-generation receipts                          0
non-monotonic generation swap                   REJECT
queue enqueue / full / disconnected accounted   3 / 3
blocking send or receive in F5-G                    0
raw payload persistence                             0
false accepts                                       0
local accepts from F5                               0
production callers                                  0
execution authority                              false
```

The queue helper owns no queue and exposes no blocking method. It only turns a
caller-owned `try_send` result into one accounted handoff verdict.

## T480 Performance

Release-mode measurement used a 2,048-mode immutable index and 4,096 warm
iterations per route. JSON construction and request hashing were prepared
outside the timed F5 call.

```text
no-match p99                         600,487 ns   WATCH (> 250,000 ns)
matched shadow p99                 1,193,242 ns   WATCH (> 1,000,000 ns)
hard latency ceiling               2,000,000 ns   PASS
conservative process RSS delta    49,623,040 B    WATCH (> 16 MiB)
```

The RSS result is deliberately conservative: the same test process constructs
the cold artifacts before retaining the hot index, so allocator retention is
included. It is not reported as a hot-registry PASS. F5 safety is complete;
representation compaction remains required before live hot integration.

No latency threshold, structural budget, or safety gate was relaxed.

## Verification

Local T480:

```text
focused F5-G                         7 PASS / 1 release-only ignored
nando-operator-runtime              46 PASS / 1 ignored
release performance probe            PASS with explicit WATCH metrics
Clippy -D warnings                    PASS
rustfmt / diff check                  PASS
```

Remote clean detached worktree:

```text
host                     e@192.168.3.94
worktree                 /home/e/projects/nando-wave-f5g-98cee36
HEAD                     98cee36bf9edc2333facaea836df5b837e2cbbe9
target                   /home/e/build/nando-wave-f5g-target
incremental              disabled
nando-core lib           176 PASS / 5 ignored
nando-operator-kernel     13 PASS / 0 FAIL
nando-operator-learning  198 PASS / 0 FAIL
nando-operator-runtime    46 PASS / 1 ignored
all integration targets  PASS
Clippy -D warnings       PASS
```

Owner-local structural gates:

```text
generation ownership     PASS / authority_ready=false
shadow pipeline          PASS / authority_ready=false
traffic and privacy      PASS / authority_ready=false
```

Live composite gate after the implementation commit:

```text
verdict                         PASS
eligible_for_local_accept       false
response ACTIVE packages        0
response M3                     WATCH
response false accepts          0
response parity mismatches      0
```

Both services were already active and remained untouched:

```text
nando-transition-serving  InvocationID=74ac3080f80b4fe387de2a94380e3657 NRestarts=0
nando-response-learning   InvocationID=8e59505eb1b943778601c9b3bacbd607 NRestarts=0
```

No deployment, restart, registry write, package promotion, or authority change
occurred.

## Boundary

F5-G is complete with explicit product-performance and organic-replay WATCH
items. These WATCH items prohibit a hot-path performance claim; they do not
weaken the completed fail-closed runtime-convergence proof.

F6 is unlocked and not started.
