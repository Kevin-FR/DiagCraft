# Sprint — Comparaison visuelle des modifications IA

Préparation du lot 114, sur la base de l’aperçu avant/après de l’item 113.
Contrat : [comparaison-visuelle.md](comparaison-visuelle.md).

## Étapes

1. **Catégoriser les changements** — construire un `DiffSet` déterministe depuis
   `report["changes"]` : pour chaque entrée avec un `id`, croiser l’avant et
   l’après de la même page pour classer en Ajout / Modification / Suppression.
   Clé `(page, id)` (identifiants par page).

2. **Surligner l’aperçu** — étendre `paint_document_preview` d’un surlignage
   optionnel :
   - contour d’accent sur les objets présents dans le jeu de différences de la
     page rendue (utilisé par Avant et Après) ;
   - en mode Différences, teinte d’accent sur les objets ajoutés/modifiés et
     légende avec compteurs.
   Le rendu de base reste inchangé quand aucun surlignage n’est fourni (les
   autres appelants : galerie, mermaid, csv, drawio, pages, minimap).

3. **Trois vues** — dans `agent_ui.rs`, remplacer la bascule deux états par
   trois états `Before` / `After` / `Diff`, construire le `DiffSet` une fois à
   partir du `Comparison`, et afficher légende + compteurs en mode Diff.

4. **i18n** — ajouter les clés (fr, en, de, es, it, pt, zh-Hans) via le script
   Python qui préserve l’ordre et `indent=2` ; `check-locales.py` doit rester
   vert.

5. **Tests** :
   - `diff_set` : rapport → catégories (ajout, modification, suppression,
     entrée sans id ignorée pour le surlignage) ;
   - catégorisation croisée avant/après ;
   - test d’interface : bascule vers Différences rend sans erreur et affiche la
     légende.

6. **Validation & publication** — commandes de validation complètes, mise à
   jour de la ROADMAP (item 114 `- [x]` + note « Fait : … »), commit et push
   sur `main`.

## Critères de fait

- Trois vues d’aperçu dans le panneau agent, différences surlignées.
- Tests d’interface et unitaires verts ; locales valides ; clippy sans
  avertissement.
- ROADMAP item 114 coché et documenté.
