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

- [x] Une seule commande par volet au niveau de la feuille, position stable fermé/ouvert et propriétés de document compactes (0.7.1).

- [x] Recherche des objets de la page par texte, type et ID, sélection additive et navigation vers les objets hors écran (0.7.0).
- [x] Parcours de création, saisie et connexion de blocs au clavier, avec placement dans la vue et focus de saisie préservé (0.7.0).
- [ ] Tests utilisateurs sur MacBook 13 pouces, clavier seul et lecteurs d’écran.
  Critère : parcours créer → relier → enregistrer → rouvrir → exporter sans blocage.
- [x] Tableau de raccourcis et recherche de commandes (0.2.0) : palette ⌘K / Ctrl+K,
  recherche sans accents, navigation clavier et aide filtrable avec F1.
- [x] Ruban compact par onglet, volets homogènes et redimensionnables, préférences
  locales et recherche dans les bibliothèques masquées (0.2.1).
- [x] Ajustement automatique du bloc au texte, avec rendu cohérent dans les exports (0.5.0).
  Mesure des polices embarquées, retours à la ligne partagés, commande de sélection et option automatique.
- [x] Saisie directement dans les blocs, curseur au double-clic et barre flottante compacte (0.6.1).
  Validation au clic extérieur, F2/Entrée, annulation, sauvegarde et composition Unicode préservée.
- [x] Sauvegarde de récupération après fermeture anormale, sans écraser le dessin source (0.4.0).
  Copie locale des onglets modifiés, reprise dans de nouveaux onglets et verrous entre instances.
- [ ] Tests d’intégration CLI/MCP sur les trois systèmes et diagnostics plus lisibles.

## Internationalisation

- [x] Interface en anglais, français, allemand, espagnol, italien, portugais et chinois simplifié (0.3.0).
- [x] Police chinoise embarquée sous OFL pour l’interface et les exports PDF.
- [ ] Relecture des traductions par des locuteurs natifs et localisation des contenus des modèles.
- [ ] Chinois traditionnel et autres langues selon les contributions.

## Diagrammes plus riches

- [x] Texte enrichi par portions dans les blocs (0.6.0) : sélection de mots, tailles et couleurs,
  édition annulable, exports vectoriels et contrat agents Unicode.

- [ ] Groupes persistants, verrouillage, couches et rotation.
- [ ] Routage évitant les obstacles, avec stabilité pendant le déplacement des blocs.
- [ ] Import d’images, cadrage et export PNG avec dimensions explicites.
- [ ] Bibliothèques métier originales avec provenance et critères de contribution.
- [ ] Import/export Mermaid et meilleure interopérabilité SVG.

## Distribution

- [ ] Signature Developer ID et notarisation Apple, après configuration des certificats.
- [ ] Icône et métadonnées natives Windows, installateur et signature éditeur.
- [x] Intégration Linux : fichier desktop, icône, MIME et paquet Debian amd64 (0.2.1).
- [ ] Paquets AppImage et RPM.
- [ ] Linux ARM64 et Windows ARM64 après disponibilité de tests dédiés.
- [x] Vérification GitHub facultative, choix du canal et téléchargement vérifié SHA-256 (0.3.0).
- [x] Paquets Mac et dépendances embarquées contrôlés `arm64`, sans Rosetta (0.3.0).
- [ ] Installation intégrée des mises à jour avec signature éditeur et reprise après échec.

## Qualité à maintenir

- [ ] Budgets de performance et corpus de grands documents synthétiques.
- [ ] Fuzzing des fichiers et patches JSON, limites de ressources et tests de corruption.
- [ ] Mesures d’accessibilité et tests de contraste, focus et navigation clavier.
- [ ] Revue régulière des avis de sécurité, licences et ressources ajoutées.

- [ ] Résoudre les cinq avis de maintenance transitifs documentés dans [docs/maintenance.md](docs/maintenance.md), puis retirer les exceptions ciblées de deny.toml.
