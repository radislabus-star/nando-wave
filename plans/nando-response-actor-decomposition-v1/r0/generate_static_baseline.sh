#!/usr/bin/env bash
# shellcheck disable=SC2016 # ast-grep meta-variables must remain literal.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(git -C "$script_dir" rev-parse --show-toplevel)
script_rel=${script_dir#"$repo_root"/}
cd "$repo_root"

ownership_tsv="$script_dir/MODULE_OWNERSHIP.tsv"
sg_bin=${SG_BIN:-/home/ubu/.cargo/bin/sg}

for command in git jq cargo sha256sum wc sort comm awk rg sed; do
    command -v "$command" >/dev/null || {
        printf 'missing required command: %s\n' "$command" >&2
        exit 1
    }
done
test -x "$sg_bin" || {
    printf 'missing ast-grep binary: %s\n' "$sg_bin" >&2
    exit 1
}

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

rg --files crates/nando-response-actor/src -g '*.rs' | sort \
    >"$tmp_dir/visible-rs.txt"
tail -n +2 "$ownership_tsv" | cut -f1 | sort >"$tmp_dir/mapped-rs.txt"

if ! cmp -s "$tmp_dir/visible-rs.txt" "$tmp_dir/mapped-rs.txt"; then
    printf 'ownership map does not match visible Rust source set\n' >&2
    comm -3 "$tmp_dir/visible-rs.txt" "$tmp_dir/mapped-rs.txt" >&2
    exit 1
fi

: >"$tmp_dir/ownership.jsonl"
while IFS=$'\t' read -r path tracked current_routes target_owners split_required policy; do
    test "$path" = "path" && continue
    if test "$tracked" = "yes"; then
        git ls-files --error-unmatch -- "$path" >/dev/null
    elif git ls-files --error-unmatch -- "$path" >/dev/null 2>&1; then
        printf 'ownership map marks tracked file as untracked: %s\n' "$path" >&2
        exit 1
    fi
    lines=$(wc -l <"$path")
    digest=$(sha256sum "$path" | awk '{print $1}')
    jq -nc \
        --arg path "$path" \
        --argjson tracked "$(test "$tracked" = yes && echo true || echo false)" \
        --arg current_routes "$current_routes" \
        --arg target_owners "$target_owners" \
        --argjson split_required "$(test "$split_required" = yes && echo true || echo false)" \
        --arg policy "$policy" \
        --argjson lines "$lines" \
        --arg sha256 "$digest" \
        '{
            path: $path,
            tracked: $tracked,
            current_routes: ($current_routes | split(",")),
            target_owners: ($target_owners | split(",")),
            split_required: $split_required,
            policy: $policy,
            lines: $lines,
            sha256: $sha256
        }' >>"$tmp_dir/ownership.jsonl"
done <"$ownership_tsv"
jq -s 'sort_by(.path)' "$tmp_dir/ownership.jsonl" >"$script_dir/MODULE_OWNERSHIP.json"

mapfile -t tracked_rs < <(
    git ls-files 'crates/nando-response-actor/src/*.rs' \
        'crates/nando-response-actor/src/**/*.rs' | sort -u
)

"$sg_bin" run --lang rust --pattern 'pub use $$$ITEMS;' --json=stream \
    crates/nando-response-actor/src/lib.rs \
    | jq -r 'select(.range.start.column == 0) | select(.text | startswith("pub use ")) | .text' \
    >"$script_dir/PUBLIC_API_SURFACE.txt"

