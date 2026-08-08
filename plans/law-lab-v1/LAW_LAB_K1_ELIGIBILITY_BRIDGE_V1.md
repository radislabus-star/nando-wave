# Law Lab K1 Eligibility Bridge V1

Status: `IMPLEMENTED / LIVE / RUNTIME OFF`

Date: `2026-08-08`

Owner: `nando-transition-serving`

Frozen parent contract:
`plans/law-lab-v1/LAW_LAB_CONTRACT_V1.json`

Parent contract root:
`7225678eb3eb5f59ab64739104316773dae03755fc1c5ba9883d00e31b3f6bcc`

## 1. Decision

Step 3A adds a read-only eligibility bridge between the signed epistemic K1
scheduler projection and the Step 2 Law Lab sandbox adapter. It does not start
the sandbox, enable research, create a candidate, write a prediction, reduce a
version space, or issue a certificate.

```text
signed epistemic scheduler ledger
-> verified K1 projection
-> active natural candidate
-> frozen ambiguous version space
-> durable K1 PROBE_PENDING receipt
-> typed active-probe plan root
-> exact executor manifest attestation
-> READY_FOR_SANDBOX_EXECUTION
```

Every missing link produces a stable blocker. There is no default probe and no
fallback to a generated fixture.

## 2. Current Live Boundary

The production scheduler is intentionally closed and its latest epistemic
generation is terminal. The deployed endpoint restores the signed projection
and finds no active candidate freeze. Its live verdict is:

```text
state       no_eligible_law_lab_probe
blocker     no_active_candidate_freeze
research    false
execute     false
authority   false
```

This is a waiting observation, not a Law Lab terminal verdict. No generation
exists for the lab to release or replace.

The deployment-bound snapshot records:

```text
scheduler ledger revision       91
scheduler ledger root           1646ee00...dcc826f
scheduler projection root       ae2f9125...e2fb9f1
latest scheduler event root     96b8e276...a98d3
latest terminal verdict root    21e09725...d2451
latest terminal blocker         all_supported_t1_protocol_modes_already_active
active candidate freeze         NONE
K1 laws / semantics / topology  1/3 / 1/3 / 1/2
```

All eight authority fields are `false`. Law Lab did not write a prediction,
issue a certificate, mutate K1 or phase memory, activate a package, or receive
economics credit.

## 3. Durable Prediction Boundary

The bridge accepts an active probe only when the existing K1 scheduler already
contains a durable `PROBE_PENDING` receipt for every surviving semantic class.
The sandbox request must bind its `durable_prediction_ledger_root_sha256` to
the exact signed projection root after that receipt was appended.

The bridge has no scheduler writer. It cannot create, repair, or move a
prediction commitment.

## 4. Active Probe Plan Root

For active probes, a caller-supplied `probe_root` is insufficient. V1 derives
the root from this exact preregistered plan:

```text
parent contract root
+ natural candidate root
+ frozen version-space root
+ immutable source-tree root
+ deterministic seed root
+ typed domain
+ surviving-hypothesis count
+ outcome contract: post_work_tree_root_sha256
+ ordered typed operations
```

The derived root must equal both the sandbox request probe root and the K1
`selected_probe_root_sha256`. Therefore source state or operations cannot be
chosen after predictions were durably committed.

## 5. Executor Boundary

A bound request is still not executable until the Step 2 executor manifest is
independently validated. V1 requires exact equality for:

- executor-manifest root;
- worker SHA-256;
- supported typed domain.

Without this attestation the state is
`awaiting_executor_attestation`, never `ready_for_sandbox_execution`.

## 6. Read-Only Endpoint

```text
GET /v2/multi-source/k1-law-lab-eligibility
```

The endpoint restores the externally anchored epistemic scheduler projection
on demand. It remains available while
`NANDO_MULTI_SOURCE_RESEARCH_ENABLED=0`, but it performs no write and starts no
timer, watcher, worker, or sandbox process.

The control dashboard consumes this endpoint through its existing read-only
snapshot route. It exposes the blocker, signed scheduler roots, probe bindings,
executor attestation, research policy, and all eight authority bits. With no
active candidate it renders `NO ELIGIBLE LAW LAB PROBE` as a waiting state,
never as success.

## 7. Authority Boundary

Every report embeds the Step 2 authority-free boundary:

- no LawCertificate;
- no package activation;
- no execution authority;
- no K1 registry mutation;
- no phase-memory mutation;
- no economics credit;
- no natural-holdout satisfaction.

`sandbox_execution_allowed` is only an isolated experiment policy bit. It can
be true only when every eligibility condition passes and research policy is
enabled. It is not production CPU authority.

## 8. Verification

Remote-only verification on the mini-PC:

```text
focused eligibility tests       5/5 PASS
transition-serving unit suite   263 PASS / 7 ignored
Clippy -D warnings              PASS
rustfmt                         PASS
structural gate                 PASS / authority false
gateway-control tests           56/56 PASS
gateway-control Clippy          PASS
```

