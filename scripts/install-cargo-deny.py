#!/usr/bin/env python3
"""Install the pinned official Linux audit tool after verifying its SHA-256."""
from pathlib import Path
import hashlib
import os
import tarfile
import tempfile
import urllib.request

VERSION = "0.20.2"
NAME = f"cargo-deny-{VERSION}-x86_64-unknown-linux-musl.tar.gz"
BASE = f"https://github.com/EmbarkStudios/cargo-deny/releases/download/{VERSION}/"

with tempfile.TemporaryDirectory() as temp:
    temp = Path(temp)
    for name in (NAME, NAME + ".sha256"):
        urllib.request.urlretrieve(BASE + name, temp / name)
    expected = (temp / (NAME + ".sha256")).read_text(encoding="utf-8").split()[0]
    assert hashlib.sha256((temp / NAME).read_bytes()).hexdigest() == expected
    with tarfile.open(temp / NAME) as archive:
        archive.extractall(temp, filter="data")
    binary = next(temp.rglob("cargo-deny"))
    destination = Path.home() / ".cargo/bin/cargo-deny"
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(binary.read_bytes())
    destination.chmod(0o755)
    print(f"Installed cargo-deny {VERSION} with verified release checksum.")
