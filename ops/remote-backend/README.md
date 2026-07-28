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
