# Changelog

Les versions de l’application suivent Semantic Versioning. Le format `.draw`
et l’API agents sont versionnés indépendamment.

## Non publié — insertion directe et accueil multilingue

- Une forme choisie dans la bibliothèque peut être insérée en cliquant sur un connecteur : repère de survol, partage atomique et annulation unique.
- En cas de manque de place, l’insertion est refusée sans créer de bloc isolé. Texte seul, symboles personnels et liaisons protégées gardent leur comportement actuel.
- Feuille de route enrichie : mise en page automatique, arborescence, liens entre pages, conteneurs/couloirs, styles nommés, composants liés et comparaison visuelle IA.

- Dessin d’accueil et modèle « De l’idée au diagramme » générés dans la langue de l’interface, dans les sept langues prises en charge.
- Le titre, les blocs et les libellés des liens sont localisés à la création ; changer la langue ne traduit jamais un dessin existant ou un modèle personnel.
- Géométrie, IDs, historique initial et format v7 conservés ; les autres modèles restent à localiser.

## 0.12.1 — mini-carte et fermeture au clavier

- Première publication du lot mini-carte, après contrôle natif de la candidate 0.12.0 restée en brouillon.
- Fermer avec Entrée ou Espace consomme l’activation avant de rendre le focus au bouton d’ouverture : la carte reste fermée.
- Le maintien et les répétitions de la touche de fermeture ne réactivent pas le bouton ; une nouvelle pression après relâchement le peut.
- Régression utilisant le vrai parcours Maj + Tab vers la fermeture, Entrée/Espace et cycles de touches dans les sept langues.
- Le tag et les paquets de la candidate 0.12.0 restent conservés ; aucune release publiée n’est remplacée.

## 0.12.0 — candidate non publiée, mini-carte de navigation

- Aperçu facultatif de la page active, avec objets visibles, sélection et cadre de la vue ; projection indépendante de la caméra.
- Clic et glissement pour déplacer la vue à zoom constant ; flèches, Maj + flèches et Début au clavier.
- Bouton permanent près du zoom, commande Affichage/palette et préférence locale masquée par défaut.
- Priorité aux saisies, modales et gestes du dessin ; capture des pressions et relâchements sans action derrière la carte.
- Focus rendu au bouton d’ouverture après fermeture ; calques masqués exclus et contenu hors page inclus.
- Aperçu simplifié utilisant le cache des connecteurs, sans export, décodage d’image ou bibliothèque supplémentaire.
- Tests d’interaction, petites fenêtres, pages extrêmes, sept langues, préférences et conservation du document, de la sélection et de la branche d’annulation.

Le format `.draw` reste **v7**, l’API de patches **v1**.

## 0.11.3 — lisibilité des textes et aides de connexion

- Rôles distincts pour les textes informatifs, actions, avertissements et erreurs ; statuts, graduations et placeholders plus lisibles sur les fonds de l’interface.
- Aides de navigation opaques et repliées à la largeur disponible ; accueil vide adapté aux fonds de page personnalisés.
- Aides de connexion valide, invalide ou libre lisibles et visibles dans tout le canevas, y compris hors de la feuille.
- Indicateur de document modifié renforcé sur les onglets actifs et inactifs.
- Sept tests de peinture réelle : couleurs des glyphes, fonds et opacité, états stabilisés, sept langues et fenêtre de 1200 × 680 points.
- Couleurs des dessins, format v7 et API v1 inchangés ; contrôles humains et lecteurs d’écran restant à réaliser.

## 0.11.2 — petites fenêtres et bibliothèques au clavier

- Fenêtre de contenu à partir de 1200 × 680 points ; largeurs préférées des volets restaurées après agrandissement.
- Corps du dialogue Agents défilant avec fermeture fixe ; matrice des modales dans les sept langues.
- Cartes de bibliothèque révélées au gain de focus, noms personnels distincts et graphiques traduits avec focus visible.
- Zone de départ conservée pour les commandes de focus exécutées depuis la palette.
- Classification F6 des éléments clippés et visibilité des cartes après redimensionnement, sans combattre le défilement manuel.

## 0.11.1 — navigation clavier et préparation de la 1.0

