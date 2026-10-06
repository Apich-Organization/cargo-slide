#!/usr/bin/env bash
# ==============================================================================
# Cargo-Slide Unified Asset Build, Packaging & Binary Synchronization Tool
#
# Automates the packaging and unification of all embedded materials & release binaries:
#   1. .typ      - Canonical theme & macro files (crates/slide-theme -> examples)
#   2. .wasm     - Leptos CSR Web player (slide-web -> cargo-slide/pkg & dist-web)
#   3. .svg      - Vector graphics & diagrams (logo.svg, architecture.svg)
#   4. .slide    - Reference demo archives (geek_demo.slide, demo.slide, slides.slide)
#   5. web-dist  - Exported static Leptos CSR presentation bundle (examples/dist-web)
#   6. binaries  - Release binaries (cargo-slide, slide-viewer, slide-editor, slides-presentation)
#   7. integrity - Cross-crate asset presence, non-emptiness & synchronization verification
# ==============================================================================

set -euo pipefail

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Determine repository root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

DO_TYP=false
DO_WASM=false
DO_SVG=false
DO_SLIDE=false
DO_WEB_DIST=false
DO_REBUILD=false
DO_CHECK=false

print_usage() {
    cat <<EOF
${BOLD}Usage:${NC} $(basename "$0") [OPTIONS]

${BOLD}Options:${NC}
  --all         Run full pipeline: assets (.typ, .wasm, .svg, .slide), web-dist, rebuild binaries & verify [Default]
  --typ         Sync canonical .typ macros to examples & templates
  --wasm        Compile Leptos CSR wasm web player and bundle to cargo-slide/pkg
  --svg         Verify and synchronize SVG vector diagrams & assets
  --slide       Package .slide demo archives into slide-viewer/assets and examples
  --web-dist    Export full standalone static web presentation bundle to examples/dist-web
  --rebuild     Recompile all release workspace binaries and install to ~/.local/bin
  --check       Validate the presence, non-emptiness, and synchronization of all assets & binaries
  -h, --help    Show this help message

${BOLD}Examples:${NC}
  ./scripts/update_assets.sh             # Full end-to-end update, packaging, rebuild and verify
  ./scripts/update_assets.sh --wasm      # Recompile only the web wasm player
  ./scripts/update_assets.sh --slide     # Re-package only .slide archives
  ./scripts/update_assets.sh --rebuild   # Rebuild release binaries and update ~/.local/bin
  ./scripts/update_assets.sh --check     # Validate all asset files and binaries without rebuilding
EOF
}

# Parse command line options
if [[ $# -eq 0 ]]; then
    DO_TYP=true
    DO_WASM=true
    DO_SVG=true
    DO_SLIDE=true
    DO_WEB_DIST=true
    DO_REBUILD=true
    DO_CHECK=true
else
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --all)
                DO_TYP=true
                DO_WASM=true
                DO_SVG=true
                DO_SLIDE=true
                DO_WEB_DIST=true
                DO_REBUILD=true
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
            --svg)
                DO_SVG=true
                shift
                ;;
            --slide)
                DO_SLIDE=true
                shift
                ;;
            --web-dist)
                DO_WEB_DIST=true
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

file_size() {
    stat -c%s "$1" 2>/dev/null || stat -f%z "$1" 2>/dev/null || echo "0"
}

# ==============================================================================
# 1. Synchronize .typ macro files
# ==============================================================================
if [[ "${DO_TYP}" == true ]]; then
    log_step "Synchronizing canonical .typ theme & macro assets..."

    SRC_THEME="crates/slide-theme/typst/theme.typ"
    SRC_SLIDE="crates/slide-theme/typst/slide.typ"
    SRC_TEMPLATE="crates/slide-theme/typst/template.typ"

    if [[ ! -f "${SRC_THEME}" || ! -f "${SRC_SLIDE}" || ! -f "${SRC_TEMPLATE}" ]]; then
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

    WASM_SIZE=$(file_size "crates/cargo-slide/pkg/slide_web_bg.wasm")
    WASM_KB=$((WASM_SIZE / 1024))
    log_ok "Generated slide_web_bg.wasm (${WASM_KB} KB) and bundled into crates/cargo-slide/pkg/"
fi

