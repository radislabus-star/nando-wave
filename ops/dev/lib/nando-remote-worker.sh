#!/usr/bin/env bash
set -euo pipefail

profile=$1
source_path=$2
target_dir=$3
package=$4
result_dir=$5
target_budget_bytes=$6
shift 6
filters=("$@")

cargo_bin=${NANDO_REMOTE_CARGO:-/home/e/.cargo/bin/cargo}
mkdir -p "$result_dir"
cd "$source_path"

case "$target_dir" in
    /home/e/projects/nando-wave-build/target-*) ;;
    *)
        printf 'unsafe remote target path: %s\n' "$target_dir" >&2
        exit 2
        ;;
esac

if test "$profile" = stop; then
    rm -rf "$target_dir"
fi
mkdir -p "$target_dir"

case "$profile" in
    fast) incremental=1 ;;
    stop | release) incremental=0 ;;
    *)
        printf 'unknown worker profile: %s\n' "$profile" >&2
        exit 2
        ;;
esac

export CARGO_TARGET_DIR="$target_dir"
export CARGO_INCREMENTAL="$incremental"

now_ns() {
    date +%s%N
}

compile_started=$(now_ns)
set +e
if test "$profile" = release; then
    "$cargo_bin" test -p "$package" --lib --no-run --release --message-format=json \
        >"$result_dir/cargo-events.jsonl" 2>"$result_dir/compile.log"
else
    "$cargo_bin" test -p "$package" --lib --no-run --message-format=json \
        >"$result_dir/cargo-events.jsonl" 2>"$result_dir/compile.log"
fi
compile_exit=$?
set -e
compile_finished=$(now_ns)

test_binary=$(jq -r '
    select(.reason == "compiler-artifact")
    | select(.profile.test == true)
    | select(.target.kind | index("lib"))
    | .executable // empty
' "$result_dir/cargo-events.jsonl" | tail -n 1)

test_exit=125
test_started=$(now_ns)
: >"$result_dir/test.log"
if test "$compile_exit" -eq 0 && test -n "$test_binary"; then
    test_exit=0
    if test "$profile" = fast; then
        for filter in "${filters[@]}"; do
            printf 'FILTER %s\n' "$filter" >>"$result_dir/test.log"
            set +e
            "$test_binary" "$filter" --format terse >>"$result_dir/test.log" 2>&1
            filter_exit=$?
            set -e
            if test "$filter_exit" -ne 0; then
                test_exit=$filter_exit
            fi
        done
    elif test "$profile" = stop; then
        set +e
        "$test_binary" --format terse >"$result_dir/test.log" 2>&1
        test_exit=$?
        set -e
    else
        set +e
        "$test_binary" --list >"$result_dir/test.log" 2>&1
        test_exit=$?
        set -e
    fi
fi
test_finished=$(now_ns)

clippy_exit=125
: >"$result_dir/clippy.log"
if test "$profile" = stop && test "$compile_exit" -eq 0; then
    set +e
    "$cargo_bin" clippy -p "$package" --all-targets -- -D warnings \
        >"$result_dir/clippy.log" 2>&1
    clippy_exit=$?
    set -e
fi

target_size_before=$(du -sb "$target_dir" | awk '{print $1}')
target_prune=none
if test "$target_size_before" -gt "$target_budget_bytes"; then
    find "$target_dir" -type d -name incremental -prune -exec rm -rf {} +
    target_prune=incremental
    target_size_after_incremental=$(du -sb "$target_dir" | awk '{print $1}')
    if test "$target_size_after_incremental" -gt "$target_budget_bytes"; then
        rm -rf "$target_dir"
        mkdir -p "$target_dir"
        target_prune=full
    fi
fi
target_size_after=$(du -sb "$target_dir" | awk '{print $1}')

worker_verdict=PASS
if test "$compile_exit" -ne 0; then
    worker_verdict=FAIL
elif test "$profile" != stop && test "$test_exit" -ne 0; then
    worker_verdict=FAIL
fi

jq -n \
    --arg verdict "$worker_verdict" \
    --arg test_binary "$test_binary" \
    --arg target_dir "$target_dir" \
    --arg target_prune "$target_prune" \
    --argjson compile_exit "$compile_exit" \
    --argjson test_exit "$test_exit" \
    --argjson clippy_exit "$clippy_exit" \
    --argjson compile_ns "$((compile_finished - compile_started))" \
    --argjson test_ns "$((test_finished - test_started))" \
    --argjson target_size_before "$target_size_before" \
    --argjson target_size_after "$target_size_after" \
    '{
        worker_verdict: $verdict,
        compile_exit: $compile_exit,
        test_exit: $test_exit,
        clippy_exit: $clippy_exit,
        compile_seconds: ($compile_ns / 1000000000),
        test_seconds: ($test_ns / 1000000000),
        test_binary: $test_binary,
        target: {
            path: $target_dir,
            bytes_before_prune: $target_size_before,
            bytes_after_prune: $target_size_after,
            prune: $target_prune
        }
    }' >"$result_dir/worker-summary.json"
