"""Embedded asset notices must supplement a crate's existing code license."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import licenses


class SupplementalNoticeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.crate = self.root / "registry/example-1.0.0"
        self.crate.mkdir(parents=True)
        (self.crate / "Cargo.toml").write_text("[package]\nname='example'\n")
        (self.crate / "LICENSE").write_text("Example code license\n")
        (self.crate / "font.ttf").write_bytes(b"synthetic font fixture")
        notice = self.root / "assets/licenses/upstream/example-1.0.0/Font-LICENSE.txt"
        notice.parent.mkdir(parents=True)
        notice.write_text("Example font license\n")
        entry = {"crate": "example 1.0.0", "file": notice.relative_to(self.root).as_posix(),
                 "sha256": licenses.sha(notice), "supplemental": True,
                 "source_asset": "font.ttf", "source_asset_sha256": licenses.sha(self.crate / "font.ttf")}
        (self.root / "assets/licenses/provenance.json").write_text(json.dumps([entry]))
        (self.root / "Cargo.lock").write_text("fixture lock\n")
        self.metadata = {"workspace_members": [], "packages": [
            {"id": "example", "name": "example", "version": "1.0.0", "license": "MIT",
             "manifest_path": str(self.crate / "Cargo.toml"), "repository": "https://example.org"}]}
        self.output = self.root / "notices"

    def collect(self):
        with patch.object(licenses, "ROOT", self.root), patch.object(
                licenses.subprocess, "check_output",
                side_effect=[json.dumps(self.metadata), "example v1.0.0\n"]):
            return licenses.collect("x86_64-unknown-linux-gnu", self.output)

    def test_code_and_embedded_font_notices_are_both_packaged(self):
        manifest = self.collect()
        notices = manifest[0]["notices"]
        self.assertEqual({Path(n["file"]).name for n in notices}, {"LICENSE", "Font-LICENSE.txt"})
        for notice in notices:
            self.assertEqual(hashlib.sha256((self.output / notice["file"]).read_bytes()).hexdigest(),
                             notice["sha256"])
        text = (self.output / "THIRD-PARTY-NOTICES.txt").read_text()
        self.assertIn("Example code license", text)
        self.assertIn("Example font license", text)

    def test_changed_font_cannot_reuse_the_curated_notice(self):
        (self.crate / "font.ttf").write_bytes(b"different font")
        with self.assertRaisesRegex(RuntimeError, "asset hash mismatch"):
            self.collect()

    def test_changed_notice_is_rejected(self):
        (self.root / "assets/licenses/upstream/example-1.0.0/Font-LICENSE.txt").write_text("changed")
        with self.assertRaisesRegex(RuntimeError, "notice hash mismatch"):
            self.collect()


if __name__ == "__main__":
    unittest.main()
