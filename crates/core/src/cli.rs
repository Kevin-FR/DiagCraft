//! Shared headless commands used by desktop and console entry points.
use crate::{document::Document, svg};
#[path = "mermaid_cli.rs"]
mod mermaid_commands;

pub fn run(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    if args.first().map(String::as_str) == Some("--agent") {
        match crate::agent_io::cli_detailed(&args[1..]) {
            Ok(value) => println!("{}", serde_json::to_string_pretty(&value)?),
            Err(error) => {
                eprintln!("{}", error.to_value());
                std::process::exit(1);
            }
        }
        return Ok(true);
    }
    if matches!(
        args.first().map(String::as_str),
        Some("--import-mermaid" | "--export-mermaid")
    ) {
        match mermaid_commands::run(args) {
            Ok(report) => println!("{}", serde_json::to_string_pretty(&report)?),
            Err(report) => {
                eprintln!("{report}");
                std::process::exit(1);
            }
        }
        return Ok(true);
    }
    if args.first().map(String::as_str) == Some("--mcp") && args.len() == 2 {
        crate::agent_io::serve(
            std::path::Path::new(&args[1]),
            std::io::stdin().lock(),
            std::io::stdout().lock(),
        )?;
        return Ok(true);
    }
    if args.first().map(String::as_str) == Some("--screenshot") && !(2..=3).contains(&args.len()) {
        return Err("Arguments invalides. Utilisez --help.".into());
    }
    if matches!(
        args.first().map(String::as_str),
        Some("--help" | "-h" | "--screenshot")
    ) {
        return Ok(false);
    }
    match args.first().map(String::as_str) {
        Some("--export") if args.len() == 3 => {
            let doc = Document::load(std::path::Path::new(&args[1]))?;
            crate::document::atomic_write(
                std::path::Path::new(&args[2]),
                svg::export(&doc)?.as_bytes(),
            )?;
            println!("SVG exporté : {}", args[2]);
            Ok(true)
        }
        Some("--import-image") if args.len() == 4 || args.len() == 6 => {
            let mut doc = Document::load(std::path::Path::new(&args[1]))?;
            let position = if args.len() == 6 {
                [args[4].parse()?, args[5].parse()?]
            } else {
                [40., 40.]
            };
            let asset = crate::images::ImageAsset::from_path(std::path::Path::new(&args[2]))?;
            doc.add_image(asset, position)?;
            crate::agent::save_new(&doc, std::path::Path::new(&args[3]))?;
            Ok(true)
        }
        Some("--export-png") if args.len() >= 3 => {
            let doc = Document::load(std::path::Path::new(&args[1]))?;
            let mut options = crate::raster::PngOptions {
                width: doc.page_size[0].round().clamp(1., u32::MAX as f32) as u32,
                height: doc.page_size[1].round().clamp(1., u32::MAX as f32) as u32,
                ..Default::default()
            };
            let mut i = 3;
            while i < args.len() {
                let key = args[i].as_str();
                if key == "--transparent" {
                    options.transparent = true;
                    i += 1;
                    continue;
                }
                let value = args.get(i + 1).ok_or("Valeur PNG manquante.")?;
                match key {
                    "--width" => options.width = value.parse()?,
                    "--height" => options.height = value.parse()?,
                    "--dpi" => options.dpi = value.parse()?,
                    "--page" => options.page = value.parse()?,
                    "--selection" => {
                        options.selection = value
                            .split(',')
                            .map(str::parse)
                            .collect::<Result<Vec<u64>, _>>()?
                    }
                    _ => return Err("Option PNG inconnue.".into()),
                }
                i += 2;
            }
            let page = doc
                .page_document(options.page)
                .ok_or("Page PNG introuvable.")?;
            let region = crate::raster::region(&page, &options.selection)?;
            let width = args.iter().any(|a| a == "--width");
            let height = args.iter().any(|a| a == "--height");
            if !width {
                options.width = (if height {
                    options.height as f32 * region[2] / region[3]
                } else {
                    region[2] * options.dpi / 96.
                })
                .round()
                .clamp(1., u32::MAX as f32) as u32;
            }
            if !height {
                options.height = (if width {
                    options.width as f32 * region[3] / region[2]
                } else {
                    region[3] * options.dpi / 96.
                })
                .round()
                .clamp(1., u32::MAX as f32) as u32;
            }
            crate::document::atomic_write(
                std::path::Path::new(&args[2]),
                &crate::raster::export(&doc, &options)?,
            )?;
            println!("PNG exporté : {}", args[2]);
            Ok(true)
        }
        Some("--export-pdf") if (3..=4).contains(&args.len()) => {
            let doc = Document::load(std::path::Path::new(&args[1]))?;
            let format =
                crate::pdf::PageFormat::parse(args.get(3).map(String::as_str).unwrap_or("dessin"))?;
            crate::document::atomic_write(
                std::path::Path::new(&args[2]),
                &crate::pdf::export(&doc, format)?,
            )?;
            println!("PDF exporté : {}", args[2]);
            Ok(true)
        }
        Some("--images-demo") if args.len() == 2 => {
            crate::images::demo().save(std::path::Path::new(&args[1]))?;
            Ok(true)
        }
        Some("--pages-demo") if args.len() == 2 => {
            crate::templates::multipage().save(std::path::Path::new(&args[1]))?;
            Ok(true)
        }
        Some("--symbols-demo") if args.len() == 2 => {
            crate::symbols::demo().save(std::path::Path::new(&args[1]))?;
            Ok(true)
        }
        Some("--styles-demo") if args.len() == 2 => {
            crate::appearance::demo().save(std::path::Path::new(&args[1]))?;
            Ok(true)
        }
        Some("--template") if args.len() == 3 => {
            let template = crate::templates::catalog()
                .into_iter()
                .find(|t| t.id == args[1])
                .ok_or("Modèle inconnu")?;
            template.document.save(std::path::Path::new(&args[2]))?;
            Ok(true)
        }
        Some("--charts-demo") if args.len() == 2 => {
            crate::templates::charts().save(std::path::Path::new(&args[1]))?;
            Ok(true)
        }
        Some("--demo") if args.len() == 2 => {
            Document::demo().save(std::path::Path::new(&args[1]))?;
            Ok(true)
        }
        Some(arg)
            if arg.starts_with('-')
                && !(arg == "--screenshot" && (2..=3).contains(&args.len())) =>
        {
            Err("Arguments invalides. Utilisez --help.".into())
        }
        _ => Ok(false),
    }
}
