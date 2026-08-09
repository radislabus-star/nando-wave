# Nando Remote Backend

This profile runs the current Rust backend on a private LAN host. It is a
staging boundary for the later OCI deployment, not the public multi-tenant
edge.

```text
LAN client
  -> nando-connector
     -> mini-PC Nginx :8787
        -> nando-transition-serving :18789
           -> CPU response
           -> 418 ABSTAIN

Private loopback roles
  -> nando-response-learning :18790
  -> nando-gateway-control :18788

Client fallback
  -> original request replayed by nando-connector
  -> TLS chatgpt.com through the client's network route
```

The public client base URL uses the standard OpenAI-compatible prefix:

```text
http://192.168.3.94:8787/v1
```

`/v1/responses` and `/v1/chat/completions` are the client surface. The existing
`/v2` aliases remain available for compatibility, while miner, MS3, runtime,
and reconciliation endpoints stay internal.

Start the local connector console with:

```bash
ops/remote-backend/nando-connect
```

The console starts the user service when needed and shows live connector
traffic counters plus remote CPU/admission status. Closing it leaves the
connector running. Codex is launched separately with `codex`; the installed
Codex wrapper checks the Nando health contract and supplies an ephemeral
provider override for `http://127.0.0.1:8787/v1`. Codex continues to own the
user's local authentication. Neither helper reads or persists tokens.
Applications that already manage their own API key can print the required
local base URL environment with:

```bash
ops/remote-backend/nando-connect env
```

The Linux transport is a separate static binary:

```text
nando-connect    manages and monitors the connector service
nando-connector  forwards the local byte stream to the Nando server
```

`nando-connector` keeps request and response bodies byte-transparent. In client
fallback mode it parses HTTP/1.1 framing and, for bounded `/v1/responses` or
`/v2/responses` requests, reads only `client_metadata.turn_id` and
`client_metadata.session_id`. It stores domain-separated identity hashes, the
request-body hash, and a confirmed remote `200` or `418`, never the raw body.
The receipt seals separate request-observed and route-confirmed timestamps;
both must precede the linked action frame.
Unknown routes, chunked requests, and unknown protocols retain the transparent
relay path without minting a route receipt. Streaming response bytes pass
through unchanged.

The mini-PC exposes `/_nando/local/v1/...` and `/_nando/local/v2/...` only on
the private LAN listener. These routes never perform server-side fallback.
`418`, `502`, `503`, `504`, or failure before a response head causes one
client-side replay to `https://chatgpt.com/backend-api/codex/...`. Once any
response head is delivered, the connector never retries the request.

Installation must first pass `nando-connector --check --client-fallback`,
which verifies the remote `nando.client-fallback.v1` contract. The installed
unit may then use `--allow-degraded-start`: if the already-verified mini-PC is
offline during a later connector restart, existing clients still receive
client-side fallback instead of losing the local listener.

Replay is bounded at `64 MiB`. The first `1 MiB` stays in memory; larger bodies
spill into a private, unlinked file under the user runtime directory. Tokens,
headers, and bodies are never logged or persisted after the request.

## Compact Learning Evidence

The connector remains a byte-transparent request relay. Post-action learning
uses a separate local `nando-evidence-agent` because the remote backend cannot
observe local Codex tool outcomes from HTTP response bytes alone.

```text
local append-only Codex session journal
  -> incremental watcher restoring its durable per-file offset
  -> existing source-neutral RelationFrame extractor and verifier
  -> exact pre-action connector route receipt
  -> private durable outbox of compact route-bound verified frames
  -> HMAC-authenticated, hash-chained batches over the private LAN
  -> exact Nginx cold-path route /_nando/evidence/v1/batches
  -> remote learner archive, receipt, and client head
  -> existing MS3 join and acquisition machine
```

Raw session rows, prompts, response bodies, authentication, and provider tokens
remain on the client. The connector ledger contains only hashes and confirmed
remote status. The outbox and remote spool contain bounded verified
`RelationFrame` records, the complete canonical route receipt, and route,
verifier, and frame commitment roots. The remote spool independently checks the
receipt root, turn/session identity, status, and
`request <= confirmation <= action` order. Legacy root-only records remain
decodable but are never considered route-bound. Frames without a pre-action
route receipt are censored locally. The server signs every ACK with the same
per-client key; the agent advances its sequence only after verifying that
signature. Acknowledged outbox segments are compacted only after the agent state
and pending-file removal are durable.

The current transport is authenticated but not encrypted. It is restricted to
the private LAN listener and must not be exposed publicly. Later mTLS can add
confidentiality without changing the evidence schema or MS3 authority boundary.

Build both binaries on the mini-PC:

```bash
cargo build --release -p nando-transition-serving \
  --bin nando-transition-serving \
  --bin nando-evidence-agent
```

All remote Rust test commands, including ignored release fixtures, must use the
bounded runner. It terminates the complete Cargo/test process group after the
registered timeout, including a child test that stops making progress:

```bash
ops/remote-backend/run-remote-rust-test.sh --timeout 1800 test --workspace
```

The raw 32-byte client key is created out of band with mode `0600` and is never
printed. Enable the remote cold spool transactionally:

```bash
ops/remote-backend/install-remote-evidence-spool.sh \
  --binary target/release/nando-transition-serving \
  --client-key /secure/path/client.key \
  --enable-k1-scheduler
```

Only the cold learner stops during this operation. If readiness fails, the
installer restores the binary, environment, and key, forces the K1 scheduler
off, and preserves append-only state written by the failed process. Hot serving
and the connector remain online. Omit `--enable-k1-scheduler` when installing
the spool without opening the scheduler.

Install the client agent separately:

