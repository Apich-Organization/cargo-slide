#!/usr/bin/env bash
# ==============================================================================
# Convenience wrapper script for cargo-slide's built-in cross command.
#
# NOTE: Cross-compilation is now natively built into cargo-slide!
# You can directly run:
#   cargo slide cross --target all
#   cargo slide cross --target windows-x86_64
# without downloading or running any separate script.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

if command -v cargo-slide >/dev/null 2>&1; then
    exec cargo slide cross "$@"
else
    exec cargo run -p cargo-slide -- cross "$@"
fi
