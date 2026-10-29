#!/usr/bin/env python3
"""Verify the actual Wayland window frame, including its title and controls.

Run under xvfb-run with weston, ImageMagick and Pillow installed. A nested
Weston compositor without server decorations exercises the same client frame
as GNOME/Ubuntu. The built-in --screenshot captures only the client content.
"""
import argparse
from collections import Counter
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from PIL import Image


def inspect_frame(path, *, legacy=False):
    with Image.open(path) as image:
        pixels = image.convert("RGB")
    # The fixed light test desktop has no settings portal. The old fallback
    # frame is #3a3a3a, regardless of the desktop's appearance preference.
    first_row = 35 if legacy else 0
    best = {"visible": False, "width": pixels.width, "height": pixels.height,
            "first_row": first_row, "header_y": None, "title_pixels": 0, "button_pixels": 0}
    # Weston may place a window directly beneath its desktop bar. The previous
    # scan skipped its first 35 rows and could miss the title at y=0..34.
    for y in range(first_row, min(300, pixels.height - 40)):
        row = [pixels.getpixel((x, y)) for x in range(pixels.width)]
        # Weston can composite a fade-in frame (e.g. 230 instead of 235).
        # The header must remain neutral/light and meet the same glyph counts.
        colors = ((235, 235, 235), (250, 250, 250)) if legacy else (
            color for color, count in Counter(row).items()
            if count >= 1000 and color[0] == color[1] == color[2] and 220 <= color[0] <= 250)
        for color in colors:
            positions = [x for x, pixel in enumerate(row) if pixel == color]
            if len(positions) < 1000:
                continue
            left, right = min(positions), max(positions)
            center = (left + right) // 2
            # The title must be drawn, not just a light rectangle. Also retain
            # the native button icons at the right end of the header.
            title = pixels.crop((center - 220, y + 3, center + 220, y + 32))
            buttons = pixels.crop((right - 105, y + 3, right, y + 32))
            def dark(image):
                data = image.tobytes()
                return sum(max(data[i:i + 3]) < 180 for i in range(0, len(data), 3))
            title_pixels, button_pixels = dark(title), dark(buttons)
            # A ribbon row can also contain text and icons. A real titlebar
            # starts at the desktop/shadow boundary, rather than inside the app.
            above = pixels.crop((left + 8, max(0, y - 2), right - 8, y))
            edge_fraction = dark(above) / (above.width * above.height) if y else 1.
            candidate = {"visible": title_pixels > 50 and button_pixels > 30 and (legacy or edge_fraction >= 0.8),
                         "width": pixels.width, "height": pixels.height, "first_row": first_row,
                         "header_y": y, "header_left": left, "header_right": right,
                         "header_color": list(color), "title_pixels": title_pixels,
                         "button_pixels": button_pixels, "edge_dark_fraction": edge_fraction}
            if min(title_pixels / 50, button_pixels / 30) > min(
                    best["title_pixels"] / 50, best["button_pixels"] / 30):
                best = candidate
            if candidate["visible"]:
                return candidate
    return best


def frame_is_visible(path):
    return inspect_frame(path)["visible"]


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
    report_path = args.output / "window-report.json"
    report = {"status": "starting", "checks": [], "error": None}
    def save_report():
        report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    save_report()
    weston = app = None
    with tempfile.TemporaryDirectory(prefix="diagcraft-wayland-") as directory:
        runtime = Path(directory)
        runtime.chmod(0o700)
        env = dict(os.environ, XDG_RUNTIME_DIR=directory,
                   XDG_CONFIG_HOME=str(runtime / "config"),
                   XDG_DATA_HOME=str(runtime / "data"),
                   XDG_CACHE_HOME=str(runtime / "cache"),
                   XDG_SESSION_TYPE="wayland", LIBGL_ALWAYS_SOFTWARE="1")
        # Isolate this deterministic light desktop from an inherited portal.
        env.pop("DBUS_SESSION_BUS_ADDRESS", None)
        env.pop("WAYLAND_DISPLAY", None)
        with (args.output / "weston.log").open("w") as compositor_log, (
                args.output / "wayland.log").open("w") as app_log:
            try:
                weston = subprocess.Popen([
                    # Weston 9's pixman renderer exits when composing these
                    # alpha shadows. Exercise the GL path with software Mesa.
                    "weston", "--backend=x11-backend.so",
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
                started = time.monotonic()
                deadline = started + 25
                while time.monotonic() < deadline:
                    if app.poll() is not None:
                        raise RuntimeError("Application exited before its window was verified")
                    subprocess.run(["import", "-window", "root", str(screenshot)],
                                   check=True, timeout=5)
                    check = inspect_frame(screenshot)
                    check["elapsed_ms"] = round((time.monotonic() - started) * 1000)
                    report["checks"].append(check)
                    save_report()
                    if check["visible"]:
                        trace = (args.output / "wayland.log").read_text()
                        if "xdg_toplevel" not in trace or "wl_subsurface" not in trace:
                            raise RuntimeError("Expected Wayland client decorations")
                        # Retain whether the older y>=35 detector would have
                        # rejected this exact image, rather than inferring why.
                        previous = inspect_frame(screenshot, legacy=True)
                        report["legacy_visible"] = previous["visible"]
                        report["legacy_header_y"] = previous["header_y"]
                        report["status"] = "passed"
                        save_report()
                        print("Wayland frame verified: light header, visible title and native buttons")
                        return
                    time.sleep(0.25)
                raise RuntimeError("Wayland window still has the fallback frame or no visible title/buttons")
            except Exception as error:
                report["status"] = "failed"
                report["error"] = str(error)
                save_report()
                for log in ("weston.log", "wayland.log"):
                    print(f"{log} (last 60 lines):", flush=True)
                    print("\n".join((args.output / log).read_text().splitlines()[-60:]), flush=True)
                raise
            finally:
                stop(app)
                stop(weston)


if __name__ == "__main__":
    main()
