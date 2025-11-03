#!/bin/bash
set -e

echo "🦀 Publishing deckyfx-dioxus-ipc-bridge to crates.io..."
echo ""

# Check if logged in
echo "🔐 Checking cargo login..."
if ! cargo login --help &> /dev/null; then
    echo "❌ Please run 'cargo login' first with your crates.io API token"
    exit 1
fi

# Check if macros dependency is published
echo "📦 Checking if deckyfx-dioxus-ipc-bridge-macros is available..."
if ! cargo search deckyfx-dioxus-ipc-bridge-macros | grep -q "deckyfx-dioxus-ipc-bridge-macros"; then
    echo "❌ Error: deckyfx-dioxus-ipc-bridge-macros must be published first"
    exit 1
fi

echo "🧪 Running dry run..."
cargo publish --dry-run

echo ""
echo "🚀 Publishing to crates.io..."
cargo publish

echo ""
echo "✅ deckyfx-dioxus-ipc-bridge published successfully!"
echo ""
echo "Verify at: https://crates.io/crates/deckyfx-dioxus-ipc-bridge"
echo ""
