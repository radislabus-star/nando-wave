#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(git -C "$script_dir" rev-parse --show-toplevel)
script_rel=${script_dir#"$repo_root"/}
test_log="$script_dir/FULL_LIB_TEST_BASELINE.log"
clippy_log="$script_dir/CLIPPY_BASELINE.log"

for path in "$test_log" "$clippy_log"; do
    test -s "$path" || {
        printf 'missing baseline log: %s\n' "$path" >&2
        exit 1
    }
done

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

last_failures_line=$(grep -n '^failures:$' "$test_log" | tail -n 1 | cut -d: -f1)
tail -n "+$((last_failures_line + 1))" "$test_log" \
    | sed -n '/^test result:/q; s/^    \([A-Za-z0-9_][A-Za-z0-9_:]*\)$/\1/p' \
    | sort -u >"$script_dir/KNOWN_TEST_FAILURES.txt"

awk '
    /^error: / && $0 !~ /could not compile/ {
        message = substr($0, 8)
        next
    }
    message != "" && /^[[:space:]]*--> crates\/nando-response-actor\/src\// {
        location = $0
        sub(/^[[:space:]]*--> /, "", location)
        print location "\t" message
        message = ""
    }
' "$clippy_log" | sort -u >"$script_dir/KNOWN_CLIPPY_DIAGNOSTICS.tsv"

test_summary=$(grep '^test result:' "$test_log" | tail -n 1)
passed=$(sed -n 's/.* \([0-9][0-9]*\) passed;.*/\1/p' <<<"$test_summary")
failed=$(sed -n 's/.* passed; \([0-9][0-9]*\) failed;.*/\1/p' <<<"$test_summary")
known_failures=$(wc -l <"$script_dir/KNOWN_TEST_FAILURES.txt")
clippy_total=$(wc -l <"$script_dir/KNOWN_CLIPPY_DIAGNOSTICS.tsv")
clippy_library=$(sed -n 's/.*(lib) due to \([0-9][0-9]*\) previous errors.*/\1/p' "$clippy_log" | tail -n 1)
clippy_all=$(sed -n 's/.*(lib test) due to \([0-9][0-9]*\) previous errors.*/\1/p' "$clippy_log" | tail -n 1)
clippy_test_only=$((clippy_all - clippy_library))

test "$failed" -eq "$known_failures"
test "$clippy_total" -eq "$clippy_all"

jq -n \
    --arg schema "nando.response-actor-behavior-baseline.v1" \
    --arg head "$(git -C "$script_dir" rev-parse HEAD)" \
    --argjson passed "$passed" \
    --argjson failed "$failed" \
    --argjson clippy_library "$clippy_library" \
    --argjson clippy_test_only "$clippy_test_only" \
    --arg test_failure_set_sha256 "$(sha256sum "$script_dir/KNOWN_TEST_FAILURES.txt" | awk '{print $1}')" \
    --arg clippy_set_sha256 "$(sha256sum "$script_dir/KNOWN_CLIPPY_DIAGNOSTICS.tsv" | awk '{print $1}')" \
    '{
        schema: $schema,
        head: $head,
        authority: false,
        full_lib: {
            passed: $passed,
            known_failed: $failed,
            failure_set_sha256: $test_failure_set_sha256
        },
        clippy: {
            library_diagnostics: $clippy_library,
            test_only_diagnostics: $clippy_test_only,
            total_diagnostics: ($clippy_library + $clippy_test_only),
            diagnostic_set_sha256: $clippy_set_sha256
        }
    }' >"$script_dir/BEHAVIOR_BASELINE.json"

(
    cd "$repo_root"
    sha256sum \
        "$script_rel/BEHAVIOR_BASELINE.json" \
        "$script_rel/KNOWN_TEST_FAILURES.txt" \
        "$script_rel/KNOWN_CLIPPY_DIAGNOSTICS.tsv"
) >"$script_dir/BEHAVIOR_RECEIPT.sha256"

printf 'R0 behavior baseline generated: %s\n' "$script_dir"
