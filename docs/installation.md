# Installation

Télécharger le paquet de votre système et `SHA256SUMS.txt` depuis
https://github.com/Kevin-FR/DiagCraft/releases

## macOS

Mac Apple Silicon (M1 ou ultérieur), macOS 13 minimum. Décompresser
`diagcraft-<version>-macos-arm64.zip` et déplacer `DiagCraft.app` dans Applications.
Le paquet contient uniquement arm64. La CLI est dans
`DiagCraft.app/Contents/MacOS/diagcraft-cli` et dans un ZIP séparé.

Cette première version est signée ad hoc, sans notarisation Apple. Si macOS
bloque le lancement, suivre les indications d’Apple et utiliser l’autorisation
pour cette application dans Réglages Système → Confidentialité et sécurité :
https://support.apple.com/fr-fr/guide/mac-help/mh40616/mac

Aucune désactivation globale des protections n’est nécessaire.

## Windows

Décompresser le ZIP portable dans un dossier accessible en écriture et lancer
`diagcraft.exe`. `diagcraft-cli.exe` est l’exécutable console pour les exports
et les agents. Les premiers paquets n’ont pas de signature éditeur Windows.

## Linux

Sur Debian/Ubuntu amd64, télécharger `diagcraft_<version>_amd64.deb`, puis :

```sh
sudo apt install ./diagcraft_<version>_amd64.deb
```

Remplacer `<version>` par le numéro téléchargé. L’application apparaît dans le
menu Graphisme ; `diagcraft` et `diagcraft-cli` sont dans le PATH. Les fichiers
`.draw` sont proposés à l’ouverture avec DiagCraft ; la préférence de votre
application par défaut n’est pas remplacée. Les notices et guides sont dans
`/usr/share/doc/diagcraft`. Pour désinstaller : `sudo apt remove diagcraft`.
Vos dessins et données locales sont conservés.

L’archive `.tar.gz` reste disponible : la décompresser et lancer `./diagcraft`.
La CLI est `./diagcraft-cli`.
Le paquet x86_64 est construit sur Ubuntu 22.04 et utilise les bibliothèques
graphiques et pilotes du système. X11 ou Wayland, EGL et xkbcommon sont requis.
Pour l’archive portable, installer les bibliothèques chargées au lancement :

```sh
sudo apt install libxkbcommon0 libxkbcommon-x11-0 libegl1 libgl1 libx11-6 libx11-xcb1 libxcursor1 libxi6 libxrandr2 libwayland-client0 libwayland-cursor0 libwayland-egl1
```
Le paquet Debian installe ces dépendances automatiquement. AppImage et RPM restent prévus.

## Données

Les fichiers `.draw` sont des documents JSON éditables. Les préférences et
versions de la galerie restent locales ; aucun service cloud n’est requis.
Le format de document v7 est indépendant du numéro de l’application.

Pour vérifier un téléchargement sur macOS : `shasum -a 256 <archive>` ;
sur Linux : `sha256sum <archive>` ; sous PowerShell :
`Get-FileHash <archive> -Algorithm SHA256`. Comparer au fichier SHA256SUMS.txt.

## Apple Silicon et mises à jour

Le paquet Mac contient uniquement des exécutables et bibliothèques `arm64`.
L’audit `Native-Architecture.json` dans les ressources de l’app liste les fichiers
Mach-O et leurs dépendances système. Rosetta n’est pas nécessaire. Le packaging
refuse les dépendances Intel, Homebrew externes et chemins non résolus.

La langue et la vérification GitHub se règlent dans Fichier → Langue et paramètres…
Voir [le guide](langues-et-mises-a-jour.md). La vérification automatique est
facultative ; l’installation des paquets reste manuelle. Pour l’archive Linux,
installer aussi `curl` et `ca-certificates` afin d’utiliser la vérification intégrée.
