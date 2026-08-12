# S1C-3G Capture Installation Terminal Report - 2026-08-12

Status: `TERMINAL ROLLBACK PASS / BASELINE RESTORED / S1C-4 CLOSED`

## Result

The sole preregistered S1C-3G transaction installed the frozen candidate,
observed a post-install stable route projection mismatch before the survival
window, and completed a verified rollback.

```text
transaction          20260812T180143Z-1369da0a49ef-s1c3g-v1
source commit        1369da0a49efcd48d3443da1eedaf51eefb73023
verdict              S1C3G_ROLLBACK_PASS
capture installed    false
baseline restored    true
S1C-4                CLOSED
attempt              1 / 1 CONSUMED
```

The transaction did not reach the S1C-3G authority-renewal observation. It
failed inside the inherited immediate post-install route check, 0.208 seconds
after the candidate process started and before the 15-second survival check.

## Exact Evidence Boundary

The durable executor error is:

```text
S1C3G_ROLLBACK_PASS:s1c3e_route_probe
```

The candidate startup log also contains:

```text
nando-response-authority refresh: response_authority_runtime_build_mismatch
```

These facts are temporally adjacent, but the failed candidate projection was
not persisted before rollback. Therefore the startup diagnostic is a
proximate observation, not a proved complete cause of the route mismatch.
S1C-3G must not be relabelled as an authority-renewal failure or a data-plane
failure.

## Immutable Roots

```text
state root       e88e30b422e279824699b3d8d65afedb8de954cf73cf28ecfb7e872fb89ef44f
receipt root     a453ef1b565d304c3e35f7b8a09d6b503098318fa9c3c9b0aa1e531b703d7965
final root       c45f668f285bebf81ca466451eba55689e16112cd0a217a0e5a6392af9f31414
journal root     6ab9cd4823f1d737ec731e9c96049c0492d1966d91b3408dd524ea23b1c8666c
compact manifest e77599a984d9ce86f412acbe60a16133ba6a1f2bf4c03d4879c07248a029ff14
terminal root    180ccb0e04748c9246a2d2316c85aa8b6aa6426ae8350576526dc8fc5c385745
```

The compact Git packet contains six exact artifacts totalling 31,538 bytes.
The five artifacts mirrored from the mini-PC are byte-identical to their
remote copies. The 62.8 MB candidate binary remains in the immutable remote
transaction and is not duplicated in Git.

## Production Preservation

After rollback and sealing:

```text
transition-serving       active, restarts 0, baseline binary/config restored
gateway-control          active, restarts 0
certification authority  active, restarts 0
response learning        active, restarts 0
Nginx gateway            active, restarts 0
connector                active, PID 2919, restarts 0
route receipt failures   0
false accepts            0
runtime parity failures  0
journal records          0, all three NTF1 prefixes preserved
```

The transition-serving PID changed because the candidate start and rollback
were intentional transaction effects. Nginx and connector were not restarted.

## Claim Boundary

```text
S1C-3G attempt          TERMINAL ROLLBACK PASS
production capture     NOT INSTALLED
S1C-4 natural census   CLOSED
grounded meaning       NOT PROVED
K2                     CLOSED
Law #2                 NOT AFFECTED
model training         FORBIDDEN
phase mutation         FORBIDDEN
```

S1C-3G may not be rerun. Any successor needs a new paper contract that moves
the authority-renewal observation ahead of transient route equality and
persists both compared candidate projections before deciding PASS or rollback.
