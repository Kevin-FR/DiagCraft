//! Shared headless commands used by desktop and console entry points.
use crate::{document::Document, svg};

pub fn run(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    if args.first().map(String::as_str) == Some("--agent") {
        match crate::agent_io::cli(&args[1..]) {
            Ok(value) => println!("{}", serde_json::to_string_pretty(&value)?),
            Err(error) => {
                eprintln!("{}", serde_json::json!({"error": error}));
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
