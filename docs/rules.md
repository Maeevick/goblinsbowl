# Goblins Bowl, règles 0.12

*English version below.*

## Le bowl

Un grand bol rond : un disque de 21 cases de diamètre, dessiné case par case (349 cases). À chaque bout, les 7 cases de la colonne la plus à l'extérieur sont la **tanière** : les gobelins de gauche (domicile) défendent la tanière ouest (D), ceux de droite (visiteurs) la tanière est (V). Au nord et au sud, le bord du bol laisse de la place pour contourner.

```
       .......
     ...........
    .............
   ...............
  .................
 ...................
 ...................
D...................V
D...................V
D...................V
D...................V
D...................V
D...................V
D...................V
 ...................
 ...................
  .................
   ...............
    .............
     ...........
       .......
```

On se déplace **en ligne droite seulement** (haut, bas, gauche, droite), jamais en diagonale. Mais un gobelin debout pèse sur les **8 cases autour de lui**, diagonales comprises : c'est sa **zone**, et il peut attaquer un ennemi sur chacune d'elles.

## Les équipes

9 gobelins par équipe : 3 Cogneurs, 4 Sprinteurs, 2 Lanceurs. Chaque gobelin peut se déplacer, attaquer et lancer, seuls ses chiffres changent (22 points chacun, répartis selon sa spécialité), et chacun a ses coups à lui : seul le Lanceur lance au chaos ou se concentre, seul le Cogneur frappe brutalement ou projette, seul le Sprinteur plonge sur la balle et intercepte à côté de la trajectoire. Un chiffre comme Attaque 4 veut dire « lance un dé à 4 faces ».

| Gobelin | Rôle | Attaque | Défense | Agilité | Santé | Mouvement |
|---|---|---|---|---|---|---|
| Cogneur | bagarreur | 4 | 4 | 3 | 7 | 4 |
| Sprinteur | marqueur | 3 | 2 | 6 | 5 | 6 |
| Lanceur | meneur | 3 | 3 | 6 | 6 | 4 |

## Un tour

Chaque équipe a **60 secondes** par tour et **12 tours par mi-temps**, deux mi-temps. Pendant son tour, chaque gobelin a son **Mouvement** en cases et peut faire **un Assaut et un Lancer**, une fois chacun, dans l'ordre qu'il veut. Ni l'un ni l'autre ne coûte de case : un gobelin peut encore agir après son dernier pas. Un Lancer au chaos et une passe concentrée coûtent quand même **tout le Mouvement** (il ne faut pas avoir bougé). Le gobelin qui porte la balle ne peut pas attaquer. Il dépense tout ça en autant d'ordres que tu veux, quand tu veux : il peut avancer, agir, laisser jouer un coéquipier, puis repartir avec les cases qui lui restent, sans jamais traverser personne.

Le tour s'arrête quand tu appuies sur **Fin du tour**, quand le temps est écoulé, quand une équipe marque ou quand une action rate, même si plus aucun gobelin ne peut jouer.

**Si une action rate, le tour s'arrête tout de suite.** C'est tout le jeu : fais d'abord ce qui est sûr, garde le risqué pour la fin.

## Se battre

Le chiffre Δ = mon dé moins le dé adverse.

- **Assaut** sur un ennemi debout sur l'une des 8 cases autour de moi, si je ne porte pas la balle : je lance mon dé d'Attaque, il lance son dé de Défense. Le gobelin qui porte la balle se défend avec −1.
  - Je gagne : il tombe, lâche la balle et perd Δ points de Santé.
  - **Égalité : personne ne tombe**, et le tour continue.
  - Je perds : c'est moi qui tombe, je perds Δ − 1 points de Santé (rien si je perds de 1), et **le tour s'arrête**.
- **Styles d'assaut** : au moment d'attaquer, je choisis comment.
  - **Simple** : comme ci-dessus.
  - **Chirurgical** : +1 à mon dé, mais il ne perd aucun point de Santé (il tombe quand même).
  - **Brutal**, pour un Cogneur : −1 à mon dé, mais s'il tombe il perd 1 point de Santé de plus.
  - **Projection**, pour un Cogneur : si je gagne, il tombe sur la case que je choisis parmi les 8 autour de moi (une case libre, ou la sienne), numérotées de 1 à 8 dans le sens des aiguilles d'une montre à partir du haut, et il perd 1 point de Santé de moins.
  - Le +1 ou le −1 décide seulement qui gagne : la Santé perdue se compte toujours sur les dés tels qu'ils sont tombés.
