#!/usr/bin/env python3
"""Verify the installed .deb, ownership, notices and desktop/MIME integration."""
from pathlib import Path
import hashlib
import json
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
PACKAGE = ROOT / "dist" / f"diagcraft_{VERSION}_amd64.deb"


def output(*args):
    return subprocess.check_output(args, text=True, encoding="utf-8").strip()


assert output("dpkg-query", "-W", "-f=${Version} ${Architecture} ${Status}", "diagcraft") == f"{VERSION} amd64 install ok installed"
assert output("/usr/bin/diagcraft", "--version") == f"DiagCraft {VERSION}"
assert output("/usr/bin/diagcraft-cli", "--version") == f"DiagCraft CLI {VERSION}"
with tempfile.TemporaryDirectory(prefix="diagcraft-deb-check-") as temporary:
    tar = Path(temporary) / "data.tar"
    with tar.open("wb") as stream:
        subprocess.run(["dpkg-deb", "--fsys-tarfile", str(PACKAGE)], stdout=stream, check=True)
    with tarfile.open(tar) as payload:
        for member in payload:
            assert member.uid == member.gid == 0, member.name
            assert not member.issym() and not member.islnk(), member.name
            assert member.name == "." or member.name.startswith("./usr/"), member.name
            assert member.mode == (0o755 if member.isdir() or member.name.startswith("./usr/bin/") else 0o644), member.name
    control = Path(temporary) / "control"
    subprocess.run(["dpkg-deb", "--control", str(PACKAGE), str(control)], check=True)
    for line in (control / "md5sums").read_text().splitlines():
        checksum, name = line.split("  ", 1)
        assert hashlib.md5((Path("/") / name).read_bytes()).hexdigest() == checksum, name

docs = Path("/usr/share/doc/diagcraft")
licenses = docs / "Dependency-Licenses"
manifest = json.loads((licenses / "manifest.json").read_text())
assert manifest["target"] == "x86_64-unknown-linux-gnu"
assert manifest["cargo_lock_sha256"] == hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest()
for package in manifest["packages"]:
    for notice in package["notices"]:
        assert hashlib.sha256((licenses / notice["file"]).read_bytes()).hexdigest() == notice["sha256"]
for name in ("LICENSE", "NOTICE", "THIRD_PARTY.md", "Rust-COPYRIGHT.html", "Rust-COPYRIGHT-library.html",
             "docs/commandes.md", "docs/images/commands.png", "docs/images/shortcuts.png"):
    assert (docs / name).is_file(), name
subprocess.run(["desktop-file-validate", "/usr/share/applications/org.diagcraft.editor.desktop"], check=True)
assert output("xdg-mime", "query", "filetype", str(ROOT / "examples/parcours.draw")) == "application/x-diagcraft"
assert Path("/usr/share/icons/hicolor/256x256/apps/org.diagcraft.editor.png").read_bytes() == (ROOT / "assets/brand/DiagCraft.iconset/icon_256x256.png").read_bytes()
print("Debian installation verified: version, root ownership, permissions, payload checksums, licenses, desktop entry and MIME.")
