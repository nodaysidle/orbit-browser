#!/bin/bash
# Install Orbit to /Applications

APP_NAME="Orbit.app"

# Find the latest DMG
DMG_PATH=$(ls -t release/*.dmg 2>/dev/null | head -1)

if [ -z "$DMG_PATH" ]; then
  echo "❌ No DMG found in release/ directory"
  echo "Run ./scripts/build-mac.sh first"
  exit 1
fi

echo "📦 Found: $DMG_PATH"
echo "💿 Mounting..."

# Mount the DMG
MOUNT_INFO=$(hdiutil attach "$DMG_PATH" -nobrowse -readonly)
MOUNT_POINT=$(echo "$MOUNT_INFO" | grep -o '/Volumes/[^ ]*' | tail -1)

if [ -z "$MOUNT_POINT" ]; then
  echo "❌ Failed to mount DMG"
  exit 1
fi

echo "📂 Mounted at: $MOUNT_POINT"
echo "🚀 Installing to /Applications..."

# Copy app to Applications
if [ -d "/Applications/$APP_NAME" ]; then
  echo "🗑️  Removing existing installation..."
  rm -rf "/Applications/$APP_NAME"
fi

cp -R "$MOUNT_POINT/$APP_NAME" /Applications/

echo "💨 Unmounting..."
hdiutil detach "$MOUNT_POINT" -quiet

echo ""
echo "✅ Orbit installed successfully!"
echo "📍 Location: /Applications/$APP_NAME"
echo ""
echo "🎉 You can now launch Orbit from Applications or Spotlight (⌘Space)"
