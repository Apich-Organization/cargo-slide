#!/usr/bin/env bash
# ==============================================================================
# Cargo-Slide Unified Asset Build & Synchronization Tool
#
# Automates the packaging and unification of all embedded materials:
#   1. .typ   - Standard theme & macro files (crates/slide-theme -> examples)
#   2. .wasm  - Leptos CSR Web player (slide-web -> cargo-slide/pkg)
#   3. .slide - Reference demo archives (geek_demo.slide, demo.slide)
# ==============================================================================

set -euo pipefail

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Determine repository root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

DO_TYP=false
DO_WASM=false
DO_SLIDE=false
DO_REBUILD=false
DO_CHECK=false

print_usage() {
    cat <<EOF
${BOLD}Usage:${NC} $(basename "$0") [OPTIONS]

${BOLD}Options:${NC}
  --all         Run all updates (.typ, .wasm, .slide) and verify [Default]
  --typ         Sync canonical .typ macros to examples & templates
  --wasm        Compile Leptos CSR wasm web player and bundle to cargo-slide/pkg
  --slide       Package .slide demo archives into slide-viewer/assets
  --rebuild     Recompile release binaries and install to ~/.local/bin
  --check       Validate the presence, non-emptiness, and synchronization of assets
  -h, --help    Show this help message

${BOLD}Examples:${NC}
  ./scripts/update_assets.sh             # Update everything and verify
  ./scripts/update_assets.sh --wasm      # Recompile only the web wasm player
  ./scripts/update_assets.sh --check     # Validate all asset files without rebuilding
EOF
}

# Parse command line options
if [[ $# -eq 0 ]]; then
    DO_TYP=true
    DO_WASM=true
    DO_SLIDE=true
    DO_CHECK=true
else
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --all)
                DO_TYP=true
                DO_WASM=true
                DO_SLIDE=true
                DO_CHECK=true
                shift
                ;;
            --typ)
                DO_TYP=true
                shift
                ;;
            --wasm)
                DO_WASM=true
                shift
                ;;
            --slide)
                DO_SLIDE=true
                shift
                ;;
            --rebuild)
                DO_REBUILD=true
                shift
                ;;
            --check)
                DO_CHECK=true
                shift
                ;;
            -h|--help)
                print_usage
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                print_usage
                exit 1
                ;;
        esac
    done
fi

log_step() {
    echo -e "\n${BOLD}${CYAN}==>${NC} ${BOLD}$1${NC}"
}

log_ok() {
    echo -e "  ${GREEN}✓${NC} $1"
}

log_warn() {
    echo -e "  ${YELLOW}⚠${NC} $1"
}

log_err() {
    echo -e "  ${RED}✗${NC} $1"
}

# ==============================================================================
# 1. Synchronize .typ macro files
# ==============================================================================
if [[ "${DO_TYP}" == true ]]; then
    log_step "Synchronizing canonical .typ theme & macro assets..."

    SRC_THEME="crates/slide-theme/typst/theme.typ"
    SRC_SLIDE="crates/slide-theme/typst/slide.typ"
    SRC_TEMPLATE="crates/slide-theme/typst/template.typ"

    if [[ ! -f "${SRC_THEME}" || ! -f "${SRC_SLIDE}" ]]; then
        log_err "Missing canonical typst files in crates/slide-theme/typst!"
        exit 1
    fi

    # Sync to examples/geek-presentation
    mkdir -p examples/geek-presentation
    cp -f "${SRC_THEME}" examples/geek-presentation/theme.typ
    cp -f "${SRC_SLIDE}" examples/geek-presentation/slide.typ
    log_ok "Synced theme.typ and slide.typ -> examples/geek-presentation/"
fi

