# Sprint — Groupes mixtes et liaisons longues : revue du 8 novembre 2026

Contrat : [groupes-mixtes.md](groupes-mixtes.md).
Guide : [mise-en-page.md](mise-en-page.md).

## Périmètre réalisé sur la branche de développement

- Groupes contenant au moins un bloc autre qu’un texte : translation commune des
  blocs, graphiques, dessins libres et points manuels des connecteurs internes.
- Enveloppe incluant les graphiques et les points des lignes/courbes ; graphiques
  mobiles retirés des obstacles fixes. Les membres protégés ou masqués gardent
  le groupe fixe, comme les connecteurs membres externes ou à extrémité libre.
- Liaisons longues incluses dans le score : rangée de départ conservée sur les
  segments intermédiaires, arrivée réelle utilisée sur le dernier segment.
- Aperçu des dessins libres visibles, aides précisées dans sept langues,
  application en une annulation et aperçu périmé refusé.
- Format `.draw` v8 et API agents v1 préservés ; aucune dépendance ajoutée.

## Corrections apportées à la revue

1. Le raccourci d’enveloppe pour un seul nœud ne convenait pas à un groupe avec
   graphique/dessin. Il pouvait décaler le successeur à la deuxième application
   à cause des arrondis `f32`. Le raccourci exige maintenant une seule géométrie.
   Le test de coordonnées décimales reproduit l’échec avec l’ancienne condition.
2. La projection des liaisons longues ignorait leur arrivée. Le dernier segment
   rejoint maintenant la rangée d’arrivée : les arrivées communes ne produisent
   plus de faux croisements et deux liaisons longues inversées sont comptées.
3. Les dessins libres étaient absents de l’aperçu. Ils sont représentés par leurs
   points visibles, avec un test excluant les dessins sur un calque masqué.
4. Le texte d’aide limitait encore les groupes aux blocs/connecteurs. Il décrit
   désormais les graphiques et dessins libres dans les sept langues.
5. L’audit des dépendances signalait `async-executor` 1.14.0 retiré du registre.
   Le verrouillage utilise maintenant 1.14.1, sans mise à jour des autres paquets.

## Vérifications

- Tests moteur : groupes mixtes, courbes/libellés, sélection d’un graphique ou
  dessin, obstacles, enveloppe, protections, métadonnées et répétition stable.
- Score : oracle combinatoire exhaustif sur 512 graphes à deux niveaux et
  4 096 graphes à trois niveaux, avec liaisons longues ; score jamais dégradé
  par les balayages et résultat déterministe.
- Interface : aperçu/application/annulation des groupes mixtes dans les sept
  langues et dans les deux directions ; dessins visibles/masqués dans l’aperçu.
- Commandes de validation : `cargo fmt --all --check`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace`, `cargo build --release --locked --workspace`,
  `cargo deny check licenses sources advisories`,
  `python3 scripts/check-publication.py`, `python3 scripts/check-locales.py`,
  `python3 -m unittest discover -s packaging -p 'test_*.py'` et
  `python3 scripts/smoke.py --bin target/release/diagcraft-cli`.

## Limites et suite

Le score reste une estimation combinatoire. Les routes réelles, les obstacles
sur ces routes et les coudes/points manuels ne participent pas à l’optimisation.
Leur prise en compte reste ouverte dans la feuille de route. Les groupes sans
bloc autre qu’un texte restent fixes. `bend` et `label_offset` sont des valeurs
relatives : ils sont conservés lors de la translation des dessins.

Résultat local : build release réussi ; 680 tests Rust réussis, aucun échec,
3 tests explicitement ignorés ; Clippy sans avertissement ; 12 tests de
packaging et 9 tests du détecteur de fenêtre réussis ; locales et publication
valides. Le smoke CLI/MCP passe sur 14 fixtures de format, SVG/PDF/PNG et
patches. L’audit des dépendances passe avec un avertissement non bloquant sur
une autorisation de licence non utilisée. Les neuf budgets de performance
en profil release passent et la capture native du panneau a été contrôlée
visuellement.

Les contrôles locaux concernent macOS Apple Silicon. Les jobs CI ne démarrent
pas lors de la revue ; Windows/Linux restent donc à confirmer après
rétablissement de leur exécution. Les essais humains d’accessibilité, les mesures de latence
et la distribution signée restent ouverts. La suite fonctionnelle prioritaire
est définie dans [la feuille de route](../ROADMAP.md) : conteneurs/couloirs puis
composants liés, avec contrats et critères de validation avant implémentation.
