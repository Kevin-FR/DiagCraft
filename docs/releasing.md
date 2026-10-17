# Publier une version

La version est définie une seule fois dans `[workspace.package]` de Cargo.toml.
Les trois packages l’héritent. Ne pas modifier la version du format ou de l’API
lors d’une simple publication de l’application.

1. Modifier la version et la dépendance workspace `diagcraft`, actualiser Cargo.lock.
2. Écrire CHANGELOG.md et `docs/releases/<version>.md`.
3. Passer formatage, clippy, tests, audit des licences, avis de sécurité et contrôle de publication.
4. Commiter sur main, créer le tag `v<version>` et le pousser.
5. GitHub Actions vérifie la correspondance tag/manifest, construit les trois
   plateformes et produit un **brouillon** de release avec notices et SHA-256.
6. Télécharger, vérifier et tester les paquets, puis publier le brouillon.

La release Mac est arm64 uniquement. Windows et Linux sont x86_64. Chaque paquet
contient aussi la CLI console. La CLI Mac a en plus un ZIP séparé. Les notices
sont collectées pour la cible depuis Cargo ; un texte manquant bloque le paquet.

Les actions sont fixées à des SHA. Seul le job de release peut écrire dans le
dépôt ; les builds utilisent des permissions de lecture. Une release publique
existante n’est jamais remplacée : une correction exige un nouveau numéro.
Les premiers paquets sont non notariés ou sans certificat éditeur ; la signature
et notarisation de production sont des tâches de la feuille de route.