- **Agression**, sur un ennemi à terre autour de moi : c'est la seule façon de l'attaquer. +1 à mon dé, et ce +1 compte dans la Santé qu'il perd.
- **Esquive** : quand je quitte une case dans la zone d'un ennemi debout, je lance mon dé d'Agilité **+1**, j'enlève 1 par ennemi debout autour de moi (les 8 cases), et il faut faire **2 ou plus** : face à un seul ennemi, c'est aussi facile qu'un jet de balle sans personne autour. Si je rate, je tombe sur place, je perds en Santé ce qui me manquait moins 1 (rien s'il ne manquait que 1) et **le tour s'arrête**. Un Cogneur (dé à 3 faces) échappe à un ennemi deux fois sur trois, à deux une fois sur trois, à trois jamais.

## La balle

Lancer de balle : je lance mon dé d'Agilité, j'enlève 1 par ennemi debout autour de moi (les 8 cases), et il faut faire **2 ou plus** (sans bonus, un 1 rate toujours).

- **Ramasser** : obligatoire quand j'entre sur la case de la balle.
- **Lancer** : vers un coéquipier debout, à n'importe quelle distance, avec −1 par case au-delà de 6. Chaque ennemi debout sous la trajectoire (sur une case que la balle traverse, même en partie) tente une **interception**, et un Sprinteur aussi **juste à côté** (dans les 8 cases autour d'une case survolée), encore −1 pour la surprise. Sinon, mon coéquipier doit **attraper**.
- **Lancer au chaos**, pour un Lanceur : si je n'ai pas encore bougé de mon tour, je peux lancer sans jet de lancer : la balle file vers le coéquipier visé mais ne lui arrive pas dans les mains, elle rebondit trois fois à partir de sa case. Les ennemis qui le peuvent tentent quand même l'interception. Si un coéquipier attrape la balle pendant ses rebonds, le tour continue ; sinon, après le troisième rebond, c'est un **fumble** et **le tour s'arrête**. Ça me coûte tout mon Mouvement.
- **Passe concentrée**, pour un Lanceur : si je n'ai pas encore bougé de mon tour, je sacrifie tout mon Mouvement et je gagne +3 à mon jet de lancer. Si je rate quand même mon jet, je tombe sur place (de dépit), sans perdre de Santé.
- **Transmission** : un lancer vers un coéquipier collé à moi ne rate jamais, pas de jet de lancer ; seul mon coéquipier lance son jet pour attraper, avec +1.
- **Attraper** : quand la balle arrive sur moi (passe, transmission, rebond, coup d'envoi).
- **Plongeon**, pour un Sprinteur : quand la balle tombe sur une case vide autour de moi (les 8 cases), je plonge pour l'attraper avec −1, plus les ennemis autour de moi comme d'habitude. Réussi, j'atterris sur cette case avec la balle, sans esquive, et je joue normalement ; raté, je tombe sur place, sans perdre de Santé, et la balle rebondit encore. Si plusieurs Sprinteurs peuvent plonger, seul celui qui fait le plus haut plonge (en cas d'égalité, seuls les ex æquo relancent pour se départager). Les autres gobelins n'attrapent que sur leur propre case.
- Un ramassage ou un lancer raté par mon équipe pendant son tour, ma passe ou ma transmission pas attrapée, mon lancer au chaos qu'aucun coéquipier ne tient après ses rebonds, ou une interception : c'est un **fumble**, la balle rebondit et **le tour s'arrête**. Sinon, rater une balle qui rebondit n'arrête pas le tour.

La balle rebondit sur l'une des 8 cases autour, au hasard (dé à 8 faces dans le sens des aiguilles d'une montre : 1 haut, 2 haut-droite, 3 droite, 4 bas-droite, 5 bas, 6 bas-gauche, 7 gauche, 8 haut-gauche), jamais hors du bowl.

## Tomber, être KO

