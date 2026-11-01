"""Build a drag-to-Applications macOS disk image for a signed DiagCraft app."""
from pathlib import Path
import argparse
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run([str(x) for x in args], check=True, **kwargs)


def create(app: Path, output: Path):
    if sys.platform != "darwin":
        raise RuntimeError("A macOS host is required to create a disk image")
    if not app.is_dir() or app.name != "DiagCraft.app":
        raise ValueError("Expected a DiagCraft.app bundle")
    if output.exists():
        raise FileExistsError(f"Refusing to replace an existing disk image: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="diagcraft-dmg-") as temp:
        temp = Path(temp)
        source = temp / "contents"
        source.mkdir()
        run("ditto", app, source / app.name)
        (source / "Applications").symlink_to("/Applications", target_is_directory=True)
        background = source / ".background"
        background.mkdir()
        run("swift", ROOT / "packaging/dmg_background.swift", ROOT / "assets/brand/diagcraft-logo.png", background / "background.png")
        writable = temp / "writable.dmg"
        run("hdiutil", "create", "-quiet", "-volname", "DiagCraft", "-srcfolder", source, "-fs", "HFS+", "-format", "UDRW", writable)
        mount = temp / "volume"
        mount.mkdir()
        run("hdiutil", "attach", "-quiet", "-noverify", "-nobrowse", "-mountpoint", mount, writable)
        try:
            # Finder writes the window background and icon positions into .DS_Store.
            run("osascript", ROOT / "packaging/dmg_layout.applescript", mount, timeout=60)
            run("sync")
        finally:
            run("hdiutil", "detach", "-quiet", mount)
        compressed = temp / "final.dmg"
        run("hdiutil", "convert", "-quiet", writable, "-format", "UDZO", "-o", compressed)
        run("hdiutil", "verify", "-quiet", compressed)
        inspect = temp / "inspect"
        inspect.mkdir()
        run("hdiutil", "attach", "-quiet", "-readonly", "-nobrowse", "-mountpoint", inspect, compressed)
        try:
            if not (inspect / "Applications").is_symlink() or (inspect / "Applications").readlink() != Path("/Applications"):
                raise RuntimeError("Missing Applications link in disk image")
            if not (inspect / ".DS_Store").is_file() or not (inspect / ".background/background.png").is_file():
                raise RuntimeError("Missing Finder layout or background")
            run("codesign", "--verify", "--deep", "--strict", inspect / app.name)
        finally:
            run("hdiutil", "detach", "-quiet", inspect)
        with compressed.open("rb") as source_file, output.open("xb") as output_file:
            shutil.copyfileobj(source_file, output_file)
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    print(create(args.app.resolve(), args.output.resolve()))


if __name__ == "__main__":
    main()
