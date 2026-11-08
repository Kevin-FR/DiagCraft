# Contrat — Styles nommés avec propagation

**Lot 111** de la feuille de route : permettre de définir des apparences
réutilisables nommées dans le document et de les propager aux objets qui les
référencent.

## Objectif

Un style nommé regroupe des valeurs d’apparence (remplissage, trait, couleur de
texte, largeur de trait, taille de police). Il est défini une fois au niveau du
document et peut être référencé par des formes. Modifier le style se **propage**
à toutes les formes qui le référencent ; le contraire (modifier une forme liée)
ne casse pas le lien tant que la référence reste posée.

## Périmètre

- Un style nommé `NamedStyle` :
  `{ name, fill, stroke, text_color, stroke_width, font_size }`, avec
  `name` unique et non vide dans le document.
- Table document-globale `Document.named_styles: Vec<NamedStyle>` (partagée par
  toutes les pages). Aucune duplication par page.
- Référence optionnelle `Node.style_ref: Option<String>` : le nom du style lié
  (une forme peut n’avoir aucun style lié).
- **Propagation** : une édition du style (remplissage, trait, couleur, largeur,
  police) met à jour la définition ET toutes les formes du document qui la
  référencent, sur toutes les pages, en une seule annulation dans l’interface.
- **Application** : appliquer un style sur une sélection pose la référence
  `style_ref` et recopie les valeurs du style sur les objets sélectionnés.
- **Détacher** une forme retire sa référence sans toucher à ses valeurs.
- Le renommage met à jour les références ; la suppression retire le style et
  détache les formes qui le référençaient.

## Format et rétrocompatibilité

- `.draw` reste **v8** : le champ `named_styles` est un nouveau champ racine de
  niveau document, ajouté aux structs manuels `File`/`Raw` de `pages.rs` avec
  `#[serde(default, skip_serializing_if = "NamedStyles::is_empty")]`.
- `Node.style_ref` est optionnel, `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- Les anciens fichiers v8 se chargent sans le champ (défaut vide) ; les
  nouveaux fichiers n’écrivent le champ que s’il est non vide, préservant la
  stabilité des sorties existantes.
- La table document-globale **survit aux changements de page** : `from_page`,
  `replace_pages`, `switch_page`, `add_page`, `remove_page`, `move_page` la
  préservent.

## Interface et agent

- Un panneau « Styles nommés » dans l’inspecteur : créer (depuis l’apparence
  courante), renommer, supprimer, détacher, appliquer à la sélection, et éditer
  les valeurs du style avec propagation.
- Un sélecteur de style sur une forme sélectionnée (aucun / styles du
  document).
- Agent (API 1) : le contexte d’inspection inclut `named_styles` ; `update_node`
  accepte un champ `style_ref` (un nom pose la référence et recopie les valeurs ;
  une chaîne vide la retire). La gestion des styles (création/suppression) reste
  une commande d’interface ; l’agent ne crée ni ne détruit de styles dans ce lot.

## i18n

Nouvelles clés (source `fr`, 7 langues) pour le panneau, les actions
(créer/renommer/supprimer/détacher/appliquer) et la propagation.

## Validation

- `cargo fmt --all`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace`, `cargo deny check licenses sources advisories`,
  `python3 scripts/check-publication.py`, `python3 scripts/check-locales.py`.
- Tests : aller-retour/compacité v8 et chargement sans le champ ; propagation
  d’une édition à toutes les pages ; renommage et suppression ; application et
  détachement ; référence via patch agent ; survie aux changements de page.