"$sg_bin" run --lang rust --pattern 'pub use $$$ITEMS;' --json=stream \
    crates/nando-response-actor/src/lib.rs \
    | jq -s --slurpfile ownership "$script_dir/MODULE_OWNERSHIP.json" '
        map(select(.range.start.column == 0) | select(.text | startswith("pub use ")))
        | map(
            . as $statement
            | ($statement.text | capture("^pub use (?<module>[A-Za-z0-9_]+)::").module) as $module
            | ([
                $ownership[0][]
                | select(
                    .path == ("crates/nando-response-actor/src/" + $module + ".rs")
                    or .path == ("crates/nando-response-actor/src/" + $module + "/mod.rs")
                )
            ] | first) as $owner
            | {
                source_module: $module,
                source_path: $owner.path,
                target_owners: $owner.target_owners,
                split_required: $owner.split_required,
                line: ($statement.range.start.line + 1),
                statement: $statement.text,
                symbols: (
                    $statement.text
                    | capture("::\\{(?<body>[\\s\\S]*)\\};$").body
                    | split(",")
                    | map(gsub("^[[:space:]]+|[[:space:]]+$"; ""))
                    | map(select(length > 0))
                )
            }
        )
        | sort_by(.source_module)
    ' >"$script_dir/PUBLIC_API_OWNERSHIP.json"

if jq -e 'length == 47 and all(.source_path != null and (.target_owners | length > 0))' \
    "$script_dir/PUBLIC_API_OWNERSHIP.json" >/dev/null; then
    :
else
    printf 'public API ownership is incomplete\n' >&2
    exit 1
fi

rg -n --no-heading --glob '*.rs' \
    --glob '!crates/nando-response-actor/src/nando-online-response-diagnose.rs' \
    'nando_response_actor(?:::|\b)' crates \
    | sort >"$script_dir/EXTERNAL_CALLERS.txt" || true

rg -n --no-heading --glob '*.rs' \
    --glob '!crates/nando-response-actor/src/nando-online-response-diagnose.rs' \
    '(std::fs|std::process|std::env|File::|OpenOptions|Command::|env::|SystemTime|Instant::now|TcpStream|UdpSocket|tokio::net|checkpoint|authority|Authority|ACTIVE)' \
    crates/nando-response-actor/src \
    | sort >"$script_dir/SIDE_EFFECT_CALLS.txt" || true

cut -d: -f1 "$script_dir/SIDE_EFFECT_CALLS.txt" | sort -u >"$tmp_dir/side-effect-paths.txt"
if ! comm -23 "$tmp_dir/side-effect-paths.txt" "$tmp_dir/mapped-rs.txt" \
    >"$tmp_dir/unowned-side-effect-paths.txt"; then
    exit 1
fi
if test -s "$tmp_dir/unowned-side-effect-paths.txt"; then
    printf 'side-effect calls found in unowned files\n' >&2
    cat "$tmp_dir/unowned-side-effect-paths.txt" >&2
    exit 1
fi

"$sg_bin" run --lang rust --pattern 'pub const $NAME: $TYPE = $VALUE;' \
    --json=stream "${tracked_rs[@]}" \
    | jq -s '
        map(select(.metaVariables.single.NAME.text | test("SCHEMA")))
        | map({
            file,
            line: (.range.start.line + 1),
            name: .metaVariables.single.NAME.text,
            type: .metaVariables.single.TYPE.text,
            value: .metaVariables.single.VALUE.text
        })
        | sort_by(.file, .line, .name)
    ' >"$script_dir/SCHEMA_CONSTANTS.json"

git ls-files '*.json' | sort | while IFS= read -r path; do
    sha256sum "$path"
done >"$script_dir/TRACKED_JSON_SHA256.txt"

printf '%s\n' "${tracked_rs[@]}" | while IFS= read -r path; do
    sha256sum "$path"
done >"$script_dir/SOURCE_FILES_SHA256.txt"

cargo metadata --no-deps --format-version 1 \
    | jq '{
        workspace_root,
        target_directory,
        packages: [
            .packages[]
            | {
                name,
                manifest_path,
                dependencies: [
                    .dependencies[]
                    | {name, source, path, kind, optional}
                ] | sort_by(.name, .kind)
            }
        ] | sort_by(.name)
    }' >"$script_dir/DEPENDENCY_DAG.json"

