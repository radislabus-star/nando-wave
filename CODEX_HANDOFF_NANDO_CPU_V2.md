# Codex Handoff: Nando CPU v2

```bash
source ~/.config/nando-wave/client.env
```

Проверка:

```bash
curl -s http://127.0.0.1:8787/v2/health | jq
```

Ожидаемо:

```text
ok: true
default_client_api_version: v2
local_accept_enabled: true
```

Если env надо прописать руками:

```bash
export OPENAI_BASE_URL=http://127.0.0.1:8787/v2
export OPENAI_API_KEY=nando-local
export NANDO_CPU_API_VERSION=v2
```

Быстрый тест:

```bash
curl -sS -H 'content-type: application/json' \
  -X POST http://127.0.0.1:8787/v2/chat/completions \
  --data '{"model":"gpt-5","metadata":{"nando_traffic_source":"dogfood_v2"},"messages":[{"role":"user","content":"nando compression"}]}' | jq -r \
  '.choices[0].message.content'
```

Должно вернуть `NANDO_COMPRESSION ...`.

Коротко: `127.0.0.1` работает только для окон на этой же машине. Для другой машины нужен адрес сервера или tunnel.
