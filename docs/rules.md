# Goblins Bowl, règles 0.2

*English version below.*

## Le bowl

Un grand bol en forme de croix : 21 × 21 cases, sans les quatre coins de 5 × 5 (341 cases). Chaque branche se termine par une rangée de 11 cases, la **tanière**. Les gobelins de gauche (domicile) défendent la tanière ouest, ceux de droite (visiteurs) la tanière est. Les branches nord et sud sont des recoins pour se cacher ou contourner.

On se déplace et on est voisin **en ligne droite seulement** (haut, bas, gauche, droite), jamais en diagonale.

## Les équipes

9 gobelins par équipe : 4 Cogneurs, 4 Sprinteurs, 1 Lanceur. Chaque gobelin peut tout faire, seuls ses chiffres changent. Un chiffre comme Attaque 4 veut dire « lance un dé à 4 faces ».

| Gobelin | Rôle | Attaque | Défense | Agilité | Santé | Mouvement |
|---|---|---|---|---|---|---|
| Cogneur | bagarreur | 4 | 4 | 2 | 6 | 3 |
| Sprinteur | marqueur | 3 | 2 | 4 | 3 | 5 |
| Lanceur | meneur | 2 | 3 | 4 | 3 | 4 |

## Un tour

Chaque équipe a **60 secondes** par tour et **12 tours par mi-temps**, deux mi-temps. Pendant son tour, chaque gobelin a son **Mouvement** en cases et **une action** : **Assaut**, **Garde** ou **Lancer**. Il les dépense en autant d'ordres que tu veux, quand tu veux : il peut avancer, agir, laisser jouer un coéquipier, puis repartir avec les cases qui lui restent, sans jamais traverser personne.

Le tour s'arrête quand tu appuies sur **Fin du tour**, quand le temps est écoulé, quand une équipe marque ou quand une action rate, même si plus aucun gobelin ne peut jouer.

**Si une action rate, le tour s'arrête tout de suite.** C'est tout le jeu : fais d'abord ce qui est sûr, garde le risqué pour la fin.

## Se battre

Le chiffre Δ = mon dé moins le dé adverse. **En cas d'égalité, celui qui agit gagne.**

- **Assaut** sur un ennemi voisin : je lance mon dé d'Attaque, il lance son dé de Défense.
  - Je gagne : il tombe, lâche la balle et perd Δ points de Santé (rien si égalité).
  - Je perds : c'est moi qui tombe, je perds Δ − 1 points de Santé (rien si je perds de 1), et **le tour s'arrête**.
- **Garde** : jusqu'au prochain tour de mon équipe, +1 à ma Défense et +1 à chaque coéquipier voisin. Ça s'additionne : trois gobelins en Garde côte à côte font un vrai mur.
- **Esquive** : quand je quitte une case à côté d'un ennemi debout, je lance mon dé d'Agilité contre le dé de Défense de l'ennemi le plus dangereux (avec sa Garde). Si je perds, je tombe sur place, je perds Δ − 1 points de Santé et **le tour s'arrête**.

## La balle

Lancer de balle : je lance mon dé d'Agilité, j'enlève 1 par ennemi debout à côté de moi, et il faut faire **2 ou plus** (un 1 rate toujours).

