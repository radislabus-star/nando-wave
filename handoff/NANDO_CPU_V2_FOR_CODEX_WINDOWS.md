# Nando CPU v2 handoff for Codex windows

Use Nando CPU v2:

```bash
source ~/.config/nando-wave/client.env
```

Endpoint:

```text
http://127.0.0.1:8787/v2
```

Health check:

```bash
curl -s http://127.0.0.1:8787/v2/health | jq
```

Expected:

```text
ok: true
default_client_api_version: v2
local_accept_enabled: true
```

Manual env, if needed:

```bash
export OPENAI_BASE_URL=http://127.0.0.1:8787/v2
export OPENAI_API_KEY=nando-local
export NANDO_CPU_API_VERSION=v2
```

Quick test:

```bash
curl -sS -H 'content-type: application/json' \
  -X POST http://127.0.0.1:8787/v2/chat/completions \
  --data '{"model":"gpt-5","metadata":{"nando_traffic_source":"dogfood_v2"},"messages":[{"role":"user","content":"nando compression"}]}' \
  | jq -r '.choices[0].message.content'
```

Expected response starts with:

```text
NANDO_COMPRESSION
```

Important: `127.0.0.1` works for windows on this same machine/server. For another
machine use the server address or a tunnel.
