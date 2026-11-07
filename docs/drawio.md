# draw.io — importer un sous-ensemble documenté de fichiers `.drawio`

Le contrat de ce sous-ensemble a été défini avant implémentation. DiagCraft traite
le fichier `.drawio` comme une donnée locale : il n’accède à aucun réseau,
n’exécute ni script ni HTML, et ne charge **aucune ressource externe**. Aucune
annonce de prise en charge de Visio ou d’autres formats n’est faite ici ; seuls
les fichiers `.drawio` (et le sous-ensemble ci-dessous) sont lus. La
[documentation du format draw.io](https://www.drawio.com/docs/manual/import/import-formats/)
sert de référence pour décrire la structure ; aucun code ni modèle draw.io n’est
incorporé, seule la géométrie et les libellés sont convertis.

## Périmètre annoncé

DiagCraft lit le format XML `.drawio` sous ses deux formes :

- un `<mxfile>` racine contenant un ou plusieurs `<diagram>` (multipages), chacun
  portant un `<mxGraphModel>` ;
- un `<mxGraphModel>` racine nu (format « simple » d’un seul diagramme).

Le contenu est lu en XML d’abord (éléments et attributs avant le texte) : les
cellules `<mxCell>` d’un `<root>` sont identifiées par `id`, rattachées à une
couche via `parent`, et classées en **sommet** (`vertex="1"`) ou **câble**
(`edge="1"`), ce qui est exclusif. La géométrie est portée par `<mxGeometry>`
(`x`, `y`, `width`, `height`, `relative`) avec `as="geometry"`.

## Éléments convertis

- **Sommets** (`vertex="1"`) sur la couche cible (couche par défaut ou couche
  visible choisie) → une forme DiagCraft, en conservant position (coin supérieur
  gauche) et taille. Le libellé (`value`) est un texte affiché, jamais rendu en
  HTML : les `&lt;br&gt;` deviennent des retours à la ligne, les entités sont
  décodées, l’attribut `html="1"` n’active aucun rendu HTML.
- **Formes** : le style (`style`) est parcouru pour reconnaître une forme connue
  parmi les plus courantes (rectangle, arrondi, ellipse, losange, texte, document,
  base de données, hexagone, trapèze, nuage, etc.). Toute forme non reconnue est
  importée comme rectangle avec un avertissement de perte.
- **Câbles** (`edge="1"`) avec `source` et `target` résolus par `id` de cellule →
  un connecteur DiagCraft entre les deux formes. Points de passage (`<mxPoint
  as="sourcePoint|targetPoint">` et `<Array as="points">`) conservés. Couleurs,
  largeur, pointes de flèche et motif repris quand le style les porte.
- **Libellés de câble** : le texte du câble est converti comme les sommets, avec
  mise en garde lorsque le câble est sans source ou sans cible.

## Limites de lecture

Au maximum : **512 Kio de fichier**, **1 diagramme lu à la fois** (page choisie),
**2000 cellules** analysées par diagramme, **512 Kio de géométrie**, **4096
caractères par libellé**, et le budget du document (16 Mio, 10 000 objets,
100 pages). Ces bornes évitent des allocations excessives. Un dépassement de
borne est une erreur qui **annule l’import sans insertion partielle**.

Un `<mxfile>` peut contenir plusieurs `<diagram>` ; l’utilisateur choisit la page
(l’interface propose un sélecteur, la CLI un `--page`, indexé à partir de 0, par
défaut 0). Les autres pages annoncées restent hors du résultat, avec un
avertissement indiquant qu’elles ne sont pas reprises.

## Avertissements de perte

Toute perte est signalée avant application, jamais silencieuse :

- forme inconnue importée en rectangle ;
- style ignoré (dégradés, ombres, images, pointillés fins, polices inconnues…) ;
- **image externe** (`image=` avec URL http(s)) : jamais chargée ; la forme est
  importée avec son libellé et un avertissement explicite ;
- image en `data:` : décodée uniquement si elle est PNG (déjà supporté par le
  moteur), sinon ignorée avec avertissement ;
- libellé dépassant 4096 caractères : tronqué avec avertissement ;
- câble sans source ou sans cible : importé en « extrémités libres » avec un
  avertissement, ou ignoré selon sa pertinence ;
- pages supplémentaires non reprises, couches invisibles non reprises ;
- cellule hors de la couche choisie ou de géométrie anormale (taille nulle,
  négative) : ignorée avec avertissement.

Le rapport précise, pour chacune, un code stable, la cellule concernée et une
piste de correction. Aucun élément inconnu n’est sauté en silence : chaque écart
est listé dans les diagnostics avant application.

## Diagnostic et avenir

L’import est ponctuel vers une **nouvelle page**, en une annulation. La position
des objets est conservée telle quelle (draw.io porte la géométrie) ; le style
fini est indicatif. Les mises à jour par réimport, l’export `.drawio` et la
lecture de formats propriétaires (`.vsdx`, dessins Visio) sont étudiés
séparément, hors de ce lot. Aucune ressource réseau implicite et aucune
exécution du contenu ne sont prévues.

## Parcours dans l’application

Ouvrir la commande d’import draw.io depuis Insérer ou la palette de commandes.
Choisir un fichier `.drawio` local, sélectionner la page à importer si le fichier
en contient plusieurs, puis demander l’aperçu. Les erreurs désignent le fichier
et la cellule ; les avertissements précisent les formes inconnues, les images
exclues et les câbles sans cible. Le fichier et le document ouvert restent
intacts pendant cet examen. L’application ajoute le résultat dans une **nouvelle
page** ; un seul Annuler retire cet import. Une modification du document entre
aperçu et validation impose un nouvel aperçu.

## CLI locale

```sh
diagcraft-cli --import-drawio schema.drawio resultat --page 0 --dry-run
diagcraft-cli --import-drawio schema.drawio resultat --page 1 --accept-losses
```

Le dry-run n’écrit aucun fichier ; le JSON expose `diagnostics`,
`requires_confirmation` et le contenu proposé (`preview`). Sans
`--accept-losses`, une conversion comportant des avertissements refuse d’écrire.
Les chemins de sortie doivent être **nouveaux** ; les fichiers existants ne sont
jamais écrasés. Les pages sont indexées à partir de 0 (par défaut 0).

La CLI et l’application utilisent le même moteur. Les quatre outils MCP restent
ceux de l’API de patches ; un agent peut utiliser cette CLI pour convertir un
`.drawio`, puis inspecter et modifier les objets obtenus par les outils
habituels. Voir aussi [les diagnostics agents](diagnostics.md).

Les commandes et le rapport courant de mise en page sont localisés dans les sept
langues. Les autres détails techniques de conversion sont actuellement en
anglais, avec des codes stables et un emplacement dans la source.