- **Ramasser** : obligatoire quand j'entre sur la case de la balle.
- **Lancer** (c'est mon action) : vers un coéquipier debout, à n'importe quelle distance, avec −1 par case au-delà de 6. Chaque ennemi debout sous la trajectoire tente une **interception** (encore −1 pour la surprise). Sinon, mon coéquipier doit **attraper**.
- **Attraper** : quand la balle arrive sur moi (passe, rebond, coup d'envoi).
- Raté par mon équipe pendant son tour, ou interception : c'est un **fumble**, la balle rebondit et **le tour s'arrête**.

La balle rebondit d'une case au hasard (dé à 4 faces : 1 haut, 2 droite, 3 bas, 4 gauche), jamais hors du bowl.

## Tomber, être KO

Un gobelin à terre ne bloque plus personne, perd sa Garde, lâche la balle et peut encore être attaqué (avec −2 à sa Défense). Il se relève tout seul au début du tour suivant de son équipe, mais ne joue pas pendant ce tour-là. À 0 Santé, il est **KO** : il sort jusqu'au prochain coup d'envoi, où il revient avec un dé de Santé (au moins 1).

## Marquer

Un gobelin debout avec la balle dans la tanière ennemie marque 1 point, même pendant le tour adverse. Ensuite, **coup d'envoi** : tout le monde reprend sa place de départ, tous les gobelins retrouvent leur Santé (sauf les KO, qui lancent leur dé de Santé), la balle tombe au centre et rebondit une fois, et l'équipe qui vient d'encaisser joue en premier.

## Le coup d'envoi

Chaque équipe se place dans sa moitié, en miroir de l'autre. On compte les colonnes depuis la colonne du centre vers sa tanière, et les rangées depuis la rangée du centre.

- **Ligne de mêlée**, 1 colonne derrière le centre, rangées −1, 0 et +1 : les Cogneurs d'abord, puis les Sprinteurs, puis le Lanceur.
- **Ailes**, 3 colonnes derrière, rangées −5, −4, +4 et +5 : les Sprinteurs d'abord.
- **Fond**, 8 colonnes derrière, à deux pas de sa tanière, rangées −2 et +2 : le Lanceur d'abord.

Avec l'équipe de base : 3 Cogneurs sur la ligne de mêlée, les 4 Sprinteurs sur les ailes, le Lanceur et le 4e Cogneur au fond.

## Pour jouer sur papier

Un quadrillage 21 × 21 (on raye les coins de 5 × 5), des jetons avec les numéros, une pièce pour le dé à 2 faces, un dé à 6 faces pour le dé à 3 faces (1-2 = 1, 3-4 = 2, 5-6 = 3), un dé à 4 faces et un dé à 6 faces. Le chronomètre du téléphone pour les 60 secondes (ou 90 pour commencer).

## Quelques chances

| Situation | Réussite |
|---|---|
| Cogneur contre Cogneur | 62 % |
| Cogneur contre Cogneur en Garde | 38 % |
| Cogneur contre un mur de deux Gardes | 19 % |
| Cogneur contre Sprinteur | 88 % |
| Cogneur contre un Cogneur à terre | 94 % |
| Sprinteur qui esquive un Cogneur | 62 % |
| Balle avec Agilité 4, 0 / 1 / 2 ennemis à côté | 75 / 50 / 25 % |

---

# Goblins Bowl, rules 0.2

## The bowl

A cross-shaped bowl: 21 × 21 squares without its four 5 × 5 corners (341 squares). Each arm ends in an 11-square **den**. Home defends the west den, away the east den; the north and south arms are side pockets.

Moves and adjacency are **orthogonal only**, never diagonal.

## Teams

9 goblins: 4 Bruisers, 4 Sprinters, 1 Thrower (rosters allow 0-8 / 0-8 / 1-2). Every goblin can do everything; only its characteristics differ. Attack, Defense and Agility are dice with that many faces.

| Archetype | Role | Attack | Defense | Agility | Health | Move |
|---|---|---|---|---|---|---|
| Bruiser | fighter | 4 | 4 | 2 | 6 | 3 |
| Sprinter | scorer | 3 | 2 | 4 | 3 | 5 |
| Thrower | leader | 2 | 3 | 4 | 3 | 4 |

## A turn

**60 seconds** per turn, **12 turns per team per half**, two halves. In its team's turn each goblin has its **Move** in steps and **one action**: **Assault**, **Stance** or **Throw**. It spends them over as many orders as its coach likes, in any order: it may walk, act, let a teammate play, then walk on with the steps it has left, never through a goblin.

A turn ends when its coach ends it, when its clock runs out, when a team scores or when an action fails, even once no goblin can act.

**Any failed action ends the turn at once.** Play safe actions first, risky ones last.

## Fight actions

Δ = actor's roll − opponent's roll. **Ties go to the actor.**

- **Assault** an adjacent enemy: d(Attack) vs d(Defense). Win: it falls, drops the ball, loses Δ Health (nothing on a tie). Lose: the attacker falls, loses |Δ| − 1 Health (nothing when losing by 1), and the turn ends.
- **Stance**: until the team's next turn, +1 Defense to itself and +1 to each adjacent teammate, stacking: a line of Stance goblins is a wall.
- **Dodge**, each step out of a square next to a standing enemy: d(Agility) vs the Defense die (plus Stance bonus) of the enemy most likely to stop it. Lose: it falls where it stands, loses |Δ| − 1 Health, and the turn ends.

## Ball actions

d(Agility) minus 1 per adjacent standing enemy must reach **2** (a 1 always fails).

- **Pick**: forced when entering the ball's square.
- **Throw** (the action): to a standing teammate at any distance, −1 per step beyond 6. Each standing opponent under the flight line tries to **intercept** (extra −1 for surprise); otherwise the receiver must **catch**.
- **Catch**: a throw, a bounce or the kick-off.
- A failed Pick, Throw or Catch by the active team, or an interception, is a **fumble**: the ball bounces and the turn ends.

The ball bounces one square in a random orthogonal direction (d4: 1 north, 2 east, 3 south, 4 west), never out of the bowl.

## Down and knocked out

A downed goblin has no reach, loses its Stance, drops the ball and can still be assaulted (−2 Defense). It stands up at the start of its team's next turn but cannot act during it. At 0 Health it is **KO** until the next kick-off, where it returns with d(Health) Health.

## Scoring

A standing goblin holding the ball in the enemy den scores, in either team's turn. Then the **kick-off**: everyone goes back to the kick-off formation, everyone heals (KO goblins roll their Health die), the ball drops on the centre square and bounces once, and the team that conceded plays first.

## Kick-off formation

Each team lines up in its own half, mirroring the other. Columns are counted from the centre column towards the team's den, rows from the centre row.

- **Scrimmage line**, 1 column behind the centre, rows −1, 0 and +1: Bruisers first, then Sprinters, then Throwers.
- **Wings**, 3 columns behind, rows −5, −4, +4 and +5: Sprinters first.
- **Back**, 8 columns behind, two steps from the team's den, rows −2 and +2: Throwers first.

With the default team: 3 Bruisers on the scrimmage line, the 4 Sprinters on the wings, the Thrower and the 4th Bruiser at the back.

## Paper playtest kit

A 21 × 21 grid with the 5 × 5 corners crossed out, numbered tokens, a coin for the d2, a d6 for the d3 (1-2, 3-4, 5-6), a d4 and a d6, and a phone timer (90 s for the first games).

Every number above is tunable in `rules.toml`.
