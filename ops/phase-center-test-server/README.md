# Nando Phase-Center Test Server v1

Purpose: deploy the current phase-center / `.nwpc` serving proof on a test
server, collect real traffic metrics, and keep local accept disabled until the
verifier and money gates are clean.

This package is for the allowed product path:

```text
real agent stream
-> phase atoms
-> online phase-center miner
-> clean survivor hot runtime
-> shadow decisions
-> provider evidence join
-> metrics
```

Forbidden in this package:

```text
.nwrb
role-binding backend
lookup authority
target_id / proof_rule_id authority
manual local_out_t
local_accept without verifier and false_accepts = 0
synthetic-only money claim
```

## Services

`nando-phase-center-appender.service`

Reads Codex session JSONL files and appends phase-atom rows to the live append
trace. This is a source adapter, not runtime authority.

`nando-phase-center-live-tail.service`

Consumes the watermark trace and append trace, updates the online miner,
keeps clean survivor profiles hot, writes a decision log, and emits the main
report. It runs shadow-only.

`nando-phase-center-provider-export-watch.timer`

Periodically scans a provider-export drop directory and tries to join external
billing/token evidence to selected `.nwpc` shadow rows. This can unblock money
metrics, but does not enable local accept.

`nando-phase-center-provider-evidence-snapshot.timer`

Periodically reads the live-tail report, creates the provider acquisition pack
from the current future-shadow billing request, and runs the evidence-chain
gate. If no external export exists, it writes a blocked report with the exact
missing evidence path.

If HTTP bridge provider-boundary metadata exists, the snapshot also runs a cold
capture-coverage gate:

```text
NANDO_PROVIDER_BRIDGE_BOUNDARY_EVENTS_JSONL
-> NANDO_PROVIDER_BOUNDARY_CAPTURE_COVERAGE_REPORT
```

This proves only that upstream provider rows can be correlated to the billing
worklist. It does not create provider cost evidence, does not enable
local_accept, and does not unlock market money claims.

`nando-phase-center-provider-export-contract-pack.timer`

Builds the cold-path handoff pack for an external billing/export process:
required columns, schema, request samples, and a template. This does not write
provider evidence and does not enable local accept.

`nando-phase-center-metrics-snapshot.sh`

Reads the latest live-tail report and writes:

```text
metrics/nando-phase-center.metrics.json
metrics/nando-phase-center.prom
```

`nando-phase-center-metrics-snapshot.timer`

Runs the metrics snapshot continuously, so a test server always has a fresh
JSON/Prometheus view without mixing metrics work into the hot runtime.

`nando-phase-center-readiness-snapshot.timer`

Reads the latest metrics snapshot plus provider-evidence snapshot and writes
one product-readiness verdict. This is the server-side guard against treating
compression evidence, money evidence, and local-accept readiness as the same
thing.

`nando-phase-center-test-server-verify.sh`

Checks the installed binary, snapshot scripts, systemd units, and latest
metrics/evidence/readiness files. It writes one JSON report that separates:

```text
install_ready
shadow_metrics_ready
market_money_claim_allowed
local_accept_promotion_allowed
```

`nando-phase-center-test-server-verify.timer`

Runs the install/readiness verification continuously, producing the same JSON
report a human would check after deployment.

`nando-phase-center-status.sh`

Reads bridge health, verify, readiness, upstream readiness, provider evidence,
metrics, and key systemd service states into one operator-facing JSON status:

```bash
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env
```

Use `--refresh` when you want it to refresh snapshots first. The command does
not mine, score, call the provider, mutate policy, or print secrets.

`nando-phase-center-status.timer`

Runs the same status snapshot continuously, writing:

```text
metrics/nando-phase-center.status.json
```

`nando-phase-center-client-env.sh`

Prints or installs a sanitized client env for other local windows. It contains
only the local bridge URL and local canary key; it never prints provider
secrets and never mutates server policy.

Print shell exports:

```bash
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-client-env.sh /etc/nando-wave/phase-center.env print
```

Install for the current user:

```bash
mkdir -p ~/.config/nando-wave
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-client-env.sh /etc/nando-wave/phase-center.env print > ~/.config/nando-wave/client.env
chmod 0600 ~/.config/nando-wave/client.env
```

Install system-wide shell defaults:

```bash
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-client-env.sh /etc/nando-wave/phase-center.env install-system
```

