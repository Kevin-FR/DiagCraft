#!/usr/bin/env python3
"""Build release archives from previously compiled workspace executables."""
from pathlib import Path
import runpy
import sys

packaging = Path(__file__).resolve().parents[1] / "packaging"
sys.path.insert(0, str(packaging))
runpy.run_path(str(packaging / "package.py"), run_name="__main__")
