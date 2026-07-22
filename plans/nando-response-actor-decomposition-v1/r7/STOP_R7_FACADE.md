# STOP-R7: Thin Facade And Consumers

Status: `PASS / COMPATIBILITY_PRESERVED / AUTHORITY_FALSE`

Code HEAD: `54c3350e8c3f2fc88032a57412526214a374e17b`

```text
lib.rs lines                                  399
root domain definitions                         0
public API surface                        342/342
public API SHA drift                             0
Cargo binary names                         13/13
largest root binary wrapper                    166
operator dependency cycles                       0
learning -> runtime/admission edges               0
```

The facade root contains module declarations, compatibility re-exports, and
the integration-test attachment only. Response-specific adapters and
cross-owner application orchestration remain internal; reusable law, proof,
runtime, admission, and learning mechanisms have explicit crate owners.

All-target compilation and the exact historical response fingerprint pass.
No binary name, deployment state, service invocation, or authority changed.

Machine receipt: `STOP_R7_FACADE.json`.
