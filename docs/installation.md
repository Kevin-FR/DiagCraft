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

Décompresser l’archive et lancer `./diagcraft`. La CLI est `./diagcraft-cli`.
Le paquet x86_64 est construit sur Ubuntu 22.04 et utilise les bibliothèques
graphiques et pilotes du système. X11 ou Wayland, EGL et xkbcommon sont requis.
Sur Ubuntu : `sudo apt install libxkbcommon0 libegl1 libgl1 libwayland-client0 libx11-6`.
La création d’un raccourci desktop et les paquets AppImage/deb/rpm sont prévus.

## Données

Les fichiers `.draw` sont des documents JSON éditables. Les préférences et
versions de la galerie restent locales ; aucun service cloud n’est requis.
Le format de document v7 est indépendant du numéro de l’application.

Pour vérifier un téléchargement sur macOS : `shasum -a 256 <archive>` ;
sur Linux : `sha256sum <archive>` ; sous PowerShell :
`Get-FileHash <archive> -Algorithm SHA256`. Comparer au fichier SHA256SUMS.txt.
