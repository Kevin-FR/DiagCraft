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

`interface_prefs.rs` conserve la disposition, la langue et les préférences de mise à jour dans un JSON local borné,
avec écriture atomique. Les captures et tests n’accèdent pas aux préférences utilisateur.
Les dimensions voulues restent distinctes des limites temporaires de fenêtre.
Le moteur documentaire et son historique ne connaissent pas ces préférences.

## Référence structurelle

L’organisation de [PhotoCraft](https://github.com/storytold/photocraft) sépare
applications, bibliothèques de moteur, documentation et packaging. DiagCraft
retient ce principe avec trois packages adaptés à son périmètre. Le code de
PhotoCraft et ses ressources n’ont pas été intégrés. Les choix de bibliothèques
sont détaillés dans [dependencies.md](dependencies.md).

## Langues et mises à jour

`i18n.rs` et `assets/locales/messages.json` traduisent les libellés de l’interface.
Les identifiants du moteur, l’API v1, le format v7 et les textes des documents
ne passent pas par le catalogue. Les valeurs des modèles de message sont
substituées après traduction, sans interpréter les accolades du texte utilisateur.
Le choix de langue est propre au thread de l’interface.

`updates.rs` filtre les releases du dépôt officiel avec SemVer et traite le
réseau dans un thread annulable. Le curl système est lancé avec des arguments
fixes, sans shell ni configuration curlrc. Les réponses, tailles et URL des assets
sont bornées ; le SHA-256 est contrôlé avant remplacement atomique du téléchargement.
`settings_ui.rs` présente les résultats sans installer ni exécuter de paquet.
La recherche automatique est facultative et ne bloque pas le dessin.

## Récupération

`recovery.rs` conserve une copie canonique des documents modifiés dans une
enveloppe locale v1 distincte du format `.draw` v7. Un worker borné écrit un seul
checkpoint atomique par instance ; les documents validés et terminés sont transmis
par `recovery_ui.rs` toutes les dix secondes. Les onglets récupérés restent marqués
non enregistrés indépendamment de leur historique et n’ont pas de chemin source.

Un verrou exclusif `std::fs::File::try_lock` protège chaque session, y compris
les sessions anciennes proposées à la récupération. Sa libération par le système
permet de reconnaître un processus arrêté sans heuristique de PID. Le numéro
de checkpoint confirme le transfert avant suppression de la copie ancienne.
Une fermeture acceptée nettoie la session courante ; les copies non ouvertes
restent disponibles. Les tests et captures utilisent seulement des dossiers temporaires.

## Texte et ajustement

`text_layout.rs` mesure les polices embarquées avec ttf-parser et coupe les lignes
aux frontières de graphèmes Unicode. Il fournit les lignes, interlignes, marges
et dimensions au canevas, au SVG et à l’inspection agents. Hack est explicitement
utilisée pour le texte monospace ; Noto Sans SC sert de repli pour le chinois.
Le PDF consomme le même SVG avec les polices embarquées.

`TextStyle::auto_fit` est un champ optionnel v7, faux par défaut et omis dans ce cas.
L’interface applique l’ajustement au commit d’un geste ; les patches l’appliquent
uniquement aux nœuds modifiés, avant validation et calcul de la révision résultante.
Une sélection est ajustée atomiquement. Les positions et IDs sont conservés ; les
routes de connecteurs suivent leurs formes par le mécanisme existant. Un redimensionnement
manuel désactive l’option dans le même geste annulable. Le moteur ne connaît pas egui.
