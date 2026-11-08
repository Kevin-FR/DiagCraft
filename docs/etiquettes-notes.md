# Étiquettes et notes d’objets

Ce document est le contrat du sous-ensemble « Étiquettes et notes d’objets »,
premier volet de la priorité « Retrouver et organiser » de la feuille de route.
Il est écrit avant l’implémentation, sur le modèle des contrats précédents
(`csv.md`, `drawio.md`, `accessibilite.md`). Le plan d’exécution est dans
[docs/sprint-etiquettes-notes.md](sprint-etiquettes-notes.md).

## Objectif

Donner aux objets d’un diagramme deux métadonnées d’organisation et de
recherche, enregistrées dans le dessin et rétrocompatibles avec le format
`.draw` v8 :

- **Étiquettes** : un petit ensemble de mots-clés courts pour catégoriser un
  objet et le retrouver (équivalent des « tags »).
- **Note** : un texte long facultatif pour documenter ou décrire un objet sans
  l’afficher dans le corps du diagramme.

Ces métadonnées s’ajoutent aux textes déjà recherchables. Elles sont éditables
à l’interface, indexées par la recherche d’objets, et leur périmètre
(enregistré/exporté) est explicite ci-dessous.

## Périmètre

### Objets concernés

Les métadonnées s’appliquent aux objets du modèle qui ont une identité et un
sens métier :

- **Formes et textes** (`Node`)
- **Connecteurs** (`Connector`)
- **Traits et courbes** (`Drawing`)
- **Graphiques** (`Chart`)

Elles ne s’appliquent pas aux `PageLink` (lien de navigation), aux calques ni
aux groupes, qui sont des structures d’organisation déjà couvertes par leurs
propres mécanismes.

### Définition des champs

- `tags: Vec<String>` — zéro à plusieurs étiquettes, chacune de **1 à 64**
  caractères, au plus **16** étiquettes par objet. Une étiquette est une chaîne
  de mots-clés sans retour de ligne (généralement un ou deux mots). On normalise
  à l’édition : suppression des espaces de tête et de queue, pas de doublon,
  ordre d’insertion conservé.
- `note: String` — **0 à 4096** caractères, texte libre multiligne. Vide = aucun
  contenu.

### Enregistré et exporté

- **Enregistré** : les deux champs sont sérialisés dans le format `.draw` v8 de
  façon rétrocompatible (champ absent si vide, lecture tolérante), une annulation
  dans l’interface, préservés par copie/collage, duplication et import/export.
- **Exporté** :
  - **Rien dans les sorties graphiques** (SVG, PDF, PNG) par défaut : les
    étiquettes et la note sont des métadonnées d’organisation internes, pas du
    contenu dessiné sur la feuille.
  - Les **étiquettes des formes et textes** (`Node`, y compris images) sont
    **exportées en métadonnées accessibles** dans le SVG : concaténées dans
    l’attribut `aria-label` de l’élément de l’objet, en plus des textes déjà
    présents, afin d’aider les lecteurs d’écran sans modifier le rendu visuel.
    La note n’est jamais exportée (contenu interne). Les autres catégories
    d’objets (connecteurs, traits/courbes, graphiques) ne reçoivent pas
    d’attribut accessible dans cette version, leur texte déjà présent suffisant.
- **Recherché** : la recherche d’objets prend en compte les étiquettes et la
  note pour retrouver un objet ; un résultat peut montrer une étiquette.
- Les exports réouvrables (future priorité 5) pourront exploiter ces champs ;
  ils restent hors de ce volet.

### Validation

Ajouter les contraintes dans `Document::validate` :

- chaque objet du périmètre : `tags` chacune 1..=64 caractères, sans retour de
  ligne, ≤ 16 éléments, sans doublon ; `note` ≤ 4096 caractères.
- Erreur signalée : « Étiquette invalide. », « Trop d’étiquettes. » ou
  « Note trop longue. »

### Rétrocompatibilité

- Format `.draw` toujours v8, aucune montée de version.
- Les fichiers existants sans `tags`/`note` restent identiques à l’écriture :
  champs non écrits quand vides (`skip_serializing_if`), lus avec `default`.
- Les versions antérieures ignorent les champs inconnus (sérialisation
  tolérante en lecture déjà en place).

## Comportements attendus

1. **Sérialisation** : `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
   pour `tags`, `#[serde(default, skip_serializing_if = "String::is_empty")]`
   pour `note`, sur chaque type d’objet du périmètre, dans
   `crates/core/src/document.rs`. Les types utilisent déjà les attributs de
   tolérance ; s’assurer de la cohérence.
2. **Édition à l’interface** :
   - Inspecteur : une section « ÉTIQUETTES » (saisie de mots-clés avec ajout et
     suppression de puces) et un champ multiligne « NOTE » sous l’objet
     sélectionné (forme, connecteur, trait/courbe, graphique).
   - Une modification = une annulation ; édition annulable depuis « Vers ».
3. **Recherche** : étendre `object_search_ui.rs` pour incorporer `tags` et la
   `note` dans la chaîne de recherche normalisée de chaque objet, sans modifier
   la navigation ni les actions. Afficher éventuellement une étiquette dans le
   détail d’un résultat.
4. **Exports** :
   - SVG : concaténer les étiquettes dans l’attribut `aria-label` de l’objet
     quand présentes (aucun changement du rendu visuel).
   - PDF/PNG : aucun changement.
5. **Agents/CLI** : l’inspection CLI/MCP expose `tags` et `note` de chaque objet
   (sérialisés automatiquement, absents quand vides). L’édition de ces champs
   par **patch agent** n’est pas couverte par ce volet : elle nécessite
   d’étendre les structures `NodeChanges`/`EdgeChanges`/`ChartChanges`
   (aujourd’hui `deny_unknown_fields`) et reste une étude séparée, non
   implémentée ni annoncée ici.
6. **Copie/collage et duplication** : les copies conservent étiquettes et note.
   La comparaison (`PartialEq`) et les diff des tests tiennent compte des champs.

## Hors périmètre (études séparées, non implémentées ici)

- Notes ou étiquettes par page ou par document (hors objets).
- Filtres dédiés dans un volet « styles nommés » / « composants » (priorité 6
  complète, volet suivant).
- Exports réouvrables utilisant ces métadonnées (priorité 5).
- Recherche globale multipage étendue aux notes (volet suivant
  « Filtres et recherche complète ») ; ici la recherche page courante inclut
  déjà les champs.

## Critères de validation

- `cargo fmt --all`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace`
- `cargo deny check licenses sources advisories`
- `python3 scripts/check-publication.py`
- `python3 scripts/check-locales.py`
- Tests : sérialisation rétrocompatible, validation des limites, recherche
  par étiquette/note, export SVG `aria-label`, édition = une annulation,
  copie/collage conservateur.
