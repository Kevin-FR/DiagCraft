# Versions et publication

DiagCraft suit `MAJEUR.MINEUR.CORRECTIF` (Semantic Versioning). Avant 1.0,
l’application est expérimentale ; les changements incompatibles sont annoncés
explicitement dans les notes de version.

- **0.2.1, 0.2.2…** : corrections, ajustements UX, documentation et packaging
  compatibles avec la série 0.2.
- **0.3.0, 0.4.0…** : nouvelles capacités importantes ou changement incompatible.
- **1.0.0** : première série stable, après validation des parcours de la feuille de route.

0.1.0 et 0.2.0 ont déjà été publiées. Leurs tags et fichiers restent immuables :
on ne recommence pas la numérotation et on ne remplace pas un téléchargement.

La version de référence est `workspace.package.version` dans `Cargo.toml`.
Les trois packages, `Cargo.lock`, le tag `v<version>` et
`docs/releases/<version>.md` doivent correspondre. Le format `.draw` **v7** et
l’API agents **v1** restent indépendants.

## Procédure

1. Réaliser les contrôles d’AGENTS.md et mettre à jour CHANGELOG.md, ROADMAP.md
   et les notes de version. Les cases cochées doivent décrire du travail terminé.
2. Mettre à jour la version du workspace et la contrainte de la dépendance interne,
   puis régénérer Cargo.lock sans mise à jour des dépendances externes.
3. Publier un nouveau tag annoté, par exemple `v0.2.1`.
4. La CI construit macOS arm64, Windows x86_64 et Linux x86_64. Elle installe,
   vérifie puis désinstalle aussi le paquet Debian amd64.
5. Le job de release exige les cinq paquets attendus, calcule `SHA256SUMS.txt`
   et crée un brouillon seulement après réussite des compilations et audits.
6. Vérifier les paquets, publier le brouillon avec son statut expérimental,
   puis vérifier l’accès public aux téléchargements.

Le paquet Debian utilise le même numéro (`diagcraft_0.2.1_amd64.deb`). Une correction
de packaging distribuée fait l’objet d’une nouvelle version de l’application,
ce qui conserve une correspondance unique entre sources et téléchargements.
