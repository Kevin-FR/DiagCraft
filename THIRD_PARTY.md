# Dépendances et attributions

DiagCraft est distribué sous licence MIT. Cette licence concerne les contributions
au projet ; elle ne remplace pas les licences des dépendances et des polices.

## Bibliothèques Rust

`Cargo.lock` fixe la résolution. `deny.toml` autorise explicitement les licences
acceptées et refuse les sources Git ou registres inconnus. Les choix alternatifs
d’une expression SPDX sont conservés : autoriser MIT n’impose pas une option
copyleft lorsqu’une dépendance est proposée sous plusieurs licences.

Principales dépendances : egui/eframe, serde/serde_json, rfd, image, tempfile,
sha2, semver, resvg/usvg, svg2pdf, pdf-writer, png, base64 et arboard. Leur rôle est décrit dans
[docs/dependencies.md](docs/dependencies.md).

`packaging/licenses.py` construit l’inventaire de la cible depuis Cargo, collecte
les textes de licence et notices, puis enregistre leurs SHA-256 dans
`Dependency-Licenses/manifest.json`. Un texte manquant bloque le paquet.
Les notices sont aussi réunies dans `THIRD-PARTY-NOTICES.txt`.

Les notices absentes d’une archive Cargo sont conservées sous
`assets/licenses/upstream/`, avec révision amont, URL et empreinte dans
`assets/licenses/provenance.json`. Pour `dispatch` 0.2.0, la déclaration MIT et
les auteurs figurent dans les métadonnées amont ; les conditions MIT sont
reproduites avec ces métadonnées, le dépôt amont n’ayant pas de fichier séparé.
Les notices de la bibliothèque standard Rust accompagnent les binaires.

## Polices

- **Source Sans 3**, Adobe : Regular, Bold, Italic, Bold Italic. Licence SIL Open
  Font License 1.1 conservée dans `assets/fonts/LICENSE.md`. Les fichiers sont
  inchangés ; sources et SHA-256 figurent dans `assets/fonts/provenance.json`.
- **Hack Regular**, Chris Simpkins et contributeurs, via les ressources
  d’egui : notices MIT, Bitstream Vera et domaine public dans
  `assets/fonts/Hack-LICENSE.txt`, provenance dans `hack-provenance.json`.
- **Noto Sans SC Regular**, projet Noto : chinois simplifié. Fichier OpenType
  inchangé, licence SIL Open Font License 1.1 dans `assets/fonts/NotoSansSC-LICENSE.txt`.
  Révision amont, source et SHA-256 dans `assets/fonts/provenance.json`. La police
  accompagne l’interface et les exports PDF, sans téléchargement à l’exécution.
- Les polices par défaut d’egui conservent leurs notices propres, dont OFL et
  Ubuntu Font Licence, dans l’inventaire de `epaint_default_fonts` des paquets.
- Les décorations Wayland Linux utilisent `sctk-adwaita` sous MIT et sa police
  Cantarell sous SIL Open Font License 1.1. Les textes sont conservés dans son
  inventaire de licences ; cette dépendance n’est pas embarquée sur Mac ou Windows.

## Marque, symboles et modèles

Le logo est une image générée pour le projet ; sa provenance et les empreintes
de ses variantes sont dans `assets/brand/provenance.json`. Les formes, icônes
vectorielles de l’interface et modèles sont définis dans les sources Rust du
projet. Aucun pack d’icônes tiers n’est embarqué. Les captures publiées montrent
l’application et ses documents de démonstration.

Les marques et noms des dépendances restent ceux de leurs titulaires. Consulter
les textes intégraux fournis avec la version effectivement téléchargée.

Depuis 0.10.0, les imports PNG/JPEG utilisent les codecs déjà verrouillés, sans
nouvelle version de dépendance. Les captures et l’exemple d’images montrent
uniquement des aplats générés dans `images::demo`, sous la licence du projet.

Le sous-ensemble Mermaid de 0.11.0 est interprété par du code original du projet,
sans SDK, code JavaScript ou ressource Mermaid incorporé. Les exemples et captures
de conversion sont des schémas synthétiques originaux sous MIT.
