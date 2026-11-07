# Sprint — description de document dans les exports (SVG <desc> et PDF /Subject)

Préparation du lot « Accessibilité des exports — description de document ».
Contrat : [accessibilite.md](accessibilite.md). Ce plan découpe l’exécution en
étapes testables, puis liste la validation et le commit.

## Étapes

### 1. Modèle — champ `description` sur `Document`
- `crates/core/src/document.rs` : ajouter `pub description: String` à la struct
  `Document`, initialisé à `String::new()` dans `Default`.
- `crates/core/src/pages.rs` (sérialisation canonique v8) :
  - `impl Serialize` : ajouter `description: &'a str` au struct `File` avec
    `#[serde(skip_serializing_if = "String::is_empty")]` → les fichiers sans
    description restent écrits à l’identique.
  - `impl Deserialize` : ajouter `#[serde(default)] description: String` à
    `Raw` et le mapper. → les fichiers v8 existants (sans champ) se lisent ;
    aucun bump de `format_version`.
- `impl PartialEq for Document` (pages.rs) : inclure la description pour que
  deux documents diffèrent si leurs descriptions diffèrent.
- Corriger les constructions explicites `Document { .. }` qui ne se ferment pas
  par `..Default::default()` (le compilateur les liste) en ajoutant
  `description: String::new()`.

### 2. Validation moteur de la longueur
- Ajouter une règle dans `Document::validate()` (ou la validation dédiée) :
  `description.len() > 4096` → erreur de validation. Tests associés.

### 3. SVG — émettre la description dans `<desc>`
- `crates/core/src/svg.rs` (bloc <desc>, l. 45-76) : insérer
  `escape(doc.description.trim())` comme première ligne du `<desc>` quand elle
  n’est pas vide, puis les alternatives d’images comme aujourd’hui. Ne pas
  émettre de `<desc>` vide. Respecter l’échappement XML existant.

### 4. PDF — écrire la description en `/Subject`
- `crates/core/src/pdf.rs` (`document_info`, l. 147-149) : ajouter
  `.subject(TextStr(&doc.description))` uniquement si non vide. La géométrie
  reste inchangée.

### 5. Interface — éditer la description
- `apps/diagcraft/src/app.rs` (inspecteur, à côté du champ NOM DU DIAGRAMME,
  l. 1487-1488) : ajouter un champ « Description » multi-lignes ou champ simple
  lié à `&mut self.doc.description`, `char_limit(4096)`, dans la section
  document de l’inspecteur.
- i18n : ajouter la clé du libellé dans les 7 langues (`fr` = clé source).

### 6. Tests
- **Moteur** (`crates/core/src/svg.rs`, module `accessibility_tests`) :
  - `svg_desc_includes_document_description_escaped` : une description non vide
    apparaît échappée en tête du `<desc>`, suivie des alt d’images.
  - `empty_description_keeps_previous_image_only_desc` : pas de description →
    `<desc>` inchangé (alt d’images seuls) ; tout vide → pas de `<desc>`.
  - `description_does_not_change_pdf_geometry_and_sets_subject` : PDF avec
    description contient `/Subject`, sans changer la géométrie des pages.
  - Validation : description > 4096 refusée.
- **UI** (`apps/diagcraft/src/*_tests.rs`, modèle `csv_tests`/`mermaid_tests`) :
  une saisie de description à l’inspecteur modifie `doc.description` sans
  casser la révision/annulation (une édition, une annulation).

### 7. Validation complète
- `cargo fmt --all`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace`
- `cargo deny check licenses sources advisories`
- `python3 scripts/check-publication.py`
- `python3 scripts/check-locales.py`
- Vérifier le round-trip `.draw` : un fichier v8 avec/sans description se
  charge, se réécrit sans réordonner ni perdre de champ.

### 8. Documentation et commit
- Mettre à jour `ROADMAP.md` (point 4 : description contrôlable livrée ;
  balisage PDF /Tag et mode présentation marqués comme études séparées).
- Commit sur `main` (base `0eb8a14`), message descriptif en français, puis push.

## Critère de fin
Le champ description est saisissable à l’interface, émettant un `<desc>` SVG et
un `/Subject` PDF, sans changer aucun pixel ni la version de format, avec tests
moteur + UI verts, i18n vérifiée et la validation complète du repo.
