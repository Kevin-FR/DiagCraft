# Architecture

## Frontières du workspace

- `crates/core` : bibliothèque `diagcraft`, sans egui, fenêtre ni SDK fournisseur.
  Elle contient le document, la géométrie, le routage, pages, styles, historique,
  catalogue, galerie, exports et contrat agents.
- `apps/diagcraft` : fenêtre et interactions natives. Les gestes modifient le
  document et son historique ; le rendu natif réutilise les géométries du moteur.
- `apps/diagcraft-cli` : programme console. Le dispatch headless est partagé
  avec l’application dans `crates/core/src/cli.rs`, y compris les mêmes validations.

MCP et l’interface Agents / IA appellent `agent.rs`. Un lot vérifie la révision,
valide le résultat complet, puis peut être prévisualisé avant application.
La CLI écrit un nouveau fichier ; l’interface conserve le lot comme une annulation.

`pages.rs` est la sérialisation canonique. Les pages sont enregistrées dans un
ordre stable indépendamment de la page affichée. Ne pas ajouter une seconde
représentation du document dans une interface ou un transport.

Les tests du moteur sont dans `crates/core/tests`, ceux des interactions natives
dans `apps/diagcraft/src/ui_tests.rs`. Les captures de documentation sont produites
par `diagcraft --screenshot`, à partir des seuls modèles du projet.

`apps/diagcraft/src/commands_ui.rs` centralise les commandes, leurs raccourcis et
leurs conditions de disponibilité. La palette, l’aide et les raccourcis utilisent
ce catalogue et appellent les actions de l’application. Les saisies de texte et
boîtes de dialogue gardent leurs touches ; une commande de dessin ne doit pas
s’exécuter derrière une fenêtre modale.

## Référence structurelle

L’organisation de [PhotoCraft](https://github.com/storytold/photocraft) sépare
applications, bibliothèques de moteur, documentation et packaging. DiagCraft
retient ce principe avec trois packages adaptés à son périmètre. Le code de
PhotoCraft et ses ressources n’ont pas été intégrés. Les choix de bibliothèques
sont détaillés dans [dependencies.md](dependencies.md).
