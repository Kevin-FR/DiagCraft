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
sudo apt-get install -y libxkbcommon-dev libwayland-dev libegl1-mesa-dev libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev
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
Les fonctions de capture lancent une fenêtre isolée et ne sauvegardent pas
la bibliothèque personnelle : `target/release/diagcraft --screenshot capture.png compact`.

Le format `.draw` v7, l’API agents v1 et leurs migrations doivent rester testés.
Les exemples utilisent des données de démonstration, sans documents personnels.
