# Format des fichiers Winamax (relevé sur corpus réel)

> **Corpus analysé** (anonymisé dans `fixtures/winamax/mtt/`) :
> | Fixture | Tournoi | Mains | Particularités |
> |---|---|---|---|
> | `space-ko/` | VELOCITY - SPACE KO (1174075270), 25/09/2026, semiturbo | 117 | 6-max, 1 table, sortie 222e/606, 1 bounty gagné |
> | `space-ko-3max-itm-reentry/` | OBELISK - TRIDENT SPACE KO (1173012730), 23/09/2026, turbo | 328 | **3-max**, **9 tables**, **re-entry**, **ITM 7e/1141**, split pots, heads-up à table 3-max |
>
> **Invariant de conservation des jetons vérifié sur 445/445 mains.** Timestamp du HandId = date de l'en-tête sur 445/445 mains.
> **Statut :** ✅ = observé et vérifié · ⚠️ = déduit, à confirmer · ❓ = non encore observé
> Règle R-FORMAT (CLAUDE.md) : toute règle ajoutée ici doit citer un fichier de `fixtures/`.

---

## 1. Constat majeur : les fichiers sont en ANGLAIS ✅
Le client Winamax de Frédéric est en **français**, mais les historiques de mains **et** les summaries sont écrits **en anglais**. Le parser cible donc le format anglais (ADR-005 révisée).

## 2. Emplacement et nommage

| Élément | Valeur | Statut |
|---|---|---|
| Racine des comptes | `%APPDATA%\winamax\documents\accounts\` (`C:\Users\<user>\AppData\Roaming\…`) | ✅ fourni par Frédéric |
| Dossier des historiques | `accounts\<PSEUDO>\history\` | ⚠️ casse exacte à confirmer |
| Fichier de mains | `AAAAMMJJ_<NOM>(<ID>)_real_holdem_no-limit.txt` (AAAAMMJJ = date de début du tournoi) | ✅ (parenthèses perdues à l'upload : ⚠️ à confirmer) |
| Fichier summary | même nom + `_summary.txt` | ✅ |
| **Un fichier par tournoi** | toutes les tables **et toutes les entrées (re-entries)** dans le même fichier | ✅ (9 tables et 2 entrées dans OBELISK) |
| `real` | argent réel ; ignorer tout ce qui n'est pas `_real_` | ⚠️ |

## 3. Encodage et structure ✅
- **UTF-8 sans BOM**, fins de ligne **LF**.
- Fichier de mains : mains séparées par **deux lignes vides** (`\n\n\n`), et le fichier se termine par `\n\n\n`. ⇒ Une main est complète quand elle est suivie de `\n\n\n`.
- Espaces en fin de ligne (`*** PRE-FLOP *** `, `won 3739 `, `You played 5min 27s `) ⇒ appliquer **`trim_end()` sur chaque ligne**.

## 4. Fichier de mains

### 4.1 En-tête ✅
```
Winamax Poker - Tournament "OBELISK - TRIDENT SPACE KO" buyIn: 1.80€ + 0.20€ level: 1 - HandId: #5038051313141678275-16-1790176801 - Holdem no limit (3/10/20) - 2026/09/23 15:20:01 UTC
```
| Champ | Remarque |
|---|---|
| `Tournament "<nom>"` | nom **sans** l'ID |
| `buyIn: A€ + F€` | A = prize **+ bounty cumulés** (1,80 = 0,80 + 1), F = fee. **Le découpage vient du summary.** |
| `level: N` | n° de niveau (1-based) = index dans `Levels` du summary (observé de 1 à 34) |
| `HandId: #X-Y-Z` | ⚠️ **X = identifiant de la TABLE** (et non du tournoi : 9 valeurs de X pour les 9 tables d'OBELISK) ; Y = n° de main **sur cette table** (il n'est pas unique dans le tournoi) ; Z = **timestamp Unix (s)** de la main. **Clé d'unicité = chaîne complète `X-Y-Z`.** |
| `(ante/SB/BB)` | avec ante : `(3/10/20)`, `(25/100/200)`. ❓ sans ante, format à observer |
| date | `AAAA/MM/JJ HH:MM:SS UTC` |

### 4.2 Table ✅
```
Table: 'OBELISK - TRIDENT SPACE KO(1173012730)#0194' 3-max (real money) Seat #3 is the button
```
- `<nom tournoi>(<ID tournoi>)#<n° table>` : **seule source de l'ID tournoi** dans le fichier de mains. Regex : `\((\d+)\)#(\d+)'`.
- Formats observés : **6-max** et **3-max** (Trident).
- Changement de table : le nom et le préfixe X du HandId changent (9 tables : `#0194 → #0155 → … → #0003 → #0000`).
- ⚠️ `#0000` est la dernière table observée, mais le Hero y finit 7e alors qu'elle compte 3 joueurs : **ne pas en déduire « table finale »**.

