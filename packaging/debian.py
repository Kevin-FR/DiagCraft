"""Build a Debian amd64 package from the already audited portable staging tree."""
from pathlib import Path
import hashlib
import math
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def build(stage: Path, version: str) -> Path:
    if subprocess.check_output(["dpkg", "--print-architecture"], text=True).strip() != "amd64":
        raise RuntimeError("The Debian release target is amd64.")
    output = stage.parent / f"diagcraft_{version}_amd64.deb"
    if output.exists():
        raise FileExistsError(f"Refusing to replace {output.name}")
    with tempfile.TemporaryDirectory(prefix="diagcraft-deb-") as temporary:
        work = Path(temporary)
        package = work / "package"
        binaries = package / "usr/bin"
        docs = package / "usr/share/doc/diagcraft"
        binaries.mkdir(parents=True)
        docs.mkdir(parents=True)
        for name in ("diagcraft", "diagcraft-cli"):
            shutil.copyfile(stage / name, binaries / name)
            (binaries / name).chmod(0o755)
        for item in sorted(stage.iterdir()):
            if item.name in ("diagcraft", "diagcraft-cli"):
                continue
            if item.is_dir():
                shutil.copytree(item, docs / item.name)
            else:
                shutil.copyfile(item, docs / item.name)
        # Debian's copyright file points to the bundled, unchanged upstream notices.
        (docs / "copyright").write_text(
            "DiagCraft contributors\nSource: https://github.com/Kevin-FR/DiagCraft\n"
            + (ROOT / "LICENSE").read_text(encoding="utf-8") + "\n"
            "Third-party components retain their own licenses. See NOTICE, THIRD_PARTY.md,\n"
            "Dependency-Licenses/, Fonts/, Rust-COPYRIGHT.html and Rust-COPYRIGHT-library.html.\n",
            encoding="utf-8")
        assets = {
            "org.diagcraft.editor.desktop": "usr/share/applications/org.diagcraft.editor.desktop",
            "org.diagcraft.editor.xml": "usr/share/mime/packages/org.diagcraft.editor.xml",
        }
        for source, destination in assets.items():
            path = package / destination
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / "packaging/linux" / source, path)
        icon = package / "usr/share/icons/hicolor/256x256/apps/org.diagcraft.editor.png"
        icon.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "assets/brand/DiagCraft.iconset/icon_256x256.png", icon)
        subprocess.run(["desktop-file-validate", str(package / assets["org.diagcraft.editor.desktop"])], check=True)
        # dpkg-shlibdeps derives minimum versions from ELF symbols. Libraries opened
        # dynamically by winit/glutin need explicit dependencies as well.
        (work / "debian").mkdir()
        (work / "debian/control").write_text("Source: diagcraft\n\nPackage: diagcraft\nArchitecture: amd64\n", encoding="utf-8")
        generated = subprocess.check_output(
            ["dpkg-shlibdeps", "-O", "-e" + str(binaries / "diagcraft"),
             "-e" + str(binaries / "diagcraft-cli")], cwd=work, text=True, encoding="utf-8")
        dependencies = set(generated.strip().removeprefix("shlibs:Depends=").split(", "))
        if not any(item.startswith("libc6 ") for item in dependencies):
            raise RuntimeError("dpkg-shlibdeps did not produce the expected runtime dependencies.")
        dependencies.update(("shared-mime-info", "hicolor-icon-theme", "libegl1", "libgl1", "libx11-6", "libxcursor1", "libxi6", "libxrandr2",
                             "libxkbcommon0", "libwayland-client0", "libwayland-cursor0", "libwayland-egl1"))
        payload = sorted(path for path in package.rglob("*") if path.is_file())
        for path in package.rglob("*"):
            if path.is_dir():
                path.chmod(0o755)
            elif path.parent != binaries:
                path.chmod(0o644)
        control = package / "DEBIAN"
        control.mkdir(mode=0o755)
        size = math.ceil(sum(path.stat().st_size for path in payload) / 1024)
        (control / "control").write_text(
            f"Package: diagcraft\nVersion: {version}\nArchitecture: amd64\n"
            "Maintainer: Kevin-FR <82709307+Kevin-FR@users.noreply.github.com>\n"
            "Section: graphics\nPriority: optional\n"
            f"Installed-Size: {size}\nDepends: {', '.join(sorted(dependencies))}\n"
            "Recommends: xdg-desktop-portal, xdg-desktop-portal-gtk | xdg-desktop-portal-kde\n"
            "Homepage: https://github.com/Kevin-FR/DiagCraft\n"
            "Description: Native diagram editor with an agent-friendly CLI\n"
            " Create multipage diagrams with shapes, connectors, styles and SVG/PDF exports.\n"
            " Works offline. Includes diagcraft-cli and its scoped MCP server.\n", encoding="utf-8")
        (control / "md5sums").write_text("".join(
            f"{hashlib.md5(path.read_bytes()).hexdigest()}  {path.relative_to(package).as_posix()}\n"
            for path in payload), encoding="utf-8")
        for path in control.iterdir():
            path.chmod(0o644)
        subprocess.run(["dpkg-deb", "--root-owner-group", "--build", str(package), str(output)], check=True)
    return output
