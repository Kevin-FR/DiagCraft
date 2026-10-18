"""Portable release archives with an explicit allowlist and complete notices."""
from pathlib import Path
import hashlib
import json
import platform
import shutil
import subprocess
import sys
import tarfile
import tomllib
import zipfile
from licenses import collect

ROOT = Path(__file__).resolve().parents[1]


def checksums():
    files = sorted(p for p in (ROOT / "dist").iterdir() if p.suffix == ".zip" or p.name.endswith(".tar.gz"))
    (ROOT / "dist/SHA256SUMS.txt").write_text("".join(
        f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n" for p in files))


def main():
    if platform.system() == "Darwin":
        subprocess.run([sys.executable, str(ROOT / "scripts/package-macos-test.py")], check=True)
        checksums()
        return
    system = platform.system().lower()
    if system not in ("linux", "windows") or platform.machine().lower() not in ("amd64", "x86_64"):
        raise SystemExit("This release packager supports Linux/Windows x86_64 and macOS arm64.")
    target = "x86_64-pc-windows-msvc" if system == "windows" else "x86_64-unknown-linux-gnu"
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    name = f"diagcraft-{version}-{system}-x86_64"
    stage = ROOT / "dist" / name
    archive = ROOT / "dist" / (name + (".zip" if system == "windows" else ".tar.gz"))
    if stage.exists() or archive.exists():
        raise SystemExit(f"Refusing to replace an existing release: {name}")
    stage.mkdir(parents=True)
    extension = ".exe" if system == "windows" else ""
    for filename in ("diagcraft", "diagcraft-cli"):
        shutil.copy2(ROOT / "target/release" / (filename + extension), stage / (filename + extension))
    for filename in ("LICENSE", "NOTICE", "THIRD_PARTY.md"):
        shutil.copy2(ROOT / filename, stage / filename)
    (stage / "README.md").write_text(
        f"DiagCraft {version}\n\n" + (ROOT / "docs/installation.md").read_text(encoding="utf-8"),
        encoding="utf-8")
    shutil.copytree(ROOT / "docs", stage / "docs")
    shutil.copytree(ROOT / "assets/fonts", stage / "Fonts")
    collect(target, stage / "Dependency-Licenses")
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True, encoding="utf-8").strip())
    for filename in ("COPYRIGHT.html", "COPYRIGHT-library.html"):
        shutil.copy2(sysroot / "share/doc/rust" / filename, stage / ("Rust-" + filename))
    if system == "windows":
        # Toolchain notices can retain Unix-epoch dates, outside ZIP's range.
        # Clamp archive timestamps while copying the file contents unchanged.
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED,
                             strict_timestamps=False) as output:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    output.write(path, path.relative_to(stage.parent))
    else:
        with tarfile.open(archive, "w:gz") as output:
            output.add(stage, arcname=name)
    checksums()
    print(f"Archive: {archive}")


if __name__ == "__main__":
    main()
