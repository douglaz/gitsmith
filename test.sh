#!/usr/bin/env bash
# Convenience script to run GitSmith integration tests with automatic relay setup

set -e

echo "🔨 Running GitSmith integration tests..."
echo

# Check if we're in nix develop shell
if [ -z "$IN_NIX_SHELL" ]; then
    echo "📦 Entering Nix development environment..."
    exec nix develop -c "$0" "$@"
fi

# Default to 'all' command if no arguments provided
if [ $# -eq 0 ]; then
    set -- all
fi

# Run the integration tests
cargo run --bin gitsmith-integration-tests -- "$@"