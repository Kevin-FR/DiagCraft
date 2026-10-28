#!/usr/bin/env python3
"""Exercise the packaged/headless CLI, migrations, exports and MCP contract."""
from pathlib import Path
import argparse
import base64
import struct
import zlib
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
            run("--export-png",source,tmp / "out.png","--width","240","--height","160","--dpi","144","--transparent",cwd=tmp)
            assert (tmp / "out.png").read_bytes().startswith(b"\x89PNG\r\n\x1a\n")
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

        # Exercise organization through the actual executable and MCP transport,
        # including one-batch layer aliases and a failed protected modification.
        ids = [node["id"] for node in context["pages"][0]["nodes"][:2]]
        assert len(ids) == 2
        organized = {"api_version": 1, "expected_revision": context["revision"], "operations": [
            {"op": "update_node", "page": 0, "target": ids[0],
             "changes": {"text": "Organization smoke fixture"}},
            {"op": "add_layer", "page": 0, "alias": "details", "name": "Details"},
            {"op": "move_objects_to_layer", "page": 0, "targets": ids, "layer": "details"},
            {"op": "group_objects", "page": 0, "targets": ids},
            {"op": "set_objects_locked", "page": 0, "targets": [ids[0]], "locked": True},
            {"op": "update_layer", "page": 0, "layer": "details", "visible": False},
        ]}
        (tmp / "organization.json").write_text(json.dumps(organized))
        run("--agent", "apply", source, tmp / "organization.json", tmp / "organized.draw", cwd=tmp)
        run("--agent", "validate", tmp / "organized.draw", cwd=tmp)
        organized_context = json.loads(run("--agent", "inspect", tmp / "organized.draw", cwd=tmp))
        organization = organized_context["pages"][0]["organization"]
        assert len(organization["groups"]) == 1
        assert not organization["layers"][0]["visible"]
        assert all(organization["objects"][str(id)]["locked"] for id in ids)
        run("--export", tmp / "organized.draw", tmp / "hidden.svg", cwd=tmp)
        assert "Organization smoke fixture" not in (tmp / "hidden.svg").read_text(encoding="utf-8")
        organization_requests = json.loads(json.dumps(requests))
        organization_requests[-1]["params"]["arguments"].update(patch=organized, output="organized-mcp.draw")
        organized_mcp = subprocess.run([str(binary), "--mcp", str(tmp)], cwd=tmp, text=True, encoding="utf-8",
                                      input="".join(json.dumps(r) + "\n" for r in organization_requests),
                                      capture_output=True, check=True)
        assert not organized_mcp.stderr
        assert not json.loads(organized_mcp.stdout.splitlines()[-1])["result"]["isError"]
        assert (tmp / "organized-mcp.draw").read_bytes() == (tmp / "organized.draw").read_bytes()
        before = (tmp / "organized.draw").read_bytes()
        blocked = {"api_version": 1, "expected_revision": organized_context["revision"], "operations": [
            {"op": "set_title", "title": "Must not persist"},
            {"op": "update_node", "page": 0, "target": ids[0], "changes": {"text": "Blocked"}},
        ]}
        (tmp / "blocked.json").write_text(json.dumps(blocked))
        rejected = subprocess.run([str(binary), "--agent", "apply", str(tmp / "organized.draw"),
                                   str(tmp / "blocked.json"), str(tmp / "blocked.draw")],
                                  cwd=tmp, text=True, encoding="utf-8", capture_output=True)
        assert rejected.returncode != 0 and not (tmp / "blocked.draw").exists()
        assert (tmp / "organized.draw").read_bytes() == before
        # Detailed failures retain the human field while exposing actionable context.
        failure = json.loads(rejected.stderr)
        diagnostic = failure["diagnostic"]
        assert failure["error"] and diagnostic["severity"] == "error" and diagnostic["hint"]
        assert diagnostic["operation"] == 1 and diagnostic["operation_name"] == "update_node"
        assert diagnostic["page"] == 0 and diagnostic["target"] == ids[0]
        rejected_requests = json.loads(json.dumps(requests))
        rejected_requests[-1]["params"]["arguments"] = {
            "file": "organized.draw", "patch": blocked, "output": "blocked-mcp.draw", "dry_run": False}
        result = subprocess.run([str(binary), "--mcp", str(tmp)], cwd=tmp, text=True, encoding="utf-8",
                                input="".join(json.dumps(r) + "\n" for r in rejected_requests), capture_output=True, check=True)
        rejection = json.loads(result.stdout.splitlines()[-1])["result"]
        assert rejection["isError"] and not (tmp / "blocked-mcp.draw").exists()
        assert rejection["structuredContent"]["diagnostic"] == diagnostic
        # Actual CLI/MCP routing contract, including immutable manual anchors.
        empty=json.loads((tmp / "cli.draw").read_text(encoding="utf-8"))
        for key in ("nodes", "connectors", "drawings", "charts", "pages"):
            empty[key]=[]
        empty["organization"]={}
        empty["page_size"]=[1280,800]
        (tmp / "routing-empty.draw").write_text(json.dumps(empty),encoding="utf-8")
        routing_patch={"api_version":1,"expected_revision":json.loads(run("--agent","inspect",tmp / "routing-empty.draw",cwd=tmp))["revision"],"operations":[
            {"op":"add_node","page":0,"alias":"a","kind":"rectangle","position":[80,260],"size":[140,80],"text":"Start"},
            {"op":"add_node","page":0,"alias":"b","kind":"rectangle","position":[1000,260],"size":[140,80],"text":"End"},
            {"op":"add_node","page":0,"alias":"wall","kind":"rectangle","position":[450,200],"size":[150,200],"text":"Obstacle"},
            {"op":"add_connector","page":0,"alias":"auto","start":{"node":"a","port":"right"},"end":{"node":"b","port":"left"},"style":{"kind":"elbow"}},
            {"op":"add_connector","page":0,"alias":"manual","start":{"node":"a","port":"bottom"},"end":{"point":[1100,650]},"style":{"kind":"elbow","waypoints":[[340,500],[800,500]],"label":"Manual"}},
        ]}
        (tmp / "routing-patch.json").write_text(json.dumps(routing_patch))
        run("--agent","apply",tmp / "routing-empty.draw",tmp / "routing-patch.json",tmp / "routing.draw",cwd=tmp)
        routing_requests=json.loads(json.dumps(requests))
        routing_requests[-1]["params"]["arguments"]={"file":"routing-empty.draw","patch":routing_patch,"output":"routing-mcp.draw","dry_run":False}
        routed_mcp=subprocess.run([str(binary),"--mcp",str(tmp)],cwd=tmp,text=True,encoding="utf-8",input="".join(json.dumps(r)+"\n" for r in routing_requests),capture_output=True,check=True)
        assert not routed_mcp.stderr and not json.loads(routed_mcp.stdout.splitlines()[-1])["result"]["isError"]
        assert (tmp / "routing.draw").read_bytes()==(tmp / "routing-mcp.draw").read_bytes()
        routed=json.loads(run("--agent","inspect",tmp / "routing.draw",cwd=tmp))["pages"][0]["connectors"]
        assert all(e["routing_status"]=="clear" for e in routed)
        assert any(p[1]<=200 or p[1]>=400 for p in routed[0]["route"])
        assert routed[1]["waypoints"]==[[340,500],[800,500]]
        assert all(p in routed[1]["route"] for p in routed[1]["waypoints"])
        assert routed[1]["route"][-1]==[1100,650]
        run("--export",tmp / "routing.draw",tmp / "routing.svg",cwd=tmp)
        run("--export-pdf",tmp / "routing.draw",tmp / "routing.pdf",cwd=tmp)
        assert (tmp / "routing.pdf").read_bytes().startswith(b"%PDF-")
        # Generate a tiny original PNG without depending on extra Python packages.
        def chunk(kind,data):
            return struct.pack(">I",len(data))+kind+data+struct.pack(">I",zlib.crc32(kind+data)&0xffffffff)
        pixels=b"\x00"+bytes([30,120,230,255])*4
        original_png=b"\x89PNG\r\n\x1a\n"+chunk(b"IHDR",struct.pack(">IIBBBBB",4,2,8,6,0,0,0))+chunk(b"IDAT",zlib.compress(pixels*2))+chunk(b"IEND",b"")
        (tmp / "original.png").write_bytes(original_png)
        run("--import-image",tmp / "routing-empty.draw",tmp / "original.png",tmp / "image-import.draw","50","70",cwd=tmp)
        assert (tmp / "original.png").read_bytes()==original_png
        image_patch={"api_version":1,"expected_revision":json.loads(run("--agent","inspect",tmp / "routing-empty.draw",cwd=tmp))["revision"],"operations":[
            {"op":"add_image","page":0,"alias":"picture","position":[50,70],"size":[160,80],"png_base64":base64.b64encode(original_png).decode(),"alt":"Original image"},
            {"op":"update_image","page":0,"target":"picture","crop":[0.25,0,0.5,1],"keep_aspect":True},
        ]}
        (tmp / "image-patch.json").write_text(json.dumps(image_patch))
        run("--agent","apply",tmp / "routing-empty.draw",tmp / "image-patch.json",tmp / "images.draw",cwd=tmp)
        image_requests=json.loads(json.dumps(requests));image_requests[-1]["params"]["arguments"]={"file":"routing-empty.draw","patch":image_patch,"output":"images-mcp.draw","dry_run":False}
        image_mcp=subprocess.run([str(binary),"--mcp",str(tmp)],cwd=tmp,text=True,encoding="utf-8",input="".join(json.dumps(r)+"\n" for r in image_requests),capture_output=True,check=True)
        assert not image_mcp.stderr and not json.loads(image_mcp.stdout.splitlines()[-1])["result"]["isError"]
        assert (tmp / "images.draw").read_bytes()==(tmp / "images-mcp.draw").read_bytes()
        image_context=json.loads(run("--agent","inspect",tmp / "images.draw",cwd=tmp));image_node=image_context["pages"][0]["nodes"][0]
        assert image_node["object_type"]=="image" and "png" not in image_node["image"]["asset"]
        assert image_node["image"]["crop"]==[0.25,0.0,0.5,1.0]
        run("--export-png",tmp / "images.draw",tmp / "selection.png","--width","640","--height","320","--dpi","144","--transparent","--selection",str(image_node["id"]),cwd=tmp)
        data=(tmp / "selection.png").read_bytes();assert struct.unpack(">II",data[16:24])==(640,320)
        assert b"pHYs" in data
        run("--export",tmp / "images.draw",tmp / "images.svg",cwd=tmp)
        run("--export-pdf",tmp / "images.draw",tmp / "images.pdf",cwd=tmp)
        assert "data:image/png;base64," in (tmp / "images.svg").read_text(encoding="utf-8")
        assert b"/Subtype /Image" in (tmp / "images.pdf").read_bytes()
        # Mermaid preflight must expose limitations before any output is created.
        mermaid_source = ROOT / "examples/mermaid-flowchart.mmd"
        original = mermaid_source.read_bytes()
        imported = tmp / "mermaid.draw"
        preview = json.loads(run("--import-mermaid", mermaid_source, imported, "--dry-run", cwd=tmp))
        assert preview["dry_run"] and preview["requires_confirmation"] and preview["diagnostics"]
        assert not imported.exists() and len(preview["preview"]["pages"][0]["nodes"]) == 6
        unconfirmed = subprocess.run([str(binary), "--import-mermaid", str(mermaid_source), str(imported)],
                                     cwd=tmp, text=True, encoding="utf-8", capture_output=True)
        assert unconfirmed.returncode != 0 and not imported.exists()
        assert json.loads(unconfirmed.stderr)["requires_confirmation"]
        run("--import-mermaid", mermaid_source, imported, "--accept-losses", cwd=tmp)
        original_draw = imported.read_bytes()
        run("--agent", "validate", imported, cwd=tmp)
        context = json.loads(run("--agent", "inspect", imported, cwd=tmp))
        assert len(context["pages"][0]["nodes"]) == 6 and len(context["pages"][0]["connectors"]) == 6
        exported = tmp / "mermaid.mmd"
        report = json.loads(run("--export-mermaid", imported, exported, "--dry-run", cwd=tmp))
        assert report["source"].startswith("flowchart ") and not exported.exists()
        run("--export-mermaid", imported, exported, "--accept-losses", cwd=tmp)
        run("--import-mermaid", exported, tmp / "mermaid-roundtrip.draw", "--accept-losses", cwd=tmp)
        roundtrip = json.loads(run("--agent", "inspect", tmp / "mermaid-roundtrip.draw", cwd=tmp))
        assert sorted(n["text"] for n in context["pages"][0]["nodes"]) == sorted(n["text"] for n in roundtrip["pages"][0]["nodes"])
        assert sorted(e["label"] for e in context["pages"][0]["connectors"]) == sorted(e["label"] for e in roundtrip["pages"][0]["connectors"])
        existing = exported.read_bytes()
        overwrite = subprocess.run([str(binary), "--export-mermaid", str(imported), str(exported), "--accept-losses"],
                                   cwd=tmp, text=True, encoding="utf-8", capture_output=True)
        assert overwrite.returncode != 0 and exported.read_bytes() == existing
        bad_source = tmp / "unsupported.mmd"
        bad_source.write_text("flowchart TB\nsubgraph secret\nA-->B\nend\n", encoding="utf-8")
        bad = subprocess.run([str(binary), "--import-mermaid", str(bad_source), str(tmp / "bad-mermaid.draw"), "--accept-losses"],
                             cwd=tmp, text=True, encoding="utf-8", capture_output=True)
        assert bad.returncode != 0 and not (tmp / "bad-mermaid.draw").exists()
        assert any(d["severity"] == "error" and d["line"] and d["hint"] for d in json.loads(bad.stderr)["diagnostics"])
        assert mermaid_source.read_bytes() == original and imported.read_bytes() == original_draw
        run("--export-png", imported, tmp / "mermaid.png", "--width", "400", cwd=tmp)
        run("--export-pdf", imported, tmp / "mermaid.pdf", cwd=tmp)
    print(f"CLI/MCP smoke passed: {count} format fixtures, SVG/PDF, dry run, identical patches, protected organization, obstacle routing, embedded images, PNG, Mermaid preflight and contextual diagnostics.")


if __name__ == "__main__":
    main()