# ==============================================================================
# 2. Build and bundle .wasm Leptos CSR web player assets
# ==============================================================================
if [[ "${DO_WASM}" == true ]]; then
    log_step "Building Leptos CSR wasm web player (slide-web)..."

    # Verify prerequisites
    if ! command -v wasm-bindgen &> /dev/null; then
        log_err "wasm-bindgen is not installed on PATH! Please install via: cargo install wasm-bindgen-cli"
        exit 1
    fi

    echo "  Building slide-web for wasm32-unknown-unknown target (release)..."
    cargo build --package slide-web --target wasm32-unknown-unknown --release

    WASM_BIN="target/wasm32-unknown-unknown/release/slide_web.wasm"
    if [[ ! -f "${WASM_BIN}" ]]; then
        log_err "Compiled WASM binary not found at ${WASM_BIN}!"
        exit 1
    fi

    echo "  Running wasm-bindgen..."
    mkdir -p crates/slide-web/pkg
    wasm-bindgen "${WASM_BIN}" --out-dir crates/slide-web/pkg --target web

    echo "  Bundling web assets to crates/cargo-slide/pkg..."
    mkdir -p crates/cargo-slide/pkg
    cp -f crates/slide-web/pkg/slide_web.js crates/cargo-slide/pkg/slide_web.js
    cp -f crates/slide-web/pkg/slide_web_bg.wasm crates/cargo-slide/pkg/slide_web_bg.wasm
    cp -f crates/slide-web/assets/index.html crates/cargo-slide/pkg/index.html
    cp -f crates/slide-web/assets/bootstrap.js crates/cargo-slide/pkg/bootstrap.js
    cp -f crates/slide-web/assets/style.css crates/cargo-slide/pkg/style.css

    WASM_SIZE=$(stat -c%s "crates/cargo-slide/pkg/slide_web_bg.wasm" 2>/dev/null || stat -f%z "crates/cargo-slide/pkg/slide_web_bg.wasm" 2>/dev/null || echo "0")
    WASM_KB=$((WASM_SIZE / 1024))
    log_ok "Generated slide_web_bg.wasm (${WASM_KB} KB) and bundled into crates/cargo-slide/pkg/"
fi

# ==============================================================================
# 3. Package .slide presentation packages
# ==============================================================================
if [[ "${DO_SLIDE}" == true ]]; then
    log_step "Packaging reference .slide demonstration archives..."

    # Ensure cargo-slide CLI binary is available
    CARGO_SLIDE_BIN=""
    if [[ -x "${HOME}/.local/bin/cargo-slide" ]]; then
        CARGO_SLIDE_BIN="${HOME}/.local/bin/cargo-slide"
    elif [[ -x "target/release/cargo-slide" ]]; then
        CARGO_SLIDE_BIN="target/release/cargo-slide"
    elif [[ -x "target/debug/cargo-slide" ]]; then
        CARGO_SLIDE_BIN="target/debug/cargo-slide"
    else
        echo "  Building cargo-slide release binary..."
        cargo build --package cargo-slide --release
        CARGO_SLIDE_BIN="target/release/cargo-slide"
    fi

    mkdir -p crates/slide-viewer/assets

    # 1. Package geek_demo.slide from examples/geek-presentation
    echo "  Packaging geek_demo.slide..."
    "${CARGO_SLIDE_BIN}" pack examples/geek-presentation/slides.typ -o crates/slide-viewer/assets/geek_demo.slide --source

    # 2. Package demo.slide from template.typ
    echo "  Packaging demo.slide..."
    "${CARGO_SLIDE_BIN}" pack crates/slide-theme/typst/template.typ -o crates/slide-viewer/assets/demo.slide --source

    # 3. Package examples/slides.slide
    echo "  Packaging examples/slides.slide..."
    "${CARGO_SLIDE_BIN}" pack examples/geek-presentation/slides.typ -o examples/slides.slide --source

    GEEK_SIZE=$(stat -c%s "crates/slide-viewer/assets/geek_demo.slide" 2>/dev/null || stat -f%z "crates/slide-viewer/assets/geek_demo.slide" 2>/dev/null || echo "0")
    DEMO_SIZE=$(stat -c%s "crates/slide-viewer/assets/demo.slide" 2>/dev/null || stat -f%z "crates/slide-viewer/assets/demo.slide" 2>/dev/null || echo "0")
    log_ok "Packaged geek_demo.slide ($((GEEK_SIZE / 1024)) KB) and demo.slide ($((DEMO_SIZE / 1024)) KB)"