This is blocked until upstream readiness reports
`ready_for_broad_provider_traffic=true`. For a reviewed local lab canary only,
the operator may pass `--allow-canary-only`, but that is not a broad production
default.

`nando-phase-center-gateway-canary-smoke.sh`

Runs the actual fail-open gateway against boxed canary routes. It requires
verified local responses for `nando health`, `nando compression`,
`nando readiness`, `nando promotion`, and `nando server`, while requiring a
broad prompt to fall back to the provider command.

`nando-phase-center-local-accept-promotion-gate.timer`

Reads readiness + verify reports and writes a disabled local-accept policy
candidate. It is allowed to say “ready for manual review”, but it never mutates
serving, never changes systemd, and never skips provider calls by itself.

`nando-phase-center-provider-activation-gate.timer`

Continuously refreshes the provider activation gate report without real provider
probes by default. It keeps the operator-facing status fresh:

```text
activation_allowed
system_client_env_install_allowed
blockers
next_action
```

It does not accept or print provider secrets, does not mutate client policy,
does not enable local_accept, and does not unlock money claims.

`nando-llm-gateway.sh`

Shared fail-open wrapper for local agents/Codex copies. On a system install,
production clients should prefer the HTTP bridge because the server policy env
is secret and mode `0600`. Use this wrapper for user-mode installs or operator
smokes where the env is readable by the process:

User-mode example:

```bash
nando-llm-gateway ~/.config/nando-wave/phase-center.env -- <normal-provider-command>
```

The gateway always preserves the normal provider path. It records request/token
telemetry, then tries a verifier-bound local command only when explicitly
enabled. Any timeout, daemon error, verifier miss, missing local response, or
`NANDO_OFFLOAD=0` falls back to the normal provider command.

`nando-provider-bridge.py`

OpenAI-compatible HTTP bridge for production canary traffic:

```text
POST /v2/chat/completions
POST /v2/responses
GET  /v2/health
POST /v1/chat/completions
POST /v1/responses
GET  /health
```

It tries the same verifier-bound local executor as `nando-llm-gateway`.
`/v2` is the default NANDA CPU compact latent transition surface. `/v1`
remains available for legacy clients. Accepted exact routes return
OpenAI-compatible JSON locally. Broad prompts proxy to
`NANDO_PROVIDER_UPSTREAM_BASE_URL` when configured; if upstream is not
configured, the bridge returns `upstream_missing` instead of faking a local
answer.

When a broad request reaches upstream, the bridge also writes metadata-only
provider boundary rows to:

```text
NANDO_PROVIDER_BRIDGE_BOUNDARY_EVENTS_JSONL
```

Those rows carry request hash, provider response ids, and provider `usage`
tokens when present. They are cold evidence input only: no score change, no
local accept, no money claim, and cost remains blocked unless real provider
cost evidence is supplied.

`nando-provider-bridge.service`

Runs the HTTP bridge continuously. Default bind:

```text
127.0.0.1:8787
```

`nando-provider-bridge-smoke.sh`

Checks the actual HTTP bridge:

```text
/health -> ok
/v2/health -> ok
/v1/chat/completions "nando compression" -> local accept
/v1/responses "nando readiness" -> local accept
/v2/chat/completions "nando compression" -> local accept with transition_runtime=true
/v2/responses "nando readiness" -> local accept with transition_runtime=true
broad prompt -> upstream fallback or upstream_missing
```

`nando-provider-bridge-v2-dogfood.sh`

Runs the live `/v2` bridge as a working dogfood workload:

```text
verified Nando commands -> local accept with transition_runtime=true
ordinary broad prompt -> decline / upstream_missing
report -> metrics/nando-phase-center.provider-bridge-v2-dogfood.json
```

This intentionally writes `traffic_source=dogfood_v2` into bridge decisions so
the metrics can separate self-use from non-dogfood traffic. It is useful for
regression and token trend checks, but it is not a market claim.

`nando-provider-bridge-upstream-smoke.sh`

Starts a temporary fake upstream and a temporary bridge on free local ports,
then proves the transport split:

```text
/health -> upstream_configured=true
"nando compression" -> verifier-bound local accept on v1 and v2
ordinary broad prompt -> reaches upstream on v1 and v2
provider boundary events -> written with provider usage tokens
```

This is a transport proof only. It does not create provider billing evidence
and does not turn synthetic upstream traffic into a money claim.