# ==============================================================================
# 3. Verify and synchronize SVG vector assets
# ==============================================================================
if [[ "${DO_SVG}" == true ]]; then
    log_step "Verifying and synchronizing SVG vector assets..."

    # 1. Project logo
    if [[ ! -f "logo.svg" || ! -s "logo.svg" ]]; then
        log_err "Missing or empty logo.svg at repository root!"
        exit 1
    fi
    LOGO_SIZE=$(file_size "logo.svg")
    log_ok "Verified repository logo: logo.svg ($((LOGO_SIZE / 1024)) KB)"

    # 2. Architecture diagram in geek-presentation
    ARCH_SRC="examples/geek-presentation/assets/architecture.svg"
    if [[ ! -f "${ARCH_SRC}" || ! -s "${ARCH_SRC}" ]]; then
        log_err "Missing or empty architecture diagram at ${ARCH_SRC}!"
        exit 1
    fi
    ARCH_SIZE=$(file_size "${ARCH_SRC}")
    log_ok "Verified presentation architecture diagram: ${ARCH_SRC} ($((ARCH_SIZE / 1024)) KB)"

    # 3. Synchronize to dist-web if dist-web exists
    if [[ -d "examples/dist-web/assets" ]]; then
        cp -f "${ARCH_SRC}" examples/dist-web/assets/architecture.svg
        log_ok "Synchronized architecture.svg -> examples/dist-web/assets/"
    fi
fi

# ==============================================================================
# 4. Package .slide presentation packages
# ==============================================================================
if [[ "${DO_SLIDE}" == true ]]; then
    log_step "Packaging reference .slide demonstration archives..."

    # Ensure cargo-slide CLI binary is available
    CARGO_SLIDE_BIN=""
    if [[ -x "target/release/cargo-slide" ]]; then
        CARGO_SLIDE_BIN="target/release/cargo-slide"
    elif [[ -x "${HOME}/.local/bin/cargo-slide" ]]; then
        CARGO_SLIDE_BIN="${HOME}/.local/bin/cargo-slide"
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

    GEEK_SIZE=$(file_size "crates/slide-viewer/assets/geek_demo.slide")
    DEMO_SIZE=$(file_size "crates/slide-viewer/assets/demo.slide")
    ROOT_SLIDE_SIZE=$(file_size "examples/slides.slide")
    log_ok "Packaged geek_demo.slide ($((GEEK_SIZE / 1024)) KB), demo.slide ($((DEMO_SIZE / 1024)) KB), examples/slides.slide ($((ROOT_SLIDE_SIZE / 1024)) KB)"
fi

# ==============================================================================
# 5. Export static Leptos CSR Web distribution bundle
# ==============================================================================
if [[ "${DO_WEB_DIST}" == true ]]; then
    log_step "Exporting static Leptos CSR web presentation bundle..."

    CARGO_SLIDE_BIN=""
    if [[ -x "target/release/cargo-slide" ]]; then
        CARGO_SLIDE_BIN="target/release/cargo-slide"
    elif [[ -x "${HOME}/.local/bin/cargo-slide" ]]; then
        CARGO_SLIDE_BIN="${HOME}/.local/bin/cargo-slide"
    else
        echo "  Building cargo-slide release binary..."
        cargo build --package cargo-slide --release
        CARGO_SLIDE_BIN="target/release/cargo-slide"
    fi

    echo "  Exporting examples/geek-presentation/slides.typ -> examples/dist-web/..."
    "${CARGO_SLIDE_BIN}" export examples/geek-presentation/slides.typ --format wasm -o examples/dist-web/

    DIST_DECK_SIZE=$(file_size "examples/dist-web/deck.json")
    DIST_WASM_SIZE=$(file_size "examples/dist-web/slide_web_bg.wasm")
    log_ok "Exported static web bundle in examples/dist-web/ (deck.json: $((DIST_DECK_SIZE / 1024)) KB, wasm: $((DIST_WASM_SIZE / 1024)) KB)"
fi

# ==============================================================================
# 6. Rebuild workspace release binaries & standalone presentation runner
# ==============================================================================
if [[ "${DO_REBUILD}" == true ]]; then
    log_step "Rebuilding all release workspace binaries with newly updated assets..."

    echo "  Compiling workspace in release mode (offline)..."
    cargo build --workspace --release --offline

    echo "  Building standalone presentation runner (slides-presentation)..."
    target/release/cargo-slide build examples/geek-presentation/slides.typ -o examples/geek-presentation/slides-presentation

    echo "  Installing release binaries to ~/.local/bin/..."
    mkdir -p "${HOME}/.local/bin"
    install -m 755 target/release/cargo-slide "${HOME}/.local/bin/cargo-slide"
    install -m 755 target/release/slide-viewer "${HOME}/.local/bin/slide-viewer"
    install -m 755 target/release/slide-editor "${HOME}/.local/bin/slide-editor"

    log_ok "Installed updated release binaries to ~/.local/bin/{cargo-slide, slide-viewer, slide-editor}"
    log_ok "Updated standalone presentation runner at examples/geek-presentation/slides-presentation"
