# S1C-3G Stable Health Projection Preregistration V1

Status: `PAPER FROZEN / AUTHORITY FALSE`

Date: `2026-08-12 Europe/Tallinn`

Immutable parent:

```text
S1C-3F transaction       20260812T163201Z-55376ab7f5fa-s1c3f-v1
S1C-3F verdict           S1C3F_ROLLBACK_PASS
S1C-3F state root        e98b72cac96a14049dc64c728fccba1609a0f2a2f2bec1c744036c19f8afd403
S1C-3F receipt root      b19c831e563f715063c2ae026a589e7f1651ab93c05b90390215529eb297a8cf
S1C-3F final root        6c64cbf4399fd7f12dfcf808ed2248a8674e1540a80cf28bbab78fca7337e7bf
S1C-3F journal root      6ab9cd4823f1d737ec731e9c96049c0492d1966d91b3408dd524ea23b1c8666c
rollback reason          s1c3e_route_probe
```

S1C-3F is terminal and is neither rerun nor relabelled. S1C-3G inherits its
candidate binary, candidate config, record-aware ledger parser, preserved
journal, rollback pair, resource/parity receipts, and optimization WATCH.

## 1. Exact Question

Can the inherited candidate remain installed when route and health parity are
evaluated over endpoint-owned stable fields, while dynamic observations and
candidate installation effects are checked by their own owners?

S1C-3G changes no production binary or config bytes. It repairs only the
transaction's comparison contract under a new paper, source, and attempt
identity.

## 2. Failure Being Repaired

S1C-3F replaced a generated POST probe with read-only health observations, but
the inherited transaction still had two equality owners:

```text
semantic_health_equal  -> equality over one broad seven-field object
route_probe equality   -> equality over the complete per-endpoint receipt
```

That contract mixed transport/safety invariants with dynamic or candidate-owned
observations. A raw object or raw hash may change after an intentional process
replacement without a route, serving, admission, or safety failure.

S1C-3G does not waive health checks. It makes their ownership explicit.

## 3. Frozen Endpoint Projections

Every endpoint must remain reachable and return JSON with `ok == true`. Missing
labels, missing stable fields, extra labels, wrong field types, or a stable
field mismatch are hard failures. Equality is exact, including `null` where
preregistered.

### hot: `http://127.0.0.1:18789/health`

```text
stable_fields
  ok
  service
  mode
  admission_verdict
  response_executor_cache_ready
  response_active_profiles

observed_not_compared_fields
  raw_sha256
  transition_active_profiles
  all counters, timestamps, revisions, errors and nested telemetry
```

### cpu: `http://192.168.3.94:8787/cpu-health`

The stable and observed fields are identical to `hot`. In addition, the frozen
projection requires `hot == cpu`; the local serving endpoint and routed CPU
endpoint may not disagree about serving/admission state.

### control: `http://127.0.0.1:18788/health`

```text
stable_fields
  ok
  service
  mode

observed_not_compared_fields
  raw_sha256
  cpu_allowed
  transport_dependency
  fields not published by this endpoint
```

### gateway: `http://192.168.3.94:8787/health`

```text
stable_fields
  ok
  service

observed_not_compared_fields
  raw_sha256
  transport
  python_transport
  scope
  fields not published by this endpoint
```

The route receipt contains only endpoint label, frozen URL, and its stable
projection. Whole-object equality, raw-hash equality, wildcard projection, and
fallback to the old seven-field object are forbidden.

## 4. Candidate-Owned Effects

These are not ignored. They are checked by dedicated owners instead of baseline
equality:

```text
transition MainPID             must change once, then survive unchanged
capture environment            must equal the two frozen candidate values
capture writer                 must open the exact existing journal
startup log                    must contain no capture-unavailable error
journal                        must preserve prefix and valid natural suffix
binary and config              must equal frozen candidate hashes
```

The transaction expects no health field to change. `expected_to_change_fields`
is therefore empty for all four endpoint comparisons. PID and capture state are
separate runtime comparisons, not hidden health exceptions.

## 5. Comparison Ownership

One pure function builds the endpoint-specific stable projection. Both inherited
owners must use it:

```text
health semantic parity
route receipt parity
```

Unit tests must fail if either path compares a whole object, raw hash, or the
old semantic object. The independent verifier binds the frozen projection
schema and requires the receipt booleans produced by both paths.

## 6. Allowed Transaction

After local tests, pushed implementation identity, remote baseline preflight,
and durable rollback arming:

```text
verify S1C-3F parent and magic-only journal
-> install exact inherited candidate binary/config
-> intentionally restart transition-serving only
-> verify endpoint-specific stable projections
-> verify candidate-owned effects
-> freeze record-count-zero opening cursor
-> survive 15 seconds
-> verify projections, service/PID, connector, economics and journal
-> seal exactly one S1C-3G result
```

Generated traffic and synthetic fixtures remain forbidden. Nginx, connector,
control, authority, and learning services are not restarted.

## 7. Failure And Rollback

Any post-arming failure restores the exact baseline binary/config. All opening
journal prefixes and naturally arriving valid suffixes are preserved. Every
mutating failure path ends in `ROLLBACK_PASS` or `VETO`; a pre-mutation failure
ends in `PREFLIGHT_FAILURE`. No `PREPARED`, `ROLLBACK_ARMED`, or pending receipt
may be left as a terminal state.

## 8. Result Boundary

```text
S1C3G_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH
-> capture installed
-> S1C-4 COLLECTING from a sealed zero-record cursor
-> scientific_authority false

any other terminal verdict
-> S1C-4 CLOSED
```

Installation alone does not prove a decision episode, grounded meaning, K2,
Law #2, model training authority, or phase mutation.

## 9. Attempt Discipline

Paper, critique, and structural receipts are committed and pushed before code.
The live implementation preflight must return `READY_TO_IMPLEMENT` with zero
blockers before implementation. Implementation and verifier are committed and
pushed before exactly one production transaction. Any newly discovered defect
gets a new identity; S1C-3G is never rerun or relabelled.
