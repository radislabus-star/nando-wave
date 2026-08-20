# K2 Self-Formed Uncertainty V5 R8B Contract V1

Status: `DRAFT FOR ADVERSARIAL REVIEW / NO EXECUTION AUTHORITY`

Date: `2026-08-20`

Predecessor: `K2_SELF_FORMED_UNCERTAINTY_V5_R8B_UNLOCK_2026-08-20.md`

## Purpose

R8B is the final non-sealed readiness run before the successor freeze. It must
show that the complete DevelopmentRehearsal path and all controls remain green
at one exact source commit under the frozen resource limits.

## Proposed Run

```text
clean exact commit
-> release build on mini-PC with 20 Cargo jobs
-> full nando-operator-learning package suite
-> 32 + 4 + 16 static controls
-> twelve DevelopmentRehearsal K1-K12 controls
-> R7H/R7I/R7J/R7K route tests
-> resource measurement
-> owner-bounded structural gates
-> R8B receipt
```

The run uses only the frozen development seed. It creates no authorization slot,
Confirm nonce, sealed attempt, scientific result, deployment or production
effect.

## Proposed Budgets

```text
child RSS          <= 512 MiB
case wall time     <= 60 seconds
batch wall time    <= 20 minutes
protocol object    < 1 MiB
manifest entries   <= 8,192
false accepts      0
sealed attempts    0
```

## Proposed Result

R8B may publish `R8B_FROZEN` only when all suites and controls pass, resource
limits hold, the worktree remains clean and the structural routes pass. Failure
publishes `R8B_FAILED` and retains the available logs.

R8B grants no scientific or deployment authority. R9B remains a separate
successor-freeze stage and R10B remains a mandatory authorization stop.