No local build or test was run.

## 9. Live Deployment

The cold learner alone was restarted. Hot serving, Nginx, gateway control, and
certification authority retained their PIDs across deployment and a separate
15-second survival check; every observed `NRestarts` value remained zero.

```text
source commit          e1ad918681aed21f696d177f720cb272db6c84a3
installed binary SHA   bab7353548ad53811ea4e29e49909e92f960171c1f87ae8c3640e9e8c9078d47
cold learner PID       1414858
hot serving PID        3901227
gateway control PID    3751169
authority PID          4138903
Nginx PID              682430
```

Durable deployment receipt:

```text
/var/lib/nando-wave/deployments/20260808T211638Z-e1ad918681ae/deployment-receipt.json
root  5afdc44a93c79c000de04b19eff25ebf1b0e2cadb281c2cf01496d5539048940
mode  0400
```

The receipt binds the eligibility snapshot SHA-256
`52aed7ea2fd010ff9818208290764556b64844434284029985f7cddd3e7a9649`.
That snapshot has report root
`af0b42120af0091c986dd171e9c64dbf12452fe7f53cf48799b8bfee1d70b7fb`
and the exact `no_eligible_law_lab_probe / no_active_candidate_freeze`
verdict. The receipt root was independently recomputed from its canonical JSON
payload after all deployment writers exited.

## 10. Next Trigger

The next code path may execute only after a new ordinary-traffic candidate
reaches a frozen version space with more than one semantic class. At that
point the existing identifier must emit a typed active-probe plan whose exact
post-work tree predictions are committed to the existing K1 ledger before the
sandbox starts.

If no such generation exists, this report remains
`no_eligible_law_lab_probe`; synthetic traffic is not created to change it.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t02 | ordinary traffic candidate | must precede | Law Lab eligibility | section 1 | 1.0 | natural evidence | experiment gate | eligibility | eligibility |
| t03 | frozen ambiguous version space | requires before | active probe selection | section 1 | 1.0 | hypothesis set | experiment plan | eligibility | eligibility |
| t04 | K1 PROBE_PENDING receipt | durably precommits | every class prediction | section 3 | 1.0 | signed prediction ledger | hypothesis predictions | precommit | precommit |
| t05 | active-probe plan root | commits before execution | source tree and typed operations | section 4 | 1.0 | immutable probe plan | isolated action | precommit | precommit |
| t06 | exact executor manifest | attests | worker hash and supported domain | section 5 | 1.0 | isolation authority | sandbox executable | sandbox | sandbox |
| t07 | eligibility endpoint | reads without writing | epistemic projection | sections 6-7 | 1.0 | observation interface | signed state | observation | observation |
| t08 | eligibility report | cannot grant | production execution authority | section 7 | 1.0 | diagnostic evidence | product authority | authority | authority |
| t09 | generated fixture | cannot create | natural candidate | section 9 | 1.0 | excluded evidence | candidate source | evidence | evidence |
| t10 | post-work tree root | is exact oracle for | active probe outcome partition | section 4 | 1.0 | independent outcome | semantic partition | outcome | outcome |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c02 | ordinary traffic candidate | must precede | Law Lab eligibility | law_lab_eligibility.rs no_active_candidate_freeze | 1.0 | natural evidence | experiment gate | eligibility | eligibility |
| c03 | frozen ambiguous version space | requires before | active probe selection | law_lab_eligibility.rs semantic_class_count | 1.0 | hypothesis set | experiment plan | eligibility | eligibility |
| c04 | K1 PROBE_PENDING receipt | durably precommits | every class prediction | law_lab_eligibility.rs pending_probe | 1.0 | signed prediction ledger | hypothesis predictions | precommit | precommit |
| c05 | active-probe plan root | commits before execution | source tree and typed operations | law_lab_eligibility.rs active_probe_plan_root | 1.0 | immutable probe plan | isolated action | precommit | precommit |
| c06 | exact executor manifest | attests | worker hash and supported domain | law_lab_eligibility.rs executor_manifest validation | 1.0 | isolation authority | sandbox executable | sandbox | sandbox |
| c07 | eligibility endpoint | reads without writing | epistemic projection | service.rs handler and main.rs/live_dashboard.rs read-only snapshot | 1.0 | observation interface | signed state | observation | observation |
| c08 | eligibility report | cannot grant | production execution authority | LawLabSandboxAuthorityBoundaryV1 | 1.0 | diagnostic evidence | product authority | authority | authority |
| c09 | generated fixture | cannot create | natural candidate | no candidate writer in bridge | 1.0 | excluded evidence | candidate source | evidence | evidence |
| c10 | post-work tree root | is exact oracle for | active probe outcome partition | LAW_LAB_ACTIVE_PROBE_OUTCOME_CONTRACT_V1 | 1.0 | independent outcome | semantic partition | outcome | outcome |
