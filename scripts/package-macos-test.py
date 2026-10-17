#!/usr/bin/env python3
"""Create the Apple Silicon application and headless CLI archives."""
from pathlib import Path
import runpy
import sys

packaging = Path(__file__).resolve().parents[1] / "packaging"
sys.path.insert(0, str(packaging))
runpy.run_path(str(packaging / "macos.py"), run_name="__main__")
