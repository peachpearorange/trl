#!/bin/sh
# Wrapper around wasm-server-runner that launches Chromium in app/kiosk mode.
# Used as the cargo runner for wasm32-unknown-unknown.

# Kill any previous chromium instance we spawned (by matching the app URL).
pkill -f 'chromium.*--app=http://127.0.0.1:133' 2>/dev/null

# Small delay so the server has time to bind before the browser connects.
(sleep 1 && chromium \
  --app=http://127.0.0.1:1334 \
  --start-fullscreen \
  --disable-http-cache \
  --disk-cache-size=1 \
  &) &

exec wasm-server-runner "$@"
