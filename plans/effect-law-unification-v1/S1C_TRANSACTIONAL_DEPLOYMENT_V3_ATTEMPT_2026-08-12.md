# S1C Transactional Deployment V3 Attempt 2026-08-12

Status: `PREFLIGHT RESOURCE FAILURE / NO PREPARATION / NO PRODUCTION MUTATION / V3 TERMINAL`

## Frozen Attempt

```text
paper commit             b1e16f6dec339f553648e933ea0ae059cc4123a3
implementation commit    d204f9b
candidate commit         a3ea27a49af397ef79e5c9ec80089ecf53a41d59
transaction id           20260812T001748Z-b1e16f6dec33-s1c3v3
```

## Exact Result

V3 repaired the V2 ownership defect. Both parity oracles built as user `e`,
the rooted ownership receipt passed, and the clean quiescence window opened.
The first three-ledger durability run then failed the frozen p99 bound:

```text
oracle ownership                         PASS
quiescence                               PASS
measurement contamination               false
hot latency                              PASS 3/3
single-ledger durability                 PASS 3/3
three-ledger durability                  FAIL 0/1

observed p99                             5,767,585 ns
frozen p99 limit                         5,000,000 ns
observed hard max                        6,125,924 ns
frozen hard-max limit                   20,000,000 ns
```

The hard maximum passed, but the p99 did not. The executor therefore stopped
before parity execution, resource receipt, preparation, or any service command.
The result is a candidate resource-contract failure, not a deployment failure.

## Ownership And Quiescence

```text
ownership root
  ea3af0ac4da08cd0209783c9e6219d0b5dd50e4fc302f3b78200166d4c491346

quiescence root
  3322984af6046e0b5a400aa64c14df8665693788b60130ffb4c27175e142957a

attempted intervals                     735
eligible CPU4 mean                      0.066006600660066%
contamination root
  2b50b23b7212cdbd8bee1d77848b80b84e09fd78ed38563a2bad9646f3817788
forbidden build-process matches          0
monitor errors                           0
maximum monitor gap                      0.508104534 s
```

The ownership receipt proves exact `e:e` ownership, directory mode `0750`,
file mode `0640`, and non-root exclusive create, fsync, unlink, and directory
fsync for both oracle workspaces. The receipt itself is mode `0400`.

## Evidence

Remote directory:

```text
/var/lib/nando-wave/deployments/
  20260812T001748Z-b1e16f6dec33-s1c3v3/
```

Local mirror:

```text
/home/ubu/.local/state/nando-wave/s1c3/
  20260812T001748Z-b1e16f6dec33-s1c3v3/
```

```text
complete local evidence listing root
  c4b5294902bcc81deea2aaede72be9010e4620d8ab321ba0c6dc402de121cb82

preflight failure SHA-256
  1b54e064c9494ea3d9a5e87fd522c2d1e3848596b4bba6dbad5f09b25f2bfbaf

ownership receipt SHA-256
  20dd91eaffa2204dd355ea2033b403015d923a2ba2936b3001ec251bde2deb62

quiescence receipt SHA-256
  23f68554856fc14abfd42e8f38d2c6b0c61682bc9c1b93af2ba05622cd8c64a8

contamination receipt SHA-256
  da84eb907d32eddd85e35f5e0c2a137f0acf145084326e781ceb1ed45c2acec4
```

## Production Preservation

```text
transition-serving          PID 165670   restarts 0   active
response-learning           PID 369456   restarts 0   active
gateway-control            PID 1035203  restarts 0   active
certification authority     PID 164668   restarts 0   active
transport / Nginx           PID 682430   restarts 0   active
local connector             PID 2919     restarts 0   active
route receipt failures      0
grounded-decision journal   ABSENT
```

```text
installed binary SHA-256
  6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58

installed config SHA-256
  cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5

post-attempt composite gate              PASS
structural routes                        4/4 PASS
false accepts                            0
runtime parity failures                  0
```

V3 is not retried. A further operational-capture attempt requires a new frozen
candidate and preregistration that preserves the 5 ms p99 and 20 ms hard-max
budgets while repairing the three-ledger durability path itself.
