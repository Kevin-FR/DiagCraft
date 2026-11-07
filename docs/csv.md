# CSV — diagrammes depuis des données locales

Le contrat de ce sous-ensemble a été défini avant implémentation. DiagCraft traite
le CSV comme une donnée locale ; il n’accède à aucun réseau, n’exécute ni formule
ni macro, et ne charge aucune ressource externe. La saisie reste limitée pour
protéger le document et l’interface.

## Import pris en charge

Un fichier CSV local, RFC 4180 (virgules, guillemets doubles, valeurs entre
guillemets, retours à la ligne entre guillemets) :

- Version à en-têtes : la première ligne contient les noms de colonnes, uniques
  et non vides. Une ligne vide ou non textuelle de tête peut être ignorée.
- Délimiteur virgule par défaut ; points-virgules reconnus quand la première
  ligne ne contient aucune virgule. Aucun délimiteur configurable en v1.
- Encodage UTF-8 (y compris sans BOM). Autres encodages refusés avec un diagnostic
  indiquant la conversion à effectuer au préalable.
- Chaque ligne (hors en-tête) devient une forme ; les colonnes deviennent des
  données utilisables pour le libellé et les relations.

Choix effectués dans l’interface avant l’aperçu :

- **Colonne de libellé** : le texte affiché dans chaque forme (nom de colonne ou
  valeur libre). Vide toléré.
- **Colonne d’identité** (facultative) : valeurs uniques servant de référence pour
  les relations. Les relations référencent ces valeurs, pas des numéros de ligne.
- **Colonnes de relation** : chaque valeur d’une telle colonne référence l’identité
  d’une autre ligne (ou plusieurs identités séparées par `|`) et crée un lien entre
  les deux formes correspondantes.
- **Rapport** avant application : nombre de formes, liens, lignes ignorées (vides),
  colonnes non reliées, valeurs d’identité manquantes ou en double.

Au maximum : **256 Kio de fichier**, **1000 lignes de données**, **64 colonnes**,
**4096 caractères par cellule**, et **10 000 objets** créés si la limite du
document est atteinte (une erreur annule l’import sans insertion partielle).
Le format v8 limite en plus le libellé d’une relation à 4096 octets UTF-8.
Ces bornes évitent des allocations excessives et restent sous les budgets du
document (16 Mio, 10 000 objets, 100 pages).

Lignes entièrement vides, en-têtes en double, colonnes sans nom et une colonne
d’identité référencée mais absente sont signalés avec le numéro de ligne et une
piste de correction. Aucune ligne ou colonne inconnue n’est silencieusement
ignorée : chaque écart est rapporté dans les diagnostics avant application.

Les textes et relations deviennent des objets DiagCraft modifiables. Le placement
est déterministe et borné (grille simple, sans croiser les liens), indépendant de
l’ordre des lignes. L’import signale que la mise en page est indicative avant
application. Les valeurs sont toujours traitées comme du texte affiché, jamais
comme du HTML ou une instruction.

## Diagnostic et avenir

Le sous-ensemble initial est un import ponctuel vers une **nouvelle page**, en une
annulation. Les mises à jour par clé stable (réimporter pour modifier) et l’import
SQL sont étudiés séparément, hors de ce lot. Aucune ressource réseau implicite et
aucune exécution du contenu ne sont prévues.

## Parcours dans l’application

Ouvrir la commande d’import CSV depuis Insérer ou la palette de commandes. Choisir
un fichier `.csv` local, puis régler la colonne de libellé, la colonne d’identité
et les colonnes de relation ; demander l’aperçu. Les erreurs désignent la ligne et
la colonne ; les avertissements précisent les lignes ignorées et les relations
sans cible. Le fichier et le document ouvert restent intacts pendant cet examen.
L’application ajoute le résultat dans une **nouvelle page** ; un seul Annuler retire
cet import. Une modification du document entre aperçu et validation impose un
nouvel aperçu.

## CLI locale

```sh
diagcraft-cli --import-csv donnees.csv resultat --identity id --label nom \
  --connect parent --dry-run
diagcraft-cli --import-csv donnees.csv resultat --identity id --label nom \
  --connect parent,voisins --accept-losses
```

Le dry-run n’écrit aucun fichier ; le JSON expose `diagnostics`,
`requires_confirmation` et le contenu proposé (`preview`). Sans
`--accept-losses`, une conversion comportant des avertissements refuse d’écrire.
Les chemins de sortie doivent être **nouveaux** ; les fichiers existants ne sont
jamais écrasés. Les pages sont indexées à partir de 0.

La CLI et l’application utilisent le même moteur. Les quatre outils MCP restent
ceux de l’API de patches ; un agent peut utiliser cette CLI pour convertir un CSV,
puis inspecter et modifier les objets obtenus par les outils habituels. Voir aussi
[les diagnostics agents](diagnostics.md).

Les commandes et le rapport courant de mise en page sont localisés dans les sept
langues. Les autres détails techniques de conversion sont actuellement en anglais,
avec des codes stables et un emplacement dans la source.
