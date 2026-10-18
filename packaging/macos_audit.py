"""Reject Intel code and dependencies outside the bundle or macOS system."""
from pathlib import Path
import json
import subprocess
import sys

MAGIC = {b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf", b"\xce\xfa\xed\xfe",
         b"\xfe\xed\xfa\xce", b"\xca\xfe\xba\xbe", b"\xbe\xba\xfe\xca",
         b"\xca\xfe\xba\xbf", b"\xbf\xba\xfe\xca"}


def output(*args):
    return subprocess.check_output([str(x) for x in args], text=True, encoding="utf-8")


def inspect(root):
    root = Path(root).resolve()
    executable_dir = root / "Contents/MacOS" if root.suffix == ".app" else root
    result = []
    for binary in sorted(root.rglob("*")):
        if binary.is_symlink():
            raise ValueError(f"Unexpected symlink in native package: {binary.name}")
        if not binary.is_file():
            continue
        with binary.open("rb") as stream:
            if stream.read(4) not in MAGIC:
                continue
        architectures = output("lipo", "-archs", binary).strip().split()
        if architectures != ["arm64"]:
            raise ValueError(f"Apple Silicon only: {binary.name}: {architectures}")
        commands = output("otool", "-l", binary).splitlines()
        rpaths = []
        for index, line in enumerate(commands):
            if line.strip() == "cmd LC_RPATH":
                path = commands[index + 2].strip().split(" (offset")[0]
                if not path.startswith("path "):
                    raise ValueError("Unreadable LC_RPATH")
                rpaths.append(path[5:])

        def expand(path):
            return Path(path.replace("@loader_path", str(binary.parent))
                        .replace("@executable_path", str(executable_dir))).resolve()

        dependencies = []
        for line in output("otool", "-L", binary).splitlines()[1:]:
            dependency = line.strip().split(" (compatibility version")[0]
            if dependency.startswith(("/System/Library/", "/usr/lib/")):
                dependencies.append(dependency)
                continue
            paths = ([expand(r) / dependency[7:] for r in rpaths]
                     if dependency.startswith("@rpath/") else [expand(dependency)])
            if not any(path.is_relative_to(root) and path.is_file() for path in paths):
                raise ValueError(f"External/unresolved dependency: {binary.name}: {dependency}")
            dependencies.append(dependency)
        result.append({"file": binary.relative_to(root).as_posix(),
                       "architectures": architectures, "dependencies": dependencies})
    if not result:
        raise ValueError("No Mach-O code found")
    return {"architecture": "arm64", "rosetta_required": False, "binaries": result}


if __name__ == "__main__":
    print(json.dumps(inspect(sys.argv[1]), indent=2))
