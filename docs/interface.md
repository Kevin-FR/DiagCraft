# Ruban et volets

Le ruban compact garde une seule ligne de commandes adaptée à l’onglet actif :
Accueil, Insérer, Conception, Affichage, Symboles ou Avancé. Changer d’onglet
conserve le choix compact ou développé. Le chevron à droite change explicitement
ce choix ; **Affichage → Volets et ruban → Adapter le ruban à la fenêtre** réactive
l’adaptation automatique.

![Ruban compact et deux volets](images/interface.png)

Les commandes utilisent les mêmes hauteurs, contours, icônes et états de focus.
Les groupes développés gardent une hauteur commune. Les lignes trop larges
restent accessibles par défilement horizontal.

## Garder de la place pour dessiner

- Glisser le bord intérieur d’un volet pour le redimensionner.
- Une seule icône par côté, sous le ruban au bord de la zone de travail, ouvre
  ou réduit le volet. À gauche, elle reste dans le rail de navigation ; à droite,
  elle conserve exactement sa position quand les propriétés sont fermées.
  La barre des documents ne contient plus de commandes de volet.
- Les largeurs choisies sont temporairement limitées sur une petite fenêtre pour
  conserver au moins environ 490 points de dessin, règles comprises, à 1200 points
  de largeur avec les deux volets. Elles sont restaurées quand on agrandit la fenêtre.
- Les propriétés du document restent masquées au premier lancement. Un simple
  clic sur un objet les laisse masquées ; le double-clic demande explicitement
  l’édition du texte.

Les deux en-têtes partagent leurs marges et leurs séparateurs. Le contenu des
propriétés défile sous l’en-tête : le bouton de fermeture reste toujours visible.
Sans sélection, le volet présente directement le nom du document, les dimensions
de la page et le gestionnaire multipage, sans longue introduction.

![Document et volets ouverts](images/panels.png)

![Commandes accessibles avec les volets fermés](images/panels-collapsed.png)

La visibilité, les largeurs, l’onglet actif et le mode du ruban sont mémorisés à
la fermeture normale dans `interface-v1.json`, près de la bibliothèque locale.
Ils ne sont pas enregistrés dans le fichier `.draw`.
**Réinitialiser l’interface**, accessible dans le menu Affichage ou avec la
palette ⌘K / Ctrl+K, rétablit les réglages initiaux sans modifier le dessin.
Les préférences illisibles ou d’une version inconnue sont préservées ; une
réinitialisation explicite permet de recommencer leur enregistrement.

## Retrouver les formes

Le bouton **+ Bibliothèques de symboles…** reste visible en bas du volet Symboles.
La recherche ignore les accents et accepte plusieurs mots. Elle recherche aussi
dans les bibliothèques masquées, sans modifier la liste de celles que vous avez
choisies. Effacer la recherche retrouve les bibliothèques et leur état replié.

## Retrouver les objets du dessin

**Affichage → Rechercher un objet…** ou **⌘F / Ctrl+F** ouvre une liste temporaire
des objets de la page active. La recherche par texte, type ou ID permet de
sélectionner un objet hors écran et de le rejoindre sans agrandir les volets.
**Maj+Entrée** ajoute un résultat à la sélection et **F2** ouvre son texte dans
le bloc. Voir [la recherche et le clavier](recherche-objets.md).

## Références de conception

La revue de [PDFCraft au commit d8b2705](https://github.com/storytold/pdfcraft/tree/d8b27058252da24a08df8ff06c23418ac4fb97a1)
a porté sur les barres compactes, en-têtes, dimensions bornées et préférences
locales de ses modules chrome, panels et docking. DiagCraft conserve son
implémentation egui 0.33.3 ; aucun code, marque ou asset de PDFCraft n’a été intégré.

La recherche dans les bibliothèques masquées et l’accès permanent au gestionnaire
reprennent des principes décrits dans la [documentation du volet de formes de draw.io](https://www.drawio.com/docs/manual/editor/panels/shapes-panel/).
Les symboles et le code restent ceux de DiagCraft.
