# STOP-F8-E Controlled Live Shadow

Status: `PASS / SHADOW_READY / AUTHORITY_FALSE`

Date: `2026-07-22 Europe/Tallinn`

## Live Route

The actual Rust service executed the complete no-authority route:

```text
provider HTTP request
-> hash-only ProviderRequestCaptureReceiptV3
-> atomic ProviderCaptureStoreV3
-> pinned controlled F7 generation
-> F5 role grounding and actor
-> independent F6 verifier
-> generation-owned durable shadow ledger
-> admission-owned phase controls
-> immutable external reconstruction
-> SHADOW_READY
-> provider fallback
```

The deployed service remained in `SHADOW`. `NANDO_LOCAL_ACCEPT_ENABLED` and
`NANDO_CLIENT_ALLOW_LOCAL_ACCEPT` were absent from the process environment.
All three health flags remained false:

```text
local_accept_enabled                    false
effective_local_accept_enabled          false
response_effective_local_accept_enabled false
response_active_profiles                0
```

## Restart And Durability

The first verified receipt published a 5,537-byte generation-owned slot. After
service restart, a second verified request restored that ledger and published a
10,613-byte next slot. The final deployed binary restored both receipts and
published a 15,702-byte third slot.

```text
verified live receipts                  3
shadow ledger publish sequence          3
provider records                        4
provider restart sequence reuse         0
provider reserved through sequence      147456
raw payload bytes persisted             0
false accepts                           0
parity mismatches                       0
local accepts                           0
```

One repeated request was deliberately replayed against the final binary. It
was classified as duplicate and reached terminal durable censoring in 20,133
microseconds instead of occupying the 500 ms unique-capture join window. It
made no semantic update.

The provider writer and generation reader are now separate types. Only the
writer performs startup recovery of stale temporary files. A live reader never
moves the writer's in-flight `.new` publication.

## External Reconstruction

The service was stopped only long enough to copy one immutable nine-file,
55,502-byte snapshot, then immediately restarted. A test-only audit process on
the remote build host read that snapshot and independently reconstructed:

```text
live denominator                 3
verified passes                  3
full phase gain                  3
negative denominator             0
censored denominator             0
verdict                          SHADOW_READY
execution authority              false
```

Canonical candidate commitments are in
`STOP_F8_E_EXTERNAL_ADMISSION_CANDIDATE.json`. File SHA-256:
`b1fdb98045600d3dd5f5c48ed366ad8b1a40ea4ccf4c5c9cba303ad5fbef77a4`.

Final deployed binary SHA-256:
`e28a56d884383b5adc0c5506ddddbff43055e8a3cba49c2dce530bf776720ad2`.

## Scope

The seed is explicitly `CONTROLLED_SHADOW_ONLY`. This result proves the live
transport, persistence, actor, independent verifier, causal controls and
external reconstruction. It does not prove a naturally learned operator,
ordinary-traffic coverage, 50 percent CPU execution, M3, or production
authority.
