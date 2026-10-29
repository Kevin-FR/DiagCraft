#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
use diagcraft::document::Document;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--version") {
        println!("DiagCraft {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        println!(
            "DiagCraft\n  diagcraft [--language en|fr|de|es|it|pt|zh-Hans] [document.draw]\n  diagcraft --agent catalog|inspect FILE|validate FILE|apply FILE PATCH OUTPUT [--dry-run]\n  diagcraft --mcp DOSSIER\n  diagcraft [document.draw]\n  diagcraft --export document.draw diagramme.svg\n  diagcraft --import-mermaid INPUT OUTPUT [--dry-run] [--accept-losses]\n  diagcraft --export-mermaid INPUT OUTPUT [--page INDEX] [--dry-run] [--accept-losses]\n  diagcraft --import-image INPUT IMAGE OUTPUT [X Y]\n  diagcraft --export-png INPUT OUTPUT [--width N --height N --dpi N --page INDEX --selection ID,ID --transparent]\n  diagcraft --export-pdf document.draw diagramme.pdf [dessin|a4-paysage|a4-portrait|lettre]\n  diagcraft --images-demo OUTPUT.draw\n  diagcraft --pages-demo dossier.draw\n  diagcraft --symbols-demo symboles.draw\n  diagcraft --styles-demo styles.draw\n  diagcraft --demo exemple.draw\n  diagcraft --template support exemple.draw\n  diagcraft --charts-demo graphiques.draw\n  diagcraft --screenshot capture.png [accueil|inserer|conception|affichage|symboles|avance|graphiques|modeles|support|contexte-forme|contexte-connecteur|contexte-page|barre-contextuelle|barre-connecteur|compact|trackpad|proprietes|remplissage|contours|ombres|ombres-prereglages|selection-multiple|selection-contours|selection-ombres|selection-cadre|ruban-compact|alignement|reperes|redimensionnement|pages|tailles-pages|connecteurs-libres|lier-blocs|lier-apercu|connexions-rapides|bibliotheques|travaux|versions|nouveaux-symboles|commandes|raccourcis|interface|ruban-inserer|parametres|recuperation|ajuster-texte|contenu-bloc|editeur-texte|recherche-objets|volets|volets-fermes|placement-clavier|organisation|routage|images|export-png|mermaid-import|mermaid-export]\n"
        );
        return Ok(());
    }
    let language = if args.first().map(String::as_str) == Some("--language") {
        let code = args.get(1).ok_or("Missing --language code")?;
        let language = app::i18n::Language::parse(code)
            .ok_or("Unsupported language: en, fr, de, es, it, pt, zh-Hans")?;
        args.drain(..2);
        Some(language)
    } else {
        None
    };
    if diagcraft::cli::run(&args)? {
        return Ok(());
    }
    let screenshot = if args.first().map(String::as_str) == Some("--screenshot") {
        Some(PathBuf::from(args.get(1).ok_or("Missing screenshot path")?))
    } else {
        None
    };
    let capture_tab = if screenshot.is_some() {
        Some(args.get(2).cloned().unwrap_or("accueil".into()))
    } else {
        None
    };
    if let Some(ref name) = capture_tab
        && ![
            "accueil",
            "inserer",
            "conception",
            "affichage",
            "symboles",
            "avance",
            "graphiques",
            "modeles",
            "support",
            "contexte-forme",
            "contexte-connecteur",
            "contexte-page",
            "barre-contextuelle",
            "barre-connecteur",
            "agents",
            "compact",
            "trackpad",
            "remplissage",
            "contours",
            "ombres",
            "ombres-prereglages",
            "selection-multiple",
            "selection-contours",
            "selection-ombres",
            "selection-cadre",
            "ruban-compact",
            "alignement",
            "reperes",
            "redimensionnement",
            "proprietes",
            "lier-blocs",
            "lier-apercu",
            "pages",
            "tailles-pages",
            "connecteurs-libres",
            "connexions-rapides",
            "bibliotheques",
            "travaux",
            "versions",
            "nouveaux-symboles",
            "commandes",
            "raccourcis",
            "interface",
            "ruban-inserer",
            "parametres",
            "recuperation",
            "ajuster-texte",
            "contenu-bloc",
            "editeur-texte",
            "recherche-objets",
            "volets",
            "volets-fermes",
            "placement-clavier",
            "organisation",
            "routage",
            "images",
            "export-png",
            "mermaid-import",
            "mermaid-export",
        ]
        .contains(&name.as_str())
    {
        return Err("Onglet de capture inconnu.".into());
    }
    let path = if screenshot.is_none() {
        args.first().map(PathBuf::from)
    } else {
        None
    };
    let doc = match &path {
        Some(p) => Document::load(p)?,
        None => Document::demo(),
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(
                if capture_tab.as_deref().is_some_and(|mode| {
                    matches!(
                        mode,
                        "interface"
                            | "parametres"
                            | "ruban-inserer"
                            | "commandes"
                            | "raccourcis"
                            | "compact"
                            | "trackpad"
                            | "selection-multiple"
                            | "selection-cadre"
                            | "ruban-compact"
                            | "alignement"
                            | "reperes"
                            | "redimensionnement"
                            | "recuperation"
                            | "ajuster-texte"
                            | "contenu-bloc"
                            | "editeur-texte"
                            | "recherche-objets"
                            | "volets"
                            | "volets-fermes"
                            | "placement-clavier"
                            | "organisation"
                            | "routage"
                            | "images"
                            | "export-png"
                            | "mermaid-import"
                            | "mermaid-export"
                    )
                }) {
                    [1280., 800.]
                } else {
                    [1440., 960.]
                },
            )
            .with_min_inner_size([1200., 680.])
            .with_app_id("org.diagcraft.editor")
            .with_icon({
                let rgba = image::load_from_memory(include_bytes!(
                    "../../../assets/brand/diagcraft-logo.png"
                ))?
                .to_rgba8();
                eframe::egui::IconData {
                    width: rgba.width(),
                    height: rgba.height(),
                    rgba: rgba.into_raw(),
                }
            }),
        ..Default::default()
    };
    eframe::run_native(
        "DiagCraft",
        options,
        Box::new(move |cc| {
            let mut app = app::DrawApp::new(cc, doc, path, screenshot);
            if let Some(language) = language {
                app.set_language(Some(language));
            }
            if let Some(name) = capture_tab {
                app.capture_tab(&name);
            }
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}
