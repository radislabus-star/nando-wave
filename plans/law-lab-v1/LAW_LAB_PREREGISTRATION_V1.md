# Law Lab V1 Preregistration

Status: `CONTRACT_FROZEN / RUNTIME_OFF`

Date: `2026-08-08`

Owner: `nando-operator-learning`

Canonical contract:
`plans/law-lab-v1/LAW_LAB_CONTRACT_V1.json`

Contract root:
`7225678eb3eb5f59ab64739104316773dae03755fc1c5ba9883d00e31b3f6bcc`

Canonical artifact SHA-256:
`1a305844fd7799882ff6f9d7abf36deaa7315aaedeee3da679f11024ccac6565`

Canonical artifact size: `4603 bytes`

## 1. Decision

Step 1 freezes the constitution for a bounded experimental lane that may
distinguish competing laws with safe active probes. It does not implement or
start the sandbox executor, open a production endpoint, alter hot serving,
modify Law #1, enter K1, or grant execution authority.

The contract is executable in the narrow sense required at preregistration:

- Rust owns the exact schema and V1 constants;
- canonical bytes derive a content root;
- deserialization rejects unknown, noncanonical, or modified policy bytes;
- normal phase transitions and terminal verdicts are machine-checkable;
- a terminal verdict is bound to the phase that can prove it;
- restart replay must reproduce byte-identical canonical JSON.

The lab runtime itself is a later step and remains absent here.

## 2. Scientific Claim

The lab may test this claim only:

> Given one candidate area grounded in ordinary traffic and a frozen,
> operator-blind semantic version space, a bounded isolated experiment can
> eliminate competing hypotheses through precommitted predictions and an
> independent exact oracle.

The lab cannot prove this stronger claim:

> The surviving hypothesis is a natural production law suitable for K1 or CPU
> authority.

That stronger claim remains owned by a new post-candidate natural holdout and
the existing external certification path.

## 3. Ownership Boundary

`nando-operator-learning` owns this contract because it already owns cold law
identification. The contract pins identification to the existing
`OperatorIdentificationMachineV1`; a parallel lab-only identifier is forbidden.

```text
ordinary traffic residual
-> source-neutral candidate area
-> existing OperatorIdentificationMachineV1
-> exact replay
-> semantic quotient
-> one class or a distinguishing probe
```

The K1 Scheduler may later choose the candidate area. It may not select or hint
the program. Program identity appears only inside the existing identifier after
the version space has been built and quotiented.

## 4. Evidence Separation

Three evidence classes remain disjoint.

```text
NATURAL SUPPORT
ordinary traffic residual
-> binds the candidate area and initial version space

LAB PROBE
isolated generated state/action
-> distinguishes already frozen hypotheses
-> may emit UniqueLawCandidate only

NATURAL HOLDOUT
new post-candidate ordinary event
-> external prediction and verifier
-> may enter LawCertificate path
```

Generated fixtures and teacher outputs cannot seed a candidate. A lab probe
cannot be relabelled as natural support or natural holdout. Replaying a probe in
a second sandbox is still lab evidence, not an independent natural future.

## 5. Frozen Identification Rules

The V1 hypothesis policy requires all of the following:

- `identification_machine = operator_identification_machine_v1`;
- `parallel_identifier_allowed = false`;
- `operator_blind = true`;
- `program_hints_allowed = false`;
- source identity grants no semantic authority;
- exact replay is required;
- semantic quotient is required;
- stable hash is a tie-break only;
- version space freezes before probe selection.

Probe selection uses `maximum_distinguishing_partition`. Every surviving
hypothesis must precommit an exact prediction before execution. The candidate
program cannot serve as its own oracle.

## 6. Executable Lifecycle

Normal transitions are separate from terminalization:

```text
contract_frozen
-> natural_residual_bound
-> version_space_frozen
-> probe_selected
-> predictions_precommitted
-> probe_executed
-> outcome_verified
-> version_space_frozen
```

The loop can repeat only through a newly frozen reduced version space. There is
no direct `probe_selected -> probe_executed` transition. `probe_pending` is not
a terminal verdict and does not release the single generation.

Terminal verdicts:

