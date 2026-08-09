# Raw Phase Executable Frontier Live Checkpoint

Status: implemented, verified, and deployed; waiting for natural evidence.

Date: 2026-08-10.

## Result

```text
natural structural frontier
-> bounded Raw Phase executable hypotheses       PASS
-> existing OperatorIdentificationMachineV1      PASS
-> selected executable receipt binding           PASS
-> independent actor reconstruction               PASS
-> verified CPU-before-upstream route             READY

natural Law #2                                    NOT PROVED
Raw Phase causal law                              NOT PROVED
scheduler authority / phase mutation              false / false
active false accepts / runtime parity              0 / 0
```

The learner can now turn a future frozen natural cohort into bounded executable
Raw Phase envelopes. The selected receipt binds the candidate freeze, program,
support evidence, actor commitment, and verifier commitment. The response actor
reconstructs the typed operator from durable evidence instead of trusting
caller-provided anchors.

The implementation does not add a second identifier, create synthetic evidence,
use future action data at runtime, lower evidence thresholds, or grant authority.

## Source

```text
branch             raw-phase-frontier-v2-20260809
deployed commit    a022ade7c5fc2c8c98484138035aaa3c979cc106
source tree        32de1dfe093566b48bec479ca44f50b003ccd863
rollback commit    aa13b75cc3478a187591961cb02509f8c12a1cb7

transition SHA     4cf739b165030e73210d5a8d49be0b0c38ba252788c1b48b8e89ad8f43617703
control SHA        68aa9597400e47b2818494a42d4da11cdb14ff4ab6da61611bfd8f29595ea89c
```

`HEAD`, the feature branch, and `origin/main` pointed to the same commit before
deployment. The release was built in the clean remote worktree
`/home/e/projects/nando-wave-release-a022ade`; heavy compilation and tests ran
only on the mini-PC.

## Verification

```text
operator-learning                              383 / 383 PASS
response-actor                                 376 / 376 PASS
transition-serving                             272 PASS / 7 ignored
gateway-control                                7 + 49 PASS
focused Raw Phase route                        4 / 4 PASS
rustfmt                                        PASS
strict Clippy for changed crates               PASS
live composite gate                            PASS
eligible for existing local accept             true
M3 complete                                    false
```

Negative coverage includes selected-receipt root, disposition, fingerprint,
capture-sequence, missing-future-evidence, and caller-anchor tampering. The
legacy non-Raw identifier path remains covered and unchanged.

## Live Resource Gate

The cold learner performed one startup reconstruction, then returned to idle
without throttling:

```text
RSS limit                                      < 2.5 GiB
HWM limit                                      < 3.5 GiB
CPU limit                                      <= 25% for 4 samples

gate RSS                                       1,582,620 KiB
post-deploy RSS                                1,618,460 KiB
post-deploy HWM                                2,022,864 KiB
final CPU samples                              0.50 / 0.50 / 0.00 / 15.00%
qualifying streak                              4
```

## Live Services

```text
cold learner PID                               3163368
hot serving PID                                3901227 unchanged
gateway-control PID                            3164094
certification authority PID                    2576117 unchanged
Nginx PID                                      682430 unchanged
local connector PID                            2919 unchanged
all relevant NRestarts                         0
edge 502 / 504 after deployment                0
```

The hot serving process, Nginx, and local connector were not restarted.

## Immutable Deployment Receipt

```text
path
/var/lib/nando-wave/deployments/20260809T225925Z-a022ade7c5fc/deployment-receipt.json

receipt root
dcbda65f80c8568b55a3a1c12857ee956ac2a6f010af4ce28e0e99107df0286b

file mode                                      0400
stored root == recomputed root                 PASS
candidate/install SHA parity                   PASS
transactional rollback snapshot                PASS
hot PID unchanged                              true
Nginx PID unchanged                            true
```

The receipt binds the source commit and tree, installed artifacts, units,
durable state manifest, K1 runtime snapshot, MS4 snapshot, health endpoints,
and rollback source.

## Live K1 Boundary

```text
state                                           waiting_for_evidence
blocker                                         no_readiness_pass_candidate
next generation                                 78
ledger revision                                 157
ledger root                                     4066192c...6576a87
restart parity                                  byte-identical root
catalog / retained / ready                      746 / 256 / 0
future eligible rows                            0
selected executable envelope                    NONE
authority / phase mutation                      false / false
```

With no active natural freeze, the dashboard correctly renders:

```text
EXECUTABLE BUILDER READY · WAITING FOR FROZEN GENERATION
```

The next readiness-PASS natural generation will be handled automatically by
the existing scheduler and identifier. Only a later independent verified
outcome may create a law certificate.

## Exact Product Economics

These product denominators remain separate from the scientific Raw Phase claim:

```text
all recorded history       156,302,125 / 7,381,270,879   2.12%
current V4 epoch           113,786,828 / 1,419,490,155   8.02%
natural MS4 package        742,779,418 /   742,779,418 100.00%
natural package accepts / avoided calls                 3,038 / 3,038
false accepts / runtime parity                          0 / 0
```

Routing and avoided upstream calls prove execution economics. They do not by
themselves prove response quality outside the independent verifier coverage.

## Control Page QA

URL:

`http://192.168.3.94:8787/control/0798c80a29c748461e00c846908e11d0cb109e0dfb5c0c6bef2962676e1ddaa0`

```text
desktop / laptop / tablet / mobile / narrow    PASS
desktop horizontal overflow                    false
mobile horizontal overflow                     false
Raw Phase builder marker                       visible
console errors / page errors                   0 / 0
browser sessions opened by QA                  closed
```

## Claim Boundary

This checkpoint proves that natural frontier evidence can enter an executable,
authority-bound Raw Phase identification path and later reach verified CPU
execution without caller-controlled anchors. It does not prove that current
traffic contains the required natural cohort, Law #2, Raw Phase causal
necessity, K1 openness, natural L2 composition, or provider-billed money
savings.
