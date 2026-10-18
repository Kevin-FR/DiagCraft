# Développement

Rust 1.94.1 est fixé dans rust-toolchain.toml. Python 3.11+ sert au packaging.

```sh
cargo run --locked -p diagcraft
cargo run --locked -p diagcraft-cli -- --agent catalog
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --release --workspace
```

Sur Ubuntu/Debian :

```sh
sudo apt-get update
sudo apt-get install -y libxkbcommon-dev libwayland-dev libegl1-mesa-dev libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libxkbcommon-x11-0
```

Installer le contrôleur de licences :

```sh
cargo install cargo-deny --version 0.20.2 --locked
cargo deny check licenses sources advisories
```

Les vérifications Python n’utilisent que la bibliothèque standard :

```sh
python3 scripts/check-publication.py
python3 scripts/smoke.py --bin target/release/diagcraft-cli
```

Sur macOS, compiler avec `MACOSX_DEPLOYMENT_TARGET=13.0`, puis
`python3 scripts/package.py`. La release Mac est uniquement Apple Silicon.
Sur Linux amd64, `python3 scripts/package.py` crée le tar.gz puis le `.deb`.
Installer `dpkg-dev`, `desktop-file-utils` et `shared-mime-info` pour le packaging.
Les dépendances ELF sont calculées par `dpkg-shlibdeps` ; les bibliothèques graphiques
chargées dynamiquement sont déclarées explicitement. La CI valide le fichier desktop,
installe le `.deb`, vérifie ses permissions, empreintes et notices, teste la CLI installée
et une capture native sous Xvfb, puis le désinstalle.

Voir [la politique de versions](versioning.md) avant de créer une release.
Les fonctions de capture lancent une fenêtre isolée et ne sauvegardent pas
la bibliothèque personnelle : `target/release/diagcraft --screenshot capture.png compact`.

Le format `.draw` v7, l’API agents v1 et leurs migrations doivent rester testés.
Les exemples utilisent des données de démonstration, sans documents personnels.

Les catalogues sont contrôlés avec `python3 scripts/check-locales.py`. Les tests
de packaging sont exécutables sur tous les systèmes avec
`python3 -m unittest discover -s packaging -p 'test_*.py'`. La vérification réseau
réelle reste explicite : `cargo test -p diagcraft github_live_download_is_verified
-- --ignored`. Elle contacte GitHub et télécharge un paquet dans un dossier temporaire.
