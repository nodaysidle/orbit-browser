#!/bin/bash
set -euo pipefail

echo "Building Orbit for macOS..."

echo "Installing frontend dependencies..."
npm ci

echo "Running frontend tests..."
npm test

echo "Building frontend..."
npm run build

echo "Running Rust checks..."
(
  cd src-tauri
  cargo fmt --check
  cargo test
  cargo clippy -- -D warnings
)

echo "Packaging Tauri app..."
CI=false npm run tauri -- build

echo "Build complete."
echo "App bundle: src-tauri/target/release/bundle/macos/Orbit.app"
codesign --verify --deep --strict --verbose=2 src-tauri/target/release/bundle/macos/Orbit.app
