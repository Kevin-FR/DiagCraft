#!/usr/bin/env python3
"""Check the public source allowlist, asset hashes, secrets and local doc links."""
from pathlib import Path
import hashlib
import json
import os
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
ALLOWED_DIRS = {".github", "apps", "crates", "assets", "docs", "examples", "packaging", "scripts"}
ALLOWED_FILES = {".gitignore", ".gitattributes", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "deny.toml",
                 "LICENSE", "NOTICE", "README.md", "THIRD_PARTY.md", "ROADMAP.md", "CHANGELOG.md",
                 "CONTRIBUTING.md", "SECURITY.md", "AGENTS.md", "CLAUDE.md"}
LOCAL_ONLY = {".git", "target", "dist", "output", "research", ".DS_Store", "__pycache__"}


def files():
    result = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
                            cwd=ROOT, capture_output=True)
    if result.returncode == 0:
        return [ROOT / p.decode() for p in result.stdout.split(b"\0") if p]
    found = []
    for base, dirs, names in os.walk(ROOT):
        dirs[:] = [d for d in dirs if d not in LOCAL_ONLY]
        found.extend(Path(base) / n for n in names if n not in LOCAL_ONLY and not n.endswith((".pyc", ".log", ".tmp")))
    return found


def check():
    paths = files()
    for path in paths:
        rel = path.relative_to(ROOT)
        if (len(rel.parts) == 1 and rel.name not in ALLOWED_FILES) or (
                len(rel.parts) > 1 and rel.parts[0] not in ALLOWED_DIRS):
            raise ValueError(f"Unexpected public path: {rel}")
        if path.is_symlink():
            raise ValueError(f"Symlinks are not release source assets: {rel}")
        data = path.read_bytes()
        if data.startswith((b'\xcf\xfa\xed\xfe', b'\xfe\xed\xfa\xcf', b'MZ', b'\x7fELF')):
            raise ValueError(f"Executable in source tree: {rel}")
        try:
            text = data.decode("utf-8")
        except UnicodeDecodeError:
            if not str(rel).startswith("assets/") and not str(rel).startswith("docs/images/"):
                raise ValueError(f"Unexpected binary asset: {rel}")
            continue
        if re.search(r"gh[pousr]_[A-Za-z0-9]{20,}|-----BEGIN (?:RSA |OPENSSH |EC )?PRIVATE KEY-----", text):
            raise ValueError(f"Possible credential: {rel}")
        if re.search(r"/(?:Users|home)/[A-Za-z0-9_.-]+/|/var/(?:folders)/", text):
            raise ValueError(f"Machine-specific path: {rel}")
        if path.suffix == ".md":
            for target in re.findall(r"!?\[[^\]]*\]\(([^)]+)\)", text):
                target = target.split("#")[0]
                if not target or re.match(r"[a-z]+://", target):
                    continue
                if not (path.parent / target).exists():
                    raise ValueError(f"Broken local link: {rel} -> {target}")
    for entry in json.loads((ROOT / "assets/fonts/provenance.json").read_text(encoding="utf-8")):
        path = ROOT / "assets/fonts" / entry["file"]
        assert hashlib.sha256(path.read_bytes()).hexdigest() == entry["sha256"], path
    entry = json.loads((ROOT / "assets/fonts/hack-provenance.json").read_text(encoding="utf-8"))
    assert hashlib.sha256((ROOT / "assets/fonts/Hack-Regular.ttf").read_bytes()).hexdigest() == entry["sha256"]
    for entry in json.loads((ROOT / "assets/licenses/provenance.json").read_text(encoding="utf-8")):
        if "file" in entry:
            assert hashlib.sha256((ROOT / entry["file"]).read_bytes()).hexdigest() == entry["sha256"], entry["file"]
    for entry in json.loads((ROOT / "assets/brand/provenance.json").read_text(encoding="utf-8"))["assets"]:
        assert hashlib.sha256((ROOT / entry["file"]).read_bytes()).hexdigest() == entry["sha256"], entry["file"]
    print(f"Public source check passed: {len(paths)} files; asset/notices hashes and documentation links verified.")


if __name__ == "__main__":
    try:
        check()
    except (ValueError, AssertionError) as error:
        sys.exit(str(error))
