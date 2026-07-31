# Development Provenance with Entire

Entire is the development-provenance and handoff layer over Git for this
repository. It records Codex sessions and checkpoints around Git work; it is
not part of the Nando runtime, learning authority, or product architecture.

## Start clean

Always start Codex from a clean Nando worktree. Run `cd` and `entire status` in
the same shell context so Entire inspects the repository Codex will use:

```bash
cd /home/ubu/projects/nando-wave
entire status
codex
```

Resolve unexpected worktree state before substantial work. Do not attach an
old, long-running session merely to manufacture development history.

## Checkpoints and handoff

Keep changes in focused commits. Each commit should form a useful checkpoint
boundary that can be inspected and handed off independently.

```bash
entire status
entire checkpoint list
entire checkpoint explain <id-or-sha>
entire dispatch --local
```

Use `checkpoint explain` to recover the reasoning and context around a known
checkpoint. Use local dispatch when a durable local handoff is needed.

Telemetry and automatic session pushes must remain disabled. Review every
locally captured checkpoint for secrets and private operational data before
any remote synchronization. Local review is mandatory even when redaction is
configured.

Heavy builds and tests may run on the mini-PC, but commits should be created
from an Entire-tracked worktree. If Codex will edit or commit directly on
another machine, install, enable, and trust Entire on that machine first.

Entire history is supporting development evidence only. It never replaces
tests, frozen receipts, the live transition gate, independent verification,
external admission, or any other Nando proof gate.