- Parcours F6 / Maj+F6 entre ruban, bibliothèque, feuille, propriétés et pages, sans modifier la disposition.
- Focus visible sur la feuille et les symboles ; le bouton du volet droit garde son focus en ouvrant/fermant le panneau.
- Libellés accessibles traduits des contrôles à icône, titre du document à fermer et nom de la feuille.
- Tests de saisie, placement, Tab/Entrée et noms AccessKit dans les sept langues, au format 1200 × 760.
- Contrôle Wayland couvrant le haut de l’écran, les couleurs de transition et le bord de fenêtre, avec rejet d’une ligne du ruban.
- Neuf régressions synthétiques, trois sessions natives par CI et conservation des captures/journaux/rapports.
- Budgets release mesurés sur trois corpus originaux, rapports JSON par plateforme et échec CI en cas de dépassement.
- Corpus déterministe de documents, patches et Mermaid corrompus ; sources et atomicité vérifiées.
- Guides de qualité et de reproduction, sans déclarer les essais utilisateurs ou la 1.0 terminés.

## 0.11.0 — Mermaid et diagnostics

- Sous-ensemble flowchart défini avant implémentation ; parseur local borné sans navigateur ni dépendance ajoutée.
- Import éditable, aperçu, ajout sur une nouvelle page, révision vérifiée et annulation unique.
- Export Mermaid de page avec textes, relations, rapport de pertes et acceptation explicite.
- CLI d’import/export avec dry-run et fichiers nouveaux ; sources préservées.
- Diagnostics CLI/MCP structurés avec opération, page, cible, code et aide ; champs texte conservés.
- Avertissements de page et de routage présentés distinctement dans les propriétés des agents.
- Interface et README dans les sept langues, exemples originaux et guides.

Format `.draw` **v7**, API de patches **v1**, quatre outils MCP inchangés.

## 0.10.0 — images embarquées et PNG

- Import PNG/JPEG, dépôt et collage d’image avec normalisation, orientation JPEG et réduction bornée, sans modifier les sources.
- Recadrage non destructif, proportions, description et remplacement ; mêmes objets et protections que les blocs.
- Données partagées entre clones d’annulation et conservées dans les copies, pages, bibliothèque, galerie et récupération.
- Export PNG page/sélection, dimensions, DPI `pHYs`, transparence et adaptation aux limites ; géométrie commune au SVG/PDF.
- CLI d’import/export, opérations `add_image` / `update_image` et inspection sans charge base64.
- Budgets contrôlés avant décompression ; tests de données malformées, alpha, crop, sérialisation et gestes ; interface dans les sept langues.

Format `.draw` **v7**, API **v1**. Les anciennes versions ignorent les images et les perdent au réenregistrement.
Aucune nouvelle bibliothèque tierce ou mise à jour de version ; usages explicites de dépendances déjà verrouillées.

## 0.9.0 — connecteurs et obstacles

- Routage orthogonal déterministe autour des blocs et graphiques visibles, entre des points manuels immuables et ordonnés.
- Ports explicites, extrémités libres et libellés préservés ; géométrie commune au canevas, aux aperçus, aux exports SVG/PDF et à l’inspection agents.
- Ajout d’un point sans figer tous les coudes automatiques ; magnétisme appliqué au déplacement, annulation conservée.
- Recherche bornée, voies stables et cache géométrique pour le rendu natif.
- État dérivé `routing_status` et diagnostics non bloquants pour les passages contraints ou recherches limitées ; propriétés traduites dans les sept langues.
- Guide des connecteurs et tests de labyrinthes, densité, visibilité, précision des coordonnées, stabilité et copies compatibles.

Format `.draw` **v7**, API agents **v1**, aucune dépendance ajoutée.
Les anciennes applications utilisent leur ancien routage. Les croisements de liens et collisions de libellés restent possibles.

## 0.8.0 — groupes, verrouillage et calques

