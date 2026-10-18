# Changelog

Les versions de l’application suivent Semantic Versioning. Le format `.draw`
et l’API agents sont versionnés indépendamment.

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
