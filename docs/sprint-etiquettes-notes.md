# Sprint — Étiquettes et notes d’objets

Préparation du sprint (version à fixer à la publication, comme pour les autres
lots). L’objectif est défini dans ROADMAP.md, priorité « Retrouver et
organiser ». Le contrat du sous-ensemble est dans
[docs/etiquettes-notes.md](etiquettes-notes.md), écrit avant implémentation.

## Objectif

Ajouter aux objets du diagramme (formes/textes, connecteurs, traits/courbes,
graphiques) des **étiquettes** (mots-clés) et une **note** (texte long),
enregistrées dans le `.draw` v8 de façon rétrocompatible, éditables à
l’interface, indexées par la recherche d’objets et exportées en partie en
métadonnées SVG accessibles. Aucune modification du rendu visuel ni des
exports par défaut.

## Portée (livrables du sprint)

1. **Modèle** `crates/core/src/document.rs`
   - `Node`, `Connector`, `Drawing` : `pub tags: Vec<String>` + `pub note: String`
     avec attributs rétrocompatibles (`default` + `skip_serializing_if`).
   - `Chart` (dans `chart.rs`) : mêmes champs (vérifier sa structure).
   - Constructeurs par défaut des objets mis à jour (champs vides).
   - `Document::validate` : limites des étiquettes (1..=64 car, 16 max, sans
     doublon, sans saut de ligne) et note (≤ 4096), avec erreurs localisables.
   - Helper `crate::organization` / inspecteur : normalisation de l’édition
     (trim, dédoublonnage) via une fonction mise à disposition.

2. **Inspection / agents** (`agent_io.rs`, `agent.rs`, `cli.rs`)
   - Afficher `tags` et `note` dans l’inspection d’un objet.
   - Accepter `tags`/`note` dans les patches API (validation via
     `Document::validate`), rejet du lot entier si invalide.

3. **Recherche** `apps/diagcraft/src/object_search_ui.rs`
   - Incorporer `tags` et `note` dans `Entry.search` (normalisé) pour les quatre
     catégories d’objets.
   - Afficher une étiquette dans le détail d’un résultat quand présente.

4. **Interface egui** `apps/diagcraft/src/app.rs`
   - Section « ÉTIQUETTES » dans l’inspecteur pour un objet sélectionné :
     saisie d’un nouveau mot-clé + suppression des puces existantes. Une
     modification = une annulation.
   - Champ multiligne « NOTE » (`char_limit(4096)`).
   - Clés i18n en 7 langues (fr = source) dans `assets/locales/messages.json`.

5. **Exports SVG** `crates/core/src/svg.rs`
   - Ajouter les étiquettes concaténées à l’attribut `aria-label` de l’élément
     de l’objet quand présentes ; aucun changement du rendu visuel. PDF/PNG
     inchangés.

6. **Tests**
   - `crates/core/src/*`: round-trip rétrocompatible (fichier sans champs →
     identique), lecture d’un fichier avec champs, validation des limites,
     copie/collage conservateur, `aria-label` SVG.
   - UI : édition étiquette/note = une annulation, recherche par étiquette et
     par note aboutit.

## Étapes

1. Contrat (`docs/etiquettes-notes.md`) — fait.
2. Plan (`docs/sprint-etiquettes-notes.md`) — ce document.
3. Implémentation : modèle → validation → inspection/agents → recherche →
   interface → i18n → SVG → tests.
4. Validation complète (liste du contrat).
5. Mise à jour ROADMAP.md (priorité « Retrouver et organiser » : volet
   étiquettes/notes fait) et commit/push sur `main`.

## Hors périmètre

- Filtres/volet dédié, styles nommés, composants.
- Recherche multipage globale étendue (volet suivant « Filtres et recherche »).
- Exports réouvrables (priorité 5).
