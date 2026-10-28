# Diagnostics des agents

Depuis 0.11.0, les erreurs de patches conservent le champ texte `error` et ajoutent
un objet `diagnostic` exploitable par une CLI ou un agent. L’API reste **v1** et le
format `.draw` reste **v7** ; la réponse est enrichie sans modifier les opérations.

## Comprendre le contexte

- `code` : catégorie stable pour identifier la correction à apporter.
- `severity` : erreur bloquante ou avertissement non bloquant.
- `message` et `hint` : explication et piste de correction.
- `operation` et `operation_name` : index à partir de **0** et nom de l’opération.
- `page` : index de page à partir de **0**.
- `target` : identifiant numérique, alias ou liste de références des objets/calques concernés, si disponible.
- `id` : ID résolu lorsque la cible est un alias connu.
- `line` et `column` : position de lecture JSON, à partir de **1**, lorsqu’elle est connue.

Les champs de contexte non disponibles sont absents ou nuls. Le texte présenté
à l’utilisateur numérote les opérations à partir de 1. Les messages d’un système
de fichiers peuvent varier selon le système ; les champs structurés évitent de
les analyser pour deviner le contexte.

La CLI écrit l’erreur JSON sur stderr et retourne un code non nul. Une erreur
d’exécution MCP conserve `isError: true`, une description textuelle et ajoute
`structuredContent` avec les mêmes champs. Les erreurs du protocole JSON-RPC
restent des erreurs du protocole. Les quatre outils MCP gardent leurs noms.

## Corriger puis réessayer

1. Lire le code, la page, la cible et l’opération concernée.
2. Suivre `hint` : actualiser le contexte si la révision a changé, vérifier l’alias
   sur la bonne page, corriger une dimension ou déverrouiller explicitement l’objet.
3. Préparer un nouveau patch avec la révision exacte de l’inspection courante.
4. Utiliser le dry-run ou l’aperçu de l’application avant validation.

Une erreur rejette tout le lot, même si des opérations précédentes étaient valides.
Aucun résultat partiel n’est écrit. La source reste intacte ; une sortie existante
n’est pas remplacée. Dans l’application, un lot validé reste annulable en une étape.

Les avertissements documentaires comportent aussi un code et une aide. Un objet
hors page, une route contrainte et une limite de recherche du routage sont des
situations distinctes ; l’interface ne les présente plus toutes comme « Hors page ».

Le [guide des agents](AGENTS-DRAW.md) décrit les contrats de patch et MCP.
Les diagnostics [Mermaid](mermaid.md) emploient un rapport de conversion avec
une liste `diagnostics`, des emplacements dans la source et les pertes éventuelles.
