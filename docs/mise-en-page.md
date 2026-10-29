# Mise en page automatique des flux

Fonction disponible dans la branche de développement, non encore distribuée.
Ouvrez **Organiser → Mise en page automatique…**, ou recherchez cette commande
dans la palette Cmd/Ctrl+K. Choisissez Horizontal ou Vertical. Avec une sélection,
la case « Sélection uniquement » est cochée ; décochez-la pour la page active.

![Aperçu de la mise en page](images/auto-layout.png)

L’aperçu ne modifie pas le dessin. Il montre les positions proposées avec des
rectangles et des liaisons simplifiées ; les textes, contours détaillés et parcours
réels des connecteurs ne sont pas reproduits. Le nombre de blocs déplacés et
conservés est indiqué. **Appliquer** valide le lot, puis ajuste la vue ; Cmd/Ctrl+Z
annule l’ensemble. **Annuler**, Échap ou un clic extérieur ferme l’aperçu sans
modification. La page peut s’agrandir pour contenir le résultat.

Les blocs verrouillés, masqués, groupés et les textes libres restent à leur place.
Les blocs non sélectionnés restent aussi fixes. Les boîtes des blocs visibles et
graphiques fixes sont évitées. Les dessins libres restent inchangés mais ne sont
pas des obstacles de placement. Les autres pages ne changent pas.

Le graphe des blocs à déplacer donne des niveaux. Les boucles sont regroupées
dans un même niveau ; les blocs déconnectés sont aussi disposés. Le résultat est
déterministe à données égales, avec 60 px d’espacement et une marge de 40 px.
Les connecteurs sont conservés : les routes automatiques suivent les blocs et
les points de passage manuels restent à leurs coordonnées, donc peuvent demander
une retouche. Les libellés, styles, IDs et appartenances aux calques sont conservés.

Cette première version traite au maximum 500 blocs éligibles. Une sélection
plus petite permet de travailler sur les grands dessins. Les dimensions trop
grandes sont refusées sans changement. Il ne s’agit pas d’un optimiseur de
croisements ; déplacement des groupes et réglages d’espacement restent prévus.

L’application vérifie le document source, la page et l’onglet avant validation.
Un aperçu périmé doit être actualisé. Les tests couvrent cycles, branches, blocs
fixes, sélection, multipage, annulation et modales dans les sept langues à
1200 × 680 points. Ils ne remplacent pas les essais utilisateurs.
Le format `.draw` reste v7 et l’API agents v1, sans dépendance ajoutée.
