#!/usr/bin/env sh
set -eu

# First-time setup script for SKM
# This script initializes the base configuration and caches the default registry

echo "SKM First-Time Setup"
echo "==================="
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Check if skm is available
if ! command -v skm >/dev/null 2>&1; then
    echo "skm command not found. Please install skm first."
    echo "You can install it from: https://github.com/skills-yaml/skm"
    exit 1
fi

# Run the setup command
echo "Initializing base configuration..."
skm init-config

echo ""
echo "Updating skill registry cache..."
skm cache-update

echo ""
echo "First-time setup completed!"
echo ""
echo "Next steps:"
echo "  1. Navigate to your project directory"
echo "  2. Run: skm init"
echo "  3. Run: skm install"
echo "  4. Run: skm list to verify"
echo ""
echo "For more information, run: skm --help"
