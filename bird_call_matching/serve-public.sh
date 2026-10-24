#!/usr/bin/env bash
# Serves Go Birdy over the internet through an ngrok tunnel.
#
# Builds a release bundle (much smaller and faster than `dx serve`'s debug build), runs its
# server on this machine, and points an ngrok tunnel at it. No router or firewall changes are
# needed: ngrok connects outward from this machine, and players reach the game through the public
# https URL ngrok prints. Quitting ngrok (Ctrl+C) also stops the game server.
#
# Usage:
#   ./serve-public.sh                 # build, then serve
#   ./serve-public.sh --skip-build    # serve the last build again
#   NGROK_URL=your-name.ngrok-free.app ./serve-public.sh   # use your reserved ngrok domain
#   PORT=9000 ./serve-public.sh       # run the game server on another local port
set -euo pipefail
cd "$(dirname "$0")"

# The game server's local port. Not 8080, so it can run alongside `dx serve`.
PORT="${PORT:-8090}"
# Deno also installs a command called `dx`, so call the Dioxus CLI by its full path.
DX="$HOME/.cargo/bin/dx"
BUNDLE_DIR="target/dx/bird_call_matching/release/web"

if ! command -v ngrok >/dev/null; then
    echo "ngrok isn't installed: https://ngrok.com/download" >&2
    exit 1
fi

if [[ "${1:-}" != "--skip-build" ]]; then
    "$DX" bundle --web --release
fi

if lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "Port $PORT is already in use; pick another with PORT=..." >&2
    exit 1
fi

# The server finds its static files (including the birds) in the `public` folder next to it.
IP=127.0.0.1 PORT="$PORT" "$BUNDLE_DIR/server" &
server_pid=$!
trap 'kill "$server_pid" 2>/dev/null' EXIT

echo "Waiting for the game server on port $PORT..."
until curl -fsS -o /dev/null "http://127.0.0.1:$PORT/"; do
    if ! kill -0 "$server_pid" 2>/dev/null; then
        echo "The game server exited before it was ready." >&2
        exit 1
    fi
    sleep 1
done

ngrok http "$PORT" ${NGROK_URL:+--url "$NGROK_URL"}
