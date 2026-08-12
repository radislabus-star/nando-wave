# S1C-3H Completion Repair V1

Status: `FROZEN ENGINEERING REPAIR / SAME S1C-3H BRANCH`

Date: `2026-08-13`

## Plain Purpose

S1C-3H installs the decision recorder required before K2 research can begin:

```text
goal frozen before action
-> applicable K1 alternatives including ABSTAIN
-> selected action
-> independently verified transition and goal satisfaction
```

The installation proves only that this recorder is installed and fail-closed.
It does not prove a grounded meaning, K2, or Law #2.

## Observed Blocker

Attempt `20260812T211329Z-570609bdef03-s1c3h-v1` prepared and started the
candidate pair successfully. Both authority `Type=oneshot` services completed
with `Result=success` and `ExecMainStatus=0`. The installer then read each
service twice. A 10-second path/timer trigger started the next normal run
between those reads, so the second read saw `activating` and falsely rejected a
successful installation.

The emergency rollback restored the old coherent runtime-authority pair, but
the same double-read race prevented terminal sealing. It also replaced the
first candidate diagnostic with a later rollback diagnostic.

Attempt `20260812T215829Z-ca820c181cc7-s1c3h-v1` exposed the remaining
lifecycle edge. Pausing an already running oneshot intentionally sends it
`SIGTERM`; systemd retained `Result=signal` and `ExecMainStatus=15` after the
runtime pair had otherwise started correctly. The restored triggers later
produced successful runs, but the installer could observe the intentional stop
result first. The primary diagnostic for this attempt was preserved.

Attempt `20260812T220528Z-05de910f9529-s1c3h-v1` showed that
`reset-failed` clears `Result` but can leave the preceding `ExecMainStatus=15`,
and a restored timer may not have started a replacement invocation yet.
Therefore neither cleared state nor timer timing is accepted as renewal proof.

Attempt `20260812T222021Z-a4f27ca36873-s1c3h-v1` completed the candidate
runtime check, but the local orchestrator inherited restrictive modes from the
first remote evidence mirror and could not replace that mirror for final
verification. Remote evidence modes remain immutable; only the local transport
copy is normalized to owner-writable bytes before refresh.

This is an installer observation race and evidence-retention defect. It is not
a negative result for decision capture or grounded meaning.

## Frozen Repair

```text
restore persistent path/timer units exactly
-> wait for one settled snapshot of both oneshot services
-> require inactive + Result=success + ExecMainStatus=0 in that snapshot
-> bind that same snapshot into the receipt
```

While every path/timer trigger is stopped, the installer may stop an in-flight
oneshot and immediately `reset-failed` only that oneshot. It must verify the
unit is inactive, then explicitly execute response-admission followed by the
live transition gate. Each explicit invocation must finish inactive with
`Result=success` and `ExecMainStatus=0`. Only then may background triggers be
restored. Reset-failed and timer timing never substitute for this renewal.

The first valid rooted candidate diagnostic is immutable. Recovery may bind to
it but cannot overwrite it or its startup log.

Every local evidence mirror must discard remote ownership and mode metadata.
Refreshing a mirror first makes the prior local copy owner-writable, then
replaces it. This normalization is transport-only and cannot mutate the remote
transaction directory or its receipt modes.

The interrupted attempt must be terminally sealed by a separately rooted
recovery receipt. That receipt may prove the old production pair is restored,
healthy, and fail-closed. It must explicitly report that the original
connector-before artifact was lost with the orchestrator session and therefore
must not claim full connector survival for that attempt.

## Acceptance

1. A regression test reproduces a timer retrigger after the settled snapshot.
2. Failed oneshots still veto installation.
3. Repeated rollback preserves the first diagnostic bytes and root.
4. The interrupted attempt reaches immutable `COMPLETE` with recovery scope.
5. A new transaction reaches `S1C3H_DEPLOYMENT_PASS` with capture installed.
6. Runtime and authority contracts match, cache is ready, active profiles are
   two, false accepts and parity failures are zero, and Nginx plus connector do
   not restart.
7. Natural journal count is reported separately and may remain zero.

## Stop Boundary

No K1 scheduler, Law #2, K2 model, synthetic traffic, phase mutation,
`graphify-out/`, or unrelated dashboard work belongs to this repair.
