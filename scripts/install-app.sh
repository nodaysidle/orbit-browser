#!/bin/bash
set -euo pipefail

APP_NAME="Orbit.app"
SOURCE_APP="src-tauri/target/release/bundle/macos/$APP_NAME"
TARGET_APP="/Applications/$APP_NAME"
STAGED_APP="/Applications/$APP_NAME.new"
BACKUP_APP=""

cleanup() {
  rm -rf "$STAGED_APP"
}
trap cleanup EXIT

if [ ! -d "$SOURCE_APP" ]; then
  echo "No app bundle found at $SOURCE_APP"
  echo "Run ./scripts/build-mac.sh first."
  exit 1
fi

echo "Installing $APP_NAME to /Applications..."

osascript -e 'tell application "Orbit" to quit' >/dev/null 2>&1 || true
for _ in {1..20}; do
  if ! pgrep -x Orbit >/dev/null && ! pgrep -x orbit >/dev/null; then
    break
  fi
  sleep 0.25
done

rm -rf "$STAGED_APP"
ditto "$SOURCE_APP" "$STAGED_APP"
codesign --verify --deep --strict --verbose=2 "$STAGED_APP"

if [ -d "$TARGET_APP" ]; then
  BACKUP_APP="/Applications/$APP_NAME.backup.$(date +%Y%m%d%H%M%S)"
  echo "Preserving existing installation..."
  mv "$TARGET_APP" "$BACKUP_APP"
fi

if ! mv "$STAGED_APP" "$TARGET_APP"; then
  if [ -n "$BACKUP_APP" ] && [ -d "$BACKUP_APP" ]; then
    mv "$BACKUP_APP" "$TARGET_APP"
  fi
  echo "Install failed; previous Orbit.app was restored."
  exit 1
fi

if ! codesign --verify --deep --strict --verbose=2 "$TARGET_APP"; then
  rm -rf "$TARGET_APP"
  if [ -n "$BACKUP_APP" ] && [ -d "$BACKUP_APP" ]; then
    mv "$BACKUP_APP" "$TARGET_APP"
  fi
  echo "Installed app failed verification; previous Orbit.app was restored."
  exit 1
fi

if [ -n "$BACKUP_APP" ] && [ -d "$BACKUP_APP" ]; then
  rm -rf "$BACKUP_APP"
fi

echo "Orbit installed successfully."
echo "Location: $TARGET_APP"
