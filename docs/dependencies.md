# Bibliothèques et choix techniques

Examen de [PhotoCraft](https://github.com/storytold/photocraft) au commit
[`1d65bd6`](https://github.com/storytold/photocraft/tree/1d65bd605b25e325a12ab3e4d846d94ba41eedb7).
Son [manifest](https://github.com/storytold/photocraft/blob/1d65bd605b25e325a12ab3e4d846d94ba41eedb7/Cargo.toml)
centralise notamment egui/eframe 0.36, serde, image, rfd, rayon, proptest et
criterion, avec des bibliothèques internes et plusieurs applications.

| Choix DiagCraft | Usage et décision |
| --- | --- |
| egui/eframe 0.33.3, glow | Interface native existante et testée. Wgpu sera évalué séparément sur performances et compatibilité. |
| serde / serde_json | Documents et patches lisibles, contrôlés avant application. |
| rfd 0.17.2 | Boîtes de dialogue de fichiers natives. |
| image, fonctionnalité PNG | Icône et captures de l’application ; aucun codec photo supplémentaire nécessaire. |
| resvg/usvg et svg2pdf | Aperçus des styles et export PDF à partir des mêmes géométries SVG. |
| pdf-writer | Assemblage du PDF multipage. |
| sha2 / tempfile | Révisions, intégrité et écritures atomiques, y compris les téléchargements vérifiés. |
| semver 1.0.28 | Comparaison des versions et filtrage des versions expérimentales. |
| curl système | Requêtes HTTPS GitHub dans un processus annulable en arrière-plan ; aucune pile HTTP Rust ajoutée. |

Les bibliothèques de traitement photo, codecs RAW, profils colorimétriques,
compositeur GPU et plugins de PhotoCraft ne répondent pas au périmètre actuel.
`craft-ui` et `craft-print` ne sont pas ajoutés sans besoin mesuré. Rayon,
proptest, criterion et muda peuvent être évalués pour traitement en lots,
tests génératifs, benchmarks et menus natifs dans des changements dédiés.

Les [intégrations officielles egui](https://github.com/emilk/egui) prennent en
charge glow et wgpu. Conserver glow pour cette publication évite de cumuler
réorganisation et changement du moteur de rendu. Cargo.lock fixe les versions
résolues ; les sources Git non approuvées sont refusées par deny.toml.
