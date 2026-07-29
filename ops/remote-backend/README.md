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

`nando-connector` does not parse OpenAI or Codex JSON payloads. In client
fallback mode it parses only HTTP/1.1 framing, the `/v1` or `/v2` route, and the
LAN response status. Bodies remain opaque and replay byte-for-byte. Unknown
routes and protocols retain the transparent relay path, and streaming response
bytes pass through unchanged. Normal Codex payload changes therefore do not
require a connector release; only a transport or security contract change
does.

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

Transport-only counters are available on loopback at
`http://127.0.0.1:18786/metrics`: active, accepted and completed connections,
uploaded/downloaded bytes, Nando responses, client fallback attempts,
successful replays, failures, fallback reasons, and replay spills. A connection
is not reported as a Codex window because Codex may reuse or multiply TCP
connections.

The distributed user service listens directly on `127.0.0.1:8787`. The
`nando-client-connector.compatibility.override.conf` file is only for a machine
that must keep an existing Nginx listener on `8787` while forwarding through
the connector on `18787`.

Build the portable x86-64 Linux artifact on the build host:

```bash
NANDO_CONNECTOR_CARGO_BIN="$HOME/.cargo/bin/cargo" \
  ops/remote-backend/build-linux-connector.sh
```

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
