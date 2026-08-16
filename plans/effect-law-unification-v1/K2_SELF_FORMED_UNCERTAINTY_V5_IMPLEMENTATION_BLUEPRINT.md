# K2 Self-Formed Uncertainty V5 Implementation Blueprint

Status: `PREFLIGHT INPUT / NO CODE OR NONCE AUTHORITY`

Date: `2026-08-16`

## Route

```text
unchanged Development wire behavior
-> separate Confirm schemas
-> one-shot authorization and nonce journal
-> public all-case preparation
-> sealed private execution
-> independent oracle controls and terminal
-> classified cleanup
-> DevelopmentRehearsal
-> successor R9B freeze
-> STOP before exact-root authorization
```

Production serving, natural traffic, K1, Wave phase memory, dashboard, connector
and every deployed service remain outside this route.

## Dependency Direction

```text
thin binaries
  -> process adapters
     -> owner-specific application modules
        -> pure V2-V5 domain models and state machines
           -> canonical hashing and existing bounded filesystem semantics

proof modules
  -> canonical schemas and immutable receipts only
  -/> planner implementation
  -/> generator construction helpers
  -/> production code
```

No module named `utils`, `helpers` or `common` may collect mixed authority.

## Existing Behavior Boundary

The following Development wire types and canonical bytes remain unchanged:

```text
K2UncertaintyGeneratorRequestV1
K2UncertaintyGeneratorResponseV1
public batch root
private batch root
private denominator root
all case roots transitively bound by those roots
```

`generator_model.rs` may broaden split-aware validation only where the existing
wire object already carries `split`; it may not add, remove or reorder a
Development field. Confirm request and response are new closed types in a new
module. A parity test reconstructs both historical inner roots and the exact
successor identity-only outer-root rebinding.

## Source Ownership

### R7G Generator

```text
confirm_generator_model.rs
  Confirm request response and split receipts only

generator.rs
  preserve Development entrypoint
  extract one shared deterministic inner generator
  dispatch by exact schema to Development or Confirm

generator_model.rs vocabulary.rs support.rs
  minimal split-aware validation changes only

closure_planner_process.rs
  closed process request response around existing pure closure ranking
```

### R7H Authorization And Journal

```text
confirm_authorization.rs
  exact-root authorization receipt
  global slot key
  append-only slot ledger

confirm_attempt_model.rs
  descriptor events terminal states classified paths

confirm_attempt_journal.rs
  crash-atomic append restart projection fault injection

confirm_artifacts.rs
  public denominator receipt
  per-case resolver and final-truth files
  atomic split publication

confirm_owner.rs
  DevelopmentRehearsal and sealed outer orchestration
  CSPRNG only in sealed mode after a claimed slot
  anonymous generator pipe and irreversible dispatch marker
```

### R7I Public And Private Process Topology

```text
confirm_public_coordinator.rs
  public preparation and ALL_CASES_PRECOMMITTED only

confirm_private_resolver.rs
  one case plus one frozen ordinal to one action/effect receipt

confirm_sandbox.rs
  frozen bwrap argv mount matrix cleared environment and limits

existing learner probe selector safety worker observer final-verifier binaries
  reused only through their closed process schemas and manifest hashes
```

The public coordinator exits before any resolver mount. The confirm owner then
supervises only frozen path and receipt transport; it does not parse private
mapping or final-truth bytes.

### R7J Independent Evaluation

```text
confirm_oracle_baseline.rs
  complete one-or-two-probe oracle and four frozen baseline aggregates
  no closure or final-elimination implementation dependency

confirm_controls.rs
  static receipt verification
  rehearsal K1-K12
  sealed attempt-bound K1-K12

confirm_terminal.rs
  separate rehearsal and sealed request schemas
  conjunctive terminal predicates only
```

### R7K Cleanup And Result

```text
confirm_cleanup_authority.rs
  frozen verdict plus classified-manifest authorization only

confirm_cleanup.rs
  deletion owner and read-only verifier paths kept as separate functions and
  separate binaries

confirm_result.rs
  joins frozen scientific verdict and CLEANUP_FROZEN receipt only
```

## Executable Ownership

Existing manifest-bound binaries remain distinct. Add these thin wrappers:

```text
nando-k2-self-formed-authorization-slot
nando-k2-self-formed-confirm-owner
nando-k2-self-formed-public-coordinator
nando-k2-self-formed-closure-planner
nando-k2-self-formed-private-resolver
nando-k2-self-formed-oracle-baseline
nando-k2-self-formed-control-evaluator
nando-k2-self-formed-terminal-evaluator
nando-k2-self-formed-cleanup-authorizer
nando-k2-self-formed-cleanup-owner
nando-k2-self-formed-cleanup-verifier
nando-k2-self-formed-result-publisher
```

Each wrapper calls one public process entrypoint and has a unique executable
SHA-256. Cargo test binaries cannot substitute for any entry.

## Spectral Budget

```text
new mixed-authority modules                 0 allowed
new owner-specific domain or process files <= 12 planned
new thin binary wrappers                   12 planned
target production module size             <= 700 lines each
target thin wrapper size                   <= 20 lines each
new production service routes               0
network callsites                            0
natural traffic reads or writes              0
K1 or phase mutations                        0
sealed attempts through R9B                  0
```

A file may exceed 700 lines only after a recorded spectral review proves that
splitting would duplicate a state machine or canonical serializer. No such
exception is preregistered.

## Fault And Parity Map

Every mutating transition receives before-write, after-write-before-event and
after-event restart tests where applicable. Mandatory parity families:

```text
Development inner-root and identity-rebinding parity
authorization slot global uniqueness
nonce-created-uncommitted projection
generator-dispatched no-replay projection
split-artifact atomic publication
all-case precommit before private mount
resolver closed-schema and safety binding
worker observer exact parity
one-or-two-probe oracle independence
static rehearsal attempt-control denominator separation
sealed versus rehearsal terminal separation
classified cleanup and retained-file parity
result publication requires both frozen receipts
executable manifest completeness and self-hash checks
```

## Build And Test Route

All heavy builds and tests run only on `e@192.168.3.94` with the remote checkout
`/home/e/build/nando-wave-k2-self-formed-r2`, target directory
`/home/e/.cache/nando-wave-k2-active-inquiry-target` and `-j 20`.

The local Entire-tracked worktree owns edits and commits. No production deploy
or service restart belongs to this experiment.

## Stop Boundary

R9B freezes exact source, executable, test, DevelopmentRehearsal and cleanup
roots while recording `sealed_attempts = 0` and no Confirm nonce. The process
then stops. A later user message must name the exact successor freeze root and
the V2-V5 contract before the slot owner can accept one claim.
