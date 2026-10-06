#!/usr/bin/env python3
"""
Cargo-Slide Unified Asset Build & Synchronization Tool (Cross-Platform)

Automates the packaging and unification of all embedded materials:
  1. .typ   - Standard theme & macro files (crates/slide-theme -> examples)
  2. .wasm  - Leptos CSR Web player (slide-web -> cargo-slide/pkg)
  3. .slide - Reference demo archives (geek_demo.slide, demo.slide)
"""

import argparse
import filecmp
import os
import shutil
import subprocess
import sys
from pathlib import Path

# ANSI escape codes for colored terminal output
GREEN = "\033[0;32m"
CYAN = "\033[0;36m"
RED = "\033[0;31m"
BOLD = "\033[1m"
RESET = "\033[0m"


def log_step(msg: str):
    print(f"\n{BOLD}{CYAN}==>{RESET} {BOLD}{msg}{RESET}")


def log_ok(msg: str):
    print(f"  {GREEN}✓{RESET} {msg}")


def log_err(msg: str):
    print(f"  {RED}✗{RESET} {msg}")


def get_repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def sync_typ_assets(repo_root: Path):
    log_step("Synchronizing canonical .typ theme & macro assets...")

    src_theme = repo_root / "crates" / "slide-theme" / "typst" / "theme.typ"
    src_slide = repo_root / "crates" / "slide-theme" / "typst" / "slide.typ"

    if not src_theme.is_file() or not src_slide.is_file():
        log_err(f"Missing canonical typst files in {src_theme.parent}")
        sys.exit(1)

    geek_dir = repo_root / "examples" / "geek-presentation"
    geek_dir.mkdir(parents=True, exist_ok=True)

    shutil.copy2(src_theme, geek_dir / "theme.typ")
    shutil.copy2(src_slide, geek_dir / "slide.typ")
    log_ok("Synced theme.typ and slide.typ -> examples/geek-presentation/")


def build_wasm_assets(repo_root: Path):
    log_step("Building Leptos CSR wasm web player (slide-web)...")

    wasm_bindgen_bin = shutil.which("wasm-bindgen")
    if not wasm_bindgen_bin:
        log_err("wasm-bindgen is not found on PATH! Please install: cargo install wasm-bindgen-cli")
        sys.exit(1)

    print("  Building slide-web for wasm32-unknown-unknown target (release)...")
    subprocess.run(
        ["cargo", "build", "--package", "slide-web", "--target", "wasm32-unknown-unknown", "--release"],
        cwd=repo_root,
        check=True,
    )

    wasm_bin = repo_root / "target" / "wasm32-unknown-unknown" / "release" / "slide_web.wasm"
    if not wasm_bin.is_file():
        log_err(f"Compiled WASM binary not found at {wasm_bin}")
        sys.exit(1)

    slide_web_pkg = repo_root / "crates" / "slide-web" / "pkg"
    cargo_slide_pkg = repo_root / "crates" / "cargo-slide" / "pkg"
    slide_web_pkg.mkdir(parents=True, exist_ok=True)
    cargo_slide_pkg.mkdir(parents=True, exist_ok=True)

    print("  Running wasm-bindgen...")
    subprocess.run(
        [wasm_bindgen_bin, str(wasm_bin), "--out-dir", str(slide_web_pkg), "--target", "web"],
        cwd=repo_root,
        check=True,
    )

    print("  Bundling web assets to crates/cargo-slide/pkg...")
    shutil.copy2(slide_web_pkg / "slide_web.js", cargo_slide_pkg / "slide_web.js")
    shutil.copy2(slide_web_pkg / "slide_web_bg.wasm", cargo_slide_pkg / "slide_web_bg.wasm")

    assets_dir = repo_root / "crates" / "slide-web" / "assets"
    for asset in ["index.html", "bootstrap.js", "style.css"]:
        shutil.copy2(assets_dir / asset, cargo_slide_pkg / asset)

    wasm_size_kb = (cargo_slide_pkg / "slide_web_bg.wasm").stat().st_size // 1024
    log_ok(f"Generated slide_web_bg.wasm ({wasm_size_kb} KB) and bundled into crates/cargo-slide/pkg/")


