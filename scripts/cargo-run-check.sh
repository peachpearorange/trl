#!/usr/bin/env bash
# Runs `cargo run`, kills the process after it survives TIMEOUT seconds of
# runtime (i.e. no startup panic), and prints only the post-build output.
# Exit code: 0 if the game launched successfully, 1 if it panicked.

TIMEOUT="${1:-5}"

tmpfile=$(mktemp)
trap 'rm -f "$tmpfile"' EXIT

cargo run --color=never 2>&1 > "$tmpfile" &
cargo_pid=$!

# Wait for build to finish: poll until "Running `target/" appears or cargo dies
while ! grep -q 'Running `target/' "$tmpfile" 2>/dev/null; do
  kill -0 "$cargo_pid" 2>/dev/null || break
  sleep 2
done

# Now the binary is running. Give it TIMEOUT seconds to crash or prove stable.
sleep "$TIMEOUT"

# Kill the game if still alive (success case — it didn't panic)
pkill -f 'target/debug/trl' 2>/dev/null
wait "$cargo_pid" 2>/dev/null

# Print only lines after "Running `target/..." (skip build/warning output)
sed -n '/Running `target\//,$p' "$tmpfile" | tail -n +2

# Exit 1 if a panic happened
grep -q 'panicked at' "$tmpfile" && exit 1
exit 0
