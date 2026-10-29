"""Synthetic native titlebar regressions; no external screenshot or font required."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

from PIL import Image, ImageDraw

SPEC = importlib.util.spec_from_file_location("native_window", Path(__file__).resolve().parents[1] / "smoke-linux-window.py")
WINDOW = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(WINDOW)


class NativeWindowTests(unittest.TestCase):
    def fixture(self, *, y=24, title=True, buttons=True, color=(235, 235, 235), width=1440):
        image = Image.new("RGB", (1700, 1100), (32, 72, 120))
        draw = ImageDraw.Draw(image)
        left, right = 100, 100 + width
        draw.rectangle((left, y, right, y + 700), fill="white")
        draw.rectangle((left, y, right, y + 34), fill=color)
        center = (left + right) // 2
        if title:
            # Broken bars stand for glyphs; their position matters, not a font.
            for x in range(center - 90, center + 90, 12):
                draw.rectangle((x, y + 8, x + 5, y + 18), fill=(40, 40, 40))
        if buttons:
            for x in (right - 90, right - 55, right - 20):
                draw.rectangle((x, y + 9, x + 7, y + 17), fill=(40, 40, 40))
        return image

    def inspect(self, image, **kwargs):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "frame.png"
            image.save(path)
            return WINDOW.inspect_frame(path, **kwargs)

    def test_valid_titles_at_the_top_are_not_skipped(self):
        for y in (0, 5, 12, 24, 35, 80, 240):
            with self.subTest(y=y):
                result = self.inspect(self.fixture(y=y))
                self.assertTrue(result["visible"])
                self.assertLessEqual(result["header_y"], y)
                self.assertGreater(result["title_pixels"], 50)
                self.assertGreater(result["button_pixels"], 30)

    def test_top_edge_reproduces_the_old_detector_failure(self):
        image = self.fixture(y=0)
        self.assertTrue(self.inspect(image)["visible"])
        self.assertFalse(self.inspect(image, legacy=True)["visible"])

    def test_light_rectangle_without_a_title_is_not_accepted(self):
        self.assertFalse(self.inspect(self.fixture(title=False))["visible"])

    def test_light_rectangle_without_native_buttons_is_not_accepted(self):
        self.assertFalse(self.inspect(self.fixture(buttons=False))["visible"])

    def test_dark_fallback_is_rejected_even_if_it_has_text_and_buttons(self):
        self.assertFalse(self.inspect(self.fixture(color=(58, 58, 58)))["visible"])

    def test_empty_and_narrow_frames_are_not_accepted(self):
        self.assertFalse(self.inspect(Image.new("RGB", (1700, 1100), "white"))["visible"])
        self.assertFalse(self.inspect(self.fixture(width=800))["visible"])

    def test_inactive_light_header_still_requires_title_and_buttons(self):
        self.assertTrue(self.inspect(self.fixture(color=(250, 250, 250)))["visible"])
        self.assertFalse(self.inspect(self.fixture(color=(250, 250, 250), title=False))["visible"])

    def test_fade_in_titlebar_is_recognized_at_the_window_edge(self):
        result = self.inspect(self.fixture(color=(230, 230, 230)))
        self.assertTrue(result["visible"])
        self.assertEqual(result["header_color"], [230, 230, 230])

    def test_text_and_icons_in_a_ribbon_row_are_not_a_native_titlebar(self):
        image = self.fixture(title=False, buttons=False, color=(230, 230, 230))
        draw = ImageDraw.Draw(image)
        draw.rectangle((100, 268, 1540, 302), fill=(250, 250, 250))
        draw.rectangle((730, 276, 910, 286), fill=(40, 40, 40))
        draw.rectangle((1450, 277, 1505, 285), fill=(40, 40, 40))
        self.assertTrue(self.inspect(image, legacy=True)["visible"])
        self.assertFalse(self.inspect(image)["visible"])


if __name__ == "__main__":
    unittest.main()
