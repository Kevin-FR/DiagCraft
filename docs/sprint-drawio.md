# Sprint — Import draw.io (sous-ensemble `.drawio`)

Préparation du sprint (version à fixer à la publication, comme pour les autres
lots). L’objectif est défini dans ROADMAP.md, priorité « Interopérabilité
progressive ». Le contrat du sous-ensemble est dans [docs/drawio.md](drawio.md),
écrit avant implémentation, sur le modèle de [docs/csv.md](csv.md).

## Objectif

Importer un fichier `.drawio` local en un diagramme DiagCraft : choix de la page
(multipages), aperçu avec diagnostics, lecture bornée d’un sous-ensemble
documenté, et création du résultat dans une **nouvelle page en une seule
annulation**. Aucun accès réseau implicite, aucune exécution du contenu, aucune
ressource externe chargée.

## Portée (livrables du sprint)

1. **Moteur de conversion** `crates/core/src/drawio.rs`
   - Parser **XML manuel borné** (pas de dépendance XML ajoutée, précédent CSV) :
     balises/attributs à guillemets simples et doubles, auto-fermantes,
     entités `&amp; &lt; &gt; &quot; &apos; &#N; &#xN;`, commentaires, CDATA,
     structure imbriquée. Aucun accès réseau.
   - Parse `<mxfile>/<diagram>/<mxGraphModel>` ou `<mxGraphModel>` nu ; lecture
     d’une **page choisie** ; sommet `vertex="1"` → `Node`, câble `edge="1"`
     → `Connector` (résolution des `id`, extrémités libres), couches.
   - Budgets publics : 512 Kio fichier, 2000 cellules, 4096 car/libellé.
   - Rapport `Notice` (code, sévérité, ligne, colonne, page, id, message, piste)
     et type retour `Import { document, diagnostics }` (modèle `csv.rs`).
   - Mapping de forme (`stylesheet` de draw.io → `ShapeKind`), style extrait
     (couleurs, pointes, motifs), `image=` externe/data jamais chargées
     (avertissement), libellés `html=1` convertis en texte.
   - Fonctions publiques :
     `pub fn pages(source: &str) -> Result<Vec<String>, Vec<Notice>>` et
     `pub fn import(source: &str, page: usize) -> Result<Import, Vec<Notice>>`.

2. **Preflight headless** `crates/core/src/drawio_cli.rs` (inclus dans `cli.rs`)
   - Flags `--import-drawio INPUT OUTPUT [--page INDEX] [--dry-run]
     [--accept-losses]`.
   - Lecture bornée, JSON `{format:"drawio-import-v1", direction, dry_run,
     requires_confirmation, diagnostics, preview, output}` ; écriture atomique
     sans écrasement (motif `write_new` de `mermaid_cli.rs:14`).
   - Enregistrement dans le help (`apps/diagcraft-cli/src/main.rs`) et le
     dispatch `cli.rs`.

3. **Interface egui** (Nouvelle page, 1 annulation, révision vérifiée)
   - Panneau `apps/diagcraft/src/drawio_ui.rs` : choix du fichier, sélecteur de
     page (via `pages()`), aperçu, liste des diagnostics.
   - Motif `append_page` de `mermaid_ui.rs:74` (atomique) et
     `apply_*_preview` (révision + `history.record` → une seule annulation).
   - Commande `ImportDrawio` dans `commands_ui.rs` (COMMANDS + gate) et entrée
     de menu Insérer.

4. **i18n** : libellés des nouvelles commandes et du panneau, dans les 7 langues,
   `fr` = clé source (test `i18n.rs`).

5. **Tests** : `crates/core/tests/drawio_features.rs` (parseur XML, budgets,
   sommets/câbles, formes inconnues, images exclues, pages, atomicité) + test UI
   (aperçu → appliquer → une annulation le retire ; document intact entre
   aperçu et validation).

6. **Docs** : `docs/drawio.md` (rédigé ci-dessus), référence dans ROADMAP.

## Hors périmètre (étudiés séparément, à documenter dans le guide)

- Export `.drawio`, mise à jour par réimport, lecture de formats propriétaires
  (`.vsdx`, Visio, autres formats non `.drawio`).
- Chargement d’images ou ressources réseau quelconques.
- Rendu HTML des libellés, styles avancés (dégradés, ombres, ligatures).

## Critères de réussite (acceptance)

- [ ] Un fichier `<mxGraphModel>` ou `<mxfile>/<diagram>` : sommet → forme,
      câble → connecteur, résolution des `id`.
- [ ] Le parseur XML gère attributs simples/doubles, auto-fermeture, entités,
      commentaires, CDATA ; un XML malformé est une erreur (aucune insertion).
- [ ] Diagnostics : forme inconnue, image externe, câble sans cible, page
      supplémentaire, limite dépassée — chacun signale cellule/page et piste.
      Aucun élément inconnu silencieusement ignoré.
- [ ] Un dépassement de budget ou une erreur annule tout, sans insertion partielle.
- [ ] L’import crée une **nouvelle page** en une annulation ; un Annuler la retire.
- [ ] Document modifié entre aperçu et validation ⇒ nouvel aperçu exigé.
- [ ] CLI : dry-run n’écrit rien ; sans `--accept-losses` un avertissement refuse
      d’écrire ; chemin de sortie nouveau exigé ; jamais d’écrasement ; `--page`.
- [ ] Commandes localisées dans les 7 langues ; `fr` = clé source.
- [ ] Validation complète : fmt, clippy -D warnings, tests workspace, deny,
      `check-publication.py`, `check-locales.py`.

## Accroches au code (gabarits vérifiés)

| Tâche | Gabarit | Fichier:ligne |
| --- | --- | --- |
| Moteur de conversion + Notices | `csv.rs` | `crates/core/src/csv.rs:15` (Notice), `:39` (Import), `:79` (import) |
| Preflight + écriture atomique | `csv_cli.rs` / `mermaid_cli.rs` | `crates/core/src/csv_cli.rs:11` (run), `:126` (write_new) |
| Dispatch CLI + help | `cli.rs` / `main.rs` | `crates/core/src/cli.rs:32` ; `apps/diagcraft-cli/src/main.rs:9` |
| Page ajoutée en 1 étape annulable | `mermaid_ui.rs` | `apps/diagcraft/src/mermaid_ui.rs:74` (append_page), `:196` (apply) |
| Atomicité / undo | `history.rs` | `crates/core/src/history.rs:20` (record borné 100) |
| Budgets | `document.rs` | `crates/core/src/document.rs:7` (16 Mio), `:1270` (10 000 objets) |
| Modèle forme/objet | `document.rs` | `crates/core/src/document.rs:137` (ShapeKind), `:306` (Node), `:455` (Connector) |

## Étapes suggérées

1. Valider ce périmètre et le contrat `docs/drawio.md`.
2. Écrire le moteur (parseur XML borné, conversion) + tests moteur.
3. Câbler la CLI (`drawio_cli.rs`, help, dispatch) + tests d’intégration CLI.
4. Câbler l’interface (`drawio_ui.rs`, commande `ImportDrawio`) + test UI.
5. i18n 7 langues.
6. Mettre à jour ROADMAP, puis validation complète.
