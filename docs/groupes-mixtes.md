# Contrat — Groupes mixtes et liaisons longues dans la mise en page automatique

**Lot 97** de la feuille de route : affiner la mise en page automatique pour
permettre aux groupes mixtes d'être déplacés comme une unité et prendre en
compte les liaisons longues dans l'optimisation du score de croisements.

## Objectif

Deux extensions de la mise en page automatique existante (`docs/mise-en-page.md`) :

1. **Groupes mixtes translatables** : un groupe contenant un graphique
   (`Chart`) ou un dessin libre (`Drawing`) et au moins un bloc autre qu’un texte
   devient une unité compatible et se déplace comme un seul bloc.
2. **Liaisons longues dans le score** : le score de croisements « entre niveaux
   voisins » prend désormais en compte les arêtes qui traversent plusieurs
   niveaux (projection verticale de leur source), au lieu de ne compter que les
   arêtes entre niveaux consécutifs.

Les connecteurs externes ou à extrémité libre et les groupes composés
uniquement de textes restent fixés, comme aujourd'hui.

## Périmètre

### Groupes mixtes

- Un `Chart` est un membre translatable : sa boîte (`position`/`size`) rejoint
  l'enveloppe du groupe et est traduite avec lui.
- Un `Drawing` est un membre translatable : les points de sa ligne ou de sa
  courbe rejoignent l’enveloppe. Ses extrémités `start`/`end` sont traduites ;
  `bend` et `label_offset`, relatifs à ces extrémités, sont conservés, ainsi que
  le libellé et le style.
- Un groupe reste **fixe** dans les seuls cas suivants :
  - un membre est protégé ou masqué, ou aucun membre n’est sélectionné dans
    une mise en page de la sélection ;
  - le groupe ne contient aucun bloc autre qu’un texte (y compris un groupe
    composé uniquement de graphiques ou dessins libres) ;
  - un membre est un connecteur avec une extrémité **hors du groupe** ou une
    **extrémité libre** (`free_start`/`free_end`) — déplacer ce connecteur
    casserait la connexion externe.
- L'enveloppe utilisée pour le placement inclut les blocs, les points manuels
  des connecteurs internes, la boîte des graphiques et les points des dessins
  libres.
- La translation applique le même décalage aux nœuds, aux points manuels, aux
  boîtes de graphiques et aux dessins libres membres.
- Les règles existantes restent valides : déterminisme, limite de 500 blocs,
  préréglages, annulation unique, préservation des métadonnées, format `.draw`
  v8 et API agents v1 sans dépendance ajoutée.

### Liaisons longues dans le score

- Le score `crossings_before`/`crossings_after` compte toujours les croisements
  **niveau par niveau** (le libellé UI « Croisements entre niveaux voisins »
  reste correct).
- Une arête `a → b` qui traverse plusieurs niveaux (`rank[b] > rank[a] + 1`)
  garde sa rangée de départ pour les segments intermédiaires
  `(rows[a], rows[a])`, puis rejoint sa vraie arrivée au dernier segment
  `(rows[a], rows[b])`. Ce dernier segment évite les faux croisements des
  liaisons partageant la même arrivée et permet de compter deux liaisons
  longues dont les arrivées sont inversées.
- Le calcul reste **combinatoire, déterministe et borné** (aucun routage A* des
  routes réelles : cela reste documenté comme extension future).
- Le score ainsi étendu continue de servir à choisir entre le placement initial
  (ordre des IDs) et les candidats réordonnés (8 balayages), en ne retenant que
  les scores strictement meilleurs, à données et réglages égaux.

## Format et compatibilité

Aucun changement de format `.draw` (toujours **v8**), ni d'API agents (toujours
**v1**), sans ajout de dépendance. La revue met seulement à jour la dépendance
transitive `async-executor` de 1.14.0 à 1.14.1. Les fichiers sources restent
intacts tant que la mise en page n’est pas appliquée puis enregistrée.

## Interface

Aucune nouvelle commande. Les aides existantes sont mises à jour. Le panneau « Mise
en page automatique » existant affiche toujours l'aperçu, les compteurs et le
score. Les dessins libres visibles sont aussi représentés dans l’aperçu simplifié,
afin de vérifier leur déplacement avant application. L'outil d'aide au survol du score
est ajusté pour mentionner les liaisons longues.

## i18n

Les aides des groupes compatibles et du score reçoivent une précision
textuelle dans les sept langues. Les clés sources françaises et leurs
traductions sont remplacées ensemble dans le catalogue.

## Validation

- `cargo fmt --all`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace`, `cargo deny check licenses sources advisories`,
  `python3 scripts/check-publication.py`, `python3 scripts/check-locales.py`.
- Tests moteur : groupes mixtes (chart/drawing) translatables avec enveloppe,
  compteurs et préservation ; groupes à connecteur externe/libre et text-only
  toujours fixes ; projection des liaisons longues dans le score (moteur +
  oracle).
- Tests interface : aperçu/application d'un groupe mixte en une annulation,
  sept langues.
- Mise à jour du guide et de la feuille de route, avec séparation explicite
  entre score estimé réalisé et optimisation des routes réelles restant à faire.
