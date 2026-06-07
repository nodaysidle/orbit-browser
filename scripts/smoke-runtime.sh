#!/bin/bash
set -euo pipefail

APP_PATH="${ORBIT_APP_PATH:-/Applications/Orbit.app}"
DB_PATH="${ORBIT_DB_PATH:-$HOME/Library/Application Support/com.orbit.browser/orbit.db}"
GOOD_URL_PATTERN="httpbin.org/get"
BLOCKED_URL_PATTERN="doubleclick.net/ad"

history_visits() {
  local pattern="$1"
  if [ ! -f "$DB_PATH" ]; then
    echo 0
    return
  fi
  python3 - "$DB_PATH" "$pattern" <<'PY'
import sqlite3
import sys

db_path, pattern = sys.argv[1], sys.argv[2]
with sqlite3.connect(db_path) as conn:
    try:
        count = conn.execute(
            "SELECT COALESCE(SUM(visit_count), 0) FROM history WHERE url LIKE ?",
            (f"%{pattern}%",),
        ).fetchone()[0]
    except sqlite3.Error:
        count = 0
print(count)
PY
}

if [ ! -d "$APP_PATH" ]; then
  echo "Orbit app not found at $APP_PATH"
  echo "Build and install Orbit before running this smoke test."
  exit 1
fi

before_good_visits="$(history_visits "$GOOD_URL_PATTERN")"
before_blocked_visits="$(history_visits "$BLOCKED_URL_PATTERN")"

osascript -e 'tell application "Orbit" to quit' >/dev/null 2>&1 || true
for _ in {1..20}; do
  if ! pgrep -x Orbit >/dev/null && ! pgrep -x orbit >/dev/null; then
    break
  fi
  sleep 0.25
done

echo "Launching Orbit from $APP_PATH..."
open -na "$APP_PATH"

for _ in {1..20}; do
  if pgrep -x Orbit >/dev/null || pgrep -x orbit >/dev/null; then
    break
  fi
  sleep 0.25
done

if ! pgrep -x Orbit >/dev/null && ! pgrep -x orbit >/dev/null; then
  echo "Orbit did not start."
  exit 1
fi

echo "Driving runtime smoke flow: new tab, httpbin navigation, blocked domain, close tab..."
if ! osascript <<'APPLESCRIPT'
tell application "Orbit" to activate
delay 1

tell application "System Events"
  if not (exists process "Orbit") then error "Orbit process is not available to System Events"
  tell process "Orbit" to set frontmost to true

  keystroke "t" using command down
  delay 0.5

  keystroke "l" using command down
  delay 0.2
  keystroke "https://httpbin.org/get"
  key code 36
  delay 4

  keystroke "l" using command down
  delay 0.2
  keystroke "https://doubleclick.net/ad"
  key code 36
  delay 2

  keystroke "w" using command down
  delay 0.5
end tell
APPLESCRIPT
then
  echo "Runtime smoke failed. If macOS denied automation, grant this terminal Accessibility permission and retry."
  exit 1
fi

if ! pgrep -x Orbit >/dev/null && ! pgrep -x orbit >/dev/null; then
  echo "Orbit exited during smoke flow."
  exit 1
fi

after_good_visits="$(history_visits "$GOOD_URL_PATTERN")"
after_blocked_visits="$(history_visits "$BLOCKED_URL_PATTERN")"

if [ "$after_good_visits" -le "$before_good_visits" ]; then
  echo "Runtime smoke failed: httpbin navigation was not recorded in history."
  echo "History visits for $GOOD_URL_PATTERN: before=$before_good_visits after=$after_good_visits"
  exit 1
fi

if [ "$after_blocked_visits" -ne "$before_blocked_visits" ]; then
  echo "Runtime smoke failed: blocked domain was recorded in history."
  echo "History visits for $BLOCKED_URL_PATTERN: before=$before_blocked_visits after=$after_blocked_visits"
  exit 1
fi

echo "History evidence: $GOOD_URL_PATTERN visits $before_good_visits -> $after_good_visits"
echo "Adblock evidence: $BLOCKED_URL_PATTERN visits remained $after_blocked_visits"
echo "Runtime smoke completed."