fi

# ==============================================================================
# 4. Validate asset integrity and synchronization
# ==============================================================================
if [[ "${DO_CHECK}" == true ]]; then
    log_step "Validating asset integrity and cross-crate synchronization..."

    CHECK_FAILED=false

    check_file() {
        local path="$1"
        local desc="$2"
        if [[ ! -f "${path}" ]]; then
            log_err "Missing: ${path} (${desc})"
            CHECK_FAILED=true
        elif [[ ! -s "${path}" ]]; then
            log_err "Empty file: ${path} (${desc})"
            CHECK_FAILED=true
        else
            local size
            size=$(stat -c%s "${path}" 2>/dev/null || stat -f%z "${path}" 2>/dev/null || echo "0")
            log_ok "${desc}: ${path} ($((size / 1024)) KB)"
        fi
    }

    check_equal() {
        local file1="$1"
        local file2="$2"
        local desc="$3"
        if ! cmp -s "${file1}" "${file2}"; then
            log_err "Desynchronized: ${file1} != ${file2} (${desc})"
            CHECK_FAILED=true
        else
            log_ok "Synchronized: ${desc}"
        fi
    }

    echo -e "${BOLD}1. Typst Macros:${NC}"
    check_file "crates/slide-theme/typst/theme.typ" "Canonical theme.typ"
    check_file "crates/slide-theme/typst/slide.typ" "Canonical slide.typ"
    check_file "crates/slide-theme/typst/template.typ" "Starter template.typ"
    check_equal "crates/slide-theme/typst/theme.typ" "examples/geek-presentation/theme.typ" "theme.typ in geek-presentation"
    check_equal "crates/slide-theme/typst/slide.typ" "examples/geek-presentation/slide.typ" "slide.typ in geek-presentation"

    echo -e "\n${BOLD}2. Web Player (.wasm & frontend assets):${NC}"
    check_file "crates/cargo-slide/pkg/slide_web_bg.wasm" "Embedded WebAssembly binary"
    check_file "crates/cargo-slide/pkg/slide_web.js" "WebAssembly JS bindings"
    check_file "crates/cargo-slide/pkg/index.html" "Player HTML container"
    check_file "crates/cargo-slide/pkg/bootstrap.js" "Player JS bootstrap"
    check_file "crates/cargo-slide/pkg/style.css" "Player stylesheet"
    check_equal "crates/slide-web/assets/style.css" "crates/cargo-slide/pkg/style.css" "style.css in cargo-slide/pkg"

    echo -e "\n${BOLD}3. Embedded .slide Packages:${NC}"
    check_file "crates/slide-viewer/assets/geek_demo.slide" "Embedded geek demo archive"
    check_file "crates/slide-viewer/assets/demo.slide" "Embedded basic demo archive"
    check_file "examples/slides.slide" "Root examples presentation archive"

    if [[ "${CHECK_FAILED}" == true ]]; then
        echo -e "\n${RED}${BOLD}Validation failed! One or more asset checks did not pass.${NC}"
        exit 1
    else
        echo -e "\n${GREEN}${BOLD}✓ All assets verified and synchronized perfectly!${NC}"
    fi
fi

# ==============================================================================
# 5. Optional Rebuild of workspace binaries
# ==============================================================================
if [[ "${DO_REBUILD}" == true ]]; then
    log_step "Rebuilding release binaries with newly updated embedded assets..."
    cargo build --workspace --release --offline
    mkdir -p "${HOME}/.local/bin"
    install -m 755 target/release/cargo-slide "${HOME}/.local/bin/cargo-slide"
    install -m 755 target/release/slide-viewer "${HOME}/.local/bin/slide-viewer"
    install -m 755 target/release/slide-editor "${HOME}/.local/bin/slide-editor"
    log_ok "Installed updated release binaries to ~/.local/bin/{cargo-slide, slide-viewer, slide-editor}"
fi

echo -e "\n${GREEN}${BOLD}🎉 Asset update process completed successfully!${NC}\n"
