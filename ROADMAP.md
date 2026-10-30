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
| **0.12.1 — livrée** | Mini-carte facultative et navigation sur les grandes feuilles. | [#10](https://github.com/Kevin-FR/DiagCraft/issues/10) |
| **0.13.1 — livrée** | Mise en page automatique avec aperçu, insertion directe sur connecteur et accueil localisé. | [Guide](docs/mise-en-page.md) |
| **1.0.0 — suivant** | Parcours utilisateurs validés, accessibilité, performances et distribution fiable. | [#5](https://github.com/Kevin-FR/DiagCraft/issues/5) |

### Lot 0.12.1 — mini-carte

- [x] Vue simplifiée de la page active et de ses objets visibles, avec repère de sélection et cadre de la caméra.
- [x] Navigation au clic, au glissement et au clavier, sans modifier le document ni l’historique.
- [x] Ouverture permanente près du zoom, commande Affichage/palette, fermeture au clavier et préférence locale.
- [x] Tests de petites fenêtres et sept langues, pages extrêmes, masquage des calques, gestes et relâchements sans effet derrière la carte.

Voir [le guide de navigation](docs/minimap.md). Ce lot n’achève pas les essais
utilisateurs ni la validation avec lecteurs d’écran du jalon 1.0.

### Lot 0.11.3 — lisibilité des textes

- [x] Inventaire des rôles, tailles et fonds ; textes informatifs, actions et erreurs séparés des décorations.
- [x] Contrastes contrôlés sur les glyphes et fonds réels, recherche et états de sélection/focus, dans sept langues à 1200 × 680 points.
- [x] Aides de connexion visibles hors page et accueil lisible sur fonds personnalisés, sans modifier les dessins.
- [ ] Parcours réel sur petit écran et lecteur d’écran ; revue humaine des contrastes et transitions ([#9](https://github.com/Kevin-FR/DiagCraft/issues/9)).

Voir [les critères de lisibilité et les limites des tests](docs/readability.md).

### Lot 0.11.2 — fenêtres courtes et clavier

- [x] Fenêtres de 1200 × 680 points, contrôles principaux atteignables et préférences des volets conservées ([#6](https://github.com/Kevin-FR/DiagCraft/issues/6)).
- [x] Cartes de bibliothèque révélées au clavier, focus visible et noms accessibles distincts/traduits ([#7](https://github.com/Kevin-FR/DiagCraft/issues/7)).
- [x] Zone d’origine conservée pour les commandes de focus exécutées depuis la palette ([#8](https://github.com/Kevin-FR/DiagCraft/issues/8)).

Ce lot prépare le jalon 1.0 ; les essais humains sur petits écrans et avec lecteurs
d’écran restent à effectuer.

### Critères du lot 0.8.0

- [x] Grouper et dégrouper une sélection ; déplacer et dupliquer un groupe entier.
- [x] Verrouiller les objets tout en gardant l’inspection, la copie et le déverrouillage accessibles.
- [x] Créer, renommer, réordonner, masquer et verrouiller des calques ; y affecter les objets.
- [x] Respecter visibilité, ordre et protection sur la feuille, dans les exports et dans les outils agents.
- [x] Conserver les états dans les documents multipages, le presse-papiers, l’historique et la récupération.
- [x] Tester les anciens fichiers et annoncer les limites de lecture dans les versions précédentes.
- [x] Traduire les commandes dans les sept langues et vérifier les paquets sur les trois systèmes.

### Lots de développement proposés

Ces lots sont ordonnés par priorité ; leur numéro de version sera fixé à la
publication. Ils accompagnent les validations 1.0, qui restent nécessaires.

1. **Insertion directe sur connecteur** : choisir une forme, repérer la liaison
   au survol et cliquer pour insérer une étape ; conserver les extrémités,
   le style, les libellés et points manuels, avec une seule annulation.
2. **Mise en page automatique** : flux horizontal/vertical et organigrammes,
   aperçu avant application, conservation des blocs verrouillés et positions
   imposées, sélection ou page entière, annulation atomique.
3. **Navigation dans les grands documents** : arborescence pages → calques →
   groupes → objets, recherche et sélection au clavier ; liens entre pages
   activables avec retour à la page précédente.
4. **Conteneurs et couloirs de processus** : regroupement par équipe/service,
   déplacement du contenu avec le conteneur, limites et connexions cohérentes.
5. **Styles nommés du document** : rôles Action/Décision/Alerte, définition
   commune et propagation aux occurrences, surcharges locales explicites.
6. **Composants réutilisables liés** : définition commune, instances, mise à jour
   contrôlée et possibilité de détacher une instance.
7. **Comparaison visuelle des modifications IA** : distinguer ajouts,
   suppressions, déplacements et changements de texte avant validation du lot ;
   révision vérifiée et une seule annulation après application.

Pour chaque lot : contrôles clavier et noms accessibles, traductions dans les
sept langues, tests moteur/interface et cohérence des exports. Les changements
éventuels de format documentaire ou d’API agents devront être explicitement
versionnés avec migration ; aucun numéro n’est promis ici.

- [x] Insertion directe sur connecteur et repère de survol (0.13.1).
- [x] Première mise en page automatique des blocs avec aperçu et annulation (0.13.1). Voir [le périmètre et les limites](docs/mise-en-page.md).
- [x] Espacements indépendants entre niveaux et entre blocs, préréglages Compact/Normal/Aéré et aperçu recalculé (0.13.1).
- [x] Réduire les croisements estimés entre niveaux voisins : balayages bornés, meilleur ordre retenu et score dans l’aperçu (0.13.1).
- [ ] Affiner la mise en page : déplacement des groupes comme unités et prise en compte des routes réelles/liaisons longues dans l’optimisation.
- [x] Première arborescence : page active → calques → groupes → objets, sélecteur de page, filtre et navigation clavier (branche de développement, non publié).
- [x] Retrouver la sélection et effacer le filtre dans l’arborescence (branche de développement, non publié).
- [x] Recherche explicite de structure sur toutes les pages, limitée aux 300 premiers résultats (branche de développement, non publié).
- [ ] Mesurer la latence de la recherche multipage sur de grands documents et déplacer son calcul hors de l’interface si nécessaire.
- [ ] Liens de navigation entre pages.
- [ ] Conteneurs et couloirs de processus.
- [ ] Styles nommés avec propagation.
- [ ] Composants réutilisables liés.
- [ ] Comparaison visuelle des modifications IA.
- [x] Mini-carte de navigation pour les grandes feuilles (0.12.1).

La préparation 1.0 garde ses critères : essais sur MacBook 13 pouces, clavier
seul et lecteurs d’écran, fluidité et latence mesurées, relecture des langues,
robustesse des fichiers et distribution signée. Les certificats éditeur restent
à configurer pour les signatures et l’installation intégrée des mises à jour.

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

- [x] Navigation F6 entre zones, focus visible, continuité des boutons de volet et noms accessibles traduits ; parcours simulés à 1200 × 760 dans sept langues (0.11.1). Voir [le guide clavier](docs/keyboard-navigation.md).

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
- [x] Localisation du nouveau dessin d’accueil et de son modèle dans les sept langues (0.13.1).
- [ ] Relecture des traductions par des locuteurs natifs et localisation des autres contenus des modèles.
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
- [x] Contrôle Wayland instrumenté, régressions de détection et conservation des captures en trois sessions CI (0.11.1).
- [ ] Signature Developer ID et notarisation Apple, après configuration des certificats.
- [ ] Icône et métadonnées natives Windows, installateur et signature éditeur.
- [x] Intégration Linux : fichier desktop, icône, MIME et paquet Debian amd64 (0.2.1).
- [ ] Paquets AppImage et RPM.
- [ ] Linux ARM64 et Windows ARM64 après disponibilité de tests dédiés.
- [x] Vérification GitHub facultative, choix du canal et téléchargement vérifié SHA-256 (0.3.0).
- [x] Paquets Mac et dépendances embarquées contrôlés `arm64`, sans Rosetta (0.3.0).
- [ ] Installation intégrée des mises à jour avec signature éditeur et reprise après échec.

## Qualité à maintenir

- [x] Budgets initiaux du moteur et corpus synthétiques (0.11.1) : 1 000 blocs, cinq pages et 12 mégapixels ; mesures et seuils en CI. Voir [la qualité](docs/quality.md).
- [ ] Mesures de cadence du canevas et latence des gestes sur petits écrans.
- [x] Corpus déterministe de corruption des documents, patches et Mermaid, avec contrôle des sources et de l’atomicité (0.11.1).
- [ ] Campagnes prolongées de fuzzing des fichiers et patches JSON.
- [ ] Mesures d’accessibilité et tests de contraste, focus et navigation clavier.
- [ ] Revue ciblée du contraste des textes informatifs, statuts et aides, avec critères par rôle ([#9](https://github.com/Kevin-FR/DiagCraft/issues/9)).
- [ ] Revue régulière des avis de sécurité, licences et ressources ajoutées.

- [ ] Résoudre les cinq avis de maintenance transitifs documentés dans [docs/maintenance.md](docs/maintenance.md), puis retirer les exceptions ciblées de deny.toml.
