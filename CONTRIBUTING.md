# Contribuer à DiagCraft

Commencer par le README, ROADMAP.md et docs/architecture.md. Ouvrir une issue
pour un changement important ; une correction ciblée peut être proposée directement.

Les contributions doivent être originales ou accompagnées d’une licence
compatible, de leur attribution et d’une source vérifiable. Ne pas importer
de code, modèles, captures, polices ou icônes dont la redistribution n’est pas
établie. Conserver les notices amont et documenter les ressources générées.
Les contributions au projet sont proposées sous sa licence MIT.

Avant une pull request :

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo deny check licenses sources advisories
python3 scripts/check-publication.py
```

Expliquer le problème concret, le résultat attendu, les vérifications et les
limites restantes. Pour un changement d’interface, joindre une capture de
DiagCraft avec un document de démonstration sans données personnelles.
Ne pas modifier le numéro du format `.draw` ou de l’API agents sans migration
et tests de compatibilité. Ne pas commettre les résultats de compilation,
archives de distribution, certificats, journaux personnels ou fichiers de travail.

Les issues et pull requests peuvent être rédigées en français ou en anglais.

## README et traductions

[README.md](README.md) est la page d’accueil en anglais. Les six traductions sont
dans [docs/readme](docs/readme) et reliées par le sélecteur de langue en haut de chaque fichier.
Lorsqu’une information commune change (version, plateformes, fonctions ou commandes),
mettre à jour les sept versions ensemble. Conserver les commandes exécutables et
vérifier les liens relatifs avec `python3 scripts/check-publication.py`.
