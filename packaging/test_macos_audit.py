import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from macos_audit import inspect


class NativeAuditTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = (Path(self.temp.name) / "DiagCraft.app").resolve()
        self.binary = self.root / "Contents/MacOS/diagcraft"
        self.binary.parent.mkdir(parents=True)
        self.binary.write_bytes(b"\xcf\xfa\xed\xfe" + b"test fixture")

    def tool(self, *args):
        if args[:2] == ("lipo", "-archs"):
            return "arm64\n"
        if args[:2] == ("otool", "-l"):
            return ""
        return str(args[-1]) + ":\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1.0.0)\n"

    def test_native_system_only(self):
        with patch("macos_audit.output", side_effect=self.tool):
            report = inspect(self.root)
        self.assertFalse(report["rosetta_required"])
        self.assertEqual(len(report["binaries"]), 1)

    def test_intel_or_universal_code_is_rejected(self):
        for architectures in ("x86_64", "x86_64 arm64"):
            with patch("macos_audit.output", return_value=architectures):
                with self.assertRaisesRegex(ValueError, "Apple Silicon only"):
                    inspect(self.root)

    def test_homebrew_and_unresolved_dependencies_are_rejected(self):
        for dependency in ("/usr/local/lib/intel.dylib", "/opt/homebrew/lib/lib.dylib", "@rpath/missing.dylib"):
            def tool(*args):
                if args[:2] == ("otool", "-L"):
                    return str(args[-1]) + ":\n\t" + dependency + " (compatibility version 1.0.0)\n"
                return self.tool(*args)
            with patch("macos_audit.output", side_effect=tool):
                with self.assertRaisesRegex(ValueError, "External/unresolved"):
                    inspect(self.root)

    def test_every_embedded_library_is_audited(self):
        library = self.root / "Contents/Resources/extra.dylib"
        library.parent.mkdir()
        library.write_bytes(self.binary.read_bytes())
        def tool(*args):
            if args[:2] == ("lipo", "-archs") and args[-1] == library:
                return "x86_64"
            return self.tool(*args)
        with patch("macos_audit.output", side_effect=tool):
            with self.assertRaisesRegex(ValueError, "Apple Silicon only"):
                inspect(self.root)


if __name__ == "__main__":
    unittest.main()
