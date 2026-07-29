# Nando Remote Backend

This profile runs the current Rust backend on a private LAN host. It is a
staging boundary for the later OCI deployment, not the public multi-tenant
edge.

```text
LAN client
  -> Nginx :8787
     -> nando-transition-serving :18789
        -> upstream fallback when the CPU operator abstains

Private loopback roles
  -> nando-response-learning :18790
  -> nando-gateway-control :18788
```

The public client base URL uses the standard OpenAI-compatible prefix:

```text
http://192.168.3.94:8787/v1
```

`/v1/responses` and `/v1/chat/completions` are the client surface. The existing
`/v2` aliases remain available for compatibility, while miner, MS3, runtime,
and reconciliation endpoints stay internal.

Users with an existing Codex login can connect without copying account data
into a Nando config file:

```bash
ops/remote-backend/nando-connect codex
```

The launcher checks the expected Nando health contract through the local
connector and supplies an ephemeral Codex provider override for
`http://127.0.0.1:8787/v1`. Codex continues to own the user's local
authentication. The launcher neither reads nor persists tokens. Applications
that already manage their own API key can print the required local base URL
environment with:

```bash
ops/remote-backend/nando-connect env
```

The Linux transport is a separate static binary:

```text
nando-connect    launches/configures Codex
nando-connector  forwards the local byte stream to the Nando server
```

`nando-connector` does not parse OpenAI or Codex payloads. Unknown routes,
headers, body fields, and streaming frames pass through unchanged, so normal
Codex releases do not require a connector release. Rebuild it only when the
Nando transport or security contract changes.

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