`nando-provider-bridge-upstream-readiness.sh`

Writes the production bridge upstream-readiness report:

`nando-phase-center-provider-activation-gate.sh`

Runs the final cold gate for broad provider traffic after upstream onboarding:

```text
verify install
status summary
upstream readiness
client env default-bridge allowance
false_accepts = 0
```

It accepts no provider key, prints no provider secret, and mutates no serving
policy. With `--allow-real-probe`, it lets the existing readiness script make
one reviewed broad upstream probe; without the flag it is report-only. PASS
means system-wide sanitized client env may be installed; it still does not
unlock money claims.

`nando-phase-center-provider-activate.sh`

One-command reviewed activation after the operator has a provider key:

```bash
printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-activate.sh \
  /etc/nando-wave/phase-center.env \
  --base-url https://api.openai.com \
  --provider openai \
  --api-key-stdin \
  --allow-real-probe \
  --install-system-client-env
```

The command chains:

```text
upstream onboarding
-> one reviewed real broad readiness probe
-> activation gate
-> optional system sanitized client env install only after activation PASS
```

It never accepts provider keys as command arguments, never prints provider
secrets, does not mutate local_accept, and does not unlock money claims.

`nando-phase-center-provider-activate-smoke.sh`

Runs the same activation wrapper against a temporary fake upstream and temporary
bridge using a temporary env copy. It proves:

```text
provider-activate wrapper reaches activation_allowed=true on fake upstream
real /etc/nando-wave/phase-center.env is unchanged
provider boundary metadata is captured
no provider secret is printed
money claim remains false
```

This is a lab proof only, not market evidence.

`nando-phase-center-provider-deactivate.sh`

Rollback command for returning the server to canary-only mode:

```bash
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-deactivate.sh \
  /etc/nando-wave/phase-center.env \
  --remove-system-client-env
```

It unsets upstream provider transport, turns real readiness probing off, refreshes
activation/status reports, and optionally removes the system-wide sanitized
client env. It does not disable verifier-bound local canary routes, does not
print provider secrets, and does not touch money claims.

```text
NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_REPORT
```

Default mode does not call the real provider. It only checks bridge health and
whether upstream is configured. To run one real broad upstream probe from the
server policy, set:

```text
NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL=1
```

That optional probe must show broad traffic reaches upstream and adds a
provider-boundary metadata row. It still does not create provider cost evidence
and does not unlock money claims.

`nando-provider-bridge-upstream-config.sh`

Sets or clears the real upstream on the server policy file without printing the
provider key:

```bash
printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-config.sh \
  /etc/nando-wave/phase-center.env \
  set --base-url https://api.openai.com --api-key-stdin --provider openai

sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-config.sh \
  /etc/nando-wave/phase-center.env status

sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-config.sh \
  /etc/nando-wave/phase-center.env probe-on

sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-config.sh \
  /etc/nando-wave/phase-center.env unset
```

Do not pass provider secrets as command-line arguments. Use `--api-key-stdin`.
The status output reports only `api_key_present`, never the key value. The
server policy env may contain the upstream key and must stay mode `0600`; deploy,
policy, and upstream config commands preserve that permission.

`nando-phase-center-upstream-onboard.sh`

One-command operator wrapper around upstream configuration, readiness refresh,
and status:

```bash
printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard.sh \
  /etc/nando-wave/phase-center.env \
  --base-url https://api.openai.com \
  --provider openai \
  --api-key-stdin
```

It never prints the provider key. Add `--allow-real-probe` only when one real
broad provider readiness call is intentionally allowed. Use `--dry-run` to
validate the flow on a temporary env copy without mutating server policy.

## Production Install

```bash
ops/phase-center-test-server/deploy.sh
```

## Release Bundle

Build one boxed server artifact with a prebuilt `nando-cli`, install entrypoint,
checksum, and manifest:

```bash
scripts/build-phase-center-test-server-package.sh
```

The bundle build runs the Rust Action Memory selector/quarantine gate before
packaging:

```bash
scripts/rust-action-memory-gate.sh
```

This writes a read-only evidence report:

```text
target/nando-wave/ram/rust-action-memory-gate.json
```

The gate allows a release when `cargo check` passes, `diagnostics_count = 0`,
and `quarantined_candidates = 0`. A selector verdict of `WATCH` is acceptable
when there are no candidates to repair; quarantine is the hard blocker.

