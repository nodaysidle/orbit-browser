#!/bin/bash
set -e

echo "🚀 Building Orbit for macOS..."

# Clean previous builds
echo "🧹 Cleaning previous builds..."
rm -rf release out

# Install dependencies
echo "📦 Installing dependencies..."
npm ci

# Run tests
echo "🧪 Running tests..."
npm run test

# Build TypeScript
echo "🔨 Building TypeScript..."
npm run build

# Package Electron app
echo "📦 Packaging Electron app..."
npx electron-builder --mac

echo "✅ Build complete!"
echo ""
echo "📦 Output files:"
ls -lh release/
