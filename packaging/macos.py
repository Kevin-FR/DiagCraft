"""Package only Apple Silicon; verify signatures before and after extraction."""
from pathlib import Path
import argparse
import hashlib
import json
import plistlib
import shutil
import subprocess
import sys
import tempfile
import tomllib
from licenses import collect
from macos_audit import inspect

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run([str(x) for x in args], check=True, **kwargs)


def main():
    if sys.platform != "darwin":
        raise SystemExit("macOS and Xcode command-line tools are required.")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arm64", type=Path, default=ROOT / "target/release/diagcraft")
    parser.add_argument("--cli", type=Path, default=ROOT / "target/release/diagcraft-cli")
    args = parser.parse_args()
    for binary in (args.arm64, args.cli):
        if not binary.is_file() or run("lipo", "-archs", binary, capture_output=True, text=True, encoding="utf-8").stdout.strip() != "arm64":
            parser.error(f"An arm64-only executable is required: {binary}")
    name = f"diagcraft-{version}-macos-arm64"
    cli_name = f"diagcraft-cli-{version}-macos-arm64"
    stage, cli_stage = ROOT / "dist" / name, ROOT / "dist" / cli_name
    for path in (stage, cli_stage, ROOT / "dist" / (name + ".zip"), ROOT / "dist" / (cli_name + ".zip")):
        if path.exists():
            parser.error(f"Refusing to replace an existing release: {path}")
    app = stage / "DiagCraft.app"
    contents = app / "Contents"
    macos, resources = contents / "MacOS", contents / "Resources"
    macos.mkdir(parents=True)
    resources.mkdir()
    for binary, filename in ((args.arm64, "diagcraft"), (args.cli, "diagcraft-cli")):
        shutil.copy2(binary, macos / filename)
        (macos / filename).chmod(0o755)
    info = {
        "CFBundleExecutable": "diagcraft", "CFBundleName": "DiagCraft",
        "CFBundleDisplayName": "DiagCraft", "CFBundleIdentifier": "org.diagcraft.editor",
        "CFBundleVersion": version, "CFBundleShortVersionString": version,
        "CFBundlePackageType": "APPL", "NSHighResolutionCapable": True,
        "CFBundleDevelopmentRegion": "en",
        "CFBundleLocalizations": ["en", "fr", "de", "es", "it", "pt", "zh-Hans"],
        "LSArchitecturePriority": ["arm64"],
        "CFBundleIconFile": "DiagCraft.icns", "LSMinimumSystemVersion": "13.0",
        "NSHumanReadableCopyright": "Copyright 2026 DiagCraft contributors",
    }
    (contents / "Info.plist").write_bytes(plistlib.dumps(info))
    shutil.copy2(ROOT / "assets/brand/DiagCraft.icns", resources / "DiagCraft.icns")
    shutil.copy2(ROOT / "assets/brand/provenance.json", resources / "Logo-Provenance.json")
    for filename in ("LICENSE", "NOTICE", "THIRD_PARTY.md"):
        shutil.copy2(ROOT / filename, resources / filename)
    shutil.copytree(ROOT / "assets/fonts", resources / "Fonts")
    agents = resources / "Agents"
    agents.mkdir()
    for filename in ("AGENTS-DRAW.md", "agent-patch.schema.json"):
        shutil.copy2(ROOT / "docs" / filename, agents / filename)
    guide = f"DiagCraft {version}\n\n" + (ROOT / "docs/installation.md").read_text(encoding="utf-8")
    (resources / "INSTALLATION.md").write_text(guide)
    manifest = collect("aarch64-apple-darwin", resources / "Dependency-Licenses")
    sysroot = Path(run("rustc", "--print", "sysroot", capture_output=True, text=True, encoding="utf-8").stdout.strip())
    for filename in ("COPYRIGHT.html", "COPYRIGHT-library.html"):
        source = sysroot / "share/doc/rust" / filename
        if not source.is_file():
            raise RuntimeError(f"Missing Rust notice: {source}")
        shutil.copy2(source, resources / ("Rust-" + filename))
    native_audit = inspect(app)
    (resources / "Native-Architecture.json").write_text(json.dumps(native_audit, indent=2) + "\n")
    for binary in macos.iterdir():
        run("codesign", "--force", "--sign", "-", binary)
    run("codesign", "--force", "--sign", "-", app)
    run("codesign", "--verify", "--deep", "--strict", app)
    archive = ROOT / "dist" / (name + ".zip")
    run("ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", app, archive)
    with tempfile.TemporaryDirectory(prefix="diagcraft-package-") as temp:
        run("ditto", "-x", "-k", archive, temp)
        extracted = Path(temp) / app.name
        run("codesign", "--verify", "--deep", "--strict", extracted)
        assert inspect(extracted) == native_audit
        for filename in ("diagcraft", "diagcraft-cli"):
            binary = extracted / "Contents/MacOS" / filename
            assert run("lipo", "-archs", binary, capture_output=True, text=True, encoding="utf-8").stdout.strip() == "arm64"
            assert binary.read_bytes() == (macos / filename).read_bytes()
            assert version in run(binary, "--version", capture_output=True, text=True, encoding="utf-8").stdout
    cli_stage.mkdir()
    shutil.copy2(macos / "diagcraft-cli", cli_stage / "diagcraft-cli")
    shutil.copytree(resources / "Dependency-Licenses", cli_stage / "Dependency-Licenses")
    for filename in ("LICENSE", "NOTICE", "THIRD_PARTY.md", "Rust-COPYRIGHT.html", "Rust-COPYRIGHT-library.html"):
        shutil.copy2(resources / filename, cli_stage / filename)
    shutil.copytree(agents, cli_stage / "Agents")
    shutil.copytree(ROOT / "assets/fonts", cli_stage / "Fonts")
    cli_archive = ROOT / "dist" / (cli_name + ".zip")
    run("ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", cli_stage, cli_archive)
    report = {"version": version, "target": "aarch64-apple-darwin", "minimum_macos": "13.0",
              "signature": "ad hoc; verified before and after extraction", "notarized": False,
              "native_architecture": native_audit,
              "dependency_notice_packages": len(manifest), "archive": archive.name,
              "sha256": hashlib.sha256(archive.read_bytes()).hexdigest()}
    (stage / "distribution-check.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"App: {app}\nArchive: {archive}\nCLI: {cli_archive}\nNotices: {len(manifest)} packages")


if __name__ == "__main__":
    main()
