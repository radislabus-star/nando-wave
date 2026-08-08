# Law Lab Sandbox Adapter V1

Status: `IMPLEMENTED / CAPABILITY SELF-TEST ONLY / RUNTIME OFF`

Date: `2026-08-08`

Owner: `nando-operator-learning`

Frozen parent contract:
`plans/law-lab-v1/LAW_LAB_CONTRACT_V1.json`

Parent contract root:
`7225678eb3eb5f59ab64739104316773dae03755fc1c5ba9883d00e31b3f6bcc`

## 1. Decision

Step 2 implements one disposable executor adapter for already frozen Law Lab
probes. It does not select a cohort, synthesize a program, create prediction
commitments, reduce the version space, issue a certificate, or serve traffic.

```text
frozen natural candidate
-> existing OperatorIdentificationMachineV1
-> frozen semantic version space
-> external durable prediction-ledger root
-> exact executor manifest
-> typed disposable sandbox
-> exact isolated outcome receipt
-> existing identifier
-> new natural holdout before any authority
```

The existing `OperatorIdentificationMachineV1` remains the sole identifier.
The sandbox is an execution adapter and exact oracle surface, not a second
identifier.

## 2. Implemented Scope

The frozen V1 contract permits five domains. Step 2 verifies only two:

| Contract domain | Step 2 backend | Typed operations |
|---|---|---|
| `filesystem` | `VERIFIED` | copy one immutable source file; remove one work path |
| `structured_data` | `VERIFIED` | canonicalize integer/string/bool/null JSON |
| `git` | `UNIMPLEMENTED / FAIL-CLOSED` | none |
| `sqlite` | `UNIMPLEMENTED / FAIL-CLOSED` | none |
| `structured_cli` | `UNIMPLEMENTED / FAIL-CLOSED` | none |

Contract permission is not implementation proof. Requests for an unimplemented
domain return `law_lab_sandbox_domain_unsupported` before workspace creation.

There is no arbitrary command string and no shell operation in the protocol.
All paths are relative UTF-8 paths. Absolute paths, `..`, `.`, empty path
components, backslashes, symlinks, overlapping mutations, oversized trees, and
unknown filesystem kinds fail closed.

## 3. Freeze Binding

Every request binds all of the following before execution:

- exact frozen parent contract root;
- exact executor-manifest root;
- trusted worker SHA-256;
- natural candidate root;
- frozen version-space root;
- external durable prediction-ledger root;
- distinguishing-probe root;
- immutable source-tree root;
- deterministic-seed root;
- surviving-hypothesis count;
- equal precommitted-prediction count;
- typed operation list.

The request phase is fixed to `predictions_precommitted`. A zero root, zero
hypotheses, missing prediction, foreign executor root, or modified request root
is rejected. The adapter receives a ledger root but has no ledger writer and
cannot mint or repair predictions.

## 4. Executor Manifest

`LawLabSandboxExecutorManifestV1` is calculated before request freeze and binds:

- SHA-256 of `/usr/bin/bwrap`;
- SHA-256 of `/usr/bin/prlimit`;
- SHA-256 and host path of the worker;
- canonical source-store and workspace-store host paths;
- content-addressed worker-path policy;
- root-ownership policy;
- root-owned source-snapshot policy;
- generated-capability-only policy when applicable;
- exact read-only runtime binds;
- deterministic environment;
- supported backend subset;
- all effective resource limits;
- network, shell, production-mount, source-write, and cleanup policy.

Strict mode requires a root-owned, non-group-writable executable installed at:

```text
.../<worker_sha256>/nando-law-lab-sandbox-worker
```

Every ancestor of the installed worker path must be traversable by the adapter
after capability drop. The deployment path is therefore a dedicated root-owned
executable hierarchy such as `/usr/libexec/nando-wave/law-lab/...`, not the
service account's mode-0700 home or state directory.

Changing a binary, timeout, supported backend, trust policy, or mount list
changes the executor root. It cannot be substituted behind an already frozen
request.

## 5. Isolation Route

The parent launches `/usr/bin/bwrap` directly with:

- `--unshare-all`;
- `--die-with-parent`;
- `--new-session`;
- `--cap-drop ALL`;
- `--clearenv`;
- a new PID, network, mount, IPC, UTS, cgroup, and user namespace;
- read-only `/usr`, library paths, exact worker, and exact `/source` snapshot;
- writable disposable `/work` only;
- fresh `/proc`, `/dev`, and tmpfs `/tmp`;
- no `/home`, `/root`, `/etc`, `/run`, secrets, or production state.

Inside the new user/PID namespace, `prlimit` applies:

| Limit | Value |
|---|---:|
| CPU | 3 seconds |
| Address space | 512 MiB |
| Processes | 16 |
| File size | 2 MiB |

The parent enforces the 5-second wall deadline. `RLIMIT_NPROC` is deliberately
applied inside the new namespace: applying 16 processes outside would count all
processes owned by the service account and prevent namespace creation instead
of limiting the probe.

The trusted worker additionally constrains total input to 8 MiB, typed output
to 2 MiB, tree entries to 1,024, operations to 64, and effective disposable
disk use below the frozen 256 MiB maximum.

## 6. Runtime Attestation

Before applying an operation, the worker proves:

- zero non-loopback IPv4 routes;
- zero non-loopback IPv6 routes;
- at most two visible PIDs in the isolated namespace;
- `NoNewPrivs: 1`;
- a real write attempt against `/source` is blocked;
- forbidden host paths are absent;
- environment equals exactly `LANG=C`, `LC_ALL=C`, `PATH=/usr/bin`,
  `PWD=/work`, and `TZ=UTC`.

Linux creates three kernel-local IPv6 entries for `lo` in an empty network
namespace. The attestation records usable non-loopback routes, which remain
zero; it does not misreport those loopback entries as external connectivity.

## 7. Exact Outcome And Independent Check

The worker:

1. scans and hashes the read-only source tree;
2. clones it into `/work`;
3. proves pre-work root equals source root;
4. executes typed operations only;
5. records each operation and effect root;
6. scans the exact post-work tree;
7. emits canonical JSON bounded to 2 MiB.

The parent does not trust the worker receipt by itself. It independently:

- re-hashes the source after execution;
- re-scans the host-side work directory;
- verifies copied bytes against immutable source bytes;
- verifies removed paths and descendants are absent;
- reparses JSON and requires byte-exact canonical output;
- recomputes every operation/effect root;
- checks worker SHA, exact-outcome root, and attestation root.

Any mismatch is `sandbox_verification_fail` evidence for the external
lifecycle; it is never a guessed outcome.

## 8. Cleanup And Failure

Each run gets a newly created mode-0700 workspace under a dedicated store.
Success, process failure, worker failure, and wall timeout all traverse the same
cleanup guard. A successful receipt is created only after `remove_dir_all` and
an independent absence check. The receipt binds the cleanup proof root.

The generated capability runner also removes its source fixtures and outer
scratch directory before sealing its report.

## 9. Authority Boundary

Every execution receipt fixes all of these fields to `false`:

- `prediction_commitments_written`;
- `natural_holdout_satisfied`;
- `law_certificate_issued`;
- `execution_authority_granted`;
- `package_activated`;
- `k1_registry_mutated`;
- `phase_memory_mutated`;
- `economics_credit_granted`.

Capability self-tests are explicitly `generated_capability_self_test`, cannot
seed a natural candidate, and list the three unimplemented backends. Tampering
an authority bit invalidates the receipt.

The only scientific continuation remains:

```text
real traffic residual
-> existing identifier
-> frozen hypotheses and predictions
-> isolated distinguishing outcome
-> reduced version space / UniqueLawCandidate
-> independent post-candidate natural traffic
-> external LawCertificate
-> Epistemic Registry
-> K1
-> later natural L2 composition
```

## 10. Artifacts

| Artifact | Ownership |
|---|---|
| `src/law_lab_sandbox/model.rs` | Requests, receipts, capability report, and authority invariants. |
| `src/law_lab_sandbox/manifest.rs` | Content-addressed trees and executor manifest. |
| `src/law_lab_sandbox/adapter.rs` | Trust checks, namespace command, parent verifier, timeout, cleanup. |
| `src/law_lab_sandbox/worker.rs` | Typed transformations and in-namespace attestation. |
| `src/bin/nando-law-lab-sandbox-worker.rs` | Direct trusted worker entrypoint. |
| `tests/law_lab_sandbox_v1.rs` | Validation, tamper, isolation, exact outcome, timeout, and cleanup tests. |
| `examples/law_lab_sandbox_capability_v1.rs` | Generated-only two-backend proof runner. |

## 11. Non-Effects

Step 2 does not:

- start a daemon, timer, watcher, endpoint, or dashboard route;
- enable `multi_source_research_enabled`;
- restart hot serving, gateway, connector, or Nginx;
- touch Law #1, K1 accounting, admission, phase memory, or economics;
- execute `git`, SQLite, a CLI, a shell, or production traffic;
- claim a new law or a `UniqueLawCandidate`.

Step 3 may connect one unresolved real-traffic version space to this adapter.
That future connection must keep the existing identifier and external natural
holdout boundary intact.