- Groupes persistants par page : sélection, déplacement et duplication collectifs, regroupement et dissociation.
- Verrouillage des objets et des calques ; contrôles d’édition et garde de transition contre les modifications accidentelles.
- Gestionnaire compact de calques : création, nom, ordre, visibilité, verrouillage et affectation des objets.
- Ordre global des calques partagé entre types d’objets ; visibilité commune sur la feuille, en SVG et en PDF, y compris les liens attachés aux formes masquées.
- Métadonnées facultatives v7 conservées en multipage, dans les copies, l’historique et la récupération ; duplication sur le même calque sans créer de calques supplémentaires.
- API agents : huit opérations, alias de calques distincts, inspection et protections atomiques ; tests CLI/MCP dans la CI des trois systèmes.
- Commandes et messages dans les sept langues ; guide, README, capture et architecture mis à jour.
- Roadmap organisée en jalons GitHub 0.8.0, 0.9.0, 0.10.0, 0.11.0 et préparation 1.0.0 avec critères d’acceptation.

Le format `.draw` reste **v7**, l’API agents **v1**. Aucune dépendance ajoutée.
Les anciennes applications ignorent l’organisation et la perdent au réenregistrement.
Les groupes restent plats ; rotation et redimensionnement collectif ne sont pas inclus.

## 0.7.3 — fenêtre Ubuntu / Wayland

- Remplacement du cadre de secours sombre sans titre par les décorations Wayland Adwaita.
- Bordure fine, ombre transparente et titre de document visible ; déplacement, redimensionnement et boutons natifs conservés.
- Activation limitée à Linux, sans changement du rendu de fenêtre Mac ou Windows ni mise à jour des dépendances existantes.
- Test de fenêtre complète sous Weston sans décorations serveur, en complément du test X11 du paquet Debian installé.
- Notices MIT et OFL de la police Cantarell ajoutées aux paquets Linux ; collecte des licences d’éléments embarqués contrôlée par empreintes et trois tests.

Le format `.draw` reste **v7**, l’API agents **v1**. `sctk-adwaita` 0.10.1 est la seule nouvelle dépendance, spécifique à Linux.

## 0.7.2 — placement précis au clavier

- Flèches de 1 px et Maj + flèches de 10 px, indépendantes du zoom et du magnétisme.
- Translation commune aux blocs, graphiques, tracés et liens libres sélectionnés : les écarts et longueurs sont conservés aux bords de page.
- Maintenir une touche produit une seule annulation ; Échap ou Ctrl/⌘ Z annule le geste en gardant la sélection et les modifications précédentes.
- Déplacements hors page corrigés sans saut sur un autre axe ; limites globales des coordonnées respectées.
- Indicateur compact de déplacement ΔX/ΔY et message de fin explicite, traduits dans les sept langues.
- Quatre commandes de placement dans la palette et l’aide clavier ; champs de texte et modales gardent leurs flèches.
- Validation avant enregistrement, changement de document, clic ou perte de focus ; répétitions bloquées jusqu’au relâchement après interruption.
- Récupération automatique différée pendant le geste ; treize nouveaux tests de non-régression.

Le format `.draw` reste **v7**, l’API agents **v1**. Aucune dépendance ajoutée.

## 0.7.1 — volets intégrés à la feuille

- Une seule icône par côté, alignée sous le ruban ; suppression des doublons dans la barre des documents et la bibliothèque.
- Réouverture du volet droit au même emplacement, même lorsqu’il est fermé.
- En-têtes, marges et séparateurs communs ; propriétés défilantes avec fermeture toujours accessible.
- Document compact : nom, page et gestionnaire directement accessibles, sans introduction encombrante.
- Largeurs mémorisées, sélection et historique conservés ; libellés des icônes d’accessibilité localisés.
- Tests des quatre combinaisons de volets, deux tailles de fenêtre, deux états du ruban et sept langues.

Le format `.draw` reste **v7**, l’API agents **v1**. Aucune dépendance ajoutée.

## 0.7.0 — retrouver les objets et dessiner au clavier

