import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import dmg


class DmgSafetyTests(unittest.TestCase):
    def test_existing_disk_image_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(dmg.sys, "platform", "darwin"):
            root = Path(temporary)
            app = root / "DiagCraft.app"
            app.mkdir()
            output = root / "existing.dmg"
            output.write_bytes(b"existing image")
            with self.assertRaises(FileExistsError):
                dmg.create(app, output)
            self.assertEqual(output.read_bytes(), b"existing image")

    def test_rejects_unexpected_app_bundle(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(dmg.sys, "platform", "darwin"):
            root = Path(temporary)
            other = root / "Other.app"
            other.mkdir()
            with self.assertRaises(ValueError):
                dmg.create(other, root / "result.dmg")


if __name__ == "__main__":
    unittest.main()
