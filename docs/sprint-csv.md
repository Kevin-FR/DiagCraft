# Sprint — Import CSV local

Préparation du sprint suivant (version à fixer à la publication, comme pour les
autres lots). L’objectif est défini dans ROADMAP.md, priorité « Données locales
vers diagrammes ». Le contrat du sous-ensemble est dans [docs/csv.md](csv.md),
écrit avant implémentation, sur le modèle de [docs/mermaid.md](mermaid.md).

## Objectif

Importer un CSV local en un diagramme DiagCraft : choix des colonnes de libellé,
d’identité et de relation, aperçu avec diagnostics, limites de taille, et création
du résultat dans une **nouvelle page en une seule annulation**. Aucun accès réseau
implicite, aucune exécution du contenu.

## Portée (livrables du sprint)

1. **Moteur de conversion** `crates/core/src/csv.rs`
   - Parse RFC 4180 borné (UTF-8, virgule/point-virgule), en-têtes, choix de
     colonnes, relations par identité.
   - Budgets publics : 256 Kio fichier, 1000 lignes, 64 colonnes, 4096 car/cellule.
   - Rapport `Notice` (code, sévérité, ligne, colonne, message, piste) réutilisant
     le modèle `mermaid.rs:15-25`, et type retour `Import { document, diagnostics }`
     sur le modèle `mermaid.rs:46-50`.
   - Placement déterministe borné en grille.
   - Fonction publique : `pub fn import(source: &str, options: &CsvOptions) -> Result<Import, Vec<Notice>>`.

2. **Preflight headless** `crates/core/src/csv_cli.rs` (inclus dans `cli.rs`)
   - Flags `--import-csv INPUT OUTPUT --identity COL --label COL --connect LIST
     [--dry-run] [--accept-losses]`.
   - Lecture bornée, JSON `{format, direction, dry_run, requires_confirmation,
     diagnostics, preview, output}` ; écriture atomique sans écrasement (motif
     `write_new` de `mermaid_cli.rs:14`).
   - Enregistrement dans le help (`apps/diagcraft-cli/src/main.rs:9`) et le
     dispatch `cli.rs`.

3. **Interface egui** (Nouvelle page, 1 annulation, révision vérifiée)
   - Panneau `apps/diagcraft/src/csv_ui.rs` : choix du fichier, colonnes de
     libellé/identité/relation, aperçu, liste des diagnostics.
   - Motif `append_page` de `mermaid_ui.rs:74` (atomique) et
     `apply_mermaid_preview` de `mermaid_ui.rs:196` (révision + `history.record`
     → une seule annulation).
   - Commande `ImportCsv` dans `commands_ui.rs` (COMMANDS + gate) et entrée de
     menu Insérer.

4. **i18n** : libellés des nouvelles commandes et du panneau, dans les 7 langues,
   `fr` = clé source (test `i18n.rs:197`).

5. **Tests** : `crates/core/tests/csv_features.rs` (parseur, budgets, relations,
   lignes ignorées, atomicité) + test UI dans `ui_tests.rs` (aperçu → appliquer →
   une annulation le retire ; document ouvert intact entre aperçu et validation).

6. **Docs** : `docs/csv.md` (rédigé), référence dans ROADMAP et README si pertinent.

## Hors périmètre (étudiés séparément, à documenter dans le guide)

- Mise à jour par clé stable (réimport modifierait des objets existants).
- Import SQL.
- Formats autres que CSV, délimiteur configurable avancé, données non-textuelles.

## Critères de réussite (acceptance)

- [ ] Un CSV à en-têtes : une ligne → une forme ; la relation référence bien
      l’identité cible, pas un numéro de ligne.
- [ ] Le placement est déterministe et borné, indépendant de l'ordre des lignes.
- [ ] Diagnostics : ligne en-tête en double, section vide, référence d’identité
      manquante, limite dépassée — chacun signale ligne/colonne et piste. Aucune
      entrée inconnue silencieusement ignorée.
- [ ] Un dépassement de budget ou une erreur annule tout, sans insertion partielle.
- [ ] L’import crée une **nouvelle page** en une annulation ; un Annuler le retire.
- [ ] Document modifié entre aperçu et validation ⇒ nouvel aperçu exigé.
- [ ] CLI : dry-run n’écrit rien ; sans `--accept-losses` un avertissement refuse
      d’écrire ; chemin de sortie nouveau exigé ; jamais d’écrasement.
- [ ] Commandes localisées dans les 7 langues ; `fr` = clé source.
- [ ] Validation complète : fmt, clippy -D warnings, tests workspace, deny,
      `check-publication.py`, `check-locales.py`.

## Accroches au code (gabarits vérifiés)

| Tâche | Gabarit | Fichier:ligne |
| --- | --- | --- |
| Moteur de conversion + Notices | `mermaid.rs` | `crates/core/src/mermaid.rs:15` (Notice), `:46` (Import), `:478` (import) |
| Preflight + écriture atomique | `mermaid_cli.rs` | `crates/core/src/mermaid_cli.rs:28` (run), `:14` (write_new) |
| Dispatch CLI + help | `cli.rs` / `main.rs` | `crates/core/src/cli.rs:6,17` ; `apps/diagcraft-cli/src/main.rs:9` |
| Page ajoutée en 1 étape annulable | `mermaid_ui.rs` | `apps/diagcraft/src/mermaid_ui.rs:74` (append_page), `:196` (apply) |
| Atomicité / undo | `history.rs` | `crates/core/src/history.rs:20` (record borné 100) |
| Budgets | `document.rs` | `crates/core/src/document.rs:7` (16 Mio), `:1270` (10 000 objets) |
| Catalogue agents | `agent.rs` | `crates/core/src/agent.rs:1209` (catalog mermaid) |
| Table limites doc | `docs/mermaid.md` / `images.md` | `docs/images.md:74` |

## Étapes suggérées

1. Finaliser/valider ce périmètre et le contrat `docs/csv.md`.
2. Écrire le moteur + tests moteur (parseur, budgets, relations, atomicité).
3. Câbler la CLI (`csv_cli.rs`, help, dispatch) + tests d’intégration CLI.
4. Câbler l’interface (`csv_ui.rs`, commande `ImportCsv`) + test UI.
5. i18n 7 langues + capture d’écran de démonstration locale.
6. Mettre à jour ROADMAP/README, puis validation complète.
