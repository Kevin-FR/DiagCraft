# Sprint — Filtres & recherche complète (lot 6b)

Préparation du lot 6b. Contrat : [filtres-recherche.md](filtres-recherche.md).

## Objectif

Compléter la recherche d'objets (⌘F / Ctrl+F) avec un **filtre par étiquette**
et une **portée « toutes les pages »**, et indexer étiquettes/note dans la
recherche multipage de l'arborescence, pour une « recherche complète »
cohérente avec 6a.

## Fichiers concernés

- `apps/diagcraft/src/object_search_ui.rs` : filtre par étiquette + portée.
- `apps/diagcraft/src/outline_ui.rs` : indexer `search_text()` des annotations.
- `assets/locales/messages.json` : nouvelles clés (7 langues).
- `docs/filtres-recherche.md`, `docs/sprint-filtres-recherche.md` : contrat/plan.
- `ROADMAP.md` : marquer la partie filtres/recherche complète en « Fait : … ».

## Étapes

1. **Modèle de recherche** (`object_search_ui.rs`)
   - Ajouter `tag: Option<String>` et `scope: Scope { Active, All }` à `Search`.
   - Étendre `Search::matches()` : sous `scope == All`, les `Entry` portent un
     `page: usize` ; filtrer par étiquette active (normalisée, marquée) et par
     type/texte comme aujourd'hui.
   - `entry()` : mémoriser `page` et la liste `tags` pour le filtre.
2. **Indexation multipage**
   - `entries(doc)` : quand `scope == All`, itérer toutes les pages avec
     `doc.page_document(page)` et agréger les types, en gardant `page` dans l'`Entry`.
   - Reconstruire la liste des étiquettes distinctes pour le rang de filtres.
3. **Filtre par étiquette (UI)**
   - Rang de puces « Toutes » + une par étiquette distincte (même limite de 16
     affichées, ordre d'apparition). Selectable, re-clic pour désactiver.
   - Combiné au type et au texte par ET (déjà le cas dans `matches`).
4. **Portée « toutes les pages » (UI + navigation)**
   - Sélecteur `ComboBox` portée : Page active / Toutes les pages.
   - Lancer une recherche multipage bornée : réutiliser une fonction locale
     `collect_all(doc, ...)` (sans thread d'abord, bornée à 300) parcourant les
     pages ; assurer la réactivité par annulation/recalcul à la saisie comme
     l'existant.
   - Naviguer : à l'activation, si `entry.page != page_index`, `switch_page_committed`
     puis `choose_object` (logique existante) ; le retour précédent reste correct.
   - Message de dépassement 300 résultats.
5. **Arborescence multipage** (`outline_ui.rs`)
   - Dans `rows()`, inclure `annotations.search_text()` dans `search` pour
     chaque type d'objet.
6. **i18n**
   - Ajouter les clés : « Toutes », « Toutes les pages », « Page active »,
     « Seuls les 300 premiers résultats sont affichés. », « Aucun résultat dans
     les pages. », etc. en 7 langues (fr = clé source), `indent=2`, ordre fixe.
7. **Tests**
   - Filtre par étiquette combiné au type/texte.
   - Portée toutes les pages : résultat sur une autre page, ouverture + sélection,
     sélection additive, limite 300, dépassement.
   - `rows()` trouve un objet par étiquette/note depuis l'arborescence.
8. **Validation**
   - `cargo fmt --all`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
     `cargo test --locked --workspace`, `cargo deny check licenses sources advisories`,
     `python3 scripts/check-publication.py`, `python3 scripts/check-locales.py`.
   - Mettre à jour `ROADMAP.md`, committer puis pousser sur `main`.

## Risques et limites

- La recherche multipage du dialogue doit rester réactive ; on démarre simple
  (évaluation directe bornée à 300) et on n'ajoute un thread d'arrière-plan que
  si la mesure le justifie, comme l'arborescence existante.
- Ne pas ré-écrire `messages.json` avec un `indent` ou un ordre différent.
