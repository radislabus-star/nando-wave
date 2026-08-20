# R8B Preimplementation Confirm Fixture Attempt History

Status: `FAILED AND SUPERSEDED ATTEMPTS RETAINED`

## Dependency-Drift Build

The initial standalone probe build began resolving newer dependencies outside
the repository `Cargo.lock`. It was stopped before any fixture was accepted.
No output from that dependency route is used as evidence.

## V1 Locked Probe

```text
probe source SHA-256  bb0e84ed95526e5f726c5c0a1401025716e396b6c795f12257a1629a3375af85
result                FAIL
error                 self_formed_development_owner_response_binding_invalid
journal prefix         ArtifactsFrozen -> GeneratorDispatched
accepted files         0
sealed attempts        0
```

Cause: the probe supplied a synthetic descriptor experiment root instead of
the experiment ID deterministically produced from the frozen Development seed.
The owner rejected the mismatch after one dispatch. The failed attempt remains
at `/home/e/.cache/r8b-preimplementation-confirm-fixture-bdcae535` and is not a
fixture source.

## V2 Locked Probe

```text
probe source SHA-256  6c196dc2eca8dac0d90d6546ae8f4d8a023e729323e0a6ed9fdacb283f1a1f60
result                PASS THEN SUPERSEDED
canonical files       41
post-write typed reopen not measured
sealed attempts        0
```

V2 proved the corrected binding but lacked final-file typed roundtrip. Its
Confirm generator/split bytes were later observed byte-identical to V3. Its
historical owner root differs from V3 because the absolute fresh lab path is a
legitimate owner-request input.

## V3 Accepted Preflight Fixture

```text
probe source SHA-256  37925c0ce536d34d73bcb3d10be7dfdec67cce9d000c5a4989ebcfa2cc75c714
result                PASS
canonical files       41
typed roundtrips      41 / 41
sealed attempts        0
```

Only V3 is copied into the repository evidence packet and may serve as the
preimplementation compatibility baseline.
