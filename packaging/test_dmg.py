import tempfile
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch, call

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

    def test_busy_volume_is_retried_without_forcing(self):
        mount = Path("/tmp/example-volume")
        busy = subprocess.CalledProcessError(16, ["hdiutil", "detach"])
        with patch.object(dmg, "run", side_effect=[busy, busy, None]) as execute, patch.object(dmg.time, "sleep") as sleep:
            dmg.detach(mount)
        self.assertEqual(execute.call_args_list, [call("hdiutil", "detach", "-quiet", mount)] * 3)
        self.assertEqual(sleep.call_args_list, [call(0.5), call(1.0)])

    def test_unexpected_detach_error_is_not_retried(self):
        failure = subprocess.CalledProcessError(1, ["hdiutil", "detach"])
        with patch.object(dmg, "run", side_effect=failure) as execute, patch.object(dmg.time, "sleep") as sleep:
            with self.assertRaises(subprocess.CalledProcessError):
                dmg.detach(Path("/tmp/example-volume"))
        self.assertEqual(execute.call_count, 1)
        sleep.assert_not_called()

    def test_persistent_busy_volume_fails_without_forcing(self):
        failure = subprocess.CalledProcessError(16, ["hdiutil", "detach"])
        with patch.object(dmg, "run", side_effect=failure) as execute, patch.object(dmg.time, "sleep"):
            with self.assertRaises(subprocess.CalledProcessError):
                dmg.detach(Path("/tmp/example-volume"))
        self.assertEqual(execute.call_count, 6)
        self.assertTrue(all("-force" not in invocation.args for invocation in execute.call_args_list))

    def test_rejects_unexpected_app_bundle(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(dmg.sys, "platform", "darwin"):
            root = Path(temporary)
            other = root / "Other.app"
            other.mkdir()
            with self.assertRaises(ValueError):
                dmg.create(other, root / "result.dmg")


if __name__ == "__main__":
    unittest.main()
