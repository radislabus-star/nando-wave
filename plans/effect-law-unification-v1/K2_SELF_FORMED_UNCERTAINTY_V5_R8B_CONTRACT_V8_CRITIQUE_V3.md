# K2 Self-Formed Uncertainty V5 R8B Contract V8 Critique V3

Status: `CRITIQUE COMPLETE / MANAGER IDENTITY REPAIRED / STRUCTURAL GATES NEXT`

Date: `2026-08-21`

## Verdict

Critique V2 was correctly reopened. Its running-manager image requirement was
strong in intent but impossible for the unprivileged M24 parent on the current
machine. The repaired V8 does not replace live bytes with a version string.
It adds an exact, pinned, noninteractive and read-only privileged hash probe
before and after the delegated child while retaining an independent D-Bus,
pidfd, procfs and system-unit identity channel.

The replacement is structurally adequate for worksheets. It creates no R8B
execution authority and does not make the privileged observer an evidence
producer.

## Live Findings

| Priority | Finding | Consequence | Repair |
|---|---|---|---|
| P0 | UID 1000 owns user manager PID 2200, but dereferencing `/proc/2200/exe` and reading `/proc/2200/maps` returns `EACCES`. | The old requirement could not be implemented by M24. A path, package hash or manager version alone would not prove the running image. | Require exact pre/post root-assisted SHA-256 of `/proc/<pid>/exe`; bind the fixed privileged argv and outputs in the resource receipt. |
| P1 | A privileged command path can silently enlarge authority if its argv is open-ended. | A generic sudo escape would be worse than the original observability defect. | Freeze canonical sudo-rs and sha256sum bytes; allow only one validated decimal PID proc path, no shell/PATH/environment input, empty stderr, exact NUL-terminated output and two invocations. |
| P1 | A live hash alone does not establish which process owns the user bus or child unit. | The right bytes could be hashed for the wrong PID. | Independently require D-Bus peer credentials and unique name, pidfd, boot/start identity, system-manager `user@UID.service`, InvocationID, ExecStart, cgroup and version convergence. |
| P1 | Two probes add processes outside the frozen Nando count. | Folding them into the 16,668 denominator would corrupt the ledger bound; omitting them would hide physical activity. | Keep the Nando and 33,336-event bounds unchanged; record exactly two sudo frontends and two sha256sum descendants as observation-tool processes in the resource receipt. |

## Adversarial Checks

| Attack or ambiguity | Required result |
|---|---|
| Reuse the on-disk package hash without reading the live proc image. | `VETO` under N42. |
| Accept only `Version=259.5-0ubuntu3.4` and the expected command line. | `VETO` under N42. |
| Substitute another PID, `/proc/self/exe`, a symlink or arbitrary path. | Request construction rejects before privileged execution. |
| Insert a shell, rely on PATH or add a second privileged command. | Command-normalization parity fails under N43. |
| Prompt for a password or write diagnostics to stderr. | Probe fails closed; P06 is ineligible. |
| Re-exec or replace the manager between probes. | pidfd/bus/unit continuity or pre/post live hash fails. |
| Count probe processes as Nando evidence invocations. | Cardinality and object-role validation rejects. |

## Machine Evidence

The live paper probe established:

```text
user-bus peer PID                         2200
system unit MainPID                       2200
system unit                               user@1000.service
user-manager cgroup                       /user.slice/user-1000.slice/user@1000.service/init.scope
manager version                           259.5-0ubuntu3.4
unprivileged /proc/2200/exe               EACCES
privileged live image SHA-256             3c4b78ddb68e29e23da0465dd273f1ee82f5b9439ebfcec9798b395c05a2c1e3
on-disk systemd SHA-256                   3c4b78ddb68e29e23da0465dd273f1ee82f5b9439ebfcec9798b395c05a2c1e3
sudo-rs SHA-256                           c11aad50d0ac8e7d8fd483a83884a2ad95a1a3f4fea399fa061f06f0b8ce65b6
uutils sha256sum SHA-256                  48893b0fb21436b54619db80486e83ef39dfccaf1aefe83dfa00c02d6146e8c0
```

These are current baseline observations, not execution authority. The V8
machine artifact must bind complete modes, lengths, canonical paths and fresh
preflight observations. Any drift is VETO.

## Residual Risk

The host policy currently permits broad passwordless sudo. V8 neither creates
nor expands that policy, but its implementation must construct the one allowed
argv internally and never accept a caller-supplied privileged program or path.
This is a host-security fact to retain in the machine baseline, not evidence
for the scientific claim.

## Next Legal Action

Create the V8 machine/cardinality artifact and the four structural worksheets.
Do not edit the dirty implementation donor or execute any R8B suite.