- Recherche temporaire des objets de la page avec ⌘F / Ctrl+F, Affichage ou la palette.
- Texte complet, types, identifiants, accents ignorés, filtres et coordonnées X/Y.
- Entrée sélectionne et rejoint l’objet ; Maj+Entrée l’ajoute à la sélection ; F2 édite son texte.
- Liste virtualisée, étiquettes longues tronquées et libellés d’accessibilité avec type et ID.
- Zoom conservé si l’objet tient dans la fenêtre, cadrage des grands objets, aucun changement du dessin.
- R / T puis Entrée place une forme ou un texte dans un espace voisin libre et ouvre la saisie directe.
- Styles des symboles conservés ; placement, texte et liens restent annulables.
- Focus et premiers caractères préservés à la sortie de la recherche ; résultats périmés rafraîchis avant activation.
- Douze nouveaux tests d’interface, seize messages dans les sept langues et guides multilingues.

Le format `.draw` reste **v7**, l’API agents **v1**. Aucune dépendance ajoutée.
La recherche porte sur la page active. Les tests utilisateurs et l’évaluation complète avec lecteurs d’écran restent prévus.

## 0.6.1 — édition du texte directement sur la feuille

- Double-clic dans un bloc : curseur à l’endroit choisi, sans fenêtre supplémentaire.
- F2 ou Entrée sur un bloc : sélectionner le texte pour le remplacer rapidement.
- Petite barre flottante : gras, italique, souligné, taille, couleur et menu Plus.
- Validation au clic extérieur, bouton Terminer ou ⌘/Ctrl+Entrée ; Échap annule le brouillon.
- Saisie conservée avant l’enregistrement, le changement d’onglet, l’ouverture ou la fermeture.
- Derniers caractères préservés si saisie, composition IME et validation arrivent dans la même image.
- Sélection Unicode, alignement, zoom, défilement du texte et navigation trackpad.
- Aperçu des connecteurs pendant l’ajustement automatique ; une seule annulation documentaire.
- Huit nouveaux tests d’interface, cinq messages dans les sept langues et documentation multilingue.

Le format `.draw` reste **v7**, l’API agents **v1**. Aucune dépendance ajoutée.
Les limites de compatibilité des styles par portions introduits en 0.6.0 restent inchangées.

## 0.6.0 — mise en forme du contenu des blocs

- Éditeur accessible par double-clic, menu contextuel ou propriétés, sans agrandir les volets.
- Sélection de mots : gras, italique, souligné, barré, monospace, taille et couleur.
- Sauts de ligne, paragraphes vides, alignement et ajustement automatique.
- Aperçu sur la feuille, Appliquer/Annuler, historique local du texte et des styles.
- Une seule annulation du dessin après application ; identifiants, positions et connecteurs conservés.
- Styles conservés lors des insertions, suppressions, copies de blocs et enregistrements multipages.
- SVG avec portions éditables, PDF avec texte et polices embarquées, sans interpréter de HTML.
- API agents : champ `text_runs`, plages Unicode validées et remplacement atomique.
- Neuf nouveaux messages traduits dans les sept langues ; README et guide mis à jour.

Le format `.draw` reste **v7**, l’API agents **v1**. Aucune dépendance ajoutée.
Les anciens dessins gardent leur style. Les anciennes applications lisent le texte
brut et ignorent les styles par portions ; les réenregistrer avec elles perd ces styles.
Les polices du lecteur influencent l’apparence d’un SVG.


## 0.5.0 — ajustement des formes au texte

- Commande d’ajustement depuis les propriétés, le menu contextuel et la palette, y compris en sélection multiple.
- Option automatique par forme ; avec retour à la ligne, la largeur est conservée et la hauteur s’adapte.
- Mesures des polices Source Sans 3, Hack et Noto Sans SC, marges adaptées aux formes et interlignage partagé.
- Retours à la ligne fondés sur la largeur réelle des glyphes, sans séparer les accents combinés ; espaces et paragraphes préservés.
- Même disposition de texte dans l’interface, SVG et PDF ; police monospace Hack explicite.
- Identifiants, positions et connecteurs conservés, une seule annulation et désactivation lors d’un redimensionnement manuel.
- API agents : `text_style.auto_fit`, dimensions calculées avant aperçu, métriques de texte et débordement dans l’inspection.
- Messages traduits dans les sept langues et README multilingues mis à jour ensemble.

Le format `.draw` reste v7 et l’API agents v1. Le champ optionnel est désactivé
par défaut ; les fichiers existants ne changent pas de taille à l’ouverture.
Les versions verrouillées de ttf-parser et unicode-segmentation, déjà présentes
dans le workspace, deviennent aussi des dépendances directes du moteur.