```bash
ops/remote-backend/install-evidence-agent.sh \
  --binary /secure/path/nando-evidence-agent \
  --server http://192.168.3.94:8787
```

The local installer does not start, stop, or reload `nando-connector`. Its
systemd unit has read-only access to `~/.codex/sessions`, the client key, and
the connector route-receipt runtime directory; write access only to
`~/.local/state/nando-evidence-agent`; a `128 MiB` memory limit; and automatic
restart if the session watcher stops.

Transport-only counters are available on loopback at
`http://127.0.0.1:18786/metrics`: active, accepted and completed connections,
uploaded/downloaded bytes, Nando responses, client fallback attempts,
successful replays, failures, fallback reasons, replay spills, route receipts,
missing route identities, and receipt-write failures. A connection is not
reported as a Codex window because Codex may reuse or multiply TCP connections.

The distributed user service listens directly on `127.0.0.1:8787`. The
`nando-client-connector.compatibility.override.conf` file is only for a machine
that must keep an existing Nginx listener on `8787` while forwarding through
the connector on `18787`.

Build the portable x86-64 Linux artifact on the build host:

```bash
NANDO_CONNECTOR_CARGO_BIN="$HOME/.cargo/bin/cargo" \
  ops/remote-backend/build-linux-connector.sh
```

Activate a tested connector release with the drain-aware installer:

```bash
ops/remote-backend/install-client-connector.sh \
  --binary dist/nando-connector/nando-connector-linux-x86_64
```

The installer validates the candidate and systemd unit before inspecting the
live service. If any client connection is active it exits with status `75`
without changing the binary, unit, or process. An activation from a drained
state is health-checked and automatically rolls back both files and the service
if readiness fails.

The first boot is fail-closed:

```text
NANDO_LOCAL_ACCEPT_ENABLED=0
NANDO_CLIENT_ALLOW_LOCAL_ACCEPT=0
NANDO_GATEWAY_CPU_ROUTE_READY=0
```

Install the LAN edge after the three Rust service units and state are ready:

```bash
NANDO_REMOTE_INSTALL_ONLY=1 \
  ops/remote-backend/install-lan-edge.sh \
  --bind 192.168.3.94:8787 \
  --allow 192.168.3.0/24
```

The installer prefers the healthy local `systemd-resolved` stub at
`127.0.0.53`; an explicit `--resolver` remains available for recovery. Updates
are transactional: the candidate config is validated before replacement, an
active gateway receives only a graceful reload, and any failed DNS, HTTPS, CPU,
control, or edge health check restores the previous config and unit.

Legacy clients still use the mini-PC two-address fallback pool. Client-fallback
connectors bypass that pool after an abstain, so a TrustTunnel outage on the
mini-PC cannot break their external fallback. The legacy route remains during
the drain period and can be removed after every client has migrated.

Start it explicitly after the internal health checks pass:

```bash
sudo systemctl enable --now nando-transport-gateway.service
curl -fsS http://192.168.3.94:8787/health
curl -fsS http://192.168.3.94:8787/cpu-health
curl -fsS http://192.168.3.94:8787/control-health
```

Only the Nginx listener is reachable over the LAN. Serving, learning, and
control remain bound to loopback. Do not change the listener to a public
interface until TLS, tenant authentication, rate limits, and restore gates
exist.

Complete the cold MS3 -> MS4 lifecycle with independent admission and lease
workers. The transaction validates the runtime ABI against the already-running
hot process and never restarts Nginx or hot serving:

```bash
ops/remote-backend/install-ms4-autonomous-loop.sh \
  --admission-binary /path/to/nando-response-admission
```

Candidate publication and authority reconciliation both have filesystem
triggers plus 10-second recovery timers. The composite gate still issues only
a bounded 30-second lease and remains fail-closed until real future evidence
passes.

After restore and shadow streaming pass, authority can be reconciled through
the composite gate:

```bash
ops/remote-backend/reconcile-authority.sh enable
```

Pin the Rust toolchain during an install-only rebuild instead of relying on
the host's default `cargo`:

```bash
NANDO_DEPLOY_CARGO_BIN="$HOME/.cargo/bin/cargo" \
NANDO_DEPLOY_RUST_TOOLCHAIN=1.97.0 \
NANDO_DEPLOY_INSTALL_ONLY=1 \
  ops/phase-center-test-server/deploy.sh
```

The reconciler rolls back to shadow if the final gate or effective runtime
health does not pass. Disable remains explicit:

```bash
ops/remote-backend/reconcile-authority.sh disable
```

## Deployment Receipt

Every live binary update has a durable two-phase receipt. `prepare` runs before
the transactional installers and preserves the previous binaries, configs,
and unit files. `finalize` runs only after readiness passes and atomically
records the source commit/tree, installed artifact hashes, service PIDs, unit
roots, state manifests, runtime snapshots, and rollback pointer:

```bash
deployment_dir="$(NANDO_DEPLOY_ALLOW_HOT_RESTART=1 \
  ops/remote-backend/deployment-receipt.sh prepare \
  --source-dir /home/e/build/nando-wave-release \
  --rollback-commit <previous-commit>)"

# Run the cold learner and control transactional installers here.

ops/remote-backend/deployment-receipt.sh finalize \
  --source-dir /home/e/build/nando-wave-release \
  --deployment-dir "${deployment_dir}"
```

By default, finalization fails if the hot-serving or Nginx PID changed. Set
`NANDO_DEPLOY_ALLOW_HOT_RESTART=1` during `prepare` only when the deployment
requires an intentional hot-serving restart. The receipt records the actual
before/after PIDs and the preregistered exception; Nginx must always remain
unchanged. The completed receipt directory is made read-only and remains under
`/var/lib/nando-wave/deployments`.
