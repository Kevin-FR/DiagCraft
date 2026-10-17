# Contrôles de licence et de publication

## Périmètre

Le code du projet est sous MIT. Les ressources redistribuées sont inventoriées
avec source, licence et empreinte. Les polices et dépendances gardent toutes
leurs notices propres ; elles ne sont pas renommées ou relicenciées.
Les scripts de paquet ne copient que les exécutables, ressources et documents
explicitement nécessaires. Les fichiers de travail ne font pas partie des sources publiques.

Les neuf modèles intégrés et les dessins d’exemple sont rédigés pour DiagCraft,
à partir de scénarios génériques : demande interne, parcours de projet,
algorithme, processus, recrutement, organisation, paiement, chronologie et
dossier multipage. Leur texte, leur disposition et leurs aperçus sont inclus
sous la licence MIT du projet. Les symboles géométriques sont dessinés par le
moteur ; aucune bibliothèque de modèles externe n’est embarquée.

## Contrôles reproductibles

- `cargo deny check licenses sources` : expressions SPDX autorisées, absence
  de registre ou dépendance Git non approuvé ; dépendances de développement incluses.
- `cargo deny check advisories` : avis RustSec, à réexécuter à chaque publication. Cinq avis de maintenance transitifs
  font l’objet d’exceptions ciblées et documentées dans [maintenance.md](maintenance.md) ;
  aucune vulnérabilité connue ne fait l’objet d’une exception.
- `python3 scripts/check-publication.py` : périmètre public, empreintes des
  polices et notices amont, marque et liens de fichiers de documentation.
- `packaging/licenses.py --target <triple> --output <dossier>` : graphe Cargo
  propre à la cible, textes complets et manifeste SHA-256. Un manque bloque la distribution.
- Les paquets incluent aussi les notices de la bibliothèque standard Rust.

Les fichiers de provenance sont dans `assets/fonts`, `assets/brand` et
`assets/licenses`. Les notices générées sont dans `Dependency-Licenses` de
chaque paquet, avec `manifest.json` et `THIRD-PARTY-NOTICES.txt`.

## Limites

Ces contrôles techniques vérifient les composants et informations présents dans
la résolution et les sources examinées ; ils ne constituent pas une certification
juridique de propriété intellectuelle ou de marque. Ils ne permettent pas non
plus d’affirmer l’absence de vulnérabilité inconnue. Toute nouvelle ressource
ou dépendance nécessite sa propre revue et ne doit pas être ajoutée sur la
seule base d’un résultat automatique.
