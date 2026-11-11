#!/bin/sh
# Copy the Dioxus web bundle (wasm/js/css) of the *deployed* Railway build into cloudflare/public,
# so the hashed asset names match the HTML the backend renders. index.html is not copied:
# "/" falls through to the Worker and is server-rendered by Railway.
#   usage: ./build-assets.sh [BACKEND_URL]
set -e
cd "$(dirname "$0")"
BACKEND=${1:-https://dioxus-shorturl-production.up.railway.app}
rm -rf public && mkdir -p public/assets
for path in $(curl -fsS "$BACKEND/" | grep -oE '/assets/[A-Za-z0-9_.-]+\.(js|wasm|css)' | sort -u); do
  curl -fsS "$BACKEND$path" -o "public$path"
done
# the JS loader fetches the wasm itself, so it is not referenced from the HTML
for js in public/assets/*.js; do
  for wasm in $(grep -oE '[A-Za-z0-9_-]+_bg-dxh[0-9a-f]+\.wasm' "$js" | sort -u); do
    [ -f "public/assets/$wasm" ] || curl -fsS "$BACKEND/assets/$wasm" -o "public/assets/$wasm"
  done
done
echo "assets ready:" && find public -type f
