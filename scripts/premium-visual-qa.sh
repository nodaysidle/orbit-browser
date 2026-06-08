#!/bin/bash
set -euo pipefail

PORT="${ORBIT_VISUAL_QA_PORT:-4178}"
URL="http://127.0.0.1:${PORT}/"
OUT_DIR="${ORBIT_VISUAL_QA_OUT:-artifacts/premium-visual-qa}"
SERVER_LOG="${OUT_DIR}/vite.log"

mkdir -p "$OUT_DIR"

if curl -s -o /dev/null -w '%{http_code}' "$URL" | grep -q '^200$'; then
  SERVER_PID=""
else
  npm run dev -- --host 127.0.0.1 --port "$PORT" >"$SERVER_LOG" 2>&1 &
  SERVER_PID="$!"
  cleanup() {
    if [ -n "${SERVER_PID:-}" ]; then
      kill "$SERVER_PID" >/dev/null 2>&1 || true
    fi
  }
  trap cleanup EXIT

  for _ in {1..40}; do
    if curl -s -o /dev/null -w '%{http_code}' "$URL" | grep -q '^200$'; then
      break
    fi
    sleep 0.25
  done
fi

if ! curl -s -o /dev/null -w '%{http_code}' "$URL" | grep -q '^200$'; then
  echo "Visual QA failed: Vite server not reachable at $URL"
  exit 1
fi

ORBIT_VISUAL_QA_URL="$URL" ORBIT_VISUAL_QA_OUT="$OUT_DIR" node scripts/premium-visual-qa.mjs
