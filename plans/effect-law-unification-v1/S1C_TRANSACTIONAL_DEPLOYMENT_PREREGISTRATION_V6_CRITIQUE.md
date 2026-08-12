# S1C Transactional Deployment Preregistration V6 Critique

Status: `ADVERSARIAL REVIEW / ACCEPTED WITH REPAIRS / PAPER VERIFIED`

Date: `2026-08-12 Europe/Tallinn`

## Findings

| Severity | Finding | Failure mode | Accepted repair |
|---|---|---|---|
| P0 | A generated oracle without a frozen lock can contact crates.io. | DNS or registry state decides proof availability and may change dependency resolution between baseline and candidate. | Freeze one checked-in oracle lock and require `--offline --locked` plus `CARGO_NET_OFFLINE=true` for both builds. |
| P0 | Baseline and candidate previously had different package names. | One generated lock could not be copied to both crates byte-for-byte. | Use one package identity, `s1c3-parity-oracle`, in both fresh manifests. |
| P0 | An offline failure could silently fall back online. | V6 could become an adaptive retry of the dependency route. | Treat missing cache closure as terminal; forbid online fallback and a second V6 transaction. |
| P0 | A copied lock could mutate during path resolution. | Receipt would cite a preregistered hash while Cargo used a changed dependency graph. | Hash each copied lock before and after build and bind both identities into the ownership receipt. |
| P0 | A V5-built baseline oracle could be reused. | The V6 comparison would mix terminal-attempt artifacts with fresh candidate evidence. | Require fresh workspaces, targets, binaries, ownership rows, and executable identities for both sides. |
| P1 | Existing Cargo cache is not itself scientific evidence. | Cache presence could be confused with parity or runtime correctness. | Treat cache as build input only; exact lock plus functional parity and executable roots own the result. |
| P1 | Offline diagnostic binaries could be promoted into V6. | A successful rehearsal would bypass the one-attempt freshness contract. | Record hashes for diagnosis only and explicitly reject their paths as V6 executable identities. |
| P1 | Fixing preflight could be called grounded meaning. | Operational installation would be promoted into a scientific result. | Keep S1C-4, grounded meaning, K2, training, phase mutation, and dashboard claims closed. |

## Rejected Alternatives

```text
retry V5 after DNS recovers
  rejected: V5 is terminal and still network-dependent

vendor the entire workspace for V6
  rejected: much larger change than the exact blocker requires

reuse the successful V5 baseline oracle
  rejected: violates fresh-attempt parity symmetry

allow Cargo online only when cache is missing
  rejected: adaptive dependency route and non-reproducible authority

change latency or quiescence thresholds
  rejected: unrelated to dependency closure
```

## Verdict

```text
runtime candidate changed                       no
resource thresholds changed                     no
production affinity changed                     no
oracle package identity unified                 yes
oracle lock frozen                              yes
offline locked diagnostic                       PASS for both sides
network fallback                                forbidden
fresh V6 artifacts                              required
V6 remote attempts                              one after final freeze
scientific authority                            false
ready for structural gate                       yes
```
