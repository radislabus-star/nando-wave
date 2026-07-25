#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="${root}/CONTENTS.json"

jq -e '.schema == "nando.release-asset-contents.v1"' "${manifest}" >/dev/null

while IFS=$'\t' read -r expected bytes member; do
  path="${root}/${member}"
  test -f "${path}"
  test "$(stat -c %s "${path}")" = "${bytes}"
  test "$(sha256sum "${path}" | cut -d' ' -f1)" = "${expected}"
done < <(jq -r '.files[] | [.sha256, (.bytes | tostring), .member] | @tsv' "${manifest}")

test "$(sha256sum "${root}/bin/nando-transition-serving" | cut -d' ' -f1)" = \
  "$(jq -r '.serving_sha256' "${manifest}")"

printf 'NANDO_RELEASE_CONTENTS_PASS files=%s serving_sha256=%s\n' \
  "$(jq '.files | length' "${manifest}")" \
  "$(jq -r '.serving_sha256' "${manifest}")"
