# R2 Leaf Kernel Checkpoint

Status: `PASS / R2_IN_PROGRESS`

Base HEAD: `8a3d58c703a376c7ce6ae9412914f136c75f05d2`

Authority: `false`

F5-B: `NOT_STARTED`

## Moved Ownership

```text
authority.rs canonical JSON/SHA helpers
  -> nando-operator-kernel::canonical

contracts.rs relation and verifier contracts
  -> nando-operator-kernel::contracts

program.rs immutable response program and validation
  -> nando-operator-kernel::program

binding_evidence.rs typed predicate vocabulary only
  -> nando-operator-kernel::binding
```

The old `contracts` and `program` modules are compatibility shims. Binding
capture, graph construction, version-space search, evidence, runtime,
verification, admission, and authority did not move.

## Verification

```text
kernel tests                               9 / 9 PASS
kernel Clippy -D warnings                      PASS
scoped rustfmt                                  PASS
response full baseline               494 PASS / 26 known FAIL
test failure fingerprint                       PASS
Clippy fingerprint                    12 + 8 / PASS
new background processes                         0
authority                                     false
```

The full-workspace rustfmt check remains contaminated by the pre-existing
unrelated dirty `crates/nando-core/src/wave.rs`; package-scoped formatting is
clean and that file was not modified.

This checkpoint authorizes continuation inside R2 only. It does not satisfy
STOP-R2 and does not unlock R3.