def package_slide_archives(repo_root: Path):
    log_step("Packaging reference .slide demonstration archives...")

    # Find cargo-slide executable
    local_bin = Path.home() / ".local" / "bin" / ("cargo-slide.exe" if os.name == "nt" else "cargo-slide")
    target_rel = repo_root / "target" / "release" / ("cargo-slide.exe" if os.name == "nt" else "cargo-slide")
    cargo_slide = None

    if local_bin.is_file() and os.access(local_bin, os.X_OK):
        cargo_slide = str(local_bin)
    elif target_rel.is_file() and os.access(target_rel, os.X_OK):
        cargo_slide = str(target_rel)
    else:
        print("  Building cargo-slide release binary...")
        subprocess.run(["cargo", "build", "--package", "cargo-slide", "--release"], cwd=repo_root, check=True)
        cargo_slide = str(target_rel)

    viewer_assets = repo_root / "crates" / "slide-viewer" / "assets"
    viewer_assets.mkdir(parents=True, exist_ok=True)

    # 1. Package geek_demo.slide
    print("  Packaging geek_demo.slide...")
    geek_typ = repo_root / "examples" / "geek-presentation" / "slides.typ"
    geek_slide = viewer_assets / "geek_demo.slide"
    subprocess.run([cargo_slide, "pack", str(geek_typ), "-o", str(geek_slide), "--source"], cwd=repo_root, check=True)

    # 2. Package demo.slide
    print("  Packaging demo.slide...")
    demo_typ = repo_root / "crates" / "slide-theme" / "typst" / "template.typ"
    demo_slide = viewer_assets / "demo.slide"
    subprocess.run([cargo_slide, "pack", str(demo_typ), "-o", str(demo_slide), "--source"], cwd=repo_root, check=True)

    # 3. Package examples/slides.slide
    print("  Packaging examples/slides.slide...")
    root_slide = repo_root / "examples" / "slides.slide"
    subprocess.run([cargo_slide, "pack", str(geek_typ), "-o", str(root_slide), "--source"], cwd=repo_root, check=True)

    geek_kb = geek_slide.stat().st_size // 1024
    demo_kb = demo_slide.stat().st_size // 1024
    log_ok(f"Packaged geek_demo.slide ({geek_kb} KB) and demo.slide ({demo_kb} KB)")


