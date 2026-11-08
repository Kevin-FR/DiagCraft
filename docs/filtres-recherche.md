# Contrat — Filtres & recherche complète (lot 6b)

Ce document définit le sous-ensemble « filtres & recherche complète » du lot
**6. Retrouver et organiser** de la feuille de route, en continuité avec
l'étiquetage et la note d'objets (see [etiquettes-notes.md](etiquettes-notes.md))
et la recherche d'objets existante ([recherche-objets.md](recherche-objets.md)).

## But

Permettre de retrouver un objet dans l'application par son texte, son type, son
identifiant, **ou ses étiquettes**, en combinant des filtres indépendants, et
d'étendre la recherche à **toutes les pages** depuis la fenêtre de recherche
d'objets, sans modifier le dessin ni l'historique d'annulation.

## Périmètre retenu

Rien n'est enregistré dans le `.draw` v8 : les filtres et la portée de recherche
sont des préférences de session uniquement (comme l'est déjà la recherche
existante). Aucun numéro de version n'est incrémenté.

1. **Filtre par étiquette** dans la recherche d'objets (⌘F / Ctrl+F).
   - Un rang de puces d'étiquettes, une par étiquette distincte présente sur la
     page (ou dans le document selon la portée), combiné au texte et au filtre
     de type déjà existants par un ET logique.
   - « Toutes » (aucune étiquette filtrante) par défaut ; une seule étiquette
     active à la fois ; re-cliquer une étiquette active la désactive.
   - Un résultat est affiché s'il possède l'étiquette active (recherche
     d'étiquettes marquée, ignore casse/accents).
2. **Recherche complète multipage** dans la même fenêtre.
   - Un sélecteur de portée **Page active** / **Toutes les pages**.
   - En portée « toutes les pages », la recherche parcourt toutes les pages,
     les résultats indiquent leur page (numéro + nom), le survol/sélection
     ouvre la bonne page et sélectionne l'objet, comme le fait déjà l'arborescence
     multipage. La sélection additive fonctionne à travers les pages.
   - Limite à 300 premiers résultats, message de dépassement affiché.
3. **Indexation des étiquettes et de la note dans l'arborescence multipage**
   (cohérence avec 6a) : `rows` de `outline_ui` inclut `search_text()` des
   annotations, de sorte qu'une recherche complète retrouve aussi les objets
   par leur étiquette ou leur note, sur toutes les pages.

## Hors périmètre

- Modification d'étiquettes/notes par patch agent (étude séparée, déferrée en 6a).
- Filtres par calque ou par visibilité dans la recherche d'objets (déjà
  disponibles dans l'arborescence).
- Filtrage prédictif, sauvegarde des filtres entre sessions, ou recherche
  instantanée compressée.

## Comportement et garanties

- Les filtres (texte, type, étiquette, portée) se combinent par ET ; vider le
  texte et ne filtrer ni type ni étiquette retrouve tous les objets de la portée.
- Changer de page ou de document ferme la fenêtre de recherche (comportement
  existant conservé).
- La recherche, les filtres et la navigation ne modifient ni le fichier ni
  l'historique d'annulation.
- Les résultats sont recalculés avant toute sélection si le document a changé
  (révision), comportement existant conservé.
- i18n dans les sept langues (fr = clé source) ; messages.json formaté
  `indent=2`, ordre des langues {fr,en,de,es,it,pt,zh-Hans}.

## Critères d'acceptation

- Sur une page avec plusieurs objets, filtrer par une étiquette ne montre que
  les objets qui la portent ; re-cliquer réaffiche tous les objets de type/texte.
- Le filtre par étiquette se combine au filtre de type et au texte.
- En portée « toutes les pages », chercher un mot présent sur une autre page
  affiche le résultat précédé de son numéro de page ; Entrée ouvre cette page et
  sélectionne l'objet ; Maj+Entrée l'ajoute à la sélection.
- La limite de 300 résultats affiche la mention de dépassement.
- Une recherche dans l'arborescence retrouve un objet par le mot d'une étiquette
  ou d'une note sur une autre page.
- `cargo fmt --all`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace`, `cargo deny check licenses sources advisories`,
  `python3 scripts/check-publication.py`, `python3 scripts/check-locales.py` : verts.
