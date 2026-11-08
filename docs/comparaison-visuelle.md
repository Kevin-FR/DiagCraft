# Contrat — Comparaison visuelle des modifications IA

**Lot 114** de la feuille de route : à partir du premier aperçu IA avant/après
(item 113), aider à *voir* ce que l’agent modifie avant de l’appliquer.

## Objectif

Un agent produit un patch API 1. Le panneau « Travailler avec un agent »
(onglet **2 Examiner le résultat**) sait déjà afficher l’avant et l’après,
mais « les différences ne sont pas surlignées ». Ce lot ajoute une comparaison
visuelle des modifications :

- un troisième mode d’aperçu **Différences** qui rend l’après et surligne les
  objets ajoutés ou modifiés, avec une légende et des compteurs ;
- un contour d’accent sur les objets concernés dans les vues **Avant** et
  **Après**, pour que les différences soient visibles dès la bascule ;
- une légende expliquant Ajout / Modification / (objets retirés signalés).

## Périmètre

- La comparaison est **déterministe** : aucun diff d’image ; elle dérive
  exclusivement du rapport du patch (`report["changes"]`, liste de
  `{ operation, page, id, description }`).
- Une catégorie est associée à chaque changement en croisant l’avant et
  l’après sur la même page :
  - l’identifiant existe avant et après → **Modification** ;
  - l’identifiant n’existe qu’après → **Ajout** ;
  - l’identifiant n’existe qu’avant → **Suppression** (signalée, non dessinée
    dans l’après car il n’y figure plus).
- Les identifiants d’objets étant **par page**, la clé de comparaison est le
  couple `(page, id)`.
- Le surlignage est purement visuel (contour et teinte d’accent sur l’après) ;
  il ne modifie pas le document et disparaît à la sortie du panneau.
- Rien n’est enregistré dans le `.draw` v8 ni dans les préférences : il s’agit
  d’une vue temporaire de l’aperçu.

## Mises en garde / cas d’erreur

- Un changement sans `id` (ex. réorganisation) ne peut pas être surligné :
  il reste décrit dans le rapport en texte.
- Une page d’un objet supprimé n’est pas nécessairement la page affichée ;
  le mode **Différences** rend la page choisie de l’après et ne trace les
  suppressions que si leur position est connue (objet encore présent avant).
- Cas limite connu de la catégorisation `(page, id)` : si un patch supprime un
  objet puis en ajoute un nouveau qui réutilise le même identifiant libéré,
  la catégorie basée sur la présence avant/après peut confondre « Ajout » et
  « Modification ». Ce cas rare n’est pas désambiguïsé (le rapport ne porte
  que `(page, id)`), et les tests l’évitent en ajoutant avant de supprimer.
- La bascule avant/après garde sa sélection de page indépendante ; le mode
  Différences réutilise la page d’après courante.

## Interface

`apps/diagcraft` — panneau agent, type `Comparison` :
- `view` passe de deux à trois états : `Before`, `After`, `Diff`.
- Un `DiffSet` est construit une seule fois depuis `report["changes"]`.
- `paint_document_preview` gagne un surlignage optionnel (contour d’accent sur
  les objets du jeu de différences, teinte sur l’après en mode Diff), avec une
  légende et les compteurs Ajout / Modification / Suppression.

## i18n

Nouvelles clés (source `fr`, 7 langues) :
- « Différences » (mode d’aperçu)
- « Ajout » / « Modification » / « Suppression » (légende et compteurs)
- « Objets modifiés par l’agent, surlignés d’accent. » (note)
- « Comparaison visuelle » (titre de la légende)

## Validation

- `cargo fmt --all`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace`, `cargo deny check licenses sources advisories`,
  `python3 scripts/check-publication.py`, `python3 scripts/check-locales.py`.
- Tests unitaires : construction du `DiffSet` depuis un rapport (ajout,
  modification, suppression, changement sans id) ; catégorisation en croisant
  avant/après.
- Test d’interface : la bascule vers Différences rend sans erreur et la légende
  est présente.
