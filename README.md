# DiagCraft

**English** · [Français](docs/readme/README.fr.md) · [Deutsch](docs/readme/README.de.md) · [Español](docs/readme/README.es.md) · [Italiano](docs/readme/README.it.md) · [Português](docs/readme/README.pt.md) · [简体中文](docs/readme/README.zh-CN.md)

**Connect your ideas.** A native, local, open-source diagram editor with an
interface in seven languages and an API for AI agents.

[![CI](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml/badge.svg)](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![DiagCraft interface in English](docs/images/interface-en.png)

## Download

Applications and their SHA-256 checksums are available on the
[releases page](https://github.com/Kevin-FR/DiagCraft/releases).

| Platform | Package |
| --- | --- |
| macOS 13+, Apple Silicon | `.app` in a ZIP archive; separate CLI archive |
| Windows x86_64 | Portable ZIP with the application and CLI |
| Linux x86_64 | `.deb` package for amd64 or `.tar.gz` archive, with the application and CLI |

**0.6.0 is experimental** and adds rich text formatting inside blocks.
The Mac package is ad hoc signed and has not been notarized by Apple. Windows
packages are not signed with a publisher certificate. See the
[installation guide](docs/installation.md).

## Features

- Rich text inside blocks: word selection, mixed styles, sizes and colors, with Apply/Cancel.
- Interface in English, French, German, Spanish, Italian, Portuguese and Simplified Chinese.
- Optional GitHub update checks, SHA-256-verified downloads and manual installation.
- Shapes, text, charts, eight symbol libraries and nine templates.
- Connectors attached to shapes or with free endpoints, connection ports and editable bends.
- Multipage documents, paper presets, custom page sizes and fit to drawing.
- Fills, gradients, patterns, outlines, arrowheads and shadows.
- Multiple selection, alignment, distribution, guides, gesture cancellation and undo history.
- Trackpad navigation, a compact tabbed ribbon and resizable panels with saved preferences.
- Command palette with ⌘K / Ctrl+K and searchable keyboard help with F1.
- Editable `.draw` files, a local project gallery with versions, and SVG/PDF exports.
- Automatic text fitting with font measurements and shared canvas/SVG/PDF layout.
- Automatic local recovery copies after a crash, without overwriting original files.
- Local CLI and MCP server: inspection, validation and atomic JSON patches.

The **v7** document format and **v1** agent API have their own version numbers,
independent of the application. Files in formats v1–v6 are migrated in memory
when opened. The source file stays unchanged until you save.

No account or API key is needed to draw. The MCP server works inside a local
folder you choose; the application does not contact an AI model.

Choose the interface language in **File → Language and settings…**. Text in
your drawings stays unchanged. The Mac package contains only `arm64` code and
uses macOS system libraries: **Rosetta is not required**.

Detailed guides are currently in French:
[languages and updates](docs/langues-et-mises-a-jour.md),
[commands](docs/commandes.md), [ribbon and panels](docs/interface.md),
[drawing recovery](docs/recuperation.md), [text fitting](docs/ajustement-texte.md)
and [versioning](docs/versioning.md).

[block content](docs/contenu-blocs.md) (FR).

## Development

Install [Rust](https://rustup.rs/) and Python 3.11 or newer, then run:

```sh
git clone https://github.com/Kevin-FR/DiagCraft.git
cd DiagCraft
cargo run --locked -p diagcraft
cargo run --locked -p diagcraft-cli -- --help
```

Debian/Ubuntu build dependencies are listed in the
[development guide](docs/development.md).

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo deny check licenses sources advisories
python3 scripts/check-publication.py
python3 scripts/check-locales.py
python3 -m unittest discover -s packaging -p 'test_*.py'
```

## Project structure

| Directory | Purpose |
| --- | --- |
| `crates/core` | Document model, geometry, pages, history, SVG/PDF, gallery and agent API |
| `apps/diagcraft` | Native egui/eframe application |
| `apps/diagcraft-cli` | Console executable without a graphical interface |
| `assets` | Fonts, branding and notices with provenance and checksums |
| `docs` | Architecture, formats, development, security and releases |
| `packaging` | Package creation and dependency license collection for each target |
| `examples` | Demo documents and format compatibility fixtures |

The interface, CLI and MCP share the same engine, which does not depend on egui.
See the [architecture](docs/architecture.md) and
[dependency choices](docs/dependencies.md).

## Contributing

Planned work and acceptance criteria are in [ROADMAP.md](ROADMAP.md).
Groups, rotation, image import, PNG export, obstacle-aware routing and integrated
installers are still planned.
SVG text rendering depends on the viewer's fonts.

Read [CONTRIBUTING.md](CONTRIBUTING.md). To work on a drawing with an agent,
see the [agent contract](docs/AGENTS-DRAW.md). Report vulnerabilities as described
in [SECURITY.md](SECURITY.md). These contributor documents are currently in French.

## License and provenance

Project code is released under the [MIT license](LICENSE). Dependencies and fonts
keep their own licenses; their notices are included in distribution packages.
[THIRD_PARTY.md](THIRD_PARTY.md) lists attributions, and the
[publication audit](docs/licensing.md) explains the checks and their limits.
