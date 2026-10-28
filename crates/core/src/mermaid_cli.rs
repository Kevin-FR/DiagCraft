//! Explicit preflight/consent for local Mermaid conversions. Sources are immutable.
use crate::{agent, document::Document, mermaid};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    path::Path,
};
const MAX_SOURCE_BYTES: u64 = 256 * 1024;

fn failure(code: &str, message: impl ToString, hint: &str) -> Value {
    let message = message.to_string();
    json!({"error":message,"diagnostics":[{"code":code,"severity":"error","message":message,"hint":hint}]})
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temporary.write_all(bytes).map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary
        .persist_noclobber(path)
        .map_err(|e| format!("A new output file is required: {e}"))?;
    Ok(())
}
/// `args` includes the command. No write occurs on preflight, errors or missing consent.
pub(super) fn run(args: &[String]) -> Result<Value, Value> {
    let usage = "Use --import-mermaid INPUT OUTPUT or --export-mermaid INPUT OUTPUT [--page INDEX], followed by --dry-run to inspect and --accept-losses to write.";
    if args.len() < 3 {
        return Err(failure(
            "mermaid_arguments",
            "Missing Mermaid input or output.",
            usage,
        ));
    }
    let importing = args[0] == "--import-mermaid";
    let (mut dry, mut accept, mut page) = (false, false, None);
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" if !dry => {
                dry = true;
                i += 1;
            }
            "--accept-losses" if !accept => {
                accept = true;
                i += 1;
            }
            "--page" if !importing && page.is_none() => {
                let value = args
                    .get(i + 1)
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or_else(|| {
                        failure(
                            "mermaid_arguments",
                            "Invalid page index.",
                            "Use --page followed by a zero-based page index.",
                        )
                    })?;
                page = Some(value);
                i += 2;
            }
            _ => {
                return Err(failure(
                    "mermaid_arguments",
                    "Unknown, repeated or misplaced Mermaid option.",
                    usage,
                ));
            }
        }
    }
    let path = Path::new(&args[1]);
    let (document, source, diagnostics) = if importing {
        let mut text = String::new();
        std::fs::File::open(path)
            .and_then(|f| f.take(MAX_SOURCE_BYTES + 1).read_to_string(&mut text))
            .map_err(|e| {
                failure(
                    "mermaid_input",
                    e,
                    "Choose a readable UTF-8 .mmd file, at most 256 KiB.",
                )
            })?;
        let imported=mermaid::import(&text).map_err(|diagnostics|json!({"error":"Mermaid import rejected; no changes were applied.","diagnostics":diagnostics}))?;
        (Some(imported.document), None, imported.diagnostics)
    } else {
        let doc = Document::load(path).map_err(|e| {
            failure(
                "mermaid_document",
                e,
                "Validate the source .draw document before exporting.",
            )
        })?;
        let exported = mermaid::export(&doc, page.unwrap_or(0)).map_err(|e| {
            failure(
                "mermaid_export",
                e,
                "Choose an existing page and consult the supported flowchart subset.",
            )
        })?;
        (None, Some(exported.source), exported.diagnostics)
    };
    let requires_confirmation = diagnostics.iter().any(|d| d.severity == "warning");
    let mut report = json!({"format":"mermaid-flowchart-v1","direction":if importing{"import"}else{"export"},"dry_run":dry,"requires_confirmation":requires_confirmation,"diagnostics":diagnostics,"output":Value::Null});
    if let Some(doc) = &document {
        report["preview"] = agent::inspect(doc).map_err(|e| {
            failure(
                "mermaid_preview",
                e,
                "Correct the input diagram before retrying.",
            )
        })?;
    }
    if let Some(source) = &source {
        report["source"] = json!(source);
        report["page"] = json!(page.unwrap_or(0));
    }
    if dry {
        return Ok(report);
    }
    if requires_confirmation && !accept {
        report["error"] = json!(
            "Confirmation required; review the diagnostics with --dry-run, then add --accept-losses to write a new output file."
        );
        return Err(report);
    }
    if let Some(doc) = document {
        agent::save_new(&doc,Path::new(&args[2])).map_err(|e|failure("mermaid_output",e,"Use a new output path in an existing writable folder; the input is never overwritten."))?;
    } else if let Some(source) = source {
        write_new(Path::new(&args[2]),source.as_bytes()).map_err(|e|failure("mermaid_output",e,"Use a new output path in an existing writable folder; the input is never overwritten."))?;
    }
    report["output"] = json!(args[2]);
    Ok(report)
}