### 4.3 Sièges ✅
```
Seat 1: AK-47-7.62mm (500, 1€ bounty)
Seat 1: CeBoLLuSSS (45494, 31.41€ bounty)
```
- Regex : `^Seat (\d+): (.+) \((\d+)(?:, ([\d.]+)€ bounty)?\)$`. Les pseudos contiennent des espaces, des points, des tirets, des underscores et des chiffres.
- **Bounty courant** avec décimales (`18.38€`). Il évolue au fil des KO.
- **Tapis de départ** : 20 000 (VELOCITY) et 500 (OBELISK). Il varie selon le tournoi.
- ⚠️ **Joueurs assis mais non distribués** (aucune ligne posts ni action) : 7 cas dans chaque fichier. Ils sont **exclus de la main** (positions, stats).
- Nombre de joueurs dans la main : de 2 à N-max. **Heads-up à une table 3-max** : 48 mains dans OBELISK.

### 4.4 Sections ✅
`*** ANTE/BLINDS ***` · `*** PRE-FLOP ***` · `*** FLOP *** [a b c]` · `*** TURN *** [a b c][d]` · `*** RIVER *** [a b c d][e]` · `*** SHOW DOWN ***` · `*** SUMMARY ***`
Après un all-in, les streets restantes sont listées sans action.

### 4.5 Actions ✅

| Ligne | Sémantique |
|---|---|
| `P posts ante N` | ante (hors mise de street) |
| `P posts small blind N` / `P posts big blind N` | comptent dans la mise préflop |
| `Dealt to Hero [Xx Yy]` | **uniquement le Hero** ⇒ identifie le pseudo Hero |
| `P folds` / `P checks` | |
| `P calls N` | N = montant **ajouté** |
| `P bets N` | |
| `P raises X to Y` | Y = **total** de la street ; ajout = Y − déjà misé sur la street (blindes incluses) |
| suffixe `and is all-in` | sur `calls`, `bets`, `raises` |
| `P shows [Xx Yy] (<libellé main>)` | libellé informatif (`One pair : Jacks`, `Straight 5 high`, `Full of 9 and Queens`, `Quads of Kings`…) |
| `P collected N from pot` | **pot partagé = plusieurs lignes `collected … from pot`** (7 split pots dans OBELISK) |
| `P collected N from main pot` / `… from side pot K` | pots multiples |

**Toujours non observées** ❓ : `mucks`, `doesn't show`, `is sitting out`, `Uncalled bet … returned`, straddle, timeout/déconnexion, run it twice. Une ligne inconnue produit `ParseError UNKNOWN_LINE`.

