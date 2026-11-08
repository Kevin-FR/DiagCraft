# Sprint — Lot 111 : Styles nommés avec propagation (terminaison)

Contrat : [styles-nommes.md](styles-nommes.md).

## État à l'ouverture de ce sprint

Le moteur et l'agent sont **terminés et testés** :
- `NamedStyle` / `NamedStyles`, `Node.style_ref: Option<String>`, `Document.named_styles`.
- Sérialisation v8 (`pages.rs` `File`/`Raw`), préservation de la table aux
  changements de page (`replace_pages`/`from_page`/…).
- Méthodes `Document` : `create_named_style`, `rename_named_style`,
  `remove_named_style`, `edit_named_style_{fill,stroke,text_color,stroke_width,font_size}`,
  `apply_named_style_to_nodes`, `detach_named_style`, `named_style_names`,
  avec propagation à toutes les pages.
- Agent : `NodeChanges.style_ref` (nom = appliquer, chaîne vide = détacher) et
  contexte `inspect().named_styles`.
- Tests : `named_styles_*` (core.rs, 3) + `update_node_assigns_and_clears_named_style_ref`
  (agent_features.rs, 1) — **verts**.

## Objectif de ce sprint : livrer le lot 111 complet

1. **Panneau « Styles nommés »** (interface egui, une annulation par commande).
   - Modèle undo : l'inspecteur mue `self.doc` directement ; `finish_frame_edit`
     puis `commit()` (app.rs l.920-970) enregistrent une seule annulation quand
     `self.doc != before`. Les commandes du panneau passent donc par
     `Page`-UI sur `self.doc` → undo natif.
   - **Branch Document** (aucune sélection, app.rs ~l.1499) : une section
     « Styles nommés » listant chaque style (nom renommable,
     valeurs fill/stroke/text_color/stroke_width/font_size éditables avec
     propagation), un bouton « Nouveau style » (depuis l'apparence d'un objet
     sélectionné ou défaut), et un bouton « Supprimer » par style (détache les
     formes liées).
   - **Branch objet unique** (app.rs ~l.1455, forme/text) : un sélecteur de
     style (Aucun / styles du document) qui applique le style (pose `style_ref`
     + recopie) ou détache.
   - Place le panneau dans `appearance_ui.rs` (fonctions `named_styles_ui` /
     `node_style_picker`) appelées depuis `app.rs::inspector`.
2. **i18n** — nouvelles clés (`fr` source, en/de/es/it/pt/zh-Hans),
   `messages.json` plat, `indent=2`, ordre et `check-locales.py` verts.
3. **Tests d'interface** — création puis propagation depuis l'inspecteur,
   application d'un style à un objet sélectionné.
4. **Validation complète** — fmt, clippy `-D warnings`, workspace tests,
   deny (licenses/sources/advisories), `check-publication.py`, `check-locales.py`.
5. **ROADMAP** — item 111 `- [x]` + note « Fait : … ».
6. **Publication** — commit en anglais, push `main`.

## Critères de fait

- Styles définis au niveau document et référencés par les formes ; l'édition se
  propage à toutes les pages en une annulation.
- Aptitude v8 : champ omis quand vide, rétrocompatible par défaut.
- Tests moteur + agent + interface verts ; clippy sans avertissement ; locales
  valides dans les 7 langues.
- ROADMAP item 111 coché et documenté ; commit poussé sur `main`.

## Après ce lot

Enchaîner les lots codables restants (97 groupes mixtes, 110 conteneurs,
112 composants, 252 rotation, 259 SVG, priorité 5 exports) puis documenter les
items humains/infra bloqués (40, 106, 222, 242, 243, 255, 265, 266, 268, 269,
272, 277, 279, 280, 281, 282, 284 / lot 276 réseau).
