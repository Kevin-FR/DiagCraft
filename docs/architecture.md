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

Sous Linux, la dépendance winit de l’interface active `wayland-csd-adwaita`
sur la version déjà utilisée par eframe. Sans cette fonctionnalité, les bureaux
Wayland sans décorations serveur affichent un cadre de secours sombre sans titre.
Le bureau continue à gérer le déplacement, le redimensionnement et les boutons.
X11 conserve ses décorations serveur ; macOS et Windows n’activent pas Adwaita.

`apps/diagcraft/src/commands_ui.rs` centralise les commandes, leurs raccourcis et
leurs conditions de disponibilité. La palette, l’aide et les raccourcis utilisent
ce catalogue et appellent les actions de l’application. Les saisies de texte et
boîtes de dialogue gardent leurs touches ; une commande de dessin ne doit pas
s’exécuter derrière une fenêtre modale.

`interface_prefs.rs` conserve la disposition, la langue et les préférences de mise à jour dans un JSON local borné,
avec écriture atomique. Les captures et tests n’accèdent pas aux préférences utilisateur.
Les dimensions voulues restent distinctes des limites temporaires de fenêtre.
Le moteur documentaire et son historique ne connaissent pas ces préférences.

`panels_ui.rs` centralise les marges et en-têtes des volets. Le rail gauche
contient son unique commande ; le volet droit fermé utilise un rail de 48 points
distinct du panneau redimensionnable pour conserver la largeur préférée.
Son bouton garde les mêmes coordonnées ouvert ou fermé. Le ruban et la barre
des documents ne dupliquent pas ces icônes. Le corps de l’inspecteur défile
indépendamment de son en-tête.

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

## Contenu enrichi des blocs

`rich_text.rs` conserve le texte brut et des portions optionnelles `text_runs`,
indexées en scalaires Unicode, ordonnées et sans chevauchement. Les frontières
respectent les graphèmes ; les styles omis héritent de la typographie du nœud.
L’insertion et la suppression préservent les portions extérieures et reconstruisent
les frontières après édition. Le calcul de lignes utilise les styles résolus ;
le canevas et SVG consomment les mêmes portions par ligne.

`rich_text_ui.rs` conserve un brouillon local et un historique borné. Un TextEdit
est placé dans la zone de contenu du bloc sur le canevas ; la barre flottante
reste dans les limites de la feuille. Le texte source n’est jamais réécrit pour
simuler les retours visuels. Le Galley normalise les positions des lignes alignées
pour préserver les indices de curseur Unicode, la sélection et la composition IME.
L’aperçu inclut la géométrie ajustée et ses connecteurs sans modifier le document.
Les commandes du dessin restent bloquées pendant la saisie, mais la navigation
au trackpad reste disponible. Un événement de texte suivi d’un clic dans la même
image est traité avant le clic, rejoué à l’image suivante via raw_input_hook.
Enregistrement, fermeture et raccourci de validation attendent le champ et sa
barre. La révision est vérifiée avant validation, avec un seul changement dans
l’historique documentaire. La récupération ne reçoit pas le brouillon.
Les anciennes applications v7 ignorent les portions optionnelles.

## Recherche et placement au clavier

`object_search_ui.rs` indexe la page active à l’ouverture de la recherche : texte
original, libellés localisés, types, IDs et positions. Les résultats sont filtrés
sans accents et rendus avec `ScrollArea::show_rows` ; leurs deux lignes ont une
hauteur fixe et leurs libellés AccessKit incluent le type et l’identifiant.
Le snapshot vérifie la révision seulement à l’activation, sans sérialiser le
document à chaque image. Un résultat périmé est rafraîchi avant toute action.
Un changement de page ou d’onglet ferme la recherche. Sélection et caméra sont
des états d’interface ; elles ne créent pas de modification documentaire.

