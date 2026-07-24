# Live CPU Polling Proof - 2026-07-24

## Purpose

Prove the complete ordinary traffic route on a real Codex/Lay session:

```text
Lay window
-> Nando Nginx gateway
-> transition serving
-> externally admitted ACTIVE package
-> role-bound actor
-> independent verifier
-> CPU response
```

This is not a synthetic transition request and not a controlled economics
probe.

## Blocker

The registry contained two `write_stdin` programs with the same structural
selector:

```text
Process running with session ID <role>
```

One emitted a passive poll. The other emitted `chars="\u0003"`. Both grounded
on the same live output, so routing correctly returned
`ambiguous_phase_route`.

The interrupt program had no runtime evidence that distinguished cancellation
intent from ordinary polling. Its non-empty literal was therefore action
intent without an applicability guard.

## Resolution

The operator kernel now requires semantic applicability evidence for non-empty
string action literals. Crystallized admission keeps such an unguarded package
as a candidate instead of blocking safe siblings.

```text
crystallized candidates  3
ACTIVE before            3
ACTIVE after             2

ACTIVE:
  write_stdin passive polling
  wait on yielded cell

not authorized:
  write_stdin Ctrl-C without a distinguishing guard
```

## Real Data Result

The active Lay session launched a long Cargo command. Its subsequent polling
turns crossed the complete production route.

```text
ordinary CPU accepts       0 -> 2
ordinary CPU input tokens  0 -> 315484
false accepts              0
runtime parity failures    0
unresolved revocations     0
gateway mode               CPU
signal-path verdict        PASS
```

At the proof snapshot:

```text
Nando input tokens         6257002344
miner-visible tokens        772708324  (12.3494%)
CPU input tokens             91294868  (1.4590% lifetime)
current accounting epoch     48779571 / 295221620 (16.5230%)
```

These numbers prove the route, not product completion. M3 remains WATCH because
the verified share is below 50%, three clean windows are not complete, and the
historical miner ledger still contains seven already-contained false accepts.

## Verification

```text
nando-operator-kernel       22 PASS
nando-operator-admission    10 PASS / 1 explicit audit ignored
online admission            20 PASS
nando-transition-serving    90 PASS
nando-gateway-control       49 PASS
production Clippy targets   PASS
composite live gate         PASS
```

Deployed admission binary SHA-256:

```text
9feae0bbcb782824ff277c63d961ad65adc63c74ce10e1e1542b78bde14c4b50
```

Authorized response registry revision:

```text
6010694863419298382
```

## Sustained Observation

After the initial proof, the same live Lay session continued without a route
change:

```text
ordinary CPU accepts          68
ordinary CPU input tokens     5714577
current process CPU share     10.8469%
current accounting epoch      17.9814%
lifetime CPU share             1.5501%
false accepts                  0
runtime parity failures        0
signal-path verdict            PASS
```

## Next Product Step

Do not increase proof counts or relax routing. Raise CPU coverage by inducing
additional independently grounded operators for the real request surfaces,
starting with the custom `exec` result that carries `session_id` in a bounded
JSON field. Every added operator must preserve the same zero-error gates.
