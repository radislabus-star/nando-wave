# Nando Gateway Product Architecture

Status: proposed product boundary. This document does not grant runtime
authority or claim that the public multi-tenant surface is deployed.

## Product Contract

Nando Gateway is an OpenAI-compatible gateway with one stable public origin:

```text
Applications: https://api.<domain>/v1
Codex:        https://api.<domain>/v2
Console:      https://console.<domain>
```

Changing the base URL and adding a Nando project key must be sufficient for a
client to connect. Nando first tries an admitted immutable CPU operator and
falls back to the tenant-selected model provider when the operator abstains.

## Plane Separation

```text
Internet
   |
   v
TLS Edge (Caddy or Nginx)
   |
   v
Nando Edge API
   |-- tenant authentication
   |-- project, quota, and rate policy
   |-- protocol normalization
   |-- provider credential resolution
   |
   v
Hot Serving
   |-- admitted BundleV4 registry
   |-- authority lease validation
   |-- route -> ground -> execute -> verify
   |-- upstream provider fallback
   |-- durable request and terminal receipts
   |
   +-----------------------+
                           v
                    Durable Event Log
                           |
                           v
                      Cold Learner
                           |
                           v
                 Sealed Candidate Bundle
                           |
                           v
                  External Admission
                           |
                           v
                    Immutable Registry
```

The learner is never a dependency of request completion. It cannot grant its
own authority. A sealed candidate becomes executable only after external
admission issues a bounded authority lease.

## Client Authentication

Application and Codex credentials have different roles and must not be
collapsed into one header.

### Applications

```text
Authorization: Bearer nando_<project_key>
Base URL: https://api.<domain>/v1
```

Provider BYOK credentials are stored encrypted in the tenant vault. An
application never sends a provider secret on every request by default.

### Codex

Codex uses its normal OpenAI or ChatGPT credential in `Authorization`. Nando
tenant authentication uses a separate environment-backed header:

```toml
[model_providers.nando]
name = "Nando Gateway"
base_url = "https://api.<domain>/v2"
wire_api = "responses"
requires_openai_auth = true
supports_websockets = false
env_http_headers = { "X-Nando-Key" = "NANDO_API_KEY" }
```

This prevents the Nando project key from replacing the upstream OpenAI
credential.

## Container Roles

One versioned Nando image contains the Rust binaries. `NANDO_ROLE` selects the
process, so single-node and production deployments use identical artifacts.

```text
ghcr.io/<org>/nando:<version>
  NANDO_ROLE=edge
  NANDO_ROLE=serving
  NANDO_ROLE=learner
  NANDO_ROLE=control
  NANDO_ROLE=migrate
```

The TLS proxy remains a separate standard container. Do not run unrelated
daemons under a supervisor in one application container.

## Single-Node Compose

The first deployable profile uses one command:

```text
docker compose up -d
```

```text
caddy
nando-edge
nando-serving
nando-learner
nando-control
postgres

volumes:
  postgres-data
  nando-spool
  nando-artifacts
  nando-checkpoints
```

PostgreSQL owns tenants, projects, API-key hashes, quotas, provider metadata,
usage aggregates, and audit indexes. The artifact volume owns immutable
bundles, proof receipts, and checkpoints. The bounded spool volume owns only
transient request and learning events.

MinIO or external S3 is optional in the single-node profile. Production swaps
the filesystem artifact backend for S3 without changing domain types.

## Production Deployment

```text
Public load balancer
  -> nando-edge replicas
  -> nando-serving replicas

Private network
  -> nando-learner workers
  -> nando-control
  -> PostgreSQL
  -> object storage
  -> durable event transport
```

The event boundary is an `EventLog` interface:

```text
single node: bounded filesystem spool
production:  JetStream or another durable ordered transport
```

The artifact boundary is an `ArtifactStore` interface:

```text
single node: filesystem
production:  S3-compatible object storage
```

## Public API

MVP endpoints:

```text
POST /v1/responses
POST /v1/chat/completions
GET  /v1/models
POST /v2/responses
GET  /health/live
GET  /health/ready
```

Control endpoints use a separate origin and authentication:

```text
POST /api/projects
POST /api/keys
POST /api/providers
GET  /api/usage
GET  /api/requests
GET  /api/operators
```

The existing local mode and admission controls must not be exposed through the
public data-plane origin.

## Security Defaults

- TLS is mandatory outside loopback.
- Public edge is the only exposed application service.
- Serving, learner, control, PostgreSQL, and object storage use a private
  container network.
- Nando API keys are random high-entropy values; only a lookup hash and short
  display prefix are stored.
- Provider secrets use envelope encryption and Docker/Kubernetes secrets for
  the master key.
- Request bodies and prompts are not logged by default.
- Tenant id, project id, request id, model, latency, token usage, route, and
  verified CPU receipt are safe structured telemetry fields.
- Every spool has byte, file-count, and age limits.
- Readiness fails closed when registry, admission, credential vault, or
  provider routing state is inconsistent.

## Existing Code Ownership

```text
nando-transition-serving  -> internal hot serving role
nando-response-learning   -> current cold learner deployment role
nando-gateway-control     -> private control and diagnostic role
nando-operator-*          -> bundle, proof, runtime, persistence, admission
new nando-edge-gateway    -> public tenant and protocol boundary
```

`nando-gateway-control` must not become the public edge: it currently owns
mode changes and watchdog behavior. The new edge crate may read admitted
public routing state but cannot own operator admission.

## Delivery Stages

1. Containerize the existing hot, learner, and control binaries with immutable
   image digests and persistent volumes.
2. Add `nando-edge-gateway` with project-key authentication and `/v1` plus
   `/v2` routing.
3. Deploy the single-node profile on the remote mini-PC and validate LAN TLS,
   Codex streaming, fallback, restart, and receipt parity.
4. Add encrypted provider BYOK storage, projects, quotas, and usage views.
5. Deploy behind a real domain and public TLS on a server with stable ingress.
6. Add production object storage and durable event transport only when
   horizontal scaling is required.

The mini-PC is a staging and self-host node. It becomes a public service only
after stable ingress, TLS, tenant authentication, rate limits, backup, and
restore gates pass.