La commande contextuelle `PlaceShape` utilise Entrée uniquement avec l’outil
Forme actif ; le même raccourci conserve l’édition d’un bloc déjà sélectionné
avec l’outil Sélection. Le placement cherche un espace voisin libre dans la
vue puis sur la page, conserve le symbole en attente et ouvre la saisie directe.
La création et le texte gardent leurs annulations distinctes. Après fermeture
d’une modale, une demande de focus refusée est réessayée ; les événements de
saisie non traités sont rejoués avant la validation du texte.

## Déplacement au clavier

`keyboard_ui.rs` conserve un snapshot transitoire par pression continue des
flèches. Une intersection de limites donne la même translation aux formes,
graphiques, tracés et géométries libres sélectionnés. Les limites hors page
incluent zéro pour éviter tout repositionnement implicite sur un autre axe.
Le magnétisme n’intervient pas dans les pas de 1 et 10 pixels.

La validation centrale termine le geste avant une autre édition ou un changement
de document ; `finish_frame_edit` et la récupération ignorent le brouillon actif.
Échap restaure le snapshot sans ajouter d’historique. Après interruption, les
répétitions sont ignorées jusqu’au relâchement. Les commandes de la palette
utilisent le même chemin, avec validation immédiate si aucune flèche n’est maintenue.

## Organisation des objets

`organization.rs` conserve des métadonnées facultatives propres à chaque page :
calques ordonnés, groupes plats et états des objets indexés par leurs IDs existants.
Les identifiants de groupe et de calque ont des espaces distincts de ceux des objets.
Le calque de base 0 reste implicite, visible et déverrouillé. Un groupe ne traverse
pas les calques. La sérialisation canonique v7 omet le champ quand il est vide.

Un `OrganizationView` calculé par page fournit les ensembles de visibilité,
verrouillage et sélection. Il n’est jamais sérialisé ; les chemins de rendu,
d’export et d’inspection le réutilisent pour éviter les recherches linéaires par
objet. L’ordre des calques prime sur le type des objets. Un connecteur attaché
à une forme masquée reste enregistré mais n’est pas affiché ou exporté.

`organization_ui.rs` centralise les commandes, l’affectation des nouveaux objets
et les contrôles de calques. Le choix du calque de dessin et les brouillons de
noms sont des états d’interface, distincts du document. Les commandes d’édition
vérifient la protection et la validation de transition empêche une écriture directe
oubliée d’altérer un objet protégé. L’historique, la galerie et la récupération
continuent à stocker le document canonique complet.

Les patches API 1 utilisent des alias de calque distincts des alias d’objets.
Chaque opération vérifie ses protections ; un déverrouillage explicite peut
précéder une modification dans un même lot atomique. Les anciens lecteurs v7
ignorent l’organisation et ne la préservent pas au réenregistrement.

## Routage des connecteurs

`routing.rs` calcule des tracés orthogonaux sans modifier le document. Un contexte
par page indexe les nœuds et obstacles visibles. Les ports produisent des amorces
cardinales ; les points manuels sont des ancres exactes et ordonnées. Chaque
portion est simplifiée séparément afin de préserver les ancres collinéaires.
Les lignes directes conservent leur géométrie précédente.

Des trajets courts sont essayés avant un A* rectiligne sur coordonnées comprimées.
Les voies viennent des enveloppes proches ; deux tableaux de différences bloquent
les arêtes pour tous les obstacles, y compris ceux sans voie dédiée. Le nombre
de voies et d’états visités est borné. L’ordre des obstacles, la caméra et le zoom
ne changent pas le résultat. Les résultats contraints ou limités sont signalés ;
ils ne donnent jamais lieu à un déplacement implicite de point manuel.

`routing_ui.rs` garde un cache transitoire de géométrie, comparée sans empreinte
approximative. Les changements de positions, tailles, types, visibilité, extrémités
ou points l’invalident ; les textes des blocs et le titre ne l’invalident pas.
Le canevas utilise le même tracé pour la ligne, les styles et le libellé. SVG et
inspection réutilisent un contexte de page. Les diagnostics agents exposent le
statut calculé sans changer le contrat de patch API 1 ou la sérialisation v7.
