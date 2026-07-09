# Nando Wave Handoff

For local dogfood/Codex windows on this machine, use Nando CPU v2:

```bash
source ~/.config/nando-wave/client.env
```

Endpoint: `http://127.0.0.1:8787/v2`

Check:

```bash
curl -s http://127.0.0.1:8787/v2/health | jq
```

Expected: `ok=true`, `default_client_api_version=v2`,
`local_accept_enabled=true`.

Full handoff:

```text
/home/ubu/projects/nando-wave/handoff/NANDO_CPU_V2_FOR_CODEX_WINDOWS.md
```