| Verdict | Earliest admissible proof phase | Meaning |
|---|---|---|
| `unique_law_candidate` | `version_space_frozen` | Exactly one semantic class remains; external natural proof is still required. |
| `no_distinguishing_probe` | `version_space_frozen` | More than one class remains and no safe probe can separate them. |
| `no_identifiable_law` | `version_space_frozen` | No semantic class survives exact evidence. |
| `sandbox_verification_fail` | `probe_selected` | The isolated environment or independent oracle cannot be verified. |
| `budget_exhausted` | `contract_frozen` | A frozen wall/resource/probe bound was reached. |
| `safety_veto` | `contract_frozen` | A hard capability or isolation rule failed. |

`unique_law_candidate` is explicitly rejected from `contract_frozen`,
`natural_residual_bound`, `probe_selected`, `predictions_precommitted`, and
`probe_executed`.

## 7. Frozen Budgets

| Resource | V1 maximum |
|---|---:|
| Active generations | 1 |
| Candidates | 1 |
| Natural support rows | 64 |
| Semantic hypotheses | 32 |
| Probes per generation | 8 |
| Generation wall time | 900,000 ms |
| Probe wall time | 5,000 ms |
| Probe CPU time | 3,000 ms |
| Memory | 512 MiB |
| Disk | 256 MiB |
| Input | 8 MiB |
| Output | 2 MiB |
| Processes | 16 |
| Model calls | 0 |
| Model tokens | 0 |

The zero model budget is deliberate. Step 1 prevents a research loop from
burning prompt tokens while waiting for evidence or generating hypotheses.

## 8. Frozen Safety Envelope

Allowed V1 domains:

- filesystem;
- Git;
- SQLite;
- structured data;
- structured CLI.

Hard rules:

- network disabled;
- no production state mount;
- no production write;
- no secrets;
- no host PID namespace;
- no arbitrary host paths;
- no shell interpretation;
- read-only source snapshot;
- disposable workspace;
- deterministic seed;
- cleanup receipt required.

The contract defines these requirements now. A later sandbox implementation
must prove that it enforces them; it cannot infer looser defaults.

## 9. Authority Boundary

The lab may emit only `UniqueLawCandidate`. It may not:

- issue `LawCertificate`;
- activate a package;
- grant execution authority;
- enter the Epistemic Registry or K1;
- mutate phase memory;
- claim product economics or avoided-upstream credit.

The only authority-bearing continuation is:

```text
UniqueLawCandidate
-> new post-candidate natural holdout
-> durable prediction before outcome
-> independent production verifier
-> existing LawCertificate authority
-> Epistemic Registry
-> K1 accounting
```

This preserves the project route toward K1 operators and later natural L2
composition without letting a laboratory capability test manufacture a law.

## 10. Step 1 Artifacts

| Artifact | Purpose |
|---|---|
| `crates/nando-operator-learning/src/law_lab_contract.rs` | Frozen schema, roots, policies, lifecycle, and authority boundary. |
| `crates/nando-operator-learning/src/law_lab_contract_tests.rs` | Restart, tamper, budget, evidence, authority, and lifecycle tests. |
| `crates/nando-operator-learning/examples/law_lab_contract_v1.rs` | Deterministic canonical artifact generator. |
| `plans/law-lab-v1/LAW_LAB_CONTRACT_V1.json` | Generated canonical V1 bytes. |

Remote verification on the mini-PC:

```text
cargo fmt --all                                      PASS
cargo test -p nando-operator-learning                367 + 1 PASS
cargo clippy -p nando-operator-learning --all-targets -- -D warnings
                                                     PASS
NANDA lab-probe structural route                     PASS / authority false
NANDA natural-holdout structural route               PASS / authority false
```

No service was built, installed, enabled, or restarted by this step.

## 11. Completion Boundary

Step 1 is complete when:

- code and canonical JSON are committed together;
- the JSON root matches the Rust-derived root;
- remote tests and Clippy pass;
- structural evidence routes pass independently;
- immutable proof bytes are installed on the mini-PC;
- production reports `multi_source_research_enabled = false`;
- hot serving and gateway PIDs remain unchanged;
- existing Law #1 remains admitted;
- no Law Lab process, timer, endpoint, or authority exists.