fi

# ==============================================================================
# 7. Validate asset integrity, cross-crate synchronization & binary readiness
# ==============================================================================
if [[ "${DO_CHECK}" == true ]]; then
    log_step "Validating asset integrity, cross-crate synchronization & binary readiness..."

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
            size=$(file_size "${path}")
            log_ok "${desc}: ${path} ($((size / 1024)) KB)"
        fi
    }

    check_equal() {
        local file1="$1"
        local file2="$2"
        local desc="$3"
        if [[ ! -f "${file1}" || ! -f "${file2}" ]]; then
            log_err "File missing for comparison: ${file1} or ${file2} (${desc})"
            CHECK_FAILED=true
        elif ! cmp -s "${file1}" "${file2}"; then
            log_err "Desynchronized: ${file1} != ${file2} (${desc})"
            CHECK_FAILED=true
        else
            log_ok "Synchronized: ${desc}"
        fi
    }

    check_binary_exec() {
        local bin_path="$1"
        local flag="$2"
        local desc="$3"
        if [[ ! -x "${bin_path}" ]]; then
            log_err "Not executable: ${bin_path} (${desc})"
            CHECK_FAILED=true
        else
            local out
            if out=$("${bin_path}" ${flag} 2>&1 | head -n 1); then
                log_ok "${desc} (${bin_path}): ${out}"
            else
                log_err "${desc} failed execution (${bin_path} ${flag})"
                CHECK_FAILED=true
            fi
        fi
    }

    echo -e "${BOLD}1. Typst Macros & Templates:${NC}"
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
    check_equal "crates/slide-web/assets/index.html" "crates/cargo-slide/pkg/index.html" "index.html in cargo-slide/pkg"
    check_equal "crates/slide-web/assets/bootstrap.js" "crates/cargo-slide/pkg/bootstrap.js" "bootstrap.js in cargo-slide/pkg"

    echo -e "\n${BOLD}3. SVG Vector Assets:${NC}"
    check_file "logo.svg" "Official repository logo"
    check_file "examples/geek-presentation/assets/architecture.svg" "Architecture SVG diagram"
    if [[ -f "examples/dist-web/assets/architecture.svg" ]]; then
        check_equal "examples/geek-presentation/assets/architecture.svg" "examples/dist-web/assets/architecture.svg" "architecture.svg in dist-web"
    fi

    echo -e "\n${BOLD}4. Embedded .slide Packages:${NC}"
    check_file "crates/slide-viewer/assets/geek_demo.slide" "Embedded geek demo archive"
    check_file "crates/slide-viewer/assets/demo.slide" "Embedded basic demo archive"
    check_file "examples/slides.slide" "Root examples presentation archive"

    echo -e "\n${BOLD}5. Exported Web Bundle (examples/dist-web):${NC}"
    check_file "examples/dist-web/index.html" "dist-web index.html"
    check_file "examples/dist-web/bootstrap.js" "dist-web bootstrap.js"
    check_file "examples/dist-web/style.css" "dist-web style.css"
    check_file "examples/dist-web/slide_web.js" "dist-web slide_web.js"
    check_file "examples/dist-web/slide_web_bg.wasm" "dist-web slide_web_bg.wasm"
    check_file "examples/dist-web/deck.json" "dist-web compiled deck.json"
    check_file "examples/dist-web/assets/architecture.svg" "dist-web architecture.svg"

    echo -e "\n${BOLD}6. Packaged & Installed Release Binaries:${NC}"
    check_binary_exec "${HOME}/.local/bin/cargo-slide" "--version" "Installed CLI"
    check_binary_exec "${HOME}/.local/bin/slide-viewer" "--version" "Installed Native Viewer"
    check_binary_exec "${HOME}/.local/bin/slide-editor" "--version" "Installed Visual Editor"
    check_file "examples/geek-presentation/slides-presentation" "Standalone Runner Binary"

    if [[ "${CHECK_FAILED}" == true ]]; then
        echo -e "\n${RED}${BOLD}Validation failed! One or more asset or binary checks did not pass.${NC}"
        exit 1
    else
        echo -e "\n${GREEN}${BOLD}✓ All assets & release binaries verified and unified perfectly!${NC}"
    fi
fi

echo -e "\n${GREEN}${BOLD}🎉 Asset & Binary update process completed successfully!${NC}\n"
