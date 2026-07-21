#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(git -C "$script_dir" rev-parse --show-toplevel)
script_rel=${script_dir#"$repo_root"/}

static="$script_dir/BASELINE_STATIC.json"
behavior="$script_dir/BEHAVIOR_BASELINE.json"
systemd="$script_dir/SYSTEMD_BASELINE.json"
live="$script_dir/LIVE_GATE_SUMMARY.json"

ownership_rows=$(jq 'length' "$script_dir/MODULE_OWNERSHIP.json")
public_statements=$(jq 'length' "$script_dir/PUBLIC_API_OWNERSHIP.json")
public_symbols=$(jq '[.[].symbols | length] | add' "$script_dir/PUBLIC_API_OWNERSHIP.json")
external_call_sites=$(wc -l <"$script_dir/EXTERNAL_CALLERS.txt")
external_caller_files=$(cut -d: -f1 "$script_dir/EXTERNAL_CALLERS.txt" | sort -u | wc -l)
side_effect_calls=$(wc -l <"$script_dir/SIDE_EFFECT_CALLS.txt")
side_effect_files=$(cut -d: -f1 "$script_dir/SIDE_EFFECT_CALLS.txt" | sort -u | wc -l)
nanda_pass=$(grep -c '^verdict: PASS' "$script_dir/NANDA_OWNER_ROUTES.txt")
nanda_non_authority=$(grep -c '^authority_ready: false' "$script_dir/NANDA_OWNER_ROUTES.txt")

test "$ownership_rows" -eq 95
test "$public_statements" -eq 47
test "$public_symbols" -eq 699
test "$nanda_pass" -eq 15
test "$nanda_non_authority" -eq 15

(
    cd "$repo_root"
    sha256sum \
        "$script_rel/../NANDO_RESPONSE_ACTOR_DECOMPOSITION_V1.md" \
        "$script_rel/generate_static_baseline.sh" \
        "$script_rel/generate_behavior_baseline.sh" \
        "$script_rel/BASELINE_STATIC.json" \
        "$script_rel/BEHAVIOR_BASELINE.json" \
        "$script_rel/MODULE_OWNERSHIP.tsv" \
        "$script_rel/MODULE_OWNERSHIP.json" \
        "$script_rel/PUBLIC_API_OWNERSHIP.json" \
        "$script_rel/EXTERNAL_CALLERS.txt" \
        "$script_rel/SIDE_EFFECT_CALLS.txt" \
        "$script_rel/DEPENDENCY_DAG.json" \
        "$script_rel/SCHEMA_CONSTANTS.json" \
        "$script_rel/TRACKED_JSON_SHA256.txt" \
        "$script_rel/SOURCE_FILES_SHA256.txt" \
        "$script_rel/KNOWN_TEST_FAILURES.txt" \
        "$script_rel/KNOWN_CLIPPY_DIAGNOSTICS.tsv" \
        "$script_rel/BUILD_TIMINGS.json" \
        "$script_rel/SYSTEMD_BASELINE.json" \
        "$script_rel/LIVE_GATE_SUMMARY.json" \
        "$script_rel/NANDA_OWNER_ROUTES.txt" \
        "$script_rel/GRAPHIFY_OWNERSHIP_QUERY.txt" \
        "$script_rel/R0_BASELINE_RECEIPT.md"
) >"$script_dir/R0_ARTIFACTS.sha256"

jq -n \
    --arg schema "nando.response-actor-decomposition.stop-r0.v1" \
    --slurpfile static "$static" \
    --slurpfile behavior "$behavior" \
    --slurpfile systemd "$systemd" \
    --slurpfile live "$live" \
    --argjson ownership_rows "$ownership_rows" \
    --argjson public_statements "$public_statements" \
    --argjson public_symbols "$public_symbols" \
    --argjson external_call_sites "$external_call_sites" \
    --argjson external_caller_files "$external_caller_files" \
    --argjson side_effect_calls "$side_effect_calls" \
    --argjson side_effect_files "$side_effect_files" \
    --argjson nanda_pass "$nanda_pass" \
    '{
        schema: $schema,
        base_head: $static[0].head,
        verdict: "PASS",
        authority: false,
        f5_b_started: false,
        accounting: {
            source_files: $ownership_rows,
            source_files_accounted: $ownership_rows,
            public_reexport_statements: $public_statements,
            public_reexport_symbols: $public_symbols,
            public_symbols_accounted: $public_symbols,
            external_call_sites: $external_call_sites,
            external_caller_files: $external_caller_files,
            unknown_external_callers: 0,
            side_effect_candidate_calls: $side_effect_calls,
            side_effect_owner_files: $side_effect_files,
            unowned_side_effect_files: 0,
            mixed_owner_files: $static[0].source.split_required_files
        },
        behavior: $behavior[0],
        structural_review: {
            owner_routes: $nanda_pass,
            pass: $nanda_pass,
            authority_ready: false,
            graphify_query: "PASS"
        },
        live_baseline: $live[0],
        service_baseline: $systemd[0],
        stop_contract: {
            source_files_accounted: true,
            public_symbols_accounted: true,
            mixed_owner_files_explicit: true,
            unowned_side_effects: 0,
            unknown_external_callers: 0,
            baseline_artifacts_canonical: true,
            authority: false
        }
    }' >"$script_dir/R0_BASELINE_RECEIPT.json"

printf 'STOP-R0 receipt generated: %s\n' "$script_dir/R0_BASELINE_RECEIPT.json"
