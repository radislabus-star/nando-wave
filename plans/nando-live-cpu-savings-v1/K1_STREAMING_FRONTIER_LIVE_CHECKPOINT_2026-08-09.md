# K1 Streaming Frontier Live Checkpoint

Status: verified production checkpoint.

Date: 2026-08-09.

## Result

```text
operator-blind K1 endpoint       PASS / LIVE
retired MS3 snapshot path        CLOSED
streaming bootstrap              PASS
full-oracle equivalence          PASS
resource budget                  PASS
active runtime false accepts     0
runtime parity failures          0
Law #2                           NOT PROVED
Raw Phase causal law             NOT PROVED
```

The production learner now keeps K1 active without materializing the retired
full MS3 snapshot, cloning the complete collection checkpoint, or repeatedly
rebuilding unchanged bootstrap evidence.

## Source

```text
branch             raw-phase-frontier-v2-20260809
deployed commit    57cfee6f0bc73b076a13abf0a8afa127c3f90116
source tree        8a8eff1eeef0acccf060790d3922862f7b9bc650
installed SHA      962b2e1de1e2a958d0d31de81a97d2534e327b37d831aae61b2f8ae3ccaa4459
rollback commit    1585c3d65eb596b40883b870165ddb587c8f9aa0
```

Relevant commits:

```text
2cb62f6  Stream K1 bootstrap evidence
1f11641  Avoid cloning collection checkpoint for K1
57cfee6  Keep K1 runtime off legacy MS3 snapshot path
```

The final change is in
`crates/nando-transition-serving/src/lib.rs`: the legacy MS3 snapshot
materialization executes only when `multi_source_research_enabled` is true.
K1 remains independently enabled.

## Remote Verification

```text
rustfmt                            PASS
transition-serving tests          272 PASS / 7 ignored
strict Clippy                      PASS
release build                      PASS
candidate/install SHA parity       PASS
```

The live gate required:

```text
warmup                             <= 90 seconds
RSS                                < 2.5 GiB
HWM                                < 3.5 GiB
CPU                                <=25% for 4 consecutive samples
false accepts / parity             0 / 0
hot serving PID                    unchanged
Nginx PID                          unchanged
```

Observed:

```text
ready after                        15 seconds
steady RSS                         1,592,600 KiB
HWM                                1,997,080 KiB
final steady CPU                   0.20%
qualifying CPU streak              4
cold learner PID                   2731134
hot serving PID                    3901227
gateway-control PID                2385918
certification authority PID        2576117
Nginx PID                          682430
all NRestarts                      0
```

The previous candidate reached about 5.18 GiB RSS. The corrected candidate
stayed below both preregistered memory limits and returned to idle without
throttling.

## Immutable Deployment Receipt

```text
path
/var/lib/nando-wave/deployments/20260809T183637Z-57cfee6f0bc7/deployment-receipt.json

receipt root
36339b9b0e47a166162ae8e4b21994c803616a4ff2e19cc1e493bc087a55b7b5

file mode                         0400
stored root == recomputed root    PASS
hot PID unchanged                true
Nginx PID unchanged              true
```

Runtime snapshots bound into the receipt:

```text
hot_health
cold_health
control_health
edge_health
k1_scheduler
ms4
signal_path
```

The default finalizer's retired MS3 acquisition/generation endpoints returned
`503` by design. Finalization therefore used the deployment-specific K1
endpoint set above. The receipt records the exact URLs and snapshot hashes.

## Live K1 Frontier

```text
state                             waiting_for_evidence
blocker                           no_readiness_pass_candidate
next generation                  78
ledger revision                  157
catalog / retained / ready       746 / 256 / 0
topology rows                    49,836
readiness PASS / schedulable      37 / 0
consequences                     scalar 648
                                 record 77
                                 collection 20
                                 rendered_sequence 1
                                 boolean 0
authority / phase mutation       false / false
```

Generation 77 ended with `ACQUISITION_FAIL / selected_role_witness_missing`.
It issued no law, package, execution authority, or phase update.

## Product Economics

From the exact Rust economics snapshot at this checkpoint:

```text
current epoch input tokens        503,961,442
verified avoided input tokens      42,297,506
input-token saving share                 8.3%
verified accepts / avoided calls        185 / 185
verification coverage                    100%
false accepts / parity                    0 / 0
missing receipts / unresolved             0 / 0
dedupe conflicts / pipeline drops         0 / 0
Product M1                               PASS
provider-billed evidence                 unavailable
```

The all-recorded-partitions share was approximately `1.31%`. The natural MS4
package remained `100%` only inside its package-matched denominator; that is
not global coverage.

## Control Page QA

URL:

`http://192.168.3.94:8787/control/0798c80a29c748461e00c846908e11d0cb109e0dfb5c0c6bef2962676e1ddaa0`

```text
desktop viewport                  1440 x 1000
mobile viewport                   390 x 844
horizontal overflow              0 / 0
JavaScript errors                0 / 0
rendered K1 state                WAITING FOR EVIDENCE
rendered frontier readiness      37 / 0
next route                       Law #2/#3 -> K1 OPEN -> natural L2
browser tabs opened by QA        closed
```

Dashboard build: `2026.08.09-b049`.

## Master Plan Structural Review

The rewritten `NANDO_LIVE_CPU_SAVINGS_MASTER_PLAN.md` passed six bounded NANDA
structural gates:

```text
end-to-end route                 PASS
discovery authority             PASS
adaptive frontier               PASS
product and accounting          PASS
durability and runtime          PASS
claim boundaries                PASS
conflicts / evidence gaps       0 / 0
foreign route pulls             0
```

Aggregate plan verdict: `PLAN_STRUCTURE_PASS`. These gates are
`STRUCTURAL_ONLY`; they grant no implementation, law, mechanism, package, or
runtime authority.

## Claim Boundary

This checkpoint proves a continuously live, resource-bounded, operator-blind
K1 discovery loop and already admitted verified CPU execution. It does not
prove:

- that one of the 37 evidence-ready cohorts is currently schedulable;
- Natural Law #2;
- Raw Phase causal necessity;
- K1 vocabulary openness;
- natural L2 composition;
- provider-billed money savings.

The next epistemic action is to partition `37 readiness-PASS / 0 schedulable`
by exact durable exclusion roots, then allow only a genuinely novel queue row
to freeze and traverse the existing identifier.
