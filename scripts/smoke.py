#!/usr/bin/env python3
"""Exercise the packaged/headless CLI, migrations, exports and MCP contract."""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin", type=Path, required=True)
    args = parser.parse_args()
    binary = args.bin.resolve()
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]

    def run(*args, cwd):
        return subprocess.check_output([str(binary), *map(str, args)], cwd=cwd, text=True, encoding="utf-8")

    with tempfile.TemporaryDirectory(prefix="diagcraft-smoke-") as tmp:
        tmp = Path(tmp)
        assert version in run("--version", cwd=tmp)
        catalog = json.loads(run("--agent", "catalog", cwd=tmp))
        assert catalog
        count = 0
        for source in sorted((ROOT / "examples").glob("*.draw")):
            before = hashlib.sha256(source.read_bytes()).hexdigest()
            run("--agent", "validate", source, cwd=tmp)
            run("--export", source, tmp / "out.svg", cwd=tmp)
            run("--export-pdf", source, tmp / "out.pdf", cwd=tmp)
            assert (tmp / "out.svg").read_text(encoding="utf-8").endswith("</svg>\n")
            assert (tmp / "out.pdf").read_bytes().startswith(b"%PDF-")
            assert hashlib.sha256(source.read_bytes()).hexdigest() == before
            count += 1
        source = ROOT / "examples/dossier-v7.draw"
        (tmp / "input.draw").write_bytes(source.read_bytes())
        context = json.loads(run("--agent", "inspect", source, cwd=tmp))
        patch = {"api_version": 1, "expected_revision": context["revision"],
                 "operations": [{"op": "set_title", "title": "Smoke test"}]}
        (tmp / "patch.json").write_text(json.dumps(patch))
        dry = json.loads(run("--agent", "apply", source, tmp / "patch.json", tmp / "dry.draw", "--dry-run", cwd=tmp))
        assert dry["dry_run"] and not (tmp / "dry.draw").exists()
        run("--agent", "apply", source, tmp / "patch.json", tmp / "cli.draw", cwd=tmp)
        requests = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "release-smoke", "version": "1"}}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
            {"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "draw_apply", "arguments": {"file": "input.draw", "patch": patch, "output": "mcp.draw", "dry_run": False}}},
        ]
        result = subprocess.run([str(binary), "--mcp", str(tmp)], cwd=tmp, text=True, encoding="utf-8",
                                input="".join(json.dumps(r) + "\n" for r in requests), capture_output=True, check=True)
        replies = [json.loads(line) for line in result.stdout.splitlines()]
        assert not result.stderr and len(replies) == 3
        assert replies[0]["result"]["serverInfo"] == {"name": "diagcraft", "version": version}
        assert {x["name"] for x in replies[1]["result"]["tools"]} == {
            "draw_catalog", "draw_inspect", "draw_validate", "draw_apply"}
        assert not replies[2]["result"]["isError"]
        assert (tmp / "mcp.draw").read_bytes() == (tmp / "cli.draw").read_bytes()
    print(f"CLI/MCP smoke passed: {count} format fixtures, SVG/PDF, dry run and identical CLI/MCP patches.")


if __name__ == "__main__":
    main()
