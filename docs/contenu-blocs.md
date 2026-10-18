# Mettre en forme le contenu d’un bloc

Depuis DiagCraft 0.6.0, un bloc peut contenir plusieurs styles de texte : gras,
italique, souligné, barré, police monospace, tailles et couleurs différentes.
Le texte reste éditable ; aucun balisage HTML n’est interprété.

## Ouvrir l’éditeur

Double-cliquer sur un bloc, choisir **Modifier le texte** dans son menu contextuel,
ou **Mise en forme du contenu…** dans **Propriétés → Texte et typographie**.
L’éditeur s’ouvre sans agrandir le volet de propriétés.

![Éditeur du contenu](images/rich-text-editor.png)

1. Sélectionner un mot ou une portion de texte avec la souris ou le clavier.
2. Utiliser **B**, **I**, **U**, **S**, **Aa**, la taille ou la couleur.
3. Choisir **Appliquer**, ou **Annuler** pour conserver le bloc original.

**Tout sélectionner** permet de modifier l’ensemble du texte. Les boutons de
style sont désactivés sans sélection ; le texte saisi hérite du style au point
d’insertion. **Entrée** ajoute une ligne, y compris une ligne vide entre deux
paragraphes. L’alignement et le retour automatique concernent tout le bloc.
**Effacer la mise en forme** rétablit, pour la sélection, le style de base du bloc.

L’aperçu sur la feuille suit le brouillon ; le document et sa copie de récupération
restent inchangés tant que l’on n’a pas appliqué. Les positions, identifiants et
connecteurs restent conservés. Si l’ajustement automatique est activé, la forme
s’adapte aussi aux tailles de police choisies.

![Styles mélangés dans un bloc](images/rich-text.png)

## Raccourcis

- **⌘B / Ctrl+B**, **⌘I / Ctrl+I**, **⌘U / Ctrl+U** : gras, italique, souligné sur la sélection.
- **⌘Z / Ctrl+Z** et **⌘⇧Z / Ctrl+Maj+Z** : annuler/rétablir dans l’éditeur, texte et styles ensemble.
- **⌘Entrée / Ctrl+Entrée** : appliquer. **Échap** : annuler le brouillon.

Après application, une seule annulation du dessin restaure le contenu précédent.
Le champ de texte du volet reste également éditable et préserve les styles des
parties inchangées. Le presse-papiers texte du système insère du texte brut ; la
copie de blocs dans DiagCraft conserve leurs styles.

## Enregistrement, exports et agents

Les portions mises en forme sont enregistrées dans le champ facultatif `text_runs`
du nœud, en complément du texte brut. Le format reste **v7** et l’API agents **v1**.
Les anciens dessins restent inchangés. Une application plus ancienne ignore ces
portions et affiche le texte avec le style de base ; réenregistrer avec elle perd
les styles par portions.

Le SVG utilise des `tspan` éditables. Le PDF conserve le texte sélectionnable et
ses polices embarquées. Le lecteur SVG doit disposer des polices utilisées.
La coupe des lignes et l’ajustement tiennent compte des styles et tailles de
chaque portion. Les accents combinés et séquences emoji ne sont pas séparés par
une frontière de style. La présence d’un glyphe dépend des polices embarquées.

L’API permet de remplacer `text_runs`, ou de conserver les styles en ne changeant
que `text`. Les plages sont en caractères Unicode, fin exclue, et non en octets
UTF-8. Elles doivent être ordonnées, sans chevauchement et aux frontières de
graphèmes. Maximum : 1 024 portions, texte de 65 536 octets, tailles de 6 à 200 px.
Voir [le contrat agents](AGENTS-DRAW.md).