### 4.6 ⚠️ Mises non suivies laissées dans le pot ✅
Il n'y a pas de ligne de remboursement. L'excédent non suivi est compté dans `Total pot` et revient via `collected`, parfois sous la forme `side pot K`.
**Invariant (445/445) :** Σ contributions = Σ `collected` = `Total pot`.
⇒ Le **pot disputé** (utilisé pour l'EV) = Total pot − excédent non suivi.

### 4.7 SUMMARY ✅
```
Total pot 6080 | No rake
Board: [Td 2s Js 7s 3s]
Seat 2: P (small blind) showed [4d Jd] and won 3040 with One pair : Jacks
Seat 3: Hero (small blind) (button) won 241
Seat 1: Hero (big blind) showed [Ad As] and lost with Two pairs : Aces and 4
```
- Rôles : `(button)`, `(small blind)`, `(big blind)` et **`(small blind) (button)`** en heads-up (le bouton poste la SB).
- Seuls les gagnants et les joueurs allés à l'abattage sont listés.

### 4.8 Knockouts et re-entry dans le fichier de mains ⚠️
- **Aucune ligne de KO.** On déduit le KO : joueur all-in qui perd tout, absent de la main suivante ; le bounty du vainqueur augmente.
- **Re-entry (OBELISK, mains 16 → 17)** : le Hero est éliminé à la main 16 (table `#0194`, 15:20:01). 51 s plus tard, il réapparaît sur une **autre table** (`#0155`) avec le **tapis de départ** (500) et un bounty réinitialisé à 1 €.
  ⇒ **Règle de découpage des entrées :** une nouvelle entrée commence quand le Hero réapparaît après avoir été éliminé (tapis à 0 à la fin d'une main). Chaque main est rattachée à une `entry_no` (1, 2, …) dans l'ordre chronologique.

## 5. Fichier summary ✅

### 5.1 Structure : **un bloc par entrée**
Le fichier contient **un bloc par entrée du Hero**, dans l'ordre chronologique :
- blocs séparés par **une ligne vide** ;
- fichier terminé par `\n\n` ;
- le bloc d'une entrée éliminée **pendant les inscriptions tardives** porte le suffixe **` - Late Registration`** sur sa première ligne.

```
Winamax Poker - Tournament summary : OBELISK - TRIDENT SPACE KO(1173012730) - Late Registration
Player : Hero
Buy-In : 0.80€ + 1€ + 0.20€
Registered players : 812
Mode : tt
Type : knockout
Speed : turbo
Flight ID : 0
Levels : Levels : [10-20:3:600:holdem-no-limit,12-25:3:150:holdem-no-limit,…]
Prizepool : 2002€
Tournament started 2026/09/23 15:15:00 UTC
You played 5min 27s
You finished in 737th place

Winamax Poker - Tournament summary : OBELISK - TRIDENT SPACE KO(1173012730)
Player : Hero
…
Registered players : 1141
…
Prizepool : 1529.60€
Tournament started 2026/09/23 15:15:00 UTC
You played 1h 36min 43s
You finished in 7th place
You won 34.79€ + Bounty 17.40€
```

### 5.2 Lignes

| Ligne | Mapping | Statut |
|---|---|---|
| `Tournament summary : <nom>(<ID>)[ - Late Registration]` | nom, ID (clé de rattachement), flag `late_reg_bust` | ✅ |
| `Player : <pseudo>` | pseudo Hero | ✅ |
| `Buy-In : P€ + B€ + F€` | prize / bounty / fee pour les KO. ❓ non-KO : probablement `P€ + F€` | ✅ KO |
| `Registered players : N` | ⚠️ **instantané au moment de l'écriture du bloc** (812 pendant les inscriptions tardives, puis 1141 à la fin). Pour le tournoi, prendre la valeur du **dernier bloc**. | ✅ |
| `Mode : tt` | ❓ sens inconnu | ⚠️ |
| `Type : knockout` | ❓ autres valeurs à observer | ✅ KO |
| `Speed : semiturbo` / `turbo` | vitesse | ✅ |
| `Flight ID : 0` | multi-flights ; 0 = aucun | ⚠️ |
| `Levels : Levels : [SB-BB:ante:durée:jeu,…]` | structure complète. Durée en s : niveau 1 = 1500 (semiturbo) ou 600 (turbo), puis 300 ou 150. ⚠️ Des durées valent `1` sur certains niveaux (niveau 7 ou 13 selon le bloc) : **pause probable**. Ignorer les durées ≤ 1 dans les calculs de temps. | ✅ format |
| `Prizepool : X€` | ⚠️ sens à confirmer : 616 € pour 606 inscrits et 1529,60 € pour 1141 (cohérent avec la part prize × nombre d'entrées, re-entries comprises). Le bloc Late Registration affiche 2002 € (garantie ?). **Ne pas utiliser pour le ROI** ; affichage seulement, depuis le dernier bloc. | ⚠️ |
| `Tournament started … UTC` | début du tournoi | ✅ |
| `You played <[Hh ][Mmin ]Ss>` | **durée de cette entrée** ⇒ $/heure | ✅ |
| `You finished in <N>th place` | place de **cette entrée** (suffixe `th` même pour 7) | ✅ |
| `You won X€ + Bounty Y€` | **ITM** : X = gains prize pool, Y = bounties | ✅ |
| `You won Bounty Y€` | hors ITM, avec bounties | ✅ |
| *(ligne absente)* | hors ITM, sans bounty | ✅ |
| ❓ ITM sans bounty (tournoi classique) | probablement `You won X€` | ❓ |
| ❓ ticket gagné / buy-in payé par ticket | à observer | ❓ |
| ❓ places payées | absentes des 3 blocs | ✅ absence |

### 5.3 Calcul du résultat d'un tournoi (validé sur OBELISK)
- **Entrées** = nombre de blocs (2) ; **coût** = 2 × (0,80 + 1 + 0,20) = **4,00 €**, dont 0,40 € de frais.
- **Gains** = Σ des `You won` de tous les blocs = 34,79 € (prize) + 17,40 € (bounties) = **52,19 €**.
- **Profit** = **+48,19 €** ; ROI = +1 204,75 %.
- **Place retenue pour le tournoi** = celle du **dernier bloc** (7e) ; ITM = oui.
- **Durée jouée** = Σ `You played` = 5 min 27 s + 1 h 36 min 43 s.

## 6. Conséquences pour le PRD (intégrées)
1. La clé d'unicité d'une main est le **HandId complet**. Le préfixe X identifie la table, pas le tournoi.
2. **Re-entries :** summary = un bloc par entrée ; dans le fichier de mains, l'entrée se déduit de la réapparition du Hero après son élimination. Ajout de la table `tournament_bullets` (PRD §15).
3. **Positions :** gérer le **3-max** et le **heads-up à une table 3-max** (BTN = SB).
4. **Pots partagés :** plusieurs `collected … from pot`.
5. **Phases (méthode A) :** structure `Levels` réelle ; ignorer les durées ≤ 1.
6. **Table finale :** pas de marqueur fiable pour l'instant. Le % de tables finales se calcule sur la **place** (≤ taille de la table finale paramétrée).

## 7. Fichiers encore nécessaires (M0-4)
Déjà couverts : ✅ ITM · ✅ re-entry · ✅ changements de table · ✅ KO (Space KO).
Reste à fournir (paires mains + summary) :
1. **tournoi classique sans KO** (idéalement ITM) ;
2. **Mystery KO** ;
3. tournoi **payé avec un ticket** ;
4. **satellite** ayant rapporté un ticket ;
5. **freeroll** ;
6. une **vraie table finale** (9 ou 6 joueurs) ;
7. si possible, un fichier copié **pendant** un tournoi en cours (lecture incrémentale).
