# DiagCraft

**Reliez vos idées.** Un éditeur de diagrammes natif, local et open source,
avec une interface en sept langues et une API pour les agents IA.

[![CI](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml/badge.svg)](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml)
[![MIT](https://img.shields.io/badge/licence-MIT-blue.svg)](LICENSE)

![Interface de DiagCraft](docs/images/interface.png)

## Télécharger

Les applications et leurs empreintes SHA-256 sont sur la page des
[releases](https://github.com/Kevin-FR/DiagCraft/releases).

| Plateforme | Paquet |
| --- | --- |
| macOS 13+, Apple Silicon | Application `.app` dans un ZIP ; CLI séparée |
| Windows x86_64 | ZIP portable avec application et CLI |
| Linux x86_64 | Paquet `.deb` amd64 ou archive `.tar.gz`, application et CLI |

La version **0.4.0** reste expérimentale et ajoute la récupération après une fermeture inattendue. Le paquet
Mac est signé ad hoc et n’est pas notarié par Apple. Les paquets Windows ne
sont pas signés avec un certificat éditeur. Voir [l’installation](docs/installation.md).

## Fonctions disponibles

- Interface anglais, français, allemand, espagnol, italien, portugais et chinois simplifié.
- Mises à jour GitHub facultatives, téléchargement vérifié SHA-256 et installation manuelle.
- Formes, textes, graphiques, huit bibliothèques de symboles et neuf modèles.
- Connecteurs liés aux blocs ou à extrémités libres, ports et coudes modifiables.
- Documents multipages, formats de papier, tailles personnalisées et ajustement au dessin.
- Remplissages, dégradés, motifs, contours, flèches et ombres.
- Sélection multiple, alignement, espacement, repères, annulation des gestes et historique.
- Navigation au trackpad, ruban compact par onglet et volets redimensionnables mémorisés.
- Palette de commandes avec ⌘K / Ctrl+K et aide clavier filtrable avec F1.
- Fichiers `.draw` éditables, galerie locale avec versions, exports SVG et PDF.
- Copies automatiques locales et récupération des dessins après un plantage, sans écraser les originaux.
- CLI et serveur MCP locaux : inspection, validation et patches JSON atomiques.

Le format de document **v7** et l’API agents **v1** ont leurs propres numéros,
indépendants de la version de l’application. Les fichiers des formats v1 à v6
sont migrés en mémoire à l’ouverture. La source reste intacte jusqu’à l’enregistrement.

Aucun compte ni clé API n’est nécessaire pour dessiner. Le serveur MCP travaille
dans un dossier local choisi ; l’application ne contacte pas de modèle IA.

La langue se choisit dans **Fichier → Langue et paramètres…**. Les textes des
dessins restent inchangés. Le paquet Mac contient uniquement du code `arm64` et
utilise les bibliothèques système macOS : **Rosetta n’est pas nécessaire**.
Voir [langues et mises à jour](docs/langues-et-mises-a-jour.md).

Voir [la recherche de commandes](docs/commandes.md), [le ruban et les volets](docs/interface.md)
et [les règles de version](docs/versioning.md).
Voir aussi [la récupération des dessins](docs/recuperation.md).

## Développer

Installer [Rust](https://rustup.rs/) et Python 3.11 ou ultérieur, puis :

```sh
git clone https://github.com/Kevin-FR/DiagCraft.git
cd DiagCraft
cargo run --locked -p diagcraft
cargo run --locked -p diagcraft-cli -- --help
```

Sur Debian/Ubuntu, les dépendances système de compilation sont indiquées dans
[le guide de développement](docs/development.md).

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo deny check licenses sources advisories
python3 scripts/check-publication.py
python3 scripts/check-locales.py
python3 -m unittest discover -s packaging -p 'test_*.py'
```

## Structure

| Dossier | Rôle |
| --- | --- |
| `crates/core` | Modèle, géométrie, pages, historique, SVG/PDF, galerie et API agents |
| `apps/diagcraft` | Application native egui/eframe |
| `apps/diagcraft-cli` | Exécutable console sans interface graphique |
| `assets` | Polices, marque et notices avec provenance et empreintes |
| `docs` | Architecture, formats, développement, sécurité et releases |
| `packaging` | Construction des paquets et collecte des licences par cible |
| `examples` | Documents de démonstration et de compatibilité du format |

Le moteur est partagé par l’interface, la CLI et MCP. Il ne dépend pas d’egui.
Voir [l’architecture](docs/architecture.md) et [les choix de bibliothèques](docs/dependencies.md).

## Contribuer

Les éléments à réaliser et leurs critères sont dans [ROADMAP.md](ROADMAP.md).
Les groupes, rotations, images, export PNG, routage autour des obstacles et
installateurs intégrés restent à développer. Le texte ne redimensionne pas
encore automatiquement les formes. Le rendu SVG dépend des polices du lecteur.

Lire [CONTRIBUTING.md](CONTRIBUTING.md). Pour piloter un dessin avec un agent,
consulter [le contrat agents](docs/AGENTS-DRAW.md). Les vulnérabilités se signalent
selon [SECURITY.md](SECURITY.md).

## Licence et provenance

Le code du projet est sous [licence MIT](LICENSE). Les dépendances et polices
gardent leurs licences propres ; les notices sont conservées dans les paquets.
[THIRD_PARTY.md](THIRD_PARTY.md) décrit les attributions, et
[l’audit de publication](docs/licensing.md) explique les contrôles et leurs limites.
