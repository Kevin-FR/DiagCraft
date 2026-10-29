//! Reproducible, opt-in release-profile budgets; no third-party benchmark runtime.
#[path = "../qa/corpus.rs"]
mod corpus;
use diagcraft::{agent, document::Document, raster, svg};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{hint::black_box, path::PathBuf, time::Instant};

fn measure<T>(
    name: &str,
    budget_ms: f64,
    mut work: impl FnMut() -> Result<T, String>,
) -> Result<Value, String> {
    drop(black_box(work()?)); // one untimed warmup
    let mut samples = vec![];
    for _ in 0..3 {
        let start = Instant::now();
        let result = black_box(work()?);
        samples.push(start.elapsed().as_secs_f64() * 1000.);
        drop(result);
    }
    samples.sort_by(f64::total_cmp);
    Ok(
        json!({"operation":name,"samples_ms":samples,"median_ms":samples[1],"max_ms":samples[2],"budget_ms":budget_ms,"passed":samples[1]<=budget_ms}),
    )
}
fn dataset(name: &str, doc: &Document, metrics: Vec<Value>) -> Result<Value, String> {
    let bytes = serde_json::to_vec(doc).map_err(|e| e.to_string())?;
    Ok(
        json!({"dataset":name,"pages":doc.page_count(),"objects":doc.object_count(),"canonical_bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes)),"metrics":metrics}),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut output: Option<PathBuf> = None;
    let mut enforce = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--output" if output.is_none() => {
                output = Some(
                    args.get(i + 1)
                        .ok_or("--output requires a JSON file")?
                        .into(),
                );
                i += 2;
            }
            "--enforce" if !enforce => {
                enforce = true;
                i += 1;
            }
            _ => return Err("Use --output FILE [--enforce]".into()),
        }
    }
    if cfg!(debug_assertions) && enforce {
        return Err("Performance budgets require a --release build".into());
    }
    let temp = tempfile::tempdir()?;
    let mut datasets = vec![];
    let d = corpus::grid(40, 25);
    d.validate()?;
    let path = temp.path().join("grid.draw");
    d.save(&path)?;
    let metrics = vec![
        measure("canonical_json", 100., || {
            serde_json::to_vec(&d).map_err(|e| e.to_string())
        })?,
        measure("load_validate", 250., || Document::load(&path))?,
        measure("inspect_routes_and_text", 2000., || agent::inspect(&d))?,
        measure("svg_export", 1500., || svg::export(&d))?,
        measure("png_800x450", 6000., || {
            raster::export(
                &d,
                &raster::PngOptions {
                    width: 800,
                    height: 450,
                    ..Default::default()
                },
            )
        })?,
    ];
    datasets.push(dataset("grid_1000_nodes_975_links", &d, metrics)?);
    let d = corpus::multipage();
    let path = temp.path().join("multipage.draw");
    d.save(&path)?;
    let metrics = vec![
        measure("load_validate", 250., || Document::load(&path))?,
        measure("inspect_all_pages", 2000., || agent::inspect(&d))?,
    ];
    datasets.push(dataset("five_pages_1000_nodes_950_links", &d, metrics)?);
    let d = corpus::images();
    let path = temp.path().join("images.draw");
    d.save(&path)?;
    let metrics = vec![
        measure("load_validate_12mp", 1000., || Document::load(&path))?,
        measure("png_800x450", 2000., || {
            raster::export(
                &d,
                &raster::PngOptions {
                    width: 800,
                    height: 450,
                    ..Default::default()
                },
            )
        })?,
    ];
    datasets.push(dataset("three_images_12mp", &d, metrics)?);
    let passed = datasets.iter().all(|d| {
        d["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["passed"] == true)
    });
    let report = json!({"schema_version":1,"source_revision":option_env!("GITHUB_SHA"),"app_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"release_profile":!cfg!(debug_assertions),"warmups":1,"samples":3,"passed":passed,"datasets":datasets});
    for d in report["datasets"].as_array().unwrap() {
        for m in d["metrics"].as_array().unwrap() {
            eprintln!(
                "{}/{}: {:.2} ms (budget {:.0} ms) {}",
                d["dataset"].as_str().unwrap(),
                m["operation"].as_str().unwrap(),
                m["median_ms"].as_f64().unwrap(),
                m["budget_ms"].as_f64().unwrap(),
                if m["passed"] == true { "PASS" } else { "FAIL" }
            );
        }
    }
    let bytes = serde_json::to_vec_pretty(&report)?;
    if let Some(path) = output {
        diagcraft::document::atomic_write(&path, &bytes)?;
    } else {
        println!("{}", String::from_utf8(bytes)?);
    }
    if enforce && !passed {
        return Err("A performance budget was exceeded; inspect the JSON report".into());
    }
    Ok(())
}
