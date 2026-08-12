# S1C Transactional Deployment Preregistration V3 Critique

Status: `ADVERSARIAL REVIEW / ACCEPTED / NO DEPLOYMENT`

Date: `2026-08-12 Europe/Tallinn`

## Findings

| Severity | Finding | Failure mode | Accepted repair |
|---|---|---|---|
| P0 | Chowning only `Cargo.toml` does not let Cargo create `Cargo.lock`. | Parent directory remains root-owned and unwritable. | Bind owner and mode for workspace and src directories, then prove a create/fsync/unlink as user e. |
| P0 | Running `test -w` as root would always look writable. | The check would not represent the Cargo user. | Run the exclusive write probe through the same `sudo -u e` boundary as Cargo. |
| P0 | Reusing the successfully built V2 harnesses would save time. | V3 would inherit unbound state from a spent attempt. | Use fresh checkout and targets; no V2 artifact reuse. |
| P0 | A probe could remain and alter the oracle package. | Cargo input and workspace identity would drift. | Require unlink plus directory fsync before build; retained probe is VETO. |
| P1 | Recursive permissive mode such as 0777 would make the build work. | It broadens write authority beyond the build owner. | Exact e:e ownership, directory 0750, file 0640. |
| P1 | Ownership could be fixed after the Cargo failure and retried in one attempt. | That is an adaptive retry after observing a result. | Verify ownership before the first Cargo oracle command; any failure terminates V3. |
| P1 | Permission repair could be described as latency evidence. | Build success would be promoted into a resource claim. | Quiescence and every metric remain downstream and unchanged. |

## Rejected Alternatives

```text
retry V2
  rejected: V2 is terminal

run oracle Cargo as root
  rejected: changes the frozen build user and ownership denominator

chmod 0777
  rejected: unnecessary authority expansion

reuse V2 target
  rejected: stale attempt state

skip parity oracle
  rejected: removes the ordinary-output parity gate
```

## Verdict

```text
delta limited to oracle workspace ownership    yes
candidate and thresholds unchanged             yes
non-root write proven before Cargo              required
fresh V3 artifacts                              required
V3 remote attempts                              one
production changed by paper                     no
scientific authority                            false
ready for structural gate                       yes
```

