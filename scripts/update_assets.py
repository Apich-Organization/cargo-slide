#!/usr/bin/env python3
"""
Cargo-Slide Unified Asset Build, Packaging & Binary Synchronization Tool (Cross-Platform)

Automates the packaging and unification of all embedded materials & release binaries:
  1. .typ      - Canonical theme & macro files (crates/slide-theme -> examples)
  2. .wasm     - Leptos CSR Web player (slide-web -> cargo-slide/pkg & dist-web)
  3. .svg      - Vector graphics & diagrams (logo.svg, architecture.svg)
  4. .slide    - Reference demo archives (geek_demo.slide, demo.slide, slides.slide)
  5. web-dist  - Exported static Leptos CSR presentation bundle (examples/dist-web)
  6. binaries  - Release binaries (cargo-slide, slide-viewer, slide-editor, slides-presentation)
  7. integrity - Cross-crate asset presence, non-emptiness & synchronization verification
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
YELLOW = "\033[1;33m"
BOLD = "\033[1m"
RESET = "\033[0m"


def log_step(msg: str):
    print(f"\n{BOLD}{CYAN}==>{RESET} {BOLD}{msg}{RESET}")


def log_ok(msg: str):
    print(f"  {GREEN}✓{RESET} {msg}")


def log_warn(msg: str):
    print(f"  {YELLOW}⚠{RESET} {msg}")


def log_err(msg: str):
    print(f"  {RED}✗{RESET} {msg}")


def get_repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def get_cargo_slide_bin(repo_root: Path) -> str:
    bin_name = "cargo-slide.exe" if os.name == "nt" else "cargo-slide"
    target_rel = repo_root / "target" / "release" / bin_name
    local_bin = Path.home() / ".local" / "bin" / bin_name
    target_dbg = repo_root / "target" / "debug" / bin_name

    if target_rel.is_file() and os.access(target_rel, os.X_OK):
        return str(target_rel)
    if local_bin.is_file() and os.access(local_bin, os.X_OK):
        return str(local_bin)
    if target_dbg.is_file() and os.access(target_dbg, os.X_OK):
        return str(target_dbg)

    print("  Building cargo-slide release binary...")
    subprocess.run(["cargo", "build", "--package", "cargo-slide", "--release"], cwd=repo_root, check=True)
    return str(target_rel)


def sync_typ_assets(repo_root: Path):
    log_step("Synchronizing canonical .typ theme & macro assets...")

    src_theme = repo_root / "crates" / "slide-theme" / "typst" / "theme.typ"
    src_slide = repo_root / "crates" / "slide-theme" / "typst" / "slide.typ"
    src_template = repo_root / "crates" / "slide-theme" / "typst" / "template.typ"

    if not src_theme.is_file() or not src_slide.is_file() or not src_template.is_file():
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


def verify_svg_assets(repo_root: Path):
    log_step("Verifying and synchronizing SVG vector assets...")

    logo_path = repo_root / "logo.svg"
    if not logo_path.is_file() or logo_path.stat().st_size == 0:
        log_err("Missing or empty logo.svg at repository root!")
        sys.exit(1)
    logo_kb = logo_path.stat().st_size // 1024
    log_ok(f"Verified repository logo: logo.svg ({logo_kb} KB)")

    arch_src = repo_root / "examples" / "geek-presentation" / "assets" / "architecture.svg"
    if not arch_src.is_file() or arch_src.stat().st_size == 0:
        log_err(f"Missing or empty architecture diagram at {arch_src}!")
        sys.exit(1)
    arch_kb = arch_src.stat().st_size // 1024
    log_ok(f"Verified presentation architecture diagram: {arch_src.relative_to(repo_root)} ({arch_kb} KB)")

    dist_assets = repo_root / "examples" / "dist-web" / "assets"
    if dist_assets.is_dir():
        shutil.copy2(arch_src, dist_assets / "architecture.svg")
        log_ok("Synchronized architecture.svg -> examples/dist-web/assets/")


def package_slide_archives(repo_root: Path):
    log_step("Packaging reference .slide demonstration archives...")

    cargo_slide = get_cargo_slide_bin(repo_root)

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
    root_kb = root_slide.stat().st_size // 1024
    log_ok(f"Packaged geek_demo.slide ({geek_kb} KB), demo.slide ({demo_kb} KB), slides.slide ({root_kb} KB)")


def export_web_distribution(repo_root: Path):
    log_step("Exporting static Leptos CSR web presentation bundle...")

    cargo_slide = get_cargo_slide_bin(repo_root)
    geek_typ = repo_root / "examples" / "geek-presentation" / "slides.typ"
    dist_web = repo_root / "examples" / "dist-web"

    print("  Exporting examples/geek-presentation/slides.typ -> examples/dist-web/...")
    subprocess.run([cargo_slide, "export", str(geek_typ), "--format", "wasm", "-o", str(dist_web)], cwd=repo_root, check=True)

    deck_size = (dist_web / "deck.json").stat().st_size // 1024
    wasm_size = (dist_web / "slide_web_bg.wasm").stat().st_size // 1024
    log_ok(f"Exported static web bundle in examples/dist-web/ (deck.json: {deck_size} KB, wasm: {wasm_size} KB)")


def rebuild_binaries(repo_root: Path):
    log_step("Rebuilding all release workspace binaries with newly updated assets...")

    print("  Compiling workspace in release mode (offline)...")
    subprocess.run(["cargo", "build", "--workspace", "--release", "--offline"], cwd=repo_root, check=True)

    bin_name = "cargo-slide.exe" if os.name == "nt" else "cargo-slide"
    cargo_slide = str(repo_root / "target" / "release" / bin_name)

    print("  Building standalone presentation runner (slides-presentation)...")
    runner_out = repo_root / "examples" / "geek-presentation" / ("slides-presentation.exe" if os.name == "nt" else "slides-presentation")
    geek_typ = repo_root / "examples" / "geek-presentation" / "slides.typ"
    subprocess.run([cargo_slide, "build", str(geek_typ), "-o", str(runner_out)], cwd=repo_root, check=True)

    dest_dir = Path.home() / ".local" / "bin"
    dest_dir.mkdir(parents=True, exist_ok=True)
    target_rel = repo_root / "target" / "release"

    for b in ["cargo-slide", "slide-viewer", "slide-editor"]:
        name = f"{b}.exe" if os.name == "nt" else b
        src = target_rel / name
        dst = dest_dir / name
        if src.is_file():
            shutil.copy2(src, dst)
            if os.name != "nt":
                dst.chmod(0o755)

    log_ok("Installed updated release binaries to ~/.local/bin/{cargo-slide, slide-viewer, slide-editor}")
    log_ok(f"Updated standalone presentation runner at {runner_out.relative_to(repo_root)}")


def validate_assets_and_binaries(repo_root: Path) -> bool:
    log_step("Validating asset integrity, cross-crate synchronization & binary readiness...")

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

    def check_binary(bin_path: Path, flag: str, desc: str):
        nonlocal failed
        if not bin_path.is_file() or not os.access(bin_path, os.X_OK):
            log_err(f"Not executable: {bin_path} ({desc})")
            failed = True
            return
        try:
            res = subprocess.run([str(bin_path), flag], capture_output=True, text=True, check=True)
            out_line = res.stdout.strip().split("\n")[0] if res.stdout.strip() else res.stderr.strip().split("\n")[0]
            log_ok(f"{desc} ({bin_path}): {out_line}")
        except Exception as e:
            log_err(f"{desc} failed execution ({bin_path} {flag}): {e}")
            failed = True

    print(f"{BOLD}1. Typst Macros & Templates:{RESET}")
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
    check_equal("crates/slide-web/assets/index.html", "crates/cargo-slide/pkg/index.html", "index.html in cargo-slide/pkg")
    check_equal("crates/slide-web/assets/bootstrap.js", "crates/cargo-slide/pkg/bootstrap.js", "bootstrap.js in cargo-slide/pkg")

    print(f"\n{BOLD}3. SVG Vector Assets:{RESET}")
    check_file("logo.svg", "Official repository logo")
    check_file("examples/geek-presentation/assets/architecture.svg", "Architecture SVG diagram")
    if (repo_root / "examples" / "dist-web" / "assets" / "architecture.svg").is_file():
        check_equal("examples/geek-presentation/assets/architecture.svg", "examples/dist-web/assets/architecture.svg", "architecture.svg in dist-web")

    print(f"\n{BOLD}4. Embedded .slide Packages:{RESET}")
    check_file("crates/slide-viewer/assets/geek_demo.slide", "Embedded geek demo archive")
    check_file("crates/slide-viewer/assets/demo.slide", "Embedded basic demo archive")
    check_file("examples/slides.slide", "Root examples presentation archive")

    print(f"\n{BOLD}5. Exported Web Bundle (examples/dist-web):{RESET}")
    check_file("examples/dist-web/index.html", "dist-web index.html")
    check_file("examples/dist-web/bootstrap.js", "dist-web bootstrap.js")
    check_file("examples/dist-web/style.css", "dist-web style.css")
    check_file("examples/dist-web/slide_web.js", "dist-web slide_web.js")
    check_file("examples/dist-web/slide_web_bg.wasm", "dist-web slide_web_bg.wasm")
    check_file("examples/dist-web/deck.json", "dist-web compiled deck.json")
    check_file("examples/dist-web/assets/architecture.svg", "dist-web architecture.svg")

    print(f"\n{BOLD}6. Packaged & Installed Release Binaries:{RESET}")
    local_bin = Path.home() / ".local" / "bin"
    check_binary(local_bin / ("cargo-slide.exe" if os.name == "nt" else "cargo-slide"), "--version", "Installed CLI")
    check_binary(local_bin / ("slide-viewer.exe" if os.name == "nt" else "slide-viewer"), "--version", "Installed Native Viewer")
    check_binary(local_bin / ("slide-editor.exe" if os.name == "nt" else "slide-editor"), "--version", "Installed Visual Editor")
    check_file("examples/geek-presentation/slides-presentation", "Standalone Runner Binary")

    if failed:
        print(f"\n{RED}{BOLD}Validation failed! One or more asset or binary checks did not pass.{RESET}")
    else:
        print(f"\n{GREEN}{BOLD}✓ All assets & release binaries verified and unified perfectly!{RESET}")

    return not failed


def main():
    parser = argparse.ArgumentParser(description="Cargo-Slide Unified Asset Build, Packaging & Binary Synchronization Tool")
    parser.add_argument("--all", action="store_true", help="Run full pipeline: assets (.typ, .wasm, .svg, .slide), web-dist, rebuild binaries & verify [Default]")
    parser.add_argument("--typ", action="store_true", help="Sync canonical .typ macros to examples & templates")
    parser.add_argument("--wasm", action="store_true", help="Compile Leptos CSR wasm web player and bundle to cargo-slide/pkg")
    parser.add_argument("--svg", action="store_true", help="Verify and synchronize SVG vector diagrams & assets")
    parser.add_argument("--slide", action="store_true", help="Package .slide demo archives into slide-viewer/assets and examples")
    parser.add_argument("--web-dist", action="store_true", help="Export full standalone static web presentation bundle to examples/dist-web")
    parser.add_argument("--rebuild", action="store_true", help="Recompile all release workspace binaries and install to ~/.local/bin")
    parser.add_argument("--check", action="store_true", help="Validate the presence, non-emptiness, and synchronization of all assets & binaries")

    args = parser.parse_args()
    repo_root = get_repo_root()

    do_all = args.all or (not args.typ and not args.wasm and not args.svg and not args.slide and not args.web_dist and not args.rebuild and not args.check)

    if do_all or args.typ:
        sync_typ_assets(repo_root)

    if do_all or args.wasm:
        build_wasm_assets(repo_root)

    if do_all or args.svg:
        verify_svg_assets(repo_root)

    if do_all or args.slide:
        package_slide_archives(repo_root)

    if do_all or args.web_dist:
        export_web_distribution(repo_root)

    if do_all or args.rebuild:
        rebuild_binaries(repo_root)

    if do_all or args.check:
        if not validate_assets_and_binaries(repo_root):
            sys.exit(1)

    print(f"\n{GREEN}{BOLD}🎉 Asset & Binary update process completed successfully!{RESET}\n")


if __name__ == "__main__":
    main()