visible_files=$(wc -l <"$tmp_dir/visible-rs.txt")
visible_lines=$(jq '[.[].lines] | add' "$script_dir/MODULE_OWNERSHIP.json")
tracked_files=$(jq '[.[] | select(.tracked)] | length' "$script_dir/MODULE_OWNERSHIP.json")
tracked_lines=$(jq '[.[] | select(.tracked) | .lines] | add' "$script_dir/MODULE_OWNERSHIP.json")
split_files=$(jq '[.[] | select(.split_required)] | length' "$script_dir/MODULE_OWNERSHIP.json")
public_exports=$("$sg_bin" run --lang rust --pattern 'pub use $$$ITEMS;' --json=stream \
    crates/nando-response-actor/src/lib.rs \
    | jq -s '[.[] | select(.range.start.column == 0) | select(.text | startswith("pub use "))] | length')
public_symbols=$(jq '[.[].symbols | length] | add' "$script_dir/PUBLIC_API_OWNERSHIP.json")
root_modules=$("$sg_bin" run --lang rust --pattern 'mod $NAME;' --json=stream \
    crates/nando-response-actor/src/lib.rs \
    | jq -s '[.[] | select(.range.start.column == 0)] | length')
bin_files=$(printf '%s\n' "${tracked_rs[@]}" | awk '/\/src\/bin\// {count++} END {print count + 0}')
bin_lines=$(jq '[.[] | select(.tracked and (.path | contains("/src/bin/"))) | .lines] | add // 0' \
    "$script_dir/MODULE_OWNERSHIP.json")
schema_constants=$(jq 'length' "$script_dir/SCHEMA_CONSTANTS.json")
tracked_json=$(wc -l <"$script_dir/TRACKED_JSON_SHA256.txt")

git status --porcelain=v1 | sed 's/^...//' | jq -Rsc 'split("\n") | map(select(length > 0))' \
    >"$tmp_dir/dirty.json"

jq -n \
    --arg schema "nando.response-actor-decomposition-baseline.v1" \
    --arg generated_at "2026-07-21" \
    --arg head "$(git rev-parse HEAD)" \
    --arg branch "$(git branch --show-current)" \
    --argjson visible_files "$visible_files" \
    --argjson visible_lines "$visible_lines" \
    --argjson tracked_files "$tracked_files" \
    --argjson tracked_lines "$tracked_lines" \
    --argjson split_files "$split_files" \
    --argjson root_modules "$root_modules" \
    --argjson public_exports "$public_exports" \
    --argjson public_symbols "$public_symbols" \
    --argjson bin_files "$bin_files" \
    --argjson bin_lines "$bin_lines" \
    --argjson schema_constants "$schema_constants" \
    --argjson tracked_json "$tracked_json" \
    --slurpfile dirty "$tmp_dir/dirty.json" \
    '{
        schema: $schema,
        generated_at: $generated_at,
        head: $head,
        branch: $branch,
        authority: false,
        f5_b_started: false,
        source: {
            visible_rust_files: $visible_files,
            visible_rust_lines: $visible_lines,
            tracked_rust_files: $tracked_files,
            tracked_rust_lines: $tracked_lines,
            split_required_files: $split_files,
            root_modules: $root_modules,
            public_reexport_statements: $public_exports,
            public_reexport_symbols: $public_symbols,
            binary_files: $bin_files,
            binary_lines: $bin_lines,
            schema_constants: $schema_constants,
            tracked_json_files: $tracked_json
        },
        dirty_paths_preserved: $dirty[0]
    }' >"$script_dir/BASELINE_STATIC.json"

sha256sum \
    "$script_rel/BASELINE_STATIC.json" \
    "$script_rel/MODULE_OWNERSHIP.json" \
    "$script_rel/PUBLIC_API_SURFACE.txt" \
    "$script_rel/PUBLIC_API_OWNERSHIP.json" \
    "$script_rel/EXTERNAL_CALLERS.txt" \
    "$script_rel/SIDE_EFFECT_CALLS.txt" \
    "$script_rel/SCHEMA_CONSTANTS.json" \
    "$script_rel/TRACKED_JSON_SHA256.txt" \
    "$script_rel/SOURCE_FILES_SHA256.txt" \
    "$script_rel/DEPENDENCY_DAG.json" \
    >"$script_rel/STATIC_RECEIPT.sha256"

printf 'R0 static baseline generated: %s\n' "$script_dir"
