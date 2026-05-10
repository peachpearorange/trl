#!/bin/sh
# Wrapper around wasm-server-runner that launches Chromium in app mode.
# Used as the cargo runner for wasm32-unknown-unknown.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
export WASM_SERVER_RUNNER_CUSTOM_INDEX_HTML="$SCRIPT_DIR/dev-index.html"

# Kill any previous chromium instance we spawned (by matching the app URL).
pkill -f 'chromium.*--app=http://127.0.0.1:133' 2>/dev/null

# Launch wasm-server-runner in the background so we can poll for readiness.
wasm-server-runner "$@" &
SERVER_PID=$!

# Wait until the server is accepting connections.
until curl -s -o /dev/null http://127.0.0.1:1334 2>/dev/null; do
  sleep 0.1
done

chromium \
  --app=http://127.0.0.1:1334 \
  --disable-http-cache \
  --disk-cache-size=1

kill $SERVER_PID 2>/dev/null