## 0.4.0 — récupération des dessins après une fermeture inattendue

- Copie locale atomique des onglets modifiés toutes les 10 secondes, hors gestes en cours.
- Récupération des documents jamais enregistrés et des dessins multipages dans de nouveaux onglets.
- Fichiers source préservés ; chaque copie récupérée demande un nouvel emplacement à l’enregistrement.
- Verrous système par instance, transfert confirmé avant suppression d’une ancienne copie.
- Dialogue de récupération traduit dans les sept langues, accessible depuis Fichier et la palette.
- Copies illisibles conservées, suppression explicite avec confirmation et action Plus tard.
- Tests de processus arrêté brutalement, concurrence, intégrité, gestes et confirmations d’enregistrement.

Le format `.draw` reste v7, l’API agents v1. Aucune dépendance ajoutée.
Rust minimum 1.89 pour les verrous de fichiers de la bibliothèque standard ; outil de compilation fixé à 1.94.1.

## 0.3.0 — sept langues et mises à jour GitHub

- Choix immédiat de langue, préférence système et mémorisation locale indépendante des dessins.
- Interface en anglais, français, allemand, espagnol, italien, portugais et chinois simplifié.
- Recherche de commandes et symboles dans la langue active ; textes des dessins préservés.
- Police Noto Sans SC sous OFL embarquée pour l’interface et les PDF.
- Vérification GitHub en arrière-plan, manuelle ou facultative au démarrage, avec canal expérimental.
- Téléchargement ciblé par plateforme et vérification SHA-256 avant écriture atomique ; installation manuelle.
- Audit de chaque Mach-O et dépendance du paquet Mac : arm64 uniquement, aucune bibliothèque Intel.
- Métadonnées macOS des sept langues, tests des traductions, glyphes, exports et téléchargements.

## 0.2.1 — interface homogène et paquet Debian

- Ruban compact adapté à chaque onglet, sans déploiement automatique au clic.
- Commandes homogènes, rail compact et en-têtes communs aux deux volets.
- Largeurs ajustables avec protection de la feuille et restauration après réduction de fenêtre.
- Préférences locales du ruban et des volets, réinitialisation indépendante du document.
- Recherche sans accents dans les bibliothèques masquées et gestionnaire toujours visible.
- `.deb` amd64, icône, entrée desktop et MIME `.draw`, installation/désinstallation vérifiées en CI.
- Politique de versions documentée ; les tags 0.1.0 et 0.2.0 restent inchangés.

Le format `.draw` reste v7 et l’API agents v1. Aucune nouvelle bibliothèque externe.

## 0.2.0 — commandes et aide clavier

- Palette de commandes accessible depuis le bouton Rechercher ou ⌘K / Ctrl+K.
- Recherche par mots, sans distinction d’accents ou de majuscules ; navigation
  avec les flèches, Entrée, Tab et Échap.
- Commandes indisponibles accompagnées d’une explication ; les actions utilisent
  l’historique, les contrôles de sauvegarde et les exports existants.
- Aide F1 filtrable, avec raccourcis adaptés au système et gestes souris/trackpad.
- Protection des saisies et boîtes de dialogue, collage natif et annulation
  des modifications en cours lors des raccourcis.

Le format `.draw` reste v7 et l’API agents v1. Aucun ajout de dépendance.

## 0.1.0 — première version publique

- Éditeur natif en français : formes, textes, graphiques, symboles et modèles.
- Connecteurs attachés ou libres, multipage, réglages de page et styles.
- Trackpad, interface compacte, sélection multiple, alignement et gestes annulables.
- Sauvegarde `.draw`, galerie locale avec versions et exports SVG/PDF.
- Moteur réutilisable, application native et CLI console avec API JSON/MCP.
- Provenance des ressources, politique de licences et notices par plateforme.
- Premiers paquets macOS Apple Silicon, Windows x86_64 et Linux x86_64.

Version expérimentale : Mac signé ad hoc, sans notarisation Apple ; Windows
sans certificat éditeur. Voir ROADMAP.md pour les fonctions et contrôles à venir.
