#!/usr/bin/env python3
"""Verify the actual Wayland window frame, including its title and controls.

Run under xvfb-run with weston, ImageMagick and Pillow installed. A nested
Weston compositor without server decorations exercises the same client frame
as GNOME/Ubuntu. The built-in --screenshot captures only the client content.
"""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile
import time

from PIL import Image


def frame_is_visible(path):
    with Image.open(path) as image:
        pixels = image.convert("RGB")
    # The fixed light test desktop has no settings portal. The old fallback
    # frame is #3a3a3a, regardless of the desktop's appearance preference.
    for y in range(35, min(300, pixels.height - 40)):
        row = [pixels.getpixel((x, y)) for x in range(pixels.width)]
        for color in ((235, 235, 235), (250, 250, 250)):
            positions = [x for x, pixel in enumerate(row) if pixel == color]
            if len(positions) < 1000:
                continue
            left, right = min(positions), max(positions)
            center = (left + right) // 2
            # The title must be drawn, not just a light rectangle. Also retain
            # the native button icons at the right end of the header.
            title = pixels.crop((center - 220, y + 3, center + 220, y + 32))
            buttons = pixels.crop((right - 105, y + 3, right, y + 32))
            dark = lambda image: sum(max(pixel) < 180 for pixel in image.getdata())
            if dark(title) > 50 and dark(buttons) > 30:
                return True
    return False


def stop(process):
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    screenshot = args.output.resolve() / "wayland-window.png"
    weston = app = None
    with tempfile.TemporaryDirectory(prefix="diagcraft-wayland-") as directory:
        runtime = Path(directory)
        runtime.chmod(0o700)
        env = dict(os.environ, XDG_RUNTIME_DIR=directory,
                   XDG_CONFIG_HOME=str(runtime / "config"),
                   XDG_DATA_HOME=str(runtime / "data"),
                   XDG_CACHE_HOME=str(runtime / "cache"),
                   XDG_SESSION_TYPE="wayland")
        # Isolate this deterministic light desktop from an inherited portal.
        env.pop("DBUS_SESSION_BUS_ADDRESS", None)
        env.pop("WAYLAND_DISPLAY", None)
        with (args.output / "weston.log").open("w") as compositor_log, (
                args.output / "wayland.log").open("w") as app_log:
            try:
                weston = subprocess.Popen([
                    "weston", "--backend=x11-backend.so", "--use-pixman",
                    "--width=1600", "--height=1100", "--idle-time=0",
                    "--socket=wayland-diagcraft-test"],
                    env=env, stdout=compositor_log, stderr=subprocess.STDOUT)
                deadline = time.monotonic() + 15
                while not (runtime / "wayland-diagcraft-test").exists():
                    if weston.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError("Wayland compositor did not start")
                    time.sleep(0.1)
                env["WAYLAND_DISPLAY"] = "wayland-diagcraft-test"
                env["WAYLAND_DEBUG"] = "1"
                app = subprocess.Popen([str(args.bin.resolve()), "--language", "en"],
                                       env=env, stdout=app_log, stderr=subprocess.STDOUT)
                deadline = time.monotonic() + 25
                while time.monotonic() < deadline:
                    if app.poll() is not None:
                        raise RuntimeError("Application exited before its window was verified")
                    subprocess.run(["import", "-window", "root", str(screenshot)],
                                   check=True, timeout=5)
                    if frame_is_visible(screenshot):
                        trace = (args.output / "wayland.log").read_text()
                        if "xdg_toplevel" not in trace or "wl_subsurface" not in trace:
                            raise RuntimeError("Expected Wayland client decorations")
                        print("Wayland frame verified: light header, visible title and native buttons")
                        return
                    time.sleep(0.25)
                raise RuntimeError("Wayland window still has the fallback frame or no visible title/buttons")
            except Exception:
                for log in ("weston.log", "wayland.log"):
                    print(f"{log} (last 60 lines):", flush=True)
                    print("\n".join((args.output / log).read_text().splitlines()[-60:]), flush=True)
                raise
            finally:
                stop(app)
                stop(weston)


if __name__ == "__main__":
    main()
