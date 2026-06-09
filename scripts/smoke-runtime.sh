#!/bin/bash
set -euo pipefail

APP_PATH="${ORBIT_APP_PATH:-/Applications/Orbit.app}"
DB_PATH="${ORBIT_DB_PATH:-$HOME/Library/Application Support/com.orbit.browser/orbit.db}"
GOOD_URL_PATTERN="httpbin.org/get"
REORDER_URL_PATTERN="www.iana.org/help/example-domains"
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

setting_value() {
  local key="$1"
  if [ ! -f "$DB_PATH" ]; then
    echo ""
    return
  fi
  python3 - "$DB_PATH" "$key" <<'PY'
import sqlite3
import sys

db_path, key = sys.argv[1], sys.argv[2]
with sqlite3.connect(db_path) as conn:
    try:
        row = conn.execute("SELECT value FROM settings WHERE key = ?", (key,)).fetchone()
    except sqlite3.Error:
        row = None
print(row[0] if row else "")
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

echo "Driving runtime smoke flow: new tabs, navigation, keyboard tab reorder, blocked domain..."
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

  keystroke "t" using command down
  delay 0.5

  keystroke "l" using command down
  delay 0.2
  keystroke "https://www.iana.org/help/example-domains"
  key code 36
  delay 4

  key code 123 using {command down, option down, shift down}
  delay 1

  keystroke "l" using command down
  delay 0.2
  keystroke "https://doubleclick.net/ad"
  key code 36
  delay 2
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
session_tabs_after_reorder="$(setting_value "session_tabs")"

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

if ! python3 - "$session_tabs_after_reorder" <<'PY'
import json
import sys

try:
    tabs = json.loads(sys.argv[1] or "[]")
except json.JSONDecodeError:
    sys.exit(1)

normalized = [str(tab).lower() for tab in tabs]
iana_indices = [index for index, value in enumerate(normalized) if "www.iana.org/help/example-domains" in value]
httpbin_indices = [index for index, value in enumerate(normalized) if "httpbin.org/get" in value]
if not iana_indices or not httpbin_indices:
    sys.exit(1)
if not any(iana_index < httpbin_index for iana_index in iana_indices for httpbin_index in httpbin_indices):
    sys.exit(1)
PY
then
  echo "Runtime smoke failed: keyboard tab reorder was not persisted before the original httpbin tab."
  echo "session_tabs=$session_tabs_after_reorder"
  exit 1
fi

echo "Closing active tab after reorder proof..."
osascript <<'APPLESCRIPT' >/dev/null 2>&1 || true
tell application "Orbit" to activate
delay 0.2
tell application "System Events"
  tell process "Orbit" to set frontmost to true
  keystroke "w" using command down
end tell
APPLESCRIPT

echo "History evidence: $GOOD_URL_PATTERN visits $before_good_visits -> $after_good_visits"
echo "Adblock evidence: $BLOCKED_URL_PATTERN visits remained $after_blocked_visits"
echo "Tab reorder evidence: $REORDER_URL_PATTERN persisted before $GOOD_URL_PATTERN"
echo "Runtime smoke completed."
