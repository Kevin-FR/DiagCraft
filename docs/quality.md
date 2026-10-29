# Préparation de la 1.0 : qualité mesurable

Ces outils sont disponibles depuis la 0.11.1. Ils ne déclarent
pas DiagCraft stable et ne remplacent pas les essais avec de vrais utilisateurs.
Les sources des corpus sont originales, sous MIT, sans dessin ni image externe.

## Fenêtre Ubuntu/Wayland

La CI installe le `.deb` puis lance trois sessions isolées de Weston, avec rendu
Mesa logiciel. Le test examine la fenêtre composée, y compris son titre et ses
boutons natifs. Une capture interne du canevas ne suffirait pas à les vérifier.

Le détecteur cherche une bande claire et neutre au bord supérieur de la fenêtre,
avec les mêmes seuils de pixels sombres pour le titre et les boutons. Il couvre
les fenêtres placées en haut de l’écran et les tons légèrement assombris pendant
l’animation d’apparition. Le bord de bureau au-dessus de la bande doit être visible ;
une ligne du ruban au milieu de l’application ne constitue pas une barre de titre.

Neuf tests synthétiques vérifient les positions, les couleurs et le rejet des
cadres sans titre, sans boutons, trop étroits ou utilisant l’ancien cadre sombre.
Les trois sessions doivent réussir ; elles sont toutes examinées, même après un
premier échec. Les captures CI ont révélé que le précédent détecteur pouvait reconnaître une
ligne du ruban pendant l’animation : les nouveaux rapports exposent les coordonnées
et les comptages exacts. La cause de l’échec historique sans capture conservée
ne peut pas être prouvée rétrospectivement.

L’artifact **`native-window-linux`**, conservé sept jours, contient les PNG,
les traces Wayland/Weston et `window-report.json` pour chaque session, même après
un échec. Le rapport indique le résultat, la position détectée, la couleur,
les pixels du titre/boutons et le temps écoulé. La comparaison avec l’ancien
détecteur sert au diagnostic ; elle n’autorise jamais la réussite du nouveau test.

Le contrôle concerne le bureau clair de test. Les thèmes sombres, l’accessibilité
réelle, X11 et les bureaux des utilisateurs restent à examiner. Les artifacts
techniques sont exclus de la collecte des paquets de release.

## Budgets initiaux du moteur

```sh
mkdir -p output
cargo run --locked --release -p diagcraft-core --example quality_report -- \
  --output output/quality.json --enforce
```

Aucune bibliothèque de benchmark n’est ajoutée. Chaque mesure utilise un
échauffement exclu puis trois échantillons ; la **médiane** est comparée au budget.
Le rapport JSON conserve aussi tous les échantillons, leur maximum, le système,
l’architecture, le profil, la version et le SHA source CI lorsqu’il est disponible.
Chaque corpus a une empreinte canonique SHA-256 et un nombre d’objets/pages.

| Corpus | Opération | Budget médian |
| --- | --- | --- |
| 1 000 blocs, 975 liens | JSON canonique | 100 ms |
| Même corpus | Charger et valider | 250 ms |
| Même corpus | Inspection, textes et routes | 2 000 ms |
| Même corpus | Export SVG | 1 500 ms |
| Même corpus | Export PNG 800 × 450 | 6 000 ms |
| 5 pages, 1 000 blocs, 950 liens | Charger et valider | 250 ms |
| Même corpus | Inspection des cinq pages | 2 000 ms |
| 3 images de 4 mégapixels | Charger et valider | 1 000 ms |
| Même corpus | Export PNG 800 × 450 | 2 000 ms |

Les images sont des aplats synthétiques avec transparence et recadrage. Leur
compression est volontairement simple ; ce corpus ne représente pas tous les
JPEG/PNG photographiques. Les temps excluent la génération des corpus et les
suppressions des valeurs retournées. Le chargement inclut lecture, décodage et
validation ; les fichiers temporaires sont locaux, jamais des dessins utilisateur.

La CI applique ces budgets en profil **release** sur Mac ARM64, Ubuntu x86_64 et
Windows x86_64. Les artifacts **`quality-<plateforme>`** conservent JSON et journal
pendant quatorze jours. Un dépassement échoue sans masquer les mesures ; analyser
le rapport et les ressources du runner avant toute modification des budgets.

Ces seuils sont des garde-fous initiaux de régression. Ils ne mesurent pas la
cadence du canevas, le GPU, le démarrage de l’application ou la latence d’un geste.
Ces scénarios et les mesures sur MacBook 13 pouces restent dans le jalon 1.0.

## Corpus de corruption déterministe

`crates/core/tests/quality_corpus.rs` applique des troncatures et 64 mutations
reproductibles aux documents JSON, patches et flowcharts Mermaid. Un cas doit
être rejeté proprement ou produire un document valide, sans panique, modification
de la source ou changement partiel du document courant.

Ce corpus complète les tests de limites d’images, de ressources et de révisions.
Il ne remplace pas une campagne prolongée avec un moteur de fuzzing.

## Essais utilisateurs restant à organiser

Le [guide clavier](keyboard-navigation.md) décrit les accès F6, les noms accessibles
et les scénarios automatisés. Ils ne remplacent pas les annonces réelles des lecteurs d’écran.

Sur MacBook 13 pouces, Ubuntu X11/Wayland et Windows : créer deux blocs, saisir
et formater leur texte, les relier, déplacer la vue, enregistrer, rouvrir et exporter.
Rejouer le parcours au clavier et avec un lecteur d’écran. Relever le système,
la version de DiagCraft, l’action, le résultat attendu et ce qui a bloqué ; joindre
un dessin synthétique et une capture uniquement s’ils sont nécessaires au constat.
Les relectures de traductions et les signatures éditeur restent aussi à réaliser.
