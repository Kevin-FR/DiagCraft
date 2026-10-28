fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--version") {
        println!("DiagCraft CLI {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.is_empty() || matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        println!(
            "DiagCraft CLI\n  diagcraft-cli --agent catalog|inspect FILE|validate FILE|apply FILE PATCH OUTPUT [--dry-run]\n  diagcraft-cli --mcp DOSSIER\n  diagcraft-cli --export FILE OUTPUT.svg\n  diagcraft-cli --import-image INPUT IMAGE OUTPUT [X Y]\n  diagcraft-cli --export-png INPUT OUTPUT [--width N --height N --dpi N --page INDEX --selection ID,ID --transparent]\n  diagcraft-cli --export-pdf FILE OUTPUT.pdf [dessin|a4-paysage|a4-portrait|lettre]\n  diagcraft-cli --demo OUTPUT.draw\n  diagcraft-cli --template ID OUTPUT.draw\n  diagcraft-cli --images-demo OUTPUT.draw\n  diagcraft-cli --pages-demo OUTPUT.draw\n  diagcraft-cli --version"
        );
        return Ok(());
    }
    if !diagcraft::cli::run(&args)? {
        return Err("Commande inconnue. Utilisez --help.".into());
    }
    Ok(())
}
