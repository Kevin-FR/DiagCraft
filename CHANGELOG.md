# Changelog

Les versions de l’application suivent Semantic Versioning. Le format `.draw`
et l’API agents sont versionnés indépendamment.

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
