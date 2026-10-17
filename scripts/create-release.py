#!/usr/bin/env python3
"""Create a draft only after all platform and audit jobs succeed."""
from pathlib import Path
import hashlib
import json
import os
import re
import subprocess

root = Path(__file__).resolve().parents[1]
tag = os.environ["RELEASE_TAG"]
assert re.fullmatch(r"v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", tag)
repository = os.environ["GITHUB_REPOSITORY"]
assets = sorted(p for p in (root / "release-assets").iterdir()
                if p.is_file() and p.name != "SHA256SUMS.txt")
expected = {f"diagcraft-{tag[1:]}-macos-arm64.zip", f"diagcraft-cli-{tag[1:]}-macos-arm64.zip",
            f"diagcraft-{tag[1:]}-windows-x86_64.zip", f"diagcraft-{tag[1:]}-linux-x86_64.tar.gz"}
assert {p.name for p in assets} == expected, "Incomplete or unexpected release asset set"
checksums = root / "release-assets/SHA256SUMS.txt"
checksums.write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n" for p in assets))
existing = subprocess.run(["gh", "release", "view", tag, "--repo", repository, "--json", "isDraft"], capture_output=True, text=True, encoding="utf-8")
if existing.returncode == 0:
    raise SystemExit("Refusing to replace an existing release. Publish the draft or use a new version.")
subprocess.run(["gh", "release", "create", tag, "--repo", repository, "--verify-tag", "--draft",
                "--title", "DiagCraft " + tag, "--notes-file", str(root / "docs/releases" / (tag[1:] + ".md")),
                *map(str, assets), str(checksums)], check=True)
