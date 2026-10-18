# Feuille de route

Les cases cochées décrivent des fonctions disponibles. Les autres sont des
travaux prévus, sans promesse de date.

## 0.1 — première publication

- [x] Édition native de formes, textes et graphiques.
- [x] Connecteurs attachés ou libres, ports et points de passage.
- [x] Multipage, formats, tailles personnalisées, bibliothèque et galerie locale.
- [x] Styles, SVG/PDF, gestes annulables, alignement et trackpad.
- [x] Moteur partagé, application native, CLI console et serveur MCP.
- [x] Sources MIT, provenance des ressources, notices par cible et contrôles de licence.
- [x] CI et releases versionnées avec SHA-256.

## Prochaine étape — stabilité et prise en main

- [ ] Tests utilisateurs sur MacBook 13 pouces, clavier seul et lecteurs d’écran.
  Critère : parcours créer → relier → enregistrer → rouvrir → exporter sans blocage.
- [x] Tableau de raccourcis et recherche de commandes (0.2.0) : palette ⌘K / Ctrl+K,
  recherche sans accents, navigation clavier et aide filtrable avec F1.
- [ ] Ajustement automatique du bloc au texte, avec rendu cohérent dans les exports.
- [ ] Sauvegarde de récupération après fermeture anormale, sans écraser le dessin source.
- [ ] Tests d’intégration CLI/MCP sur les trois systèmes et diagnostics plus lisibles.

## Diagrammes plus riches

- [ ] Groupes persistants, verrouillage, couches et rotation.
- [ ] Routage évitant les obstacles, avec stabilité pendant le déplacement des blocs.
- [ ] Import d’images, cadrage et export PNG avec dimensions explicites.
- [ ] Bibliothèques métier originales avec provenance et critères de contribution.
- [ ] Import/export Mermaid et meilleure interopérabilité SVG.

## Distribution

- [ ] Signature Developer ID et notarisation Apple, après configuration des certificats.
- [ ] Icône et métadonnées natives Windows, installateur et signature éditeur.
- [ ] Intégration Linux : fichier desktop, MIME, AppImage, puis deb/rpm si demandés.
- [ ] Linux ARM64 et Windows ARM64 après disponibilité de tests dédiés.
- [ ] Vérification de mise à jour facultative, sans télémétrie.

## Qualité à maintenir

- [ ] Budgets de performance et corpus de grands documents synthétiques.
- [ ] Fuzzing des fichiers et patches JSON, limites de ressources et tests de corruption.
- [ ] Mesures d’accessibilité et tests de contraste, focus et navigation clavier.
- [ ] Revue régulière des avis de sécurité, licences et ressources ajoutées.

- [ ] Résoudre les cinq avis de maintenance transitifs documentés dans [docs/maintenance.md](docs/maintenance.md), puis retirer les exceptions ciblées de deny.toml.
