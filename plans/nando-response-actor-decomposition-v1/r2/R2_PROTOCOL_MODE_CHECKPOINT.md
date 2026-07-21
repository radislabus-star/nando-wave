# R2 ProtocolMode Checkpoint

Status: `PASS / R2_IN_PROGRESS`

Base HEAD: `cd448e912108c9adddecb9b5d4245d0e13f69e03`

Authority: `false`

F5-B: `NOT_STARTED`

## Ownership Cut

```text
ProtocolMode wire IR, canonical bytes, digests, validation
  -> nando-operator-kernel::protocol_mode

candidate generation, structural selector search, exact cover
  -> nando-response-actor::protocol_mode

evidence, runtime, verifier, admission, authority
  -> unchanged
```

The old `nando_response_actor` public names remain compatibility re-exports.
No duplicate ProtocolMode definition remains in the response actor.

## Measured Result

```text
response protocol_mode.rs + selector.rs       1,414 -> 866 lines
kernel ProtocolMode owner                               721 lines
kernel ProtocolMode tests                              2 / 2 PASS
F4/F5-A focused tests                                21 / 21 PASS
kernel Clippy -D warnings                                  PASS
response full baseline                  494 PASS / 26 known FAIL
test failure fingerprint                                  PASS
Clippy fingerprint                               12 + 8 / PASS
new background build processes                              0
kernel forbidden side-effect imports                         0
authority                                                  false
```

The response denominator is 520 after eight owner tests moved into the kernel;
the previous 528-test baseline is therefore `494 PASS / 26 known FAIL` in this
crate. The focused suite includes canonical restart, tamper rejection, F4R2 mode-set,
and F5-A executable-payload parity. The STOP runner result is recorded in
`R2_PROTOCOL_MODE_REMOTE_STOP.json`; exit code 101 is the frozen known-debt
baseline, while `fingerprint_verdict=PASS` proves no failure-set drift.

Both live services retained their original invocation IDs and zero restarts.
This checkpoint permits the remaining immutable R2 moves only. It does not
satisfy STOP-R2 and does not unlock R3.