def validate_assets(repo_root: Path) -> bool:
    log_step("Validating asset integrity and cross-crate synchronization...")

    failed = False

    def check_file(rel_path: str, desc: str):
        nonlocal failed
        p = repo_root / rel_path
        if not p.is_file():
            log_err(f"Missing: {rel_path} ({desc})")
            failed = True
        elif p.stat().st_size == 0:
            log_err(f"Empty file: {rel_path} ({desc})")
            failed = True
        else:
            log_ok(f"{desc}: {rel_path} ({p.stat().st_size // 1024} KB)")

    def check_equal(rel1: str, rel2: str, desc: str):
        nonlocal failed
        p1 = repo_root / rel1
        p2 = repo_root / rel2
        if not p1.is_file() or not p2.is_file() or not filecmp.cmp(p1, p2, shallow=False):
            log_err(f"Desynchronized: {rel1} != {rel2} ({desc})")
            failed = True
        else:
            log_ok(f"Synchronized: {desc}")

    print(f"{BOLD}1. Typst Macros:{RESET}")
    check_file("crates/slide-theme/typst/theme.typ", "Canonical theme.typ")
    check_file("crates/slide-theme/typst/slide.typ", "Canonical slide.typ")
    check_file("crates/slide-theme/typst/template.typ", "Starter template.typ")
    check_equal("crates/slide-theme/typst/theme.typ", "examples/geek-presentation/theme.typ", "theme.typ in geek-presentation")
    check_equal("crates/slide-theme/typst/slide.typ", "examples/geek-presentation/slide.typ", "slide.typ in geek-presentation")

    print(f"\n{BOLD}2. Web Player (.wasm & frontend assets):{RESET}")
    check_file("crates/cargo-slide/pkg/slide_web_bg.wasm", "Embedded WebAssembly binary")
    check_file("crates/cargo-slide/pkg/slide_web.js", "WebAssembly JS bindings")
    check_file("crates/cargo-slide/pkg/index.html", "Player HTML container")
    check_file("crates/cargo-slide/pkg/bootstrap.js", "Player JS bootstrap")
    check_file("crates/cargo-slide/pkg/style.css", "Player stylesheet")
    check_equal("crates/slide-web/assets/style.css", "crates/cargo-slide/pkg/style.css", "style.css in cargo-slide/pkg")

    print(f"\n{BOLD}3. Embedded .slide Packages:{RESET}")
    check_file("crates/slide-viewer/assets/geek_demo.slide", "Embedded geek demo archive")
    check_file("crates/slide-viewer/assets/demo.slide", "Embedded basic demo archive")
    check_file("examples/slides.slide", "Root examples presentation archive")

    if failed:
        print(f"\n{RED}{BOLD}Validation failed! One or more asset checks did not pass.{RESET}")
    else:
        print(f"\n{GREEN}{BOLD}✓ All assets verified and synchronized perfectly!{RESET}")

    return not failed


def rebuild_binaries(repo_root: Path):
    log_step("Rebuilding release binaries with newly updated embedded assets...")
    subprocess.run(["cargo", "build", "--workspace", "--release", "--offline"], cwd=repo_root, check=True)

    dest_dir = Path.home() / ".local" / "bin"
    dest_dir.mkdir(parents=True, exist_ok=True)
    target_rel = repo_root / "target" / "release"

    for b in ["cargo-slide", "slide-viewer", "slide-editor"]:
        bin_name = f"{b}.exe" if os.name == "nt" else b
        src = target_rel / bin_name
        dst = dest_dir / bin_name
        if src.is_file():
            shutil.copy2(src, dst)
            if os.name != "nt":
                dst.chmod(0o755)

    log_ok("Installed updated release binaries to ~/.local/bin/{cargo-slide, slide-viewer, slide-editor}")


def main():
    parser = argparse.ArgumentParser(description="Cargo-Slide Unified Asset Build & Synchronization Tool")
    parser.add_argument("--all", action="store_true", help="Run all updates (.typ, .wasm, .slide) and verify [Default]")
    parser.add_argument("--typ", action="store_true", help="Sync canonical .typ macros to examples & templates")
    parser.add_argument("--wasm", action="store_true", help="Compile Leptos CSR wasm web player and bundle to cargo-slide/pkg")
    parser.add_argument("--slide", action="store_true", help="Package .slide demo archives into slide-viewer/assets")
    parser.add_argument("--rebuild", action="store_true", help="Recompile release binaries and install to ~/.local/bin")
    parser.add_argument("--check", action="store_true", help="Validate the presence, non-emptiness, and synchronization of assets")

    args = parser.parse_args()
    repo_root = get_repo_root()

    do_all = args.all or (not args.typ and not args.wasm and not args.slide and not args.rebuild and not args.check)

    if do_all or args.typ:
        sync_typ_assets(repo_root)

    if do_all or args.wasm:
        build_wasm_assets(repo_root)

    if do_all or args.slide:
        package_slide_archives(repo_root)

    if do_all or args.check:
        if not validate_assets(repo_root):
            sys.exit(1)

    if args.rebuild:
        rebuild_binaries(repo_root)

    print(f"\n{GREEN}{BOLD}🎉 Asset update process completed successfully!{RESET}\n")


if __name__ == "__main__":
    main()
