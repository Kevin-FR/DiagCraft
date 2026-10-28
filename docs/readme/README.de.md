# DiagCraft

[English](../../README.md) · [Français](README.fr.md) · **Deutsch** · [Español](README.es.md) · [Italiano](README.it.md) · [Português](README.pt.md) · [简体中文](README.zh-CN.md)

**Verbinde deine Ideen.** Ein nativer, lokaler Open-Source-Editor für Diagramme
mit einer Oberfläche in sieben Sprachen und einer API für KI-Agenten.

[![CI](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml/badge.svg)](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml)
[![MIT](https://img.shields.io/badge/Lizenz-MIT-blue.svg)](../../LICENSE)

![DiagCraft-Oberfläche auf Englisch](../images/interface-en.png)

## Herunterladen

Die Anwendungen und ihre SHA-256-Prüfsummen stehen auf der
[Releases-Seite](https://github.com/Kevin-FR/DiagCraft/releases).

| Plattform | Paket |
| --- | --- |
| macOS 13+, Apple Silicon | `.app` in einem ZIP-Archiv; separates CLI-Archiv |
| Windows x86_64 | Portables ZIP mit Anwendung und CLI |
| Linux x86_64 | `.deb`-Paket für amd64 oder `.tar.gz`-Archiv mit Anwendung und CLI |

**0.10.0 ist experimentell** und ergänzt eingebettete Bilder, verlustfreien Zuschnitt und konfigurierbaren PNG-Export. Das Mac-Paket ist ad hoc signiert und wurde von Apple nicht notarisiert.
Die Windows-Pakete sind nicht mit einem Herausgeberzertifikat signiert.
Siehe die [Installationsanleitung](../installation.md).

Anwendungen vor 0.10.0 verwerfen eingebettete Bilder beim Speichern. Der [Leitfaden zu Bildern und PNG](../images.md) beschreibt Formate, Tastenkürzel und Grenzen.

## Funktionen

- PNG/JPEG importieren, Bilder ablegen oder den eigenen Befehl zum Einfügen von Bildern verwenden; Seiten oder Auswahlen mit gewählter Größe, Auflösung und optionaler Transparenz exportieren.
- Rechtwinklige Verbindungen umgehen sichtbare Blöcke und Diagramme; manuelle Wegpunkte, gewählte Anschlüsse und freie Endpunkte bleiben erhalten.
- Dauerhafte Gruppen, Objekt- und Ebenensperren sowie seitenbezogene Ebenen mit gemeinsamer Sichtbarkeit in Zeichenfläche, SVG und PDF.
- Pfeile verschieben um 1 px, Umschalt um 10 px; ein gehaltener Pfeil erzeugt eine abbrechbare Verschiebung und hält die Auswahl zusammen.
- Ein Schalter je Seitenleiste, unter dem Menüband ausgerichtet und auch bei geschlossener Leiste erreichbar.
- Seitenobjekte nach Text, Typ oder ID suchen; Blöcke per Tastatur platzieren, bearbeiten und verbinden.
- Text direkt in Blöcken bearbeiten: Doppelklick oder F2/Enter, Wörter auswählen und die schwebende Formatierungsleiste verwenden. Ein Klick außerhalb übernimmt die Änderungen; Escape verwirft sie.
- Oberfläche auf Englisch, Französisch, Deutsch, Spanisch, Italienisch, Portugiesisch und vereinfachtem Chinesisch.
- Optionale GitHub-Updateprüfung, Downloads mit SHA-256-Prüfung und manuelle Installation.
- Formen, Texte, Diagramme, acht Symbolbibliotheken und neun Vorlagen.
- Verbinder mit Anbindung an Formen oder freien Endpunkten, Anschlusspunkten und bearbeitbaren Knicken.
- Mehrseitige Dokumente, Papierformate, eigene Seitengrößen und Anpassung an die Zeichnung.
- Füllungen, Farbverläufe, Muster, Konturen, Pfeilspitzen und Schatten.
- Mehrfachauswahl, Ausrichtung, Verteilung, Hilfslinien, Abbrechen von Gesten und Rückgängig-Verlauf.
- Trackpad-Navigation, kompakte Menüband-Tabs und verstellbare Seitenleisten mit gespeicherten Einstellungen.
- Befehlspalette mit ⌘K / Ctrl+K und durchsuchbare Tastaturhilfe mit F1.
- Bearbeitbare `.draw`-Dateien, lokale Projektgalerie mit Versionen und SVG/PDF-Export.
- Automatische Größenanpassung an den Text mit Schriftmessung und gemeinsamer Anordnung in Oberfläche, SVG und PDF.
- Automatische lokale Wiederherstellungskopien nach einem Absturz, ohne Originaldateien zu überschreiben.
- Lokale CLI und lokaler MCP-Server: Inspektion, Validierung und atomare JSON-Patches.

Die Organisation wird in optionalen v7-Metadaten gespeichert. Ältere Anwendungen zeigen ausgeblendete Objekte an und verlieren Gruppen, Ebenen und Sperren beim Speichern.

[Gruppen und Ebenen](../organisation.md) (FR).

Das Dokumentformat **v7** und die Agenten-API **v1** haben eigene Versionsnummern,
unabhängig von der Anwendung. Dateien der Formate v1–v6 werden beim Öffnen im
Arbeitsspeicher migriert. Die Quelldatei bleibt bis zum Speichern unverändert.

Zum Zeichnen sind weder Konto noch API-Schlüssel nötig. Der MCP-Server arbeitet
in einem gewählten lokalen Ordner; die Anwendung kontaktiert kein KI-Modell.

Die Sprache lässt sich unter **Datei → Sprache und Einstellungen…** auswählen.
Texte in Zeichnungen bleiben unverändert. Das Mac-Paket enthält nur `arm64`-Code
und verwendet macOS-Systembibliotheken: **Rosetta ist nicht erforderlich**.

Die ausführlichen Anleitungen sind derzeit auf Französisch:
[Sprachen und Updates](../langues-et-mises-a-jour.md), [Befehle](../commandes.md),
[Menüband und Seitenleisten](../interface.md), [Wiederherstellung](../recuperation.md)
[Textanpassung](../ajustement-texte.md) und [Versionierung](../versioning.md).

[Blockinhalt](../contenu-blocs.md) (FR).
[Objektsuche und Tastaturbedienung](../recherche-objets.md) (FR).

## Entwicklung

[Rust](https://rustup.rs/) und Python 3.11 oder neuer installieren, dann:

```sh
git clone https://github.com/Kevin-FR/DiagCraft.git
cd DiagCraft
cargo run --locked -p diagcraft
cargo run --locked -p diagcraft-cli -- --help
```

Die Systemabhängigkeiten zum Bauen unter Debian/Ubuntu stehen in der
[Entwicklungsanleitung](../development.md).

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo deny check licenses sources advisories
python3 scripts/check-publication.py
python3 scripts/check-locales.py
python3 -m unittest discover -s packaging -p 'test_*.py'
```

## Projektstruktur

| Verzeichnis | Aufgabe |
| --- | --- |
| `crates/core` | Dokumentmodell, Geometrie, Seiten, Verlauf, SVG/PDF, Galerie und Agenten-API |
| `apps/diagcraft` | Native egui/eframe-Anwendung |
| `apps/diagcraft-cli` | Konsolenprogramm ohne grafische Oberfläche |
| `assets` | Schriften, Markenressourcen und Lizenzhinweise mit Herkunft und Prüfsummen |
| `docs` | Architektur, Formate, Entwicklung, Sicherheit und Releases |
| `packaging` | Paketerstellung und Sammlung der Abhängigkeitslizenzen je Zielplattform |
| `examples` | Beispieldokumente und Testdateien zur Formatkompatibilität |

Oberfläche, CLI und MCP verwenden dieselbe Engine, die nicht von egui abhängt.
Siehe [Architektur](../architecture.md) und [Bibliotheksauswahl](../dependencies.md).

## Mitwirken

Geplante Arbeiten und Abnahmekriterien stehen in [ROADMAP.md](../../ROADMAP.md).
Gruppen, Drehung, Bildimport, PNG-Export, Hindernisse umgehende Verbinder und
integrierte Installationsprogramme sind noch geplant. SVG-Text hängt von den Schriften des Betrachters ab.

Bitte [CONTRIBUTING.md](../../CONTRIBUTING.md) lesen. Für die Arbeit an Zeichnungen
mit einem Agenten gilt der [Agentenvertrag](../AGENTS-DRAW.md). Sicherheitslücken
gemäß [SECURITY.md](../../SECURITY.md) melden. Diese Dokumente sind derzeit auf Französisch.

## Lizenz und Herkunft

Der Projektcode steht unter der [MIT-Lizenz](../../LICENSE). Abhängigkeiten und
Schriften behalten ihre eigenen Lizenzen; die Hinweise liegen den Paketen bei.
[THIRD_PARTY.md](../../THIRD_PARTY.md) nennt die Quellen und Urheber. Die
[Publikationsprüfung](../licensing.md) erklärt die Kontrollen und ihre Grenzen.

[Connector routing guide / Guide des connecteurs](../connecteurs.md).