Step 2 may implement the disposable sandbox adapter against this contract. It
may not revise V1 limits or authority rules in place; any change requires a new
schema and a new root.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t01 | ordinary traffic residual | seeds | candidate area | section 4 | 1.0 | natural source | candidate area | identification | natural-identification |
| t02 | generated fixture or teacher output | cannot seed | candidate area | section 4 | 1.0 | excluded source | candidate area | identification | natural-identification |
| t03 | Law Lab contract | delegates identification to | OperatorIdentificationMachineV1 | section 3 | 1.0 | preregistration | identifier | identification | natural-identification |
| t04 | frozen version space | undergoes | exact replay and semantic quotient | section 5 | 1.0 | hypothesis set | proof transform | identification | natural-identification |
| t05 | probe selector | chooses | maximum distinguishing partition | section 5 | 1.0 | selector | probe | lab-probe |
| t06 | surviving hypotheses | precommit before | probe execution | section 6 | 1.0 | predictors | execution | lab-probe | lab-probe |
| t07 | independent oracle | verifies | exact probe outcome | section 5 | 1.0 | verifier | outcome | lab-probe | lab-probe |
| t08 | lab probe | may produce only | UniqueLawCandidate | section 9 | 1.0 | experimental evidence | candidate | promotion | promotion-boundary |
| t09 | lab probe | cannot satisfy | natural holdout | section 4 | 1.0 | experimental evidence | natural evidence | promotion | promotion-boundary |
| t10 | post-candidate natural holdout | may feed | external LawCertificate authority | section 9 | 1.0 | natural evidence | certification | promotion | promotion-boundary |
| t11 | Law Lab | cannot grant | K1 membership or execution authority | section 9 | 1.0 | research owner | production authority | authority | authority-boundary |
| t12 | terminal verdict | is bound to | proving lifecycle phase | section 6 | 1.0 | verdict | phase | lifecycle | lifecycle-boundary |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c01 | ordinary traffic residual | seeds | candidate area | law_lab_contract.rs evidence_policy_v1 | 1.0 | natural source | candidate area | identification | natural-identification |
| c02 | generated fixture or teacher output | cannot seed | candidate area | law_lab_contract.rs evidence_policy_v1 | 1.0 | excluded source | candidate area | identification | natural-identification |
| c03 | Law Lab contract | delegates identification to | OperatorIdentificationMachineV1 | law_lab_contract.rs hypothesis_policy_v1 | 1.0 | preregistration | identifier | identification | natural-identification |
| c04 | frozen version space | undergoes | exact replay and semantic quotient | law_lab_contract.rs hypothesis_policy_v1 | 1.0 | hypothesis set | proof transform | identification | natural-identification |
| c05 | probe selector | chooses | maximum distinguishing partition | law_lab_contract.rs probe_policy_v1 | 1.0 | selector | probe | lab-probe | lab-probe |
| c06 | surviving hypotheses | precommit before | probe execution | law_lab_contract.rs lifecycle_policy_v1 | 1.0 | predictors | execution | lab-probe | lab-probe |
| c07 | independent oracle | verifies | exact probe outcome | law_lab_contract.rs probe_policy_v1 | 1.0 | verifier | outcome | lab-probe | lab-probe |
| c08 | lab probe | may produce only | UniqueLawCandidate | law_lab_contract.rs authority_boundary_v1 | 1.0 | experimental evidence | candidate | promotion | promotion-boundary |
| c09 | lab probe | cannot satisfy | natural holdout | law_lab_contract.rs evidence_policy_v1 | 1.0 | experimental evidence | natural evidence | promotion | promotion-boundary |
| c10 | post-candidate natural holdout | may feed | external LawCertificate authority | law_lab_contract.rs authority_boundary_v1 | 1.0 | natural evidence | certification | promotion | promotion-boundary |
| c11 | Law Lab | cannot grant | K1 membership or execution authority | law_lab_contract.rs authority_boundary_v1 | 1.0 | research owner | production authority | authority | authority-boundary |
| c12 | terminal verdict | is bound to | proving lifecycle phase | law_lab_contract.rs terminal_policy_v1 | 1.0 | verdict | phase | lifecycle | lifecycle-boundary |
