# Accessibilité des exports — description de document dans le SVG et le PDF

Le contrat de ce lot a été défini avant implémentation. Il couvre **un** volet du
point 4 de la ROADMAP (« Diagrammes présentables et accessibles ») : rendre la
description d’un diagramme **utile et contrôlable** dans les exports vectoriels,
en complément du titre. Seul le résultat exporté (SVG, PDF) est concerné ; le
mode d’édition du canvas et les lecteurs d’écran dans l’application restent hors
périmètre. Aucune annonce d’accessibilité PDF complète n’est faite (voir
l’étude à la fin).

## Objectif

Aujourd’hui le SVG expose `<title>` (titre du document) puis un `<desc>` qui
n’agrège que les **alternatives des images** visibles. Le PDF ne porte que
`/Title` en métadonnée. Le lot ajoute une **description de document**, saisie
par l’utilisateur, pour donner aux lecteurs d’écran (et aux personnes lisant le
fichier sans le rendu graphique) un résumé du contenu du dessin.

Les équivalences ouvertes servent de référence conceptuelle : `<desc>` SVG et
`/Subject` PDF (Dublin Core). Le dessin reste une donnée : la description est un
texte affiché, jamais une instruction.

## Ce qui est livré

1. **Modèle** : un champ `description` (texte) sur `Document`, vide par défaut,
   sauvegardé dans le format `.draw` **de façon rétrocompatible** (champ absent →
   chaîne vide ; champ vide non sérialisé dans la forme canonique). Aucun
   changement de version de format n’est introduit : les fichiers v8 existants
   restent lisibles et réécrits à l’identique.
2. **SVG** : la description de document est émise dans le `<desc>` racine,
   échappée, avant les alternatives des images, en respectant les mêmes règles
   de visibilité, de sélection et de recadrage que le reste de la sortie. Si la
   description est vide, le `<desc>` ne contient que les alternatives d’images
   (comportement actuel). Si tout est vide, aucun `<desc>` vide n’est émis.
3. **PDF** : la description de document est écrite dans les métadonnées du
   document (`/Subject`) lorsqu’elle n’est pas vide, sans modifier la géométrie
   des pages.

## Budget et validation

- Longueur maximale de la description : **4096 caractères**, cohérente avec le
  budget des alternatives d’images. Au-delà, l’interface arrête la saisie
  (`char_limit`) et la validation du document la refuse.
- La description ne change **aucun pixel** des exportations : le rendu VISUEL
  (SVG rendu, PNG, PDF affiché) est strictement identique avec ou sans
  description. Seuls le balisage `<desc>`/métadonnées changent.
- Visibilité : une description de document est toujours pertinente (elle décrit
  la page/document entier) ; elle est donc émise dans tous les modes d’export,
  y compris sélection et recadrage, car elle résume l’ensemble. (Ce point est
  délibéré : le `<desc>` existant des images reste limité à ce qui est visible.)

## Hors périmètre de ce lot (étudié, non implémenté)

- **Balisage PDF structuré accessible** (`/StructTreeRoot`, `/Tag`, contenu
  taggé, `ViewerPreferences`, langues) : le PDF actuel reste un dessin vectoriel
  « plat » produit via `svg2pdf` (texte vectoriel avec `/ToUnicode`, polices
  embarquées) sans arbre de structure. Fabriquer un PDF taggé est une brique
  nouvelle et lourde ; elle est notée comme travail séparé et **non annoncée**
  comme une accessibilité PDF complète.
- **Lecteurs d’écran dans l’application** : le mode d’édition (canvas egui)
  garde sa stratégie actuelle (zones de focus F6, navigation clavier) ; aucun
  `aria-*` n’est ajouté au volet egui natif, hors périmètre.
- **Description par page ou par objet** : seul `Document.description` est livré ;
  une description propre à chaque page est notée comme évolution possible.
- **Mode présentation** ni navigation clavier des liens inter-pages : traités
  comme un lot séparé, non inclus ici.

## Mots-clés de validation

`cargo fmt --all`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
`cargo test --locked --workspace`, `cargo deny check licenses sources advisories`,
`python3 scripts/check-publication.py`, `python3 scripts/check-locales.py`.