Un gobelin à terre n'a plus de zone, lâche la balle et peut encore être agressé (+1 à l'attaque). Il se relève tout seul au début du tour suivant de son équipe, mais ne joue pas pendant ce tour-là ; une fois debout, il retrouve sa zone, même avant d'avoir pu rejouer. Un gobelin blessé ne récupère rien de tout le match. À 0 Santé, il est **KO** et sort. À chaque coup d'envoi, chaque gobelin KO lance son dé de Santé moins 1 : sur 0 il reste dehors jusqu'au coup d'envoi suivant, sinon il revient avec autant de Santé (il s'est reposé sur le banc).

## Marquer

Un gobelin debout avec la balle dans la tanière ennemie marque 1 point, même pendant le tour adverse. Ensuite, **coup d'envoi** : tout le monde reprend sa place de départ, les blessés sans retrouver de Santé, les KO tentent de revenir, ceux qui restent dehors laissent leurs places aux autres, la balle tombe chez l'équipe qui vient d'encaisser, et c'est elle qui joue en premier.

## Le coup d'envoi

Chaque équipe se place dans sa moitié, en miroir de l'autre. On compte les colonnes depuis la colonne du centre vers sa tanière, et les rangées depuis la rangée du centre.

- **Ligne de mêlée**, 1 colonne derrière le centre, rangées −1, 0 et +1 : les Cogneurs d'abord, puis les Sprinteurs, puis le Lanceur.
- **Ailes**, 3 colonnes derrière, rangées −5, −4, +4 et +5 : les Sprinteurs d'abord.
- **Fond**, 8 colonnes derrière, à deux pas de sa tanière, rangées −2 et +2 : le Lanceur d'abord.

Avec l'équipe de base : les 3 Cogneurs sur la ligne de mêlée, les 4 Sprinteurs sur les ailes, les 2 Lanceurs au fond.

Puis la balle tombe dans la moitié de l'équipe qui reçoit : 4 à 7 colonnes derrière le centre (un dé à 4 faces, plus 3) et jusqu'à 5 rangées d'un côté ou de l'autre (un dé à 12 faces, on relance le 12, moins 6). Elle rebondit ensuite trois fois, sauf si un gobelin debout l'attrape en route.

## Pour jouer sur papier

Un quadrillage 21 × 21 où l'on recopie le disque du plan, des jetons avec les numéros et de quoi noter la Santé, une pièce pour le dé à 2 faces, un dé à 4 faces, un dé à 6 faces (qui sert aussi de dé à 3 faces : 1-2 = 1, 3-4 = 2, 5-6 = 3), un dé à 8 faces pour les rebonds et un dé à 12 faces pour le coup d'envoi. Pour les retours de KO, un dé à 6 ou à 8 faces : on relance au-dessus de la Santé du gobelin. Le chronomètre du téléphone pour les 60 secondes (ou 90 pour commencer).

## Quelques chances

| Situation | Chances |
|---|---|
| Cogneur contre Cogneur | renverse 38 %, égalité 25 %, tombe 38 % |
| Cogneur contre Cogneur, chirurgical | renverse 62 %, égalité 19 %, tombe 19 % |
| Cogneur contre Cogneur, brutal | renverse 19 %, égalité 19 %, tombe 62 % |
| Cogneur contre Cogneur, projection | renverse 38 %, égalité 25 %, tombe 38 %, 1 PV de moins |
| Cogneur contre Sprinteur | renverse 62 %, égalité 25 %, tombe 12 % |
| Cogneur contre un Sprinteur qui porte la balle | renverse 88 %, égalité 12 % |
| Cogneur qui agresse un Cogneur à terre | blesse 62 %, égalité 19 %, tombe 19 % |
| Sprinteur contre Cogneur | renverse 25 %, égalité 25 %, tombe 50 % |
| Sprinteur qui quitte 1 / 2 / 3 ennemis | 83 / 67 / 50 % |
| Cogneur qui quitte 1 / 2 / 3 ennemis | 67 / 33 / 0 % |
| Balle avec Agilité 6, 0 / 1 / 2 ennemis autour | 83 / 67 / 50 % |
| Transmission à un Sprinteur, 0 / 1 / 2 ennemis autour | 100 / 83 / 67 % |
| Plongeon d'un Sprinteur, 0 / 1 ennemi autour | 67 / 50 % |
| Passe de 10 cases d'un Lanceur sans ennemi autour : simple / concentrée à +3 | 17 / 67 % |
| Passe de 6 cases d'un Lanceur marqué par 2 ennemis : simple / concentrée à +3 | 50 / 100 % |
| Lancer au chaos vers un Sprinteur seul / un Lanceur seul, sans ennemi autour | 67 / 15 % |
| Lancer au chaos vers un Lanceur entouré de 4 Sprinteurs, sans ennemi autour | 97 % |
| Retour d'un KO : Cogneur / Lanceur / Sprinteur | 86 / 83 / 80 % |

---

# Goblins Bowl, rules 0.12

## The bowl

A round bowl: a pixelated disc 21 squares across (349 squares). At each end, the 7 squares of the outermost column are a **den**: home defends the west den, away the east den; north and south, the rim leaves room to go round.

Moves are **orthogonal only**, never diagonal. But a standing goblin weighs on the **eight squares around it**, diagonals included: its **reach**, and it may assault an enemy on any of them.

## Teams

9 goblins: 3 Bruisers, 4 Sprinters, 2 Throwers (rosters allow 0-8 / 0-8 / 1-2). Every goblin can move, assault and throw, only its characteristics differ (22 points each, spread after its speciality), and each has moves of its own: only Throwers throw into chaos or focus, only Bruisers hit brutally or project, only Sprinters dive for the ball and intercept beside the flight line. Attack, Defense and Agility are dice with that many faces.

| Archetype | Role | Attack | Defense | Agility | Health | Move |
|---|---|---|---|---|---|---|
| Bruiser | fighter | 4 | 4 | 3 | 7 | 4 |
| Sprinter | scorer | 3 | 2 | 6 | 5 | 6 |
| Thrower | leader | 3 | 3 | 6 | 6 | 4 |

## A turn

**60 seconds** per turn, **12 turns per team per half**, two halves. In its team's turn each goblin has its **Move** in steps and may take **one Assault and one Throw**, once each, in any order. Neither costs a step: a goblin may still act after its last step. A chaos throw and a focused throw still cost **its whole Move** (it must not have moved yet). The goblin holding the ball may not assault. It spends them over as many orders as its coach likes, in any order: it may walk, act, let a teammate play, then walk on with the steps it has left, never through a goblin.

A turn ends when its coach ends it, when its clock runs out, when a team scores or when an action fails, even once no goblin can act.

**Any failed action ends the turn at once.** Play safe actions first, risky ones last.

## Fight actions

Δ = actor's roll − opponent's roll.

- **Assault** a standing enemy on one of the eight squares around, unless holding the ball: d(Attack) vs d(Defense); the goblin holding the ball defends at −1. Win: it falls, drops the ball and loses Δ Health. **Tie: nobody falls** and the turn goes on. Lose: the attacker falls, loses |Δ| − 1 Health (nothing when losing by 1), and the turn ends.
- **Assault styles**, chosen when assaulting: **simple** as above; **chirurgical**, +1 to the attack roll but the target loses no Health (it still falls); **brutal**, for a Bruiser, −1 to the attack roll but a fallen target loses 1 more Health; **projection**, for a Bruiser: on a win the target falls on the square its attacker picks among the eight around itself (a free one, or the target's own), numbered 1 to 8 clockwise from north, and loses 1 Health less. The +1 or −1 only decides who wins: Health lost always counts the dice as rolled.
- **Aggression**, on a downed enemy around, the only way to assault it: +1 to the attack roll, counted in the Health it loses.
- **Dodge**, each step out of a square within a standing enemy's reach: d(Agility) **+ 1**, minus 1 per standing enemy in the eight squares around, must reach **2**: leaving a single enemy is as easy as a ball roll with nobody around. Miss: it falls where it stands, loses its shortfall minus 1 in Health (nothing when 1 short), and the turn ends. A Bruiser (d3) gets away from one enemy two times in three, from two one time in three, from three never.

## Ball actions

d(Agility) minus 1 per standing enemy in the eight squares around must reach **2** (with no bonus, a 1 always fails).

- **Pick**: forced when entering the ball's square.
- **Throw**: to a standing teammate at any distance, −1 per step beyond 6. Each standing opponent under the flight line (on a square the ball crosses, even partly) tries to **intercept**, a Sprinter **next to it** too (in the eight squares around a square the ball flies over), with an extra −1 for surprise; otherwise the receiver must **catch**.
- **Chaos throw**, for a Thrower that has not moved yet this turn: no throw roll; the ball flies towards the teammate aimed at but does not reach its hands, bouncing three times from its square instead. Enemies able to intercept still try. A teammate catching the ball during its bounces keeps the turn going; otherwise, after the third bounce, it is a **fumble** and the turn ends. It costs the thrower its whole Move.
- **Focused throw**, for a Thrower that has not moved yet this turn: it sacrifices its whole Move for +3 to its throw roll. If it still misses the roll, it falls where it stands (in despair), losing no Health.
- **Hand-off**: a throw to an adjacent teammate never fails, with no throw roll; only the receiver rolls to catch, at +1.
- **Catch**: a throw, a hand-off, a bounce or the kick-off.
- **Diving catch**, for a Sprinter: when the ball lands on an empty square among the eight around it, it dives for it at −1, its own maluses as usual. Caught, it lands on that square holding the ball, with no dodge, and plays on as usual; missed, it falls where it stands, unhurt, and the ball bounces on. When several Sprinters could dive, only the highest roll dives (ties: only the tied roll again to tell who). Other goblins catch only on their own square.
- A failed Pick or Throw by the active team, its own throw or hand-off not caught, its chaos throw held by no teammate once its bounces are over, or an interception, is a **fumble**: the ball bounces and the turn ends. Otherwise, missing a bouncing ball does not end a turn.

The ball bounces to one of the eight squares around at random (d8 clockwise from north: 1 north, 2 north-east, 3 east, 4 south-east, 5 south, 6 south-west, 7 west, 8 north-west), never out of the bowl.

## Down and knocked out

A downed goblin has no reach, drops the ball and can still suffer an aggression (+1 to the attack). It stands up at the start of its team's next turn but cannot act during it; once up, it has its reach again, even before it gets to play. A hurt goblin heals nothing for the whole match. At 0 Health it is **KO** and leaves the bowl. At each kick-off, each KO goblin rolls its Health die minus 1: on 0 it stays out until the next kick-off, otherwise it comes back with that much Health (it rested on the bench).

## Scoring

A standing goblin holding the ball in the enemy den scores, in either team's turn. Then the **kick-off**: everyone goes back to the kick-off formation, hurt goblins without healing, KO goblins try to come back, those staying out leave their squares to the others, the ball drops in the half of the team that conceded, and that team plays first.

## Kick-off formation

Each team lines up in its own half, mirroring the other. Columns are counted from the centre column towards the team's den, rows from the centre row.

- **Scrimmage line**, 1 column behind the centre, rows −1, 0 and +1: Bruisers first, then Sprinters, then Throwers.
- **Wings**, 3 columns behind, rows −5, −4, +4 and +5: Sprinters first.
- **Back**, 8 columns behind, two steps from the team's den, rows −2 and +2: Throwers first.

With the default team: the 3 Bruisers on the scrimmage line, the 4 Sprinters on the wings, the 2 Throwers at the back.

Then the ball drops in the receiving team's half, 4 to 7 columns behind the centre (d4 + 3) and up to 5 rows to either side (d11 − 6), and bounces three times unless a standing goblin catches it on the way.

## Some odds

| Situation | Odds |
|---|---|
| Bruiser on Bruiser | knocks down 38 %, tie 25 %, falls 38 % |
| Bruiser on Bruiser, chirurgical | knocks down 62 %, tie 19 %, falls 19 % |
| Bruiser on Bruiser, brutal | knocks down 19 %, tie 19 %, falls 62 % |
| Bruiser on Bruiser, projection | knocks down 38 %, tie 25 %, falls 38 %, 1 Health less |
| Bruiser on Sprinter | knocks down 62 %, tie 25 %, falls 12 % |
| Bruiser on a Sprinter holding the ball | knocks down 88 %, tie 12 % |
| Bruiser's aggression on a downed Bruiser | hurts 62 %, tie 19 %, falls 19 % |
| Sprinter on Bruiser | knocks down 25 %, tie 25 %, falls 50 % |
| Sprinter leaving 1 / 2 / 3 enemies | 83 / 67 / 50 % |
| Bruiser leaving 1 / 2 / 3 enemies | 67 / 33 / 0 % |
| Ball roll with Agility 6, 0 / 1 / 2 enemies around | 83 / 67 / 50 % |
| Hand-off to a Sprinter, 0 / 1 / 2 enemies around | 100 / 83 / 67 % |
| A Sprinter's diving catch, 0 / 1 enemy around | 67 / 50 % |
| A Thrower's 10-step pass with no enemy around: simple / focused at +3 | 17 / 67 % |
| A Thrower's 6-step pass, marked by 2 enemies: simple / focused at +3 | 50 / 100 % |
| Chaos throw towards a lone Sprinter / a lone Thrower, no enemy near | 67 / 15 % |
| Chaos throw towards a Thrower ringed by 4 Sprinters, no enemy near | 97 % |
| Back from a knock-out: Bruiser / Thrower / Sprinter | 86 / 83 / 80 % |

## Paper playtest kit

A 21 × 21 grid with the disc of the French plan drawn on it, numbered tokens and a way to mark Health, a coin for the d2, a d4, a d6 (also the d3: 1-2, 3-4, 5-6), a d8 for bounces and a d12 for the kick-off (reroll the 12); for returns from a knock-out, a d6 or a d8 rerolled above the goblin's Health. A phone timer (90 s for the first games).

Every number above is tunable in `rules.toml`.