The script writes:

```text
target/nando-wave/deploy/nando-phase-center-test-server-<version>.tar.gz
target/nando-wave/deploy/nando-phase-center-test-server-<version>.tar.gz.sha256
target/nando-wave/deploy/nando-phase-center-test-server-<version>.package.json
```

Install from an unpacked bundle:

```bash
tar -xzf nando-phase-center-test-server-<version>.tar.gz
cd nando-phase-center-test-server-<version>
./install-from-bundle.sh
```

Install files without enabling/starting services, useful for CI or dry-run
bundle validation:

```bash
NANDO_DEPLOY_INSTALL_ONLY=1 ./install-from-bundle.sh --user
```

This uses the bundled binary through `NANDO_DEPLOY_NANDO_CLI_BIN`, so a target
server does not need to rebuild Rust just to install the canary stack.

It builds the release binary, installs:

```text
/opt/nando-wave
/etc/nando-wave/phase-center.env
/var/lib/nando-wave
/var/log/nando-wave
/usr/local/bin/nando-llm-gateway
/etc/systemd/system/nando-phase-center-*
```

and enables the production-like services/timers.

By default, deploy preserves an existing server policy file. It only appends
new missing keys. To intentionally reset the server policy from the packaged
example:

```bash
sudo env NANDO_DEPLOY_OVERWRITE_ENV=1 ops/phase-center-test-server/deploy.sh
```

## Client Handoff

Short copy-paste instructions for other local agent windows live here:

```text
ops/phase-center-test-server/CLIENT_HANDOFF.md
```

Default local OpenAI-compatible endpoint:

```text
http://127.0.0.1:8787/v2
```

## Server Safety Policy

The server policy lives in:

```text
/etc/nando-wave/phase-center.env
```

Clients do not decide safety by local shell aliases. The server env controls:

```text
NANDO_LOCAL_ACCEPT_ENABLED
NANDO_CLIENT_ALLOW_LOCAL_ACCEPT
NANDO_CLIENT_SAFETY_POLICY
NANDO_CLIENT_REQUIRE_VERIFIER
NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO
NANDO_CLIENT_KILL_SWITCH
```

Use the packaged policy tool:

```bash
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-policy-set.sh /etc/nando-wave/phase-center.env shadow
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-policy-set.sh /etc/nando-wave/phase-center.env canary-health
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-policy-set.sh /etc/nando-wave/phase-center.env canary-verified
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-policy-set.sh /etc/nando-wave/phase-center.env kill
```

Modes:

```text
shadow:
  telemetry and miner only; local_accept off.

canary-health:
  compatibility alias for the first health/status canary mode.
  broad LLM replacement remains forbidden.

canary-verified:
  exact built-in health/status and artifact-backed status/compression
  routes can local_accept.
  broad LLM replacement remains forbidden.

kill:
  fail-open provider fallback only; no Nando offload attempt.
```

## Health

```bash
systemctl status nando-phase-center-appender.service
systemctl status nando-phase-center-live-tail.service
systemctl status nando-phase-center-metrics-snapshot.timer
systemctl status nando-phase-center-readiness-snapshot.timer
systemctl status nando-phase-center-status.timer
/opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-metrics-snapshot.sh
/opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-export-contract-pack.sh
/opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
/opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-gateway-canary-smoke.sh
/opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-smoke.sh
/opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-smoke.sh
/opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-readiness.sh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env status
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard.sh /etc/nando-wave/phase-center.env --status
printf 'nando gateway health' | sudo nando-llm-gateway /etc/nando-wave/phase-center.env -- cat
curl http://127.0.0.1:8787/health
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.metrics.json
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.prom
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.readiness.json
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.test-server-verify.json
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.status.json
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.local-accept-promotion.json
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.gateway-canary-smoke.json
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-smoke.json
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-smoke.json
cat /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-readiness.json
tail -20 /var/lib/nando-wave/streaming/nando-provider-bridge.provider-boundary-events.jsonl
```

## Shared Gateway For Agents

Install the wrapper into PATH:

```bash
sudo ln -sf /opt/nando-wave/ops/phase-center-test-server/bin/nando-llm-gateway.sh /usr/local/bin/nando-llm-gateway
```

Local development can use:

```bash
ln -sf /home/ubu/projects/nando-wave/ops/phase-center-test-server/bin/nando-llm-gateway.sh ~/.local/bin/nando-llm-gateway
```

