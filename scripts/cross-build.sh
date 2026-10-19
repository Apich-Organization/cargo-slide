#!/usr/bin/env bash
# ==============================================================================
# Cross-Platform Multi-Architecture Compilation Script for Cargo Slide
# Supported Target Platforms:
#   1. Linux x86_64      (x86_64-unknown-linux-gnu / x86_64-unknown-linux-musl)
#   2. Linux aarch64     (aarch64-unknown-linux-gnu)
#   3. Windows x86_64    (x86_64-pc-windows-gnu)
#   4. Windows aarch64   (aarch64-pc-windows-gnullvm)
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

OUTPUT_DIR="${REPO_ROOT}/dist"
PROFILE="release"
CARGO_FLAGS="--release"
TOOL="auto"
TARGET_REQ="all"

print_usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Options:
    -t, --target <TARGET>   Target platform to build:
                            'all' (default: builds all 4 desktop targets)
                            'linux-x86_64' (x86_64-unknown-linux-gnu)
                            'linux-aarch64' (aarch64-unknown-linux-gnu)
                            'windows-x86_64' (x86_64-pc-windows-gnu)
                            'windows-aarch64' (aarch64-pc-windows-gnullvm)
    --tool <TOOL>           Cross toolchain backend: 'auto' (default), 'zig', 'cross'
    --dev                   Build debug binaries instead of release
    -o, --output-dir <DIR>  Output directory for compiled binaries (default: ./dist)
    -h, --help              Show this help message

Examples:
    $(basename "$0") --target all
    $(basename "$0") --target windows-x86_64 --tool zig
    $(basename "$0") --target linux-aarch64 --tool cross
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -t|--target)
            TARGET_REQ="$2"
            shift 2
            ;;
        --tool)
            TOOL="$2"
            shift 2
            ;;
        --dev)
            PROFILE="debug"
            CARGO_FLAGS=""
            shift
            ;;
        -o|--output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo "[ERR] Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

mkdir -p "${OUTPUT_DIR}"

ensure_target() {
    local target="$1"
    if ! rustup target list --installed | grep -q "^${target}\$"; then
        echo "[SETUP] Adding rust target: ${target}"
        rustup target add "${target}"
    fi
}

build_single() {
    local triple="$1"
    local label="$2"
    local bin_name="$3"
    local is_windows=false
    if [[ "${triple}" == *"windows"* ]]; then
        is_windows=true
    fi

    echo ""
    echo "================================================================================"
    echo "  Building [${bin_name}] for [${label}] (${triple})"
    echo "================================================================================"

    local selected_tool="${TOOL}"
    if [[ "${selected_tool}" == "auto" ]]; then
        if [[ "${is_windows}" == "true" ]]; then
            if command -v cargo-zigbuild >/dev/null 2>&1 || command -v zig >/dev/null 2>&1; then
                selected_tool="zig"
            elif command -v cross >/dev/null 2>&1; then
                selected_tool="cross"
            else
                selected_tool="cargo"
            fi
        elif [[ "${triple}" == "aarch64-unknown-linux-gnu" ]]; then
            if command -v cross >/dev/null 2>&1; then
                selected_tool="cross"
            elif command -v cargo-zigbuild >/dev/null 2>&1; then
                selected_tool="zig"
            else
                selected_tool="cargo"
            fi
        else
            selected_tool="cargo"
        fi
    fi

    ensure_target "${triple}"

    local bin_ext=""
    if [[ "${is_windows}" == "true" ]]; then
        bin_ext=".exe"
    fi

    echo "Using compilation backend: ${selected_tool}"
    if [[ "${selected_tool}" == "zig" ]]; then
        cargo zigbuild --target "${triple}" ${CARGO_FLAGS} -p "${bin_name}"
    elif [[ "${selected_tool}" == "cross" ]]; then
        if [[ -z "${CROSS_CONTAINER_ENGINE:-}" ]]; then
            if command -v podman >/dev/null 2>&1 && ! command -v docker >/dev/null 2>&1; then
                export CROSS_CONTAINER_ENGINE=podman
            fi
        fi
        cross build --target "${triple}" ${CARGO_FLAGS} -p "${bin_name}"
    else
        cargo build --target "${triple}" ${CARGO_FLAGS} -p "${bin_name}"
    fi

    local built_file="target/${triple}/${PROFILE}/${bin_name}${bin_ext}"
    local dest_file="${OUTPUT_DIR}/${bin_name}-${label}${bin_ext}"
    if [[ -f "${built_file}" ]]; then
        cp -f "${built_file}" "${dest_file}"
        echo "[OK] Successfully built: ${dest_file} ($(du -h "${dest_file}" | cut -f1))"
    else
        echo "[ERR] Built artifact not found at: ${built_file}"
        exit 1
    fi
}

declare -a TARGETS=()

case "${TARGET_REQ}" in
    all)
        TARGETS=(
            "x86_64-unknown-linux-gnu:linux-x86_64"
            "aarch64-unknown-linux-gnu:linux-aarch64"
            "x86_64-pc-windows-gnu:windows-x86_64"
            "aarch64-pc-windows-gnullvm:windows-aarch64"
        )
        ;;
    linux-x86_64)
        TARGETS=("x86_64-unknown-linux-gnu:linux-x86_64")
        ;;
    linux-aarch64)
        TARGETS=("aarch64-unknown-linux-gnu:linux-aarch64")
        ;;
    windows-x86_64)
        TARGETS=("x86_64-pc-windows-gnu:windows-x86_64")
        ;;
    windows-aarch64)
        TARGETS=("aarch64-pc-windows-gnullvm:windows-aarch64")
        ;;
    *)
        TARGETS=("${TARGET_REQ}:${TARGET_REQ}")
        ;;
esac

echo "[CROSS] Starting compilation for target(s): ${TARGET_REQ}"
echo "[CROSS] Output directory: ${OUTPUT_DIR}"

for entry in "${TARGETS[@]}"; do
    IFS=":" read -r triple label <<< "${entry}"
    build_single "${triple}" "${label}" "cargo-slide"
done

echo ""
echo "================================================================================"
echo "  All requested cross-platform builds completed successfully!"
echo "  Artifacts available in: ${OUTPUT_DIR}"
ls -lh "${OUTPUT_DIR}"
echo "================================================================================"
