#!/usr/bin/env python3
from pathlib import Path
import os
import re
import tomllib

root = Path(__file__).resolve().parents[1]
version = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
tag = os.environ["RELEASE_TAG"]
assert re.fullmatch(r"v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", tag), "Invalid release tag"
assert tag == "v" + version, "Tag differs from workspace version"
assert (root / "docs/releases" / (version + ".md")).is_file(), "Release notes are required"
print(f"Release manifest and notes match {tag}")
