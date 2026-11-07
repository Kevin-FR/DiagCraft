//! Explicit preflight/consent for local CSV conversions. Sources are immutable.
use crate::{agent, csv, document::Document};
use serde_json::{Value, json};
use std::{io::Read, path::Path};
const MAX_SOURCE_BYTES: u64 = 256 * 1024;

fn failure(code: &str, message: impl ToString, hint: &str) -> Value {
    let message = message.to_string();
    json!({"error":message,"diagnostics":[{"code":code,"severity":"error","message":message,"hint":hint}]})
}
/// `args` includes the command. No write occurs on preflight, errors or missing consent.
pub(super) fn run(args: &[String]) -> Result<Value, Value> {
    let usage = "Use --import-csv INPUT OUTPUT --identity COL --label COL --connect LIST [--dry-run] [--accept-losses].";
    if args.len() < 3 {
        return Err(failure(
            "csv_arguments",
            "Missing CSV input or output.",
            usage,
        ));
    }
    let mut dry = false;
    let mut accept = false;
    let mut identity: Option<String> = None;
    let mut label: Option<String> = None;
    let mut connect: Vec<String> = Vec::new();
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
            "--identity" if identity.is_none() => {
                let value = args.get(i + 1).ok_or_else(|| {
                    failure(
                        "csv_arguments",
                        "Missing identity column.",
                        "Use --identity followed by a header name.",
                    )
                })?;
                identity = Some(value.clone());
                i += 2;
            }
            "--label" if label.is_none() => {
                let value = args.get(i + 1).ok_or_else(|| {
                    failure(
                        "csv_arguments",
                        "Missing label column.",
                        "Use --label followed by a header name.",
                    )
                })?;
                label = Some(value.clone());
                i += 2;
            }
            "--connect" if connect.is_empty() => {
                let value = args.get(i + 1).ok_or_else(|| {
                    failure(
                        "csv_arguments",
                        "Missing connect columns.",
                        "Use --connect followed by a comma-separated list of header names.",
                    )
                })?;
                connect = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                i += 2;
            }
            _ => {
                return Err(failure(
                    "csv_arguments",
                    "Unknown, repeated or misplaced CSV option.",
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
                "csv_input",
                e,
                "Choose a readable UTF-8 .csv file, at most 256 KiB.",
            )
        })?;
    let options = csv::CsvOptions {
        label,
        identity,
        connect,
    };
    let imported = csv::import(&text, &options).map_err(|diagnostics| {
        json!({"error":"CSV import rejected; no changes were applied.","diagnostics":diagnostics})
    })?;
    let diagnostics = imported.diagnostics;
    let requires_confirmation = diagnostics.iter().any(|d| d.severity == "warning");
    let mut report = json!({
        "format":"csv-import-v1",
        "direction":"import",
        "dry_run":dry,
        "requires_confirmation":requires_confirmation,
        "diagnostics":diagnostics,
        "output":Value::Null,
    });
    report["preview"] = agent::inspect(&imported.document)
        .map_err(|e| failure("csv_preview", e, "Correct the input CSV before retrying."))?;
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
            "csv_output",
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
