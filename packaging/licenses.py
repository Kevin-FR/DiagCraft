"""Collect the actual Cargo target graph's notices; fail if any text is missing."""
from pathlib import Path
import argparse
import hashlib
import json
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def collect(target, output):
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version", "1"], cwd=ROOT, text=True, encoding="utf-8"))
    tree = subprocess.check_output(
        ["cargo", "tree", "--locked", "--workspace", "--target", target,
         "--edges", "normal,build", "--prefix", "none", "--format", "{p}"],
        cwd=ROOT, text=True, encoding="utf-8")
    selected = set(re.findall(r"^([\w-]+) v([^\s]+)", tree, re.M))
    provenance = json.loads((ROOT / "assets/licenses/provenance.json").read_text(encoding="utf-8"))
    for entry in provenance:
        if "file" in entry and sha(ROOT / entry["file"]) != entry["sha256"]:
            raise RuntimeError(f"Upstream notice hash mismatch: {entry['file']}")
    output.mkdir(parents=True, exist_ok=False)
    manifest, combined = [], []
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        name, version = package["name"], package["version"]
        if (name, version) not in selected or package["id"] in metadata["workspace_members"]:
            continue
        if not package.get("license"):
            raise RuntimeError(f"Missing license declaration: {name} {version}")
        source = Path(package["manifest_path"]).parent
        cargo_source = source
        files = []
        for file in source.rglob("*"):
            if not file.is_file() or file.stat().st_size > 1024 * 1024:
                continue
            base = file.name.lower()
            if any(s in base for s in ("license", "licence", "copyright", "notice")) or base in (
                    "copying", "copying.txt", "copying.md", "ofl.txt", "ufl.txt") or (
                    name == "epaint_default_fonts" and file.suffix == ".txt"):
                files.append(file)
        if not files:
            source = ROOT / "assets/licenses/upstream" / f"{name}-{version}"
            files = [p for p in source.glob("*") if p.is_file()]
        if not files:
            raise RuntimeError(f"Missing license text: {name} {version}")
        records = []

        def add_notice(file, relative):
            text = file.read_text(encoding="utf-8")
            destination = output / f"{name}-{version}" / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(file, destination)
            records.append({"file": destination.relative_to(output).as_posix(), "sha256": sha(destination)})
            combined.append(f"\n{'=' * 72}\n{name} {version} — {relative}\n"
                            f"Declared license: {package['license']}\n"
                            f"Repository: {package.get('repository') or 'See Cargo metadata'}\n\n{text}\n")

        for file in sorted(files):
            add_notice(file, file.relative_to(source))
        # A crate may include its code license but omit the license of an
        # embedded font. Supplement, rather than replace, its Cargo notices.
        for entry in provenance:
            if entry.get("supplemental") and entry["crate"] == f"{name} {version}":
                if sha(cargo_source / entry["source_asset"]) != entry["source_asset_sha256"]:
                    raise RuntimeError(f"Supplemental notice asset hash mismatch: {name} {version}")
                notice = ROOT / entry["file"]
                add_notice(notice, notice.name)
        # OpenGL registries carry their own copyright/licensing notices inside
        # XML comments. Retain these in addition to the enclosing crate license.
        if name == "khronos_api":
            for registry in sorted(cargo_source.rglob("*.xml")):
                comments = re.findall(r"<comment>([\s\S]*?)</comment>", registry.read_text(encoding="utf-8"))
                notices = [text for text in comments if "copyright" in text.lower() and "licens" in text.lower()]
                if not notices:
                    continue
                relative = registry.relative_to(cargo_source).as_posix().replace("/", "__") + ".NOTICE.txt"
                destination = output / f"{name}-{version}" / relative
                text = "\n\n".join(notices)
                destination.write_text(text, encoding="utf-8")
                records.append({"file": destination.relative_to(output).as_posix(), "sha256": sha(destination),
                                "source_asset": registry.relative_to(cargo_source).as_posix()})
                combined.append(f"\n{'=' * 72}\n{name} {version} — {relative}\n\n{text}\n")
        manifest.append({"crate": name, "version": version, "license": package["license"],
                         "repository": package.get("repository"), "notices": records})
    shutil.copy2(ROOT / "assets/licenses/provenance.json", output / "upstream-provenance.json")
    report = {"target": target, "cargo_lock_sha256": sha(ROOT / "Cargo.lock"), "packages": manifest}
    (output / "manifest.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    (output / "THIRD-PARTY-NOTICES.txt").write_text(
        "DiagCraft — Cargo dependency notices\n" + "".join(combined), encoding="utf-8")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    packages = collect(args.target, args.output)
    print(f"Notices verified: {len(packages)} packages ({args.target})")
