# Feuille de route

Les cases cochées décrivent des fonctions disponibles. Les autres sont des
travaux prévus, sans promesse de date.

## Jalons et prochaines versions

Les jalons ci-dessous sont ordonnés par priorité. Chaque lot doit être utilisable,
documenté et testé avant publication ; les correctifs utilisent une version patch.
Les critères détaillés et l’avancement sont suivis dans les issues GitHub.

| Jalon | Objectif | Suivi |
| --- | --- | --- |
| **0.8.0 — livrée** | Groupes persistants, verrouillage et calques visibles ou masqués. | [#1](https://github.com/Kevin-FR/DiagCraft/issues/1) |
| **0.9.0 — livrée** | Connecteurs évitant les obstacles, avec parcours stables et respect des points manuels. | [#2](https://github.com/Kevin-FR/DiagCraft/issues/2) |
| **0.10.0 — livrée** | Images embarquées, recadrage, collage et export PNG configurable. | [#3](https://github.com/Kevin-FR/DiagCraft/issues/3) |
| **0.11.0 — livrée** | Interopérabilité Mermaid et diagnostics CLI/MCP plus compréhensibles. | [#4](https://github.com/Kevin-FR/DiagCraft/issues/4) |
| **1.0.0 — suivant** | Parcours utilisateurs validés, accessibilité, performances et distribution fiable. | [#5](https://github.com/Kevin-FR/DiagCraft/issues/5) |

### Critères du lot 0.8.0

- [x] Grouper et dégrouper une sélection ; déplacer et dupliquer un groupe entier.
- [x] Verrouiller les objets tout en gardant l’inspection, la copie et le déverrouillage accessibles.
- [x] Créer, renommer, réordonner, masquer et verrouiller des calques ; y affecter les objets.
- [x] Respecter visibilité, ordre et protection sur la feuille, dans les exports et dans les outils agents.
- [x] Conserver les états dans les documents multipages, le presse-papiers, l’historique et la récupération.
- [x] Tester les anciens fichiers et annoncer les limites de lecture dans les versions précédentes.
- [x] Traduire les commandes dans les sept langues et vérifier les paquets sur les trois systèmes.

### Fonctionnalités à explorer ensuite

- [ ] Mise en page automatique d’organigrammes et de flux, avec aperçu et annulation.
- [ ] Composants réutilisables liés : mise à jour des instances depuis une définition commune.
- [ ] Mini-carte de navigation pour les grandes feuilles.

Ces explorations n’ont pas encore de numéro de version attribué. Elles complètent
les jalons ci-dessus sans retarder les corrections et les tests utilisateurs.

## 0.1 — première publication

- [x] Édition native de formes, textes et graphiques.
- [x] Connecteurs attachés ou libres, ports et points de passage.
- [x] Multipage, formats, tailles personnalisées, bibliothèque et galerie locale.
- [x] Styles, SVG/PDF, gestes annulables, alignement et trackpad.
- [x] Moteur partagé, application native, CLI console et serveur MCP.
- [x] Sources MIT, provenance des ressources, notices par cible et contrôles de licence.
- [x] CI et releases versionnées avec SHA-256.

## Prochaine étape — stabilité et prise en main

- [x] Placement précis au clavier : translation commune, geste continu annulable, indicateur et commandes dans l’aide (0.7.2).

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
- [x] Tests d’intégration CLI/MCP sur les trois systèmes : migrations, SVG/PDF, dry-run et patches identiques en CI.
- [x] Diagnostics CLI/MCP plus lisibles, avec page, objet, opération et piste de correction (0.11.0).

## Internationalisation

- [x] Interface en anglais, français, allemand, espagnol, italien, portugais et chinois simplifié (0.3.0).
- [x] Police chinoise embarquée sous OFL pour l’interface et les exports PDF.
- [ ] Relecture des traductions par des locuteurs natifs et localisation des contenus des modèles.
- [ ] Chinois traditionnel et autres langues selon les contributions.

## Diagrammes plus riches

- [x] Texte enrichi par portions dans les blocs (0.6.0) : sélection de mots, tailles et couleurs,
  édition annulable, exports vectoriels et contrat agents Unicode.

- [x] Groupes persistants, verrouillage et calques (0.8.0).
- [ ] Rotation des objets, avec géométrie cohérente des liens et des exports.
- [x] Routage évitant les obstacles, entre points manuels conservés, avec voies stables et calcul borné (0.9.0).
- [x] Images PNG/JPEG embarquées, recadrage non destructif et export PNG page/sélection avec dimensions, DPI et transparence (0.10.0).
- [ ] Bibliothèques métier originales avec provenance et critères de contribution.
- [x] Import/export d’un sous-ensemble flowchart Mermaid, aperçu et confirmation des pertes (0.11.0).
- [ ] Meilleure interopérabilité SVG.

## Distribution

- [x] Décorations Ubuntu/Wayland adaptées au bureau, sans cadre de secours sombre, avec contrôle de fenêtre complète en CI (0.7.3).
- [x] Contrôle Wayland instrumenté, régressions de détection et conservation des captures en trois sessions CI (branche de développement).
- [ ] Signature Developer ID et notarisation Apple, après configuration des certificats.
- [ ] Icône et métadonnées natives Windows, installateur et signature éditeur.
- [x] Intégration Linux : fichier desktop, icône, MIME et paquet Debian amd64 (0.2.1).
- [ ] Paquets AppImage et RPM.
- [ ] Linux ARM64 et Windows ARM64 après disponibilité de tests dédiés.
- [x] Vérification GitHub facultative, choix du canal et téléchargement vérifié SHA-256 (0.3.0).
- [x] Paquets Mac et dépendances embarquées contrôlés `arm64`, sans Rosetta (0.3.0).
- [ ] Installation intégrée des mises à jour avec signature éditeur et reprise après échec.

## Qualité à maintenir

- [x] Budgets initiaux du moteur et corpus synthétiques (branche de développement) : 1 000 blocs, cinq pages et 12 mégapixels ; mesures et seuils en CI. Voir [la qualité](docs/quality.md).
- [ ] Mesures de cadence du canevas et latence des gestes sur petits écrans.
- [x] Corpus déterministe de corruption des documents, patches et Mermaid, avec contrôle des sources et de l’atomicité (branche de développement).
- [ ] Campagnes prolongées de fuzzing des fichiers et patches JSON.
- [ ] Mesures d’accessibilité et tests de contraste, focus et navigation clavier.
- [ ] Revue régulière des avis de sécurité, licences et ressources ajoutées.

- [ ] Résoudre les cinq avis de maintenance transitifs documentés dans [docs/maintenance.md](docs/maintenance.md), puis retirer les exceptions ciblées de deny.toml.