For user-local Codex copies on this machine:

```text
/etc/nando-wave/phase-center.env
~/.local/bin/nando-codex
~/.bashrc alias codex='nando-codex'
```

This mode reads server policy from `/etc` and writes gateway telemetry under:

```text
~/.local/state/nando-wave/streaming/
```

It does not require `/etc` or `/var/lib` permissions.

`nando-codex` is guarded fail-open. It preserves the original OpenAI
environment, checks `http://127.0.0.1:8787/v2/health`, and routes Codex through
the local OpenAI-compatible bridge only when the bridge is healthy and upstream
is configured. Otherwise it restores the original environment and starts the
real Codex CLI directly.

Control knobs:

```text
NANDO_CODEX_PROVIDER_BRIDGE=auto  # default guarded mode
NANDO_CODEX_REQUIRE_UPSTREAM=1    # default: do not route broad traffic into Nando unless upstream is ready
NANDO_CODEX_HEALTH_TIMEOUT_MS=300
NANDO_CODEX_ALIAS=0               # emergency bypass
NANDO_OFFLOAD=0                   # emergency bypass
```

Default safety policy:

```text
NANDO_GATEWAY_TIMEOUT_MS=200
NANDO_OFFLOAD=1
NANDO_LOCAL_ACCEPT_ENABLED=0 in shadow, 1 only after server policy promotion
NANDO_GATEWAY_CAPTURE_RAW=0
```

Why `200 ms`, not `20-50 ms`:

```text
20-50 ms is a good future hot-runtime budget.
200 ms is safer for the first shared bridge rollout.
The gateway is fail-open, so timeout means fallback to the normal LLM channel,
not a broken agent.
```

Required local-command contract before real skipping:

```text
stdin:
  normal provider request

stdout JSON:
  {"local_accept":true,"verifier_ok":true,"response":"..."}
```

If this contract is not met, the gateway falls back. This makes the bridge safe
for all Codex copies before local accept is enabled.

Current boxed local executor routes:

```text
nando health       -> exact gateway health
nando status       -> server verify status from JSON report
nando compression  -> clean token compression from metrics JSON
nando readiness    -> readiness gate from JSON report
nando promotion    -> local-accept promotion gate from JSON report
nando server       -> server verify status from JSON report
```

Any other prompt falls back to the normal provider command.

## Real Savings Rule

Count only:

```text
stable_clean_token_compression_unique_cpu_accepts_over_exact_cache
stable_clean_token_compression_saved_tokens
stable_clean_token_compression_false_accepts
```

and only when:

```text
stable_clean_token_compression_claim_allowed = true
stable_clean_token_compression_false_accepts = 0
local_accept_promotion_allowed = true
```

The readiness report separates three claims:

```text
compression_claim_allowed
local_accept_promotion_allowed
money_evidence_ready
market_money_claim_allowed
```

`market_money_claim_allowed` must stay false until external provider evidence
joins and the upstream live report agrees.

Local accept is a separate gate. The promotion report must show:

```text
promotion_allowed = true
local_accept_enabled = false
requires_manual_activation_after_review = true
```

before a separate reviewed serving change may be considered.

If money claim is blocked by `external_provider_export_missing`, drop provider
export JSONL files into `NANDO_PROVIDER_EXPORT_DROP_DIR` and let the provider
watch service join them.

The evidence snapshot writes:

```text
NANDO_PROVIDER_EVIDENCE_SNAPSHOT_REPORT
NANDO_PROVIDER_ACQUISITION_REPORT
NANDO_PROVIDER_ACQUISITION_PACK_DIR/provider-export-acquisition.manifest.jsonl
NANDO_PROVIDER_ACQUISITION_PACK_DIR/provider-boundary-capture-request.jsonl
NANDO_PROVIDER_EVIDENCE_CHAIN_REPORT
NANDO_PROVIDER_EXPORT_CONTRACT_REPORT
NANDO_PROVIDER_EXPORT_CONTRACT_DIR/README_PROVIDER_EXPORT.md
NANDO_PROVIDER_BOUNDARY_CAPTURE_COVERAGE_REPORT
```

## Test Server Boundary

This package prepares a real server metric stand. It does not automatically
skip provider calls. Actual traffic saving is enabled only after a separate
promotion gate proves verifier-bound local accept with zero false accepts.
