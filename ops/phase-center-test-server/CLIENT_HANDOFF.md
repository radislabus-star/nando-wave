# Nando Client Handoff

Purpose: give another local agent/window the shortest safe way to route traffic
through the NANDA CPU canary server.

## Server Endpoint

Use this only on the same machine unless the service is intentionally exposed
behind a reviewed proxy:

```bash
curl -s http://127.0.0.1:8787/v2/health | jq
```

Expected shape:

```json
{
  "ok": true,
  "default_client_api_version": "v2",
  "local_accept_enabled": true
}
```

OpenAI-compatible base URL:

```bash
export OPENAI_BASE_URL=http://127.0.0.1:8787/v2
export OPENAI_API_KEY=nando-local
```

Or load the generated sanitized env:

```bash
source ~/.config/nando-wave/client.env
```

Operator command to print the current sanitized env:

```bash
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-client-env.sh /etc/nando-wave/phase-center.env print
```

Before sending broad provider traffic through that URL, check server upstream
readiness:

```bash
jq .summary /var/lib/nando-wave/streaming/metrics/nando-phase-center.status.json
jq .verdict /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-readiness.json
```

Or refresh and print the single status JSON:

```bash
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
```

If the verdict says `WATCH_CANARY_ONLY_UPSTREAM_UNSET`, use the normal provider
channel for broad prompts. The bridge is then only ready for verified Nando
canary routes.

System-wide default bridge env is blocked until the server reports
`broad_provider_traffic_ready=true`. Do not put `/etc/profile.d` defaults on
ordinary windows while upstream is unset.

Compatibility aliases some clients use:

```bash
export OPENAI_API_BASE=http://127.0.0.1:8787/v2
export OPENAI_API_KEY=nando-local
```

Legacy compatibility:

```text
/v1 remains available for old clients.
/v2 is the default NANDA CPU compact latent transition runtime surface.
```

The key is not a provider secret for local canary traffic. Real upstream
provider secrets live only in server policy, not in client windows.

Server-side upstream is configured only by the operator:

```bash
printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard.sh \
  /etc/nando-wave/phase-center.env \
  --base-url https://api.openai.com --provider openai --api-key-stdin
```

The tool never prints the key. Client windows should not receive provider
secrets.

For full reviewed activation, the operator can use the boxed one-command path:

```bash
printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-activate.sh \
  /etc/nando-wave/phase-center.env \
  --base-url https://api.openai.com \
  --provider openai \
  --api-key-stdin \
  --allow-real-probe \
  --install-system-client-env
```

This performs upstream onboarding, one reviewed real readiness probe, activation
gate, and system sanitized client env install only after activation PASS. It
does not print provider secrets and does not unlock money claims.

Rollback to canary-only mode:

```bash
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-deactivate.sh \
  /etc/nando-wave/phase-center.env \
  --remove-system-client-env
```

This removes broad upstream routing and optional system default client env while
leaving verifier-bound local canary routes available.

## Quick Smoke

Local verified route:

```bash
curl -s http://127.0.0.1:8787/v2/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{"model":"gpt-5","messages":[{"role":"user","content":"nando compression"}]}'
```

Broad prompt behavior:

```text
if NANDO_PROVIDER_UPSTREAM_BASE_URL is configured on the server:
  proxy to upstream and record provider boundary metadata

if upstream is not configured:
  return upstream_missing instead of faking an answer
```

## Provider-Command Clients

If an agent supports only a provider command wrapper instead of an HTTP base URL,
use a user-mode install or a separate sanitized client env. Do not hand the
system server policy env to ordinary client windows.

User-mode example:

```bash
nando-llm-gateway ~/.config/nando-wave/phase-center.env -- <normal-provider-command>
```

The wrapper is fail-open:

```text
local verifier accept -> CPU response
timeout / miss / daemon error / broad prompt -> normal provider command
```

This is the safer default while upstream readiness is not PASS.

For Codex windows use `nando-codex`, not a blind `OPENAI_BASE_URL` export, when
work continuity matters. The launcher checks `/v2/health` with a short timeout
and chooses:

```text
health ok + upstream configured -> OPENAI_BASE_URL=http://127.0.0.1:8787/v2
health down / upstream missing -> original direct Codex/OpenAI environment
```

The emergency bypass stays:

```bash
export NANDO_CODEX_ALIAS=0
export NANDO_OFFLOAD=0
```

## Safety Boundary

Client windows do not decide production safety. Server policy controls it:

```text
/etc/nando-wave/phase-center.env
```

This server policy file may contain provider credentials and is installed as
mode `0600`. Client windows should not receive or edit provider secrets.

Required server-side gates for local accept:

```text
NANDO_LOCAL_ACCEPT_ENABLED=1
NANDO_CLIENT_ALLOW_LOCAL_ACCEPT=1
NANDO_CLIENT_REQUIRE_VERIFIER=1
NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO=1
NANDO_CLIENT_KILL_SWITCH=0
```

Money claims stay blocked until real provider billing/cost evidence is joined.
Token compression can be reported separately from money.

## Text To Paste Into Another Window

```text
Используй Nando CPU v2:

source ~/.config/nando-wave/client.env

Проверка:

curl -s http://127.0.0.1:8787/v2/health | jq

Должно быть:

ok: true
default_client_api_version: v2
local_accept_enabled: true

Если агенту надо руками прописать env:

export OPENAI_BASE_URL=http://127.0.0.1:8787/v2
export OPENAI_API_KEY=nando-local
export NANDO_CPU_API_VERSION=v2

Внимание: ручной OPENAI_BASE_URL не умеет сам откатиться, если локальный bridge
умрёт. Для Codex по умолчанию используйте nando-codex: он fail-open и не
переключает полный OpenAI-трафик на Nando, пока upstream не настроен.

Endpoint:

http://127.0.0.1:8787/v2

Быстрый тест:

curl -sS -H 'content-type: application/json' \
  -X POST http://127.0.0.1:8787/v2/chat/completions \
  --data '{"model":"gpt-5","metadata":{"nando_traffic_source":"dogfood_v2"},"messages":[{"role":"user","content":"nando compression"}]}' | jq -r \
  '.choices[0].message.content'

Должно вернуть NANDO_COMPRESSION ....

Важно: 127.0.0.1 работает для окон на этом же сервере/машине.
Для другой машины нужен адрес сервера или tunnel.
```

## Minimal Copy Block

```text
Используй Nando CPU v2:
source ~/.config/nando-wave/client.env

Endpoint:
http://127.0.0.1:8787/v2

Проверка:
curl -s http://127.0.0.1:8787/v2/health | jq

Важно: 127.0.0.1 работает для окон на этом же сервере/машине.
Для другой машины нужен адрес сервера или tunnel.
```
