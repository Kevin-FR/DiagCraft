//! Explicit preflight/consent for local `.drawio` conversions. Sources are
//! immutable and never overwritten.
use crate::{agent, document::Document, drawio};
use serde_json::{Value, json};
use std::{io::Read, path::Path};
const MAX_SOURCE_BYTES: u64 = 512 * 1024;

fn failure(code: &str, message: impl ToString, hint: &str) -> Value {
    let message = message.to_string();
    json!({"error":message,"diagnostics":[{"code":code,"severity":"error","message":message,"hint":hint}]})
}
/// `args` includes the command. No write occurs on preflight, errors or missing consent.
pub(super) fn run(args: &[String]) -> Result<Value, Value> {
    let usage = "Use --import-drawio INPUT OUTPUT [--page INDEX] [--dry-run] [--accept-losses].";
    if args.len() < 3 {
        return Err(failure(
            "drawio_arguments",
            "Missing draw.io input or output.",
            usage,
        ));
    }
    let mut dry = false;
    let mut accept = false;
    let mut page = 0usize;
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
            "--page" => {
                let value = args.get(i + 1).ok_or_else(|| {
                    failure(
                        "drawio_arguments",
                        "Missing page index.",
                        "Use --page followed by a 0-based page index.",
                    )
                })?;
                page = value.parse::<usize>().map_err(|_| {
                    failure(
                        "drawio_arguments",
                        "The page index must be a number.",
                        "Use a 0-based page index.",
                    )
                })?;
                i += 2;
            }
            _ => {
                return Err(failure(
                    "drawio_arguments",
                    "Unknown, repeated or misplaced draw.io option.",
                    usage,
                ));
            }
        }
    }
    let mut text = String::new();
    std::fs::File::open(Path::new(&args[1]))
        .and_then(|f| f.take(MAX_SOURCE_BYTES + 1).read_to_string(&mut text))
        .map_err(|e| {
            failure(
                "drawio_input",
                e,
                "Choose a readable UTF-8 .drawio file, at most 512 KiB.",
            )
        })?;
    let imported = drawio::import(&text, page).map_err(|diagnostics| {
        json!({"error":"draw.io import rejected; no changes were applied.","diagnostics":diagnostics})
    })?;
    let diagnostics = imported.diagnostics;
    let requires_confirmation = diagnostics.iter().any(|d| d.severity == "warning");
    let mut report = json!({
        "format":"drawio-import-v1",
        "direction":"import",
        "dry_run":dry,
        "page":page,
        "requires_confirmation":requires_confirmation,
        "diagnostics":diagnostics,
        "output":Value::Null,
    });
    report["preview"] = agent::inspect(&imported.document).map_err(|e| {
        failure(
            "drawio_preview",
            e,
            "Correct the input .drawio before retrying.",
        )
    })?;
    if dry {
        report["document"] = document_to_value(&imported.document);
        return Ok(report);
    }
    if requires_confirmation && !accept {
        report["error"] = json!(
            "Confirmation required; review the diagnostics with --dry-run, then add --accept-losses to write a new output file."
        );
        return Err(report);
    }
    agent::save_new(&imported.document, Path::new(&args[2])).map_err(|e| {
        failure(
            "drawio_output",
            e,
            "Use a new output path in an existing writable folder; the input is never overwritten.",
        )
    })?;
    report["output"] = json!(args[2]);
    Ok(report)
}

fn document_to_value(doc: &Document) -> Value {
    json!({
        "title": doc.title,
        "nodes": doc.nodes.len(),
        "connectors": doc.connectors.len(),
    })
}
