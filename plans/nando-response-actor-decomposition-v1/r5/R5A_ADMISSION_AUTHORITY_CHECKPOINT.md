# R5-A Admission Authority Checkpoint

Status: `PASS / AUTHORITY_FALSE / R5_IN_PROGRESS`

## Cut

`nando-operator-admission` now owns:

- composite admission and authority wire contracts;
- digest-bound package projections consumed by policy;
- authority validation and lease material;
- post-verifier and runtime verification receipt construction;
- durable runtime parity receipt sealing.

`nando-response-actor` remains a compatibility adapter. It validates the
concrete registry, derives immutable package records, and delegates policy to
the admission owner. Mutable learner state and runtime objects do not cross the
new crate boundary.

## Gates

```text
nando-operator-admission tests       PASS
nando-operator-admission Clippy      PASS
response authority focused tests     PASS
response admission-bundle tests      PASS
runtime imports                      0
learning imports                     0
authority change                     0
```

Machine receipt: `R5A_ADMISSION_REMOTE_STOP.json`.

R5 is not complete. Package policy, lifecycle policy, and online admission
policy still require owner separation before `STOP-R5`.
