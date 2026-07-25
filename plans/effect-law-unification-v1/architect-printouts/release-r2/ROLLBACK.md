# Nando Stable Runtime R2 Rollback

Release tag:

`nando-stable-live-operator-machine-2026-07-26-r2`

The release asset is the authoritative byte-for-byte rollback source. The tag
contains `RELEASE_MANIFEST.json`, which binds the asset name, byte size, and
SHA-256.

## Verify

```bash
tag=nando-stable-live-operator-machine-2026-07-26-r2
git checkout "$tag"
manifest=plans/effect-law-unification-v1/architect-printouts/release-r2/RELEASE_MANIFEST.json
asset=$(jq -r '.asset.name' "$manifest")
expected=$(jq -r '.asset.sha256' "$manifest")

gh release download "$tag" --repo radislabus-star/nando-wave --pattern "$asset"
test "$(sha256sum "$asset" | cut -d' ' -f1)" = "$expected"

mkdir -p /tmp/nando-r2-restore
zstd -dc "$asset" | tar -xf - -C /tmp/nando-r2-restore
/tmp/nando-r2-restore/nando-stable-live-operator-machine-2026-07-26-r2/verify-release.sh
```

## Restore Binaries

Nginx keeps the provider fallback available during the brief Rust service
restart.

```bash
root=/tmp/nando-r2-restore/nando-stable-live-operator-machine-2026-07-26-r2

sudo install -m 0755 "$root/bin/nando-transition-serving" \
  /opt/nando-wave/bin/nando-transition-serving
sudo install -m 0755 "$root/bin/nando-gateway-control" \
  /opt/nando-wave/bin/nando-gateway-control
sudo install -m 0755 "$root/bin/nando-live-transition-gate" \
  /opt/nando-wave/bin/nando-live-transition-gate
sudo install -m 0755 "$root/bin/nando-record-build" \
  /opt/nando-wave/bin/nando-record-build

sudo systemctl restart nando-gateway-control.service
sudo systemctl restart nando-response-learning.service
sudo systemctl restart nando-transition-serving.service
nando-live-transition-gate --project-root /home/ubu/projects/nando-wave --status-mode
```

## Restore Frozen State

Do this only when binary rollback is insufficient and the current state has
already been backed up. Never overwrite live checkpoints while either Rust
service is running.

```bash
root=/tmp/nando-r2-restore/nando-stable-live-operator-machine-2026-07-26-r2

sudo systemctl stop nando-transition-serving.service nando-response-learning.service
sudo cp -a /var/lib/nando-wave/transition \
  "/var/lib/nando-wave/transition.before-r2-restore.$(date +%s)"

install -m 0600 "$root/state/response-online-miner.checkpoint" \
  /var/lib/nando-wave/transition/response-online-miner.checkpoint
install -m 0600 "$root/state/online-collection-program-pools-v37.checkpoint" \
  /var/lib/nando-wave/transition/online-collection-program-pools-v37.checkpoint
install -m 0600 "$root/state/economics-live-v4.checkpoint" \
  /var/lib/nando-wave/transition/economics-live-v4.checkpoint
install -m 0600 "$root/state/response-registry.json" \
  /var/lib/nando-wave/transition/response-registry.json
install -m 0600 "$root/state/admission.json" \
  /var/lib/nando-wave/transition/admission.json
install -m 0600 "$root/state/response-admission-controller-report.json" \
  /var/lib/nando-wave/transition/response-admission-controller-report.json

install -D -m 0600 \
  "$root/state/learning-structure-bridge-v2/request-learning-v2.checkpoint.cbor" \
  /var/lib/nando-wave/transition/learning-structure-bridge-v2/request-learning-v2.checkpoint.cbor
install -D -m 0600 "$root/state/streaming-evidence-v2/checkpoint.cbor" \
  /var/lib/nando-wave/transition/streaming-evidence-v2/checkpoint.cbor

sudo systemctl start nando-response-learning.service nando-transition-serving.service
nando-live-transition-gate --project-root /home/ubu/projects/nando-wave --status-mode
```

The release remains `M3 WATCH`. Rollback restores a known-good runtime and its
optional state snapshot; it does not manufacture additional M3 windows.
