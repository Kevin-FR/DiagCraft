# Maintenance des dépendances

L’audit RustSec de préparation de la 0.1.0 signale cinq avis de **maintenance**
dans les dépendances transitives. Ils ne déclarent pas une vulnérabilité, mais
indiquent des migrations à suivre. Aucune exception de vulnérabilité ou de
comportement non sûr n’est configurée. Les nouveaux avis restent bloquants.

| Avis | Dépendance | Suivi |
| --- | --- | --- |
| [RUSTSEC-2026-0206](https://rustsec.org/advisories/RUSTSEC-2026-0206) | rustybuzz | Migration amont du moteur SVG vers harfrust |
| [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192) | ttf-parser | Migration des piles egui et SVG/PDF vers le remplacement amont |
| [RUSTSEC-2026-0371](https://rustsec.org/advisories/RUSTSEC-2026-0371) | zbus_names | Intégration zbus 6 dans l’accessibilité Linux |
| [RUSTSEC-2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370) | zvariant | Intégration zbus 6 dans l’accessibilité Linux |
| [RUSTSEC-2026-0372](https://rustsec.org/advisories/RUSTSEC-2026-0372) | zvariant_derive | Intégration zbus 6 dans l’accessibilité Linux |

`deny.toml` contient les cinq identifiants précis, avec une raison par avis.
Les exceptions doivent être réexaminées à chaque release, retirées après
migration et ne doivent pas servir à autoriser de nouvelles dépendances concernées.
Conserver l’accessibilité Linux pendant cette migration. La base RustSec évolue :
le succès d’un audit ne garantit pas l’absence de problème inconnu.
