# BACKLOG — Graphite V1

Statuts : `TODO` · `DOING` · `DONE` · `BLOCKED(raison)`
Priorités (MoSCoW) : **M** = Must, **S** = Should, **C** = Could (glisse en V1.1 si retard)
Références : `§x` = section de `docs/PRD.md`.

## Planning (V1 le 31/12/2026)

| Jalon | Semaines | Dates | Livrable démontrable |
|---|---|---|---|
| M0 Fondations | S1 | 28/09 → 04/10 | Repo, CI verte, app Tauri vide aux couleurs Graphite, corpus reçu et format documenté |
| M1 Parser Winamax | S2–S3 | 05/10 → 18/10 | 100 % du corpus parsé, snapshots validés |
| M2 Stockage + import en masse + logs | S4 | 19/10 → 25/10 | Import d'un dossier complet en base, écran Logs |
| M3 Temps réel + sessions | S5 | 26/10 → 01/11 | Main visible < 2 s pendant le jeu |
| M4 Moteur de stats | S6–S7 | 02/11 → 15/11 | Stats Hero correctes par position, profondeur et phase |
| M5 Résultats MTT + EV all-in | S8 | 16/11 → 22/11 | KPIs au centime, équité validée |
| M6 Écrans principaux | S9–S10 | 23/11 → 06/12 | Accueil, Résultats, Tournois, Mains, filtres |
| M7 Analyse | S11–S12 | 07/12 → 20/12 | Rapports, Leaks, Replayer, tags, classification |
| M8 Finitions + release | S13–S14 | 21/12 → 31/12 | Paramètres, sauvegardes, i18n, perf, **v1.0.0 portable** |

Le calendrier est serré. En cas de retard, les stories **C** glissent d'abord en V1.1, puis les **S**.

---

## M0 — Fondations

### M0-1 · Création du repo GitHub · M · `DONE` — repo créé, premier commit poussé, `main` protégée (PR obligatoire, 0 approbation requise, pas de push direct, admins inclus) le 26/09.
**Action de Frédéric (manuelle, ~15 min), guidée par Claude Code :**
1. Créer un compte GitHub (si besoin) et installer **Git for Windows** et **GitHub CLI** : `winget install Git.Git GitHub.cli`.
2. `gh auth login` (HTTPS, via le navigateur).
3. Laisser Claude Code exécuter : `gh repo create graphite --private --clone`.
4. Dans GitHub, Settings > Branches : protéger `main` (PR requise + CI verte).

**Critères d'acceptation :**
- Repo privé `graphite` créé.
- Premier commit contenant `CLAUDE.md`, `docs/PRD.md`, `docs/BACKLOG.md`, `.gitignore` (Rust/Node/Tauri + `graphite-data/`, `*.db`, `fixtures/raw/`).

### M0-2 · Toolchain et squelette du workspace · M · `DONE` — le 26/09.
- Installé : Rust stable MSVC (rustup), VS Build Tools 2022 (C++), Node **22.17** (déjà présent, pas de LTS 20 réinstallé — à surveiller si un module natif l'exige), pnpm, `just`. `sccache` non installé (optionnel).
- Workspace Cargo créé (10 crates vides de la §7.3, lint `clippy::pedantic` en warn), `src-tauri` (Tauri 2 via `cargo tauri init`) et `ui` (Vite + React 19 + TS strict + Tailwind v4 + TanStack Query/Table + Zustand + i18next + ECharts + Vitest/Testing Library). **shadcn/ui non initialisé** : ses presets de police (Geist/Lucide) entrent en conflit avec les polices Inter/JetBrains Mono du PRD §16 — à faire dans M0-5 avec les tokens de design.
- `justfile` créé avec les commandes de `CLAUDE.md` §4 (`just perf` dépend de `tools/perf.ps1`, pas encore créé — prévu en M8-4).
- Bug résolu : conflit de versions `windows-core` (0.61.2 vs 0.62.2) entre `tauri-runtime` 2.12.0 et `tauri-runtime-wry` 2.11.4 — épinglé `tauri-runtime = 2.11.3` dans `Cargo.lock`.

**CA :**
- `just dev` ouvre une fenêtre « Graphite ». ✅ (vérifié, process tué après confirmation)
- `just check` passe. ✅ (fmt, clippy, cargo test, ui lint/typecheck/test)
- Profil `dev` configuré (`[profile.dev.package."*"] opt-level = 1`). ✅

### M0-3 · CI et release GitHub Actions · M · `DONE` — le 27/09.
- CI verte sur `main` (job `linux` ~10-20s, job `windows` ~3min avec cache chaud). Les deux checks sont désormais obligatoires dans la protection de `main`.
- `release.yml` validé de bout en bout sur le tag `v0.0.1` → [release GitHub](https://github.com/FredM-AI/TrackFit/releases/tag/v0.0.1) avec `graphite-v0.0.1-portable.zip`. Trois bugs corrigés en cours de route :
  1. chemin de l'exe faux (`src-tauri/target/...` au lieu de `target/...` : un workspace Cargo partage un seul `target/` à la racine) ;
  2. `--features analytics-duckdb` retiré du build (la feature n'existe pas encore, elle arrive en M7-6) ;
  3. `permissions: contents: write` manquant sur le workflow (le `GITHUB_TOKEN` par défaut est en lecture seule).

**CA :**
- CI verte sur `main`. ✅
- Un tag `v0.0.1` produit `graphite-v0.0.1-portable.zip` qui démarre sur la machine de Frédéric. ✅ (build validé sur CI ; démarrage à confirmer par Frédéric sur sa machine)
- `ci.yml` :
  - job `linux` (ubuntu-latest) : fmt, clippy et tests des crates pures (`gr-core`, `gr-parser-*`, `gr-stats`, `gr-equity`) ;
  - job `windows` (windows-latest) : build Tauri, tests complets et tests `ui` ;
  - cache `Swatinem/rust-cache` + cache pnpm ;
  - déclenchement sur push et PR.
- `release.yml` : sur tag `v*`, `tauri build --no-bundle` avec `--features analytics-duckdb`, zip de l'exe + README, création d'une GitHub Release.

**CA :**
- CI verte sur `main`.
- Un tag `v0.0.1` produit `graphite-v0.0.1-portable.zip` qui démarre sur la machine de Frédéric.

**Note sur le quota :** repo privé = 2 000 min/mois gratuites, et les minutes Windows comptent double. Le job Linux fait l'essentiel des tests. Le job Windows ne tourne que sur les PR et `main`.

### M0-4 · Corpus réel et documentation du format Winamax · M · `DONE` — corpus élargi (620+ tournois réels, pseudo `AFeuDoux`) déposé et anonymisé le 27/09. Liste des inconnues validée par Frédéric le 27/09 (les 3 cas encore manquants sont différés, voir « Idées / à trier »).
**Déjà fait :**
- `docs/formats/winamax.md`, `tools/anonymize.py` (porté en Python, exécuté via un Python installé pour l'occasion — `just` ne le requiert pas, seul Claude Code s'en sert ponctuellement).
- 5 nouveaux fixtures anonymisés à partir du corpus réel déposé dans `fixtures/raw/` (1056 fichiers, 528 tournois, gitignoré) : `classic-itm/` (classique sans KO, ITM), `mystery-ko/` (Mystery KO), `freeroll/` (freeroll MTT 16 580 inscrits), `final-table-heads-up/` (table finale + heads-up réel sur 218 inscrits), `multi-flight-day1/` (multi-flights, `Flight ID` non nul). Plus un edge-case `escaped-currency-in-title/` (nom de tournoi mal ré-encodé).
- 7/7 cas du §7 de `winamax.md` traités : 4 couverts avec fixture (classique, Mystery KO, freeroll, table finale/heads-up), 3 confirmés **toujours non observables** dans ce corpus de 620+ tournois (ticket payé, satellite ticket gagné, fichier copié en cours de tournoi — voir §7 de `winamax.md` pour le détail).
- Bonus hors périmètre initial : multi-flights documentés (`Type: flight`, `Flight ID` réel), formats hors scope D2 identifiés dans le corpus brut (Expresso `Mode: sng`, Omaha `*_real_omaha_pot-limit*`) — à faire détecter/rejeter par le parser plutôt qu'à parser.
- M1-1 à M1-6 peuvent démarrer sans blocage ; seules les parties « payé/gagné via ticket » de M1-6/M5-2 resteront non implémentables tant qu'un exemple n'existe pas.

**Action de Frédéric :** copier dans `fixtures/raw/` (ignoré par git) le contenu de `%APPDATA%\winamax\documents\accounts\<pseudo>\History\`. Le corpus doit contenir au minimum 300 mains et 20 summaries couvrant : classique, PKO, Mystery KO, Space KO, freeroll, satellite avec ticket gagné, tournoi joué via ticket, re-entry, heads-up final, all-in multiway avec side pots.

**Claude Code :**
- écrire `tools/anonymize` (Rust bin ou script) : pseudos → `P0001…` de façon déterministe, Hero → `Hero` ;
- produire `fixtures/winamax/mtt/<format>/` ;
- compléter **`docs/formats/winamax.md`** : nommage des fichiers, encodage/BOM, en-têtes, toutes les lignes d'action, sections, summary, bounties, tickets, re-entries. Préciser si le nombre de joueurs restants ou les places payées apparaissent (impact §10.5), ainsi que les formats de titre de fenêtre si connus.

**CA :**
- Chaque type de ligne rencontré est listé avec un exemple réel anonymisé.
- La liste des inconnues restantes est rédigée et validée par Frédéric.

### M0-5 · Shell UI et design tokens · S · `DONE` — le 27/09.
- `ui/src/styles/tokens.css` (§16) : palette monochrome stricte, option couleurs sémantiques discrètes désactivée par défaut (`data-semantic-colors`). Polices Inter Variable et JetBrains Mono Variable embarquées via `@fontsource-variable/*` (locales, pas de CDN — R-NET).
- Layout (`ui/src/app/AppShell.tsx`) : barre latérale (icônes `lucide-react` + libellés), barre de filtres vide (placeholder, remplie en M6-1), barre d'état (statut import, mains du jour, dernière main, sélecteur de langue).
- Routeur : **TanStack Router** (file-based, `ui/src/routes/`), choisi pour la cohérence avec TanStack Query/Table déjà présents — décision validée avec Frédéric (absente de CLAUDE.md/PRD). Routes vides pour les 8 écrans (`ui/src/screens/`) + Logs, code-splittées automatiquement.
- i18next FR/EN branché (`ui/src/lib/i18n.ts`, `ui/src/locales/{fr,en}/common.json`) avec sélecteur de langue dans la barre d'état.
- Règle ESLint `i18next/no-literal-string` activée (`markupOnly`, désactivée sur les fichiers de test).

**CA :**
- Navigation fonctionnelle. ✅ (`just dev` : fenêtre « Graphite », 9 liens de la barre latérale, testé par Frédéric)
- Aucune chaîne en dur (règle ESLint `i18next/no-literal-string` active). ✅ (`pnpm lint` / `just check` verts)

---

## M1 — Parser Winamax (§8.1–8.3, `docs/formats/winamax.md`)

### M1-1 · Types de domaine `gr-core` · M · `DONE` — le 27/09.
Card, Rank, Suit, Street, ActionKind, Position, Money(cents), Chips, HandRecord, SeatInfo, ActionRecord, PotResult, TournamentSummary, KoType. `ActionRecord` porte aussi `pot: Option<PotKind>` (pot vise par un `collected`) et `shown_cards`/`shown_label` (pour `Shows`), afin de rester fidele au texte brut sans logique de calcul (celle-ci arrive avec `gr-parser-winamax` et `gr-stats`).

**CA :**
- Sérialisation serde. ✅ (roundtrip testé sur `ActionRecord`, `HandRecord`, `TournamentSummary`)
- Parsing et affichage des cartes (`"Ah"`). ✅ (`FromStr`/`Display` sur `Card`, erreurs typées via `thiserror`)
- Tests unitaires. ✅ (14 tests, `cargo clippy -p gr-core -- -D warnings` et `cargo fmt --check` verts)

### M1-2 · Trait `RoomParser` et détection · M · `DONE` — le 27/09.
`gr-parser-api` : trait `RoomParser` (`detect`/`parse_hand`/`parse_summary`), `Detection`/`Room`/`Language`, `ParseError`/`ParseErrorCode` (`UNKNOWN_LINE`, `CHIP_MISMATCH`), `strip_bom`. `gr-parser-winamax` : `WinamaxParser::detect` (signature commune main/summary `"Winamax Poker - Tournament"`). Repli Windows-1252 **non implémenté** : aucun fichier du corpus (620+ tournois) n'en a jamais eu besoin — à ajouter seulement si un cas se présente. `parse_hand`/`parse_summary` restent à implémenter (M1-3 à M1-6) ; `WinamaxParser` n'implémente donc pas encore le trait complet, seulement `detect`.

**CA :** les fichiers du corpus sont détectés `winamax` à 100 %. ✅ (test sur tous les fichiers commités de `fixtures/winamax/**`, 8 tournois)

### M1-3 · Découpage des mains et blocs incomplets · M · `DONE` — le 27/09.
`gr-parser-winamax::split_hand_blocks(text) -> (Vec<&str>, usize)` : decoupe sur le separateur `\n\n\n`, renvoie les blocs complets + l'offset (octets) où reprendre une lecture incrementale (PAR-3).

**CA :**
- Un fichier tronqué au milieu d'une main renvoie N mains complètes, plus un offset qui s'arrête avant la main incomplète. ✅
- Tests avec des troncatures à chaque ligne d'une main (property test). ✅ (troncature à chaque ligne de la 2e main **et** de la dernière main du fixture réel `space-ko/`, 117 mains)

### M1-4 · En-tête, table, sièges, blindes/antes · M · `DONE` (PAR-4/5/6) — le 27/09.
`WinamaxParser::parse_hand` (parsing manuel, sans dépendance `regex` — formats assez rigides pour s'en passer) : en-tête (nom, `HandId`, level, ante/SB/BB, date → epoch ms via un petit convertisseur maison, validé contre le timestamp Unix du `HandId` d'une vraie main), table (nom, ID tournoi, taille, bouton), sièges (pseudo, tapis, bounty). Actions/board/pots restent vides (M1-5) ; `dealt_in` vaut toujours `true` pour l'instant (l'exclusion des joueurs non distribués dépend du flux d'actions, affinée en M1-5).
**Bug découvert et corrigé au passage :** 6 des fixtures anonymisées en M0-4 avaient dérivé en CRLF dans ma copie de travail locale (le blob git était resté en LF, correct) — corrigé directement sur disque, sans impact sur le dépôt.

**CA :**
- Snapshots validés sur tout le corpus. ✅ (`insta`, un snapshot par fichier de mains — 9 fichiers, 1679 mains réelles + 1 main synthétique)
- Les pseudos avec espaces et caractères spéciaux sont couverts par un edge-case. ✅ `fixtures/winamax/edge-cases/special-pseudos/` — fixture **synthétique** (l'anonymisation ayant effacé ces caractères des pseudos réels dans tous les autres fixtures), pseudos fictifs avec espace (« Jean Dupont ») et points/tirets/underscores (« Marie-Claire.99_x »)

### M1-5 · Actions, streets, board, showdown, pots · M · `DONE` (PAR-7 à PAR-10) — le 27/09.
Parsing complet des actions (posts/folds/checks/calls/bets/raises/shows/collected, suffixe `and is all-in`), streets (board reconstruit en concatenant les groupes `[...]` de chaque en-tête `*** FLOP/TURN/RIVER ***`), pots (`PotResult.winners` en `Vec<(pseudo, part)>` pour gérer les pots partagés à parts inégales). `uncalled_excess` calculé par une **formule arithmétique générale** (écart entre les deux plus fortes contributions totales de la main) plutôt qu'en détectant le texte « side pot » — valable pour n'importe quel nombre de side pots, vérifié sur un cas réel multiway. Bug trouvé et corrigé en cours de route : le passage de `*** ANTE/BLINDS ***` à `*** PRE-FLOP ***` ne doit **pas** réinitialiser les mises de la street (les blindes restent la mise de référence pour les relances preflop) — seul le passage vers flop/turn/river en ouvre une nouvelle.

**CA :**
- Invariant de conservation PAR-11 vérifié sur 100 % du corpus. ✅ (9 fichiers, 1679+ mains réelles, aucune `ChipMismatch`/`UnknownLine`)
- Side pots corrects sur le fixture multiway (main 49 du fixture Space KO : `side pot 1` = excédent non suivi). ✅ (main pot 35015 + side pot 1 = 3195, `uncalled_excess` = 3195)
- Joueurs assis mais non distribués exclus (mains 1 et 2 du fixture Space KO). ✅
- Pots partagés (7 cas dans le fixture OBELISK) et heads-up à une table 3-max (`(small blind) (button)`). ✅ (7/7 détectés ; le heads-up ne demande aucun code dédié, il découle de l'ordre séquentiel des actions dans le texte)
- Calcul de l'excédent non suivi par main (champ `uncalled_excess`), utilisé par l'EV all-in (M5-4). ✅

### M1-6 · Parser de summaries · M · `DONE` (PAR-12, §8.6) — le 27/09.
`WinamaxParser::parse_summary` : découpage en blocs (un par entrée, séparés par une ligne vide), buy-in à 2 ou 3 composantes (`P€+F€` ou `P€+B€+F€`), les 3 formes de `You won` (prize seul / bounty seul / les deux), durée `Xh Ymin Zs` (tout sous-ensemble), place (suffixe ordinal ignoré). `RoomParser` est désormais **entièrement implémenté** pour `WinamaxParser` (detect + parse_hand + parse_summary tous présents).
**Précision de périmètre :** le « découpage des mains par `entry_no` » (réapparition du Hero après élimination) cité dans la CA relève en réalité de la corrélation mains↔summary, explicitement prévue en **M2-6** (« les re-entries sont comptées ») — `parse_summary` produit un `entry_no` par bloc (1, 2, … dans l'ordre du fichier), mais le rattachement aux mains individuelles n'est pas fait ici.

**CA :**
- Buy-in décomposé prize/bounty/fee. ✅ (y compris freeroll `0€+0€`)
- Summaries **multi-blocs** (un par entrée), suffixe ` - Late Registration`, `You won X€ + Bounty Y€` / `You won Bounty Y€` / ligne absente. ✅
- Découpage des mains par `entry_no` (réapparition du Hero après élimination). ⚠️ différé en M2-6 (voir précision de périmètre ci-dessus)
- Test chiffré : OBELISK → 2 entrées, coût 4,00 €, gains 52,19 €, profit +48,19 €, place 7. ✅ (exact, calculé à partir du `TournamentSummary` parsé)
- Tickets : uniquement quand le fixture sera fourni. ✅ (toujours aucun exemple, non implémenté — cf. Idées/à trier)
- Snapshots validés. ✅ (9 fichiers summary, `insta`)

### M1-7 · Robustesse et benchs du parser · M · `DONE` (PAR-14/15) — le 27/09. **M1 (Parser Winamax) est entièrement terminé.**
7 tests `proptest` (512 cas chacun) : octets/chaînes arbitraires sur `detect`/`parse_hand`/`parse_summary`/`split_hand_blocks`, troncature de la vraie main OBELISK à *tout* octet (pas seulement les frontières de ligne comme M1-3), et corruption caractère par caractère. **3 vrais risques de panique trouvés et corrigés** au passage (aucun n'était couvert par les tests précédents, qui n'utilisaient que des nombres réalistes) :
1. `Money`/`Chips` faisaient `+`/`-` sur `i64` bruts → overflow/underflow paniquent en debug avec des montants fuzzés énormes. Passés en arithmétique saturante (`saturating_add`/`saturating_sub`).
2. `parse_euros_to_cents` faisait `whole * 100 + cents` sans protection → passé en `checked_mul`/`checked_add` (renvoie `None`, donc une `ParseError`, plutôt que de paniquer).
3. `parse_duration_seconds` faisait `heures * 3600` sur `u32` sans protection → passé en `saturating_mul`/`saturating_add`.
4. Deux `expect()` restaient dans `parse_hand` (hors tests, violation de R-NOPANIC) sur le montant/pot d'un `Collected` — remplacés par un `if let` qui ignore silencieusement plutôt que de paniquer si l'invariant interne venait à être brisé.

**CA :**
- Fuzz léger (proptest) sans panique. ✅ (7 tests, voir ci-dessus)
- Bench `criterion` ≥ 5 000 mains/s mesuré sur la machine de Frédéric. ✅ **~30 600 mains/s** (29,5–31,7 k élém/s), mesuré sur le fixture réel OBELISK (328 mains, 3-max/re-entry) — largement au-dessus de la cible.
- Chaque `ParseError` porte un code et une ligne. ✅ (garanti par construction : `code`/`line_no` sont des champs non-optionnels de la struct)

**Durcissement post-M1 (M3-1, 28/09) — trouvé par le premier import réel de Frédéric (61 375 mains, 634 tournois) :** sur 3979 erreurs d'import remontées, 3 vrais bugs de grammaire identifiés et corrigés, chacun avec une fixture dédiée sous `fixtures/winamax/edge-cases/` (R-FORMAT) :
1. **Apostrophe dans le nom du tournoi** (`Hold'em [180 Max]`, `Deepstack Hold'em`) : la ligne `Table:` s'arrêtait à la première apostrophe pour trouver le guillemet fermant, tronquant le nom. ~1128 mains réelles affectées, le bug le plus impactant. Corrigé (`rfind("' ")` au lieu du premier `'`).
2. **Blindes sans ante `(SB/BB)`** au niveau 1 de certains freerolls (déjà noté « à observer » en §4.1, maintenant confirmé) : le parser n'acceptait que 3 composantes. Corrigé (2 ou 3 composantes acceptées, ante = 0 si absent).
3. **Pseudo contenant un mot d'action** (`Big Bets 99`) : la recherche du verbe par sous-chaîne (`rfind(" bets ")`) matchait à tort dans le pseudo lui-même avant d'atteindre le vrai verbe. Corrigé en profondeur : le pseudo est désormais isolé **en premier** par correspondance avec la liste des pseudos connus de la main (lignes `Seat N:`, toujours lues avant les actions), pas par recherche de verbe.

Non corrigé, noté dans « Idées / à trier » : ~2765 mains Omaha (`_real_omaha_pot-limit`) et quelques Expresso échouent encore proprement (`UNKNOWN_LINE`, aucune donnée corrompue) — hors périmètre V1 (D2), le PRD §6 prévoyait déjà que `detect()` les rejette silencieusement plutôt que de les tenter, jamais implémenté.

---

## M2 — Stockage, import en masse, logs (§8.4, §15, §13.10)

### M2-1 · Migrations SQLite initiales · M · `DONE`
DDL §15 en `0001_init.sql` ; PRAGMA WAL ; pool de lecture et writer unique.

**CA :**
- Base créée au premier lancement dans le dossier de données (ADR-007, mode portable compris). ✅ `gr-store::paths::resolve_data_dir` implémente la logique ADR-007 (dossier `graphite-data/` existant ou lecteur amovible → mode portable ; sinon `%LOCALAPPDATA%\Graphite\`, avec repli portable si la variable est absente), testée en pur (4 tests). `Store::open` crée le dossier de données et le fichier `graphite.db` s'il n'existe pas encore (test `creates_the_database_file_on_first_launch`, `creates_the_data_dir_when_it_does_not_exist_yet`).
- Tests de migration. ✅ `gr-store::migrate` : création de la table `schema_migrations`, application idempotente (`is_idempotent_when_run_twice`), et rejet d'une migration déjà appliquée mais modifiée (`rejects_a_tampered_already_applied_migration`, empreinte non cryptographique — protège R-SCHEMA contre une dérive silencieuse). PRAGMA `journal_mode=WAL`/`foreign_keys=ON`/`synchronous=NORMAL` appliqués à chaque connexion (writer + chaque connexion du pool `r2d2`), contrainte FK vérifiée par un test dédié. 12 tests au total, `cargo test/clippy/fmt --workspace` verts.

### M2-2 · Repositories et insertion par lots · M · `DONE`
Insertion `hands`, `hand_players` (sans flags de stats pour l'instant), `actions`, `hand_raw` (zstd) et `tournaments` provisoires.

**CA :**
- Dédoublonnage `UNIQUE(room, hand_id)` testé. ✅ `gr-store::repo::insert_hands` utilise `INSERT OR IGNORE` sur `hands` et lit `changes()` : une main déjà présente (même `room_id`/`room_hand_id`) est comptée dans `duplicates` et n'écrit ni `hand_players`, ni `actions`, ni `hand_raw` une seconde fois (testé avec une vraie main du corpus OBELISK, ré-insérée telle quelle).
- Transaction par lot de 500 mains. ✅ `BATCH_SIZE = 500` ; `insert_hands` découpe la tranche reçue en lots (`chunks(BATCH_SIZE)`), chacun dans sa propre transaction SQLite, avec récupération/création du tournoi provisoire (`status = 'PROVISIONAL'`) une fois par lot. Testé sur un lot fractionné en plusieurs transactions successives (aucune main perdue ni dupliquée).
- Insertion testée de bout en bout contre une vraie main parsée du corpus (`fixtures/winamax/mtt/space-ko-3max-itm-reentry/`) : `hands`, `hand_players`, `actions` et `hand_raw` (compressé zstd) tous peuplés et cohérents en nombre de lignes. `players`/`rooms` upsertés par `(room_id, screen_name)`/`code`. Positions, profondeurs, `hand_class`, `net_chips`, showdown et flags de stats restent volontairement hors périmètre (M4).

### M2-3 · Import en masse · M · `DONE`
Sélection de dossiers ou de fichiers, progression (événement IPC), annulation, rapport final.

**CA :**
- Import du corpus complet sans erreur. ✅ `gr-ingest::run_import` (nouveau crate) : découverte récursive des fichiers `.txt` (hors `_summary.txt`, docs/formats/winamax.md §2), parsing + insertion par lots via `gr-store`. Testé sur tout le corpus committé (`fixtures/winamax/`) : 0 échec, 0 doublon au premier passage.
- Réimport = 0 insertion, N doublons. ✅ Deuxième passage sur le même corpus : `hands_inserted = 0`, `hands_duplicate` = total de la première passe.
- Premier jet de ≥ 1 000 mains/s sur 100 000 mains synthétiques. ✅ **Validé le 28/09 une fois M2-4 livré** : test `gr-ingest::import::tests::perf_100k_synthetic_hands_imports_at_least_1000_hands_per_second` (ignoré par défaut, `cargo test -p gr-ingest --release -- --ignored perf_100k`) — **1430 mains/s** mesurées sur 100 000 mains synthétiques (100% insérées, 0 échec) sur la machine de dev (Celeron N5095A). Marge modeste (×1,4) au-dessus de la cible ; à surveiller lors de la campagne de perf M8-4.

Câblage : `gr-ingest` est un crate pur (testable, aucune dépendance Tauri) ; `src-tauri` expose `import_paths`/`cancel_import` (état `ImportState` géré via `app.manage`, `Store` ouvert au démarrage via `resolve_data_dir`+ADR-007) et émet `import://progress`. Annulation coopérative testée (`CancelToken`, arrêt avant le fichier suivant). Aucun écran ne consomme encore ces commandes (sélecteur de dossier/fichier still à faire en UI, cf. M3-1) : câblage backend uniquement à ce stade.

### M2-4 · `gr-synth` : générateur de mains synthétiques · M · `DONE`
Générateur de tournois cohérents au format Winamax, basé **exclusivement** sur les motifs documentés en M0-4, avec une graine déterministe.

**CA :**
- `just synth N=1000000` génère environ 1 M de mains parsables à 100 %. ✅ Exécuté réellement (`cargo run -p gr-synth --release -- --count 1000000 --out ./target/synth`) : **1 000 000 mains générées dans 4412 fichiers en ~20 s**. Parsabilité à 100% revalidée par un test dédié (`gr-synth::corpus::tests::one_million_synthetic_hands_are_100_percent_parsable`, ignoré par défaut vu son coût) : 1 000 000/1 000 000 mains parsées sans erreur par `WinamaxParser`.

Conception : PRNG interne déterministe (SplitMix64, pas de nouvelle dépendance) ; motif de main généré = un tour préflop où tous les joueurs se couchent sauf la grosse blinde (`P folds` puis `P collected N from pot`), littéralement observé dans le corpus réel (docs/formats/winamax.md §4.5/§4.6) et suffisant pour PAR-4 à PAR-11 (le parser ignore les lignes `Board:`/`Seat N: … won` du `*** SUMMARY ***`, non requises). Tailles de table (3/6/7-max), structures de blindes de niveau 1 (`3/10/20`, `25/100/200`, `40/175/350`) et tapis de départ (500, 20000) tirés exclusivement parmi des valeurs déjà documentées ou déjà utilisées comme fixture de test (R-FORMAT). Heads-up (bouton = petite blinde) généré et vérifié explicitement. Un tournoi = un fichier, une seule table, pas de re-entry ni de summary (hors périmètre de cette story).

### M2-5 · Logs et erreurs d'import · M · `DONE` (backend ; écran différé, voir note)
`tracing` + rotation quotidienne, rétention 30 jours ; table `import_errors` ; écran Logs (visionneuse + onglet Erreurs d'import, avec les actions Reparser / Ignorer / Copier).

**Décision de périmètre (Frédéric, 28/09) :** backend complet et testé (`tracing`/rotation/rétention, `import_files`/`import_errors`, logique Reparser/Ignorer, commandes Tauri prêtes) ; l'écran React réel (visionneuse + onglet, boutons) est **différé à M6** (écrans), comme le reste des écrans — aucun outillage IPC (`specta`/`ts-rs`, `ui/src/lib/api.ts`) n'existe encore côté frontend, et sa mise en place mérite sa propre story plutôt que d'être improvisée ici. Les CA ci-dessous sont donc validés au niveau données/IPC (tests bout en bout Rust), pas visuellement dans l'écran (toujours un placeholder).

**CA :**
- Une main corrompue injectée apparaît dans l'onglet Erreurs. ✅ Validé au niveau données : `gr-ingest::import_one_file` consigne toute main/fichier en échec dans `import_errors` (via `gr-store::record_import_error`, avec `raw_excerpt` conservé) sans interrompre le reste de l'import (PAR-15). Testé de bout en bout (`a_corrupted_hand_is_recorded_in_import_errors`) : une main injectée avec une ligne invalide produit exactement 1 ligne `import_errors` (code `UNKNOWN_LINE`, statut `OPEN`, texte brut conservé).
- « Reparser » fonctionne après correction du parser. ✅ `gr-ingest::reparse_import_error` relit `raw_excerpt`, retente `WinamaxParser::parse_hand`, insère la main et marque l'erreur `RESOLVED` en cas de succès ; sinon rafraîchit le message et laisse `OPEN`. Testé (impossible de « corriger le parser » dans un test — on simule la même mécanique en corrigeant le texte source conservé, ce qui exerce exactement le chemin de code de l'action Reparser) : succès → `RESOLVED` + main insérée ; échec persistant → `OPEN` + message rafraîchi ; id inconnu → erreur typée.

Réalisé en plus des CA : `tracing`/`tracing-subscriber`/`tracing-appender` (fichier tournant quotidien dans `<data_dir>/logs/`, purge des fichiers >30 jours au démarrage, testée avec une horloge injectée) ; commandes Tauri `list_import_errors`/`reparse_import_error_cmd`/`ignore_import_error` prêtes (non consommées par l'UI). Build `cargo tauri build --no-bundle` validé avec le câblage complet.

### M2-6 · Rattachement des summaries et des tournois · M · `DONE` (§8.5). **M2 est intégralement terminé.**
**CA :**
- Un tournoi passe de `PROVISIONAL` à `COMPLETE` à l'import de son summary, quel que soit l'ordre d'import. ✅ `gr-store::summary_repo::attach_summary` crée le tournoi "provisoire" au besoin (summary arrivé avant toute main) via le même `get_or_create_tournament` que M2-2, met à jour buy-in exact/`ko_type`/`is_freeroll`/`entrants` et passe `status = 'COMPLETE'`. Testé dans les deux ordres (hands puis summary, et summary puis hands) sur le vrai fixture OBELISK, ainsi qu'en un seul `run_import` (hands + summary du même dossier).
- Les re-entries sont comptées. ✅ `tournament_entries.entries_count` = nombre de blocs du summary (2 pour OBELISK) ; gains = somme de tous les blocs (3479+1740 = 5219 centimes, conforme à `docs/formats/winamax.md` §5.3) ; place retenue = celle du dernier bloc (7e). `tournament_bullets` : une ligne par entrée (late_reg_bust, place, durée, gains). Rattachement idempotent (réattacher le même summary ne duplique ni l'entrée ni les bullets).
- Un tournoi sans summary passe `INCOMPLETE` après 24 h sans nouvelle main. ✅ `gr-store::Store::mark_stale_provisional_tournaments_incomplete(now_ms, stale_after_hours)` — fonction pure testée avec une horloge injectée (23h → toujours `PROVISIONAL`, 25h → `INCOMPLETE`), sans jamais rétrograder un tournoi déjà `COMPLETE`. Appelée une fois au démarrage de `src-tauri` (pas de tâche planifiée récurrente à ce stade — cohérent avec le watcher temps réel de M3-2 qui, lui, tournera en continu).

Extension du parser (M2-6) : `TournamentSummary.hero_pseudo` capture la ligne `Player : <pseudo>` déjà matchée mais jusqu'ici jetée par `gr-parser-winamax` — nécessaire pour savoir à quel joueur rattacher `tournament_entries`. Les 9 snapshots `insta` de summary régénérés (un seul champ ajouté, aucune régression).

Limite documentée (R-FORMAT) : le champ `Type` du summary ne distingue pas KO/PKO/Mystery/Space (les 4 valent littéralement `knockout`, docs/formats/winamax.md §5.2) ; `ko_type` n'est donc renseigné qu'en `KO`/`NONE` (présence ou non d'un bounty), pas dans le détail du sous-type — une heuristique sur le nom du tournoi serait non documentée. `prize_pool_cents`/`paid_places` restent `NULL` : la ligne `Prizepool` du summary n'est pas encore parsée (marquée « non utilisée pour l'instant » depuis M1-6) et les places payées n'apparaissent pas dans les summaries observés.

---

## M3 — Temps réel et sessions (§8.4)

### M3-1 · Assistant de premier lancement · M · `DONE` (UC1) — validé par Frédéric le 28/09 (détection + sélection d'un compte + import initial déroulés sans erreur jusqu'au résumé).
Détection de `%APPDATA%\winamax\documents\accounts\*\History`, choix des pseudos → profil Hero, import initial.

**CA :** un ami configure l'application en moins de 5 min sans aide. ✅ Test manuel via `just dev` par Frédéric (compte réel, 61 375 mains) : détection et choix du pseudo fluides, import complet sans blocage après correction du bug i18n et des 3 bugs parser réels ci-dessous.

Réalisé :
- `gr-ingest::detect_winamax_accounts` — scan de `accounts_dir`, un compte par sous-dossier ayant un sous-dossier `history` (casse tolérée insensible, §8.1 ⚠️), avec décompte de fichiers de mains déjà présents. Testé (dossier manquant, casse `History`, comptes sans historique ignorés, tri par pseudo).
- `gr-store::hero` — `hero_profiles`/`hero_accounts` (D19) : création de profil (un seul par défaut à la fois), rattachement de pseudo idempotent (réutilise `get_or_create_player_id` de M2-6), liste des profils. Table `settings` (clé/valeur) pour le flag « premier lancement terminé ».
- **Première infra IPC frontend** : `ts-rs` génère `ui/src/bindings.ts` (types `WinamaxAccountPayload`/`ImportProgressPayload`/`ImportSummaryPayload`) à chaque `cargo test -p graphite` (donc `just check`) ; `ui/src/lib/api.ts` encapsule les appels `invoke`/`listen`. `@tauri-apps/api` épinglé en `~2.11.1` pour rester sur la même mineure que le crate `tauri` (2.11.6) — `cargo tauri build` refuse sinon (« version mismatched Tauri packages »).
- Commandes Tauri : `detect_winamax_accounts`, `create_hero_profile_and_accounts`, `is_first_launch_complete`, `mark_first_launch_complete`. L'import initial réutilise `import_paths` (M2-3) tel quel, avec sa progression (`import://progress`).
- Écran `ui/src/screens/Setup.tsx` (route `/setup`) : détection → sélection des comptes (ou saisie manuelle si aucun détecté) → nom du profil → import avec barre de progression → résumé. `routes/index.tsx` redirige vers `/setup` tant que le premier lancement n'est pas terminé (dégrade silencieusement vers l'accueil si l'appel IPC échoue, ex. hors contexte Tauri).

Limites documentées (simplifications volontaires, périmètre M3-1 uniquement) :
- L'assistant s'affiche dans l'`AppShell` normal (barre latérale visible) plutôt que dans une mise en page dédiée sans navigation — accepté pour rester dans le budget de cette story.
- Aucun sélecteur de dossier manuel (dialogue de fichiers) si la détection automatique échoue : seule une saisie de pseudo est proposée, sans import associé.
- `ImportSummaryPayload` (Tauri) a été complété avec `summaries_attached`/`summaries_failed`, oubliés lors du câblage initial de M2-6.

**Bug réel trouvé par le premier test manuel de Frédéric (28/09) :** l'import semblait « ne plus rien faire » après « Import en cours... ». Cause : `ui/src/lib/i18n.ts` utilise le plugin `i18next-icu` (format ICU MessageFormat, accolades simples `{var}`), alors que toutes les clés de traduction — y compris `statusBar.handsToday`/`lastHand` de M0-5, jamais vraiment testées à l'affichage — utilisaient la syntaxe i18next par défaut `{{var}}`. Résultat : aucune substitution, le texte brut `{{count}}` s'affichait tel quel, y compris dans la barre de progression de l'import (`setup.importProgress`), ce qui donnait l'impression d'un blocage alors que l'import continuait probablement en tâche de fond (CPU actif observé). Corrigé dans les deux locales (fr/en), plus un test de régression dédié (`ui/src/lib/i18n.test.ts`, 6 cas dont un avec apostrophe française pour vérifier que `l'import`/`aujourd'hui` ne perturbent pas le parseur ICU — confirmé sans problème). Petits polish associés : attributs `name`/`autoComplete` sur les champs du formulaire (warning navigateur).

**2e bug réel trouvé après réinitialisation manuelle de `first_launch_completed` (28/09) :** rejouer l'assistant avec un profil déjà existant en base levait `erreur SQLite: UNIQUE constraint failed: hero_profiles.name` (`create_hero_profile` faisait un `INSERT` simple sans gérer le conflit). Corrigé en rendant `create_hero_profile` idempotent (`ON CONFLICT(name) DO UPDATE`, réutilise l'id existant) ; ce même crash aurait pu survenir après toute interruption réelle entre la création du profil et le marquage « premier lancement terminé », pas seulement après un reset manuel. Test de régression ajouté.

### M3-2 · Watcher temps réel · M · `DONE` — le 28/09. CA complet mesuré : 2376 mains sur 12 fichiers/10 min, **p95 = 62 ms** (max 248 ms, cible < 2 s), 0 perte, 0 doublon.
`notify` + polling de secours (2 s) ; lecture incrémentale par offset ; backoff sur fichier verrouillé ; gestion de la troncature et du renommage.

**Décisions validées par Frédéric avant implémentation** (nouvelle dépendance + architecture, `CLAUDE.md` §2.8) :
- Dépendance `notify` (8.2, watcher natif `ReadDirectoryChangesW` sous Windows) ajoutée à `gr-ingest`, aucun appel réseau.
- Nouvelle clé `settings.watched_roots` (JSON) : les dossiers choisis à la fin de l'assistant M3-1 sont persistés et réarment le watcher à chaque démarrage de l'app (rien ne le faisait avant M3-2).

**Réalisé :**
- `gr-store::import_log` — `import_files.last_offset` (colonne déjà prévue au schéma M2-1 mais jamais alimentée) : `get_import_file_progress`/`update_import_file_progress`/`list_tracked_hand_file_paths`/`mark_import_file_missing`.
- `gr-ingest::import` — extraction de `insert_blocks` (parsing + insertion d'une liste de blocs déjà découpés), partagée entre l'import en masse (`import_one_file`, M2-3, inchangé) et la nouvelle lecture incrémentale.
- `gr-ingest::incremental::import_incremental_file` — lit uniquement les octets ajoutés depuis `last_offset` (`split_hand_blocks` renvoie déjà l'octet du dernier bloc complet, M1-3, jusqu'ici ignoré par l'import en masse qui relit tout à chaque fois) ; fichier tronqué/réécrit (taille < offset connu) → relecture complète depuis 0 (dédoublonnage `UNIQUE(room_id, room_hand_id)`, sans risque) ; fichier verrouillé → 2 tentatives rapprochées (100 ms, 300 ms) puis abandon pour ce passage.
- `gr-ingest::watch::spawn_watcher` — un seul thread de fond : reveil sur le premier évènement `notify` reçu OU au plus tard après 2 s (polling de secours), les deux déclenchent le même passage (redécouverte + lecture incrémentale par fichier). Fichiers disparus du scan disque (renommage/suppression) → `import_files.status = 'MISSING'`, mains conservées ; un fichier qui réapparaît au même chemin repasse `OK` automatiquement (`upsert_import_file`).
- `src-tauri::watch` — `set_watched_roots` (commande IPC, persiste + démarre le watcher immédiatement, sans attendre un redémarrage) appelée par `Setup.tsx` juste après l'import initial ; `start_from_persisted_roots` relit `watched_roots` au démarrage de l'app. Évènement `hands://new` (`HandsNewPayload`) émis uniquement quand `hands_inserted > 0` — pas encore consommé côté UI (la barre d'état réelle est le périmètre de M3-3).
- Tests : `gr-store` (4 nouveaux, offset/missing), `gr-ingest::incremental` (5, dont troncature et bloc partiel non consommé), `gr-ingest::watch` (4 unitaires + 1 test CA "smoke" rapide non ignoré + 1 test CA complet `#[ignore]` fidèle à l'énoncé : 12 fichiers, 1 main/3 s, 10 min réelles, `cargo test -p gr-ingest --release -- --ignored watcher_meets_the_m3_2`).

**Écarts documentés par rapport à l'énoncé PRD §8.4 :**
- Le backoff « 100 ms → 2 s » sur fichier verrouillé est réparti sur deux échelles plutôt qu'une seule boucle bloquante : 2 tentatives rapprochées (100 ms, 300 ms) dans `import_incremental_file` pour absorber une écriture Winamax en cours, puis le polling de secours du watcher (au plus 2 s) fournit la suite naturellement au passage suivant — évite qu'un fichier récalcitrant bloque le thread du watcher (et donc la latence des 11 autres fichiers) jusqu'à 5 s.
- Le filtre PRD « fichiers modifiés depuis moins de 12h » (optimisation du polling) est remplacé par une comparaison taille-disque-courante vs `last_offset` connu (`import_incremental_file` retourne immédiatement si égales) : plus précis (détecte toute reprise d'écriture, même sur un vieux fichier), et déjà suffisant en pratique (un historique Winamax de plusieurs centaines de fichiers reste un `read_dir` + quelques stats bon marché toutes les 2 s).
- CA « CPU < 10 % en moyenne » non mesuré automatiquement (mesure fiable et portable du temps CPU d'un test Rust hors périmètre raisonnable de cette story) — à vérifier par Frédéric via le Gestionnaire des tâches pendant une session réelle, comme le CA manuel de M0-2.

### M3-3 · Tray et barre d'état · S · `DONE` — le 29/09.
Icône dans la zone de notification (Pause/Reprendre import, Ouvrir, Quitter) ; barre d'état avec le statut d'import, les mains du jour et la dernière main ; priorité processus Below Normal (§6.2).

**CA :** fermer la fenêtre garde l'import actif (option). ✅ Validé par Frédéric via `just dev` le 29/09 : fermeture de la fenêtre = masquage (pas de fermeture réelle) tant que la bascule est cochée, priorité Below Normal confirmée visible dans le Gestionnaire des tâches.

**Non re-vérifié visuellement (couvert par les tests unitaires uniquement) :** le menu tray "Pause/Reprendre l'import" — aucun écran ne montre encore de mains ou d'activité d'import (les écrans de données arrivent en M6), Frédéric n'a donc pas pu observer d'effet visible en cliquant dessus. La logique elle-même n'est pas nouvelle : `pause`/`resume` appellent directement `WatcherHandle::stop`/`spawn_watcher`, déjà testés dans M3-2. À revalider visuellement une fois un écran affichant l'activité d'import disponible (ex. Accueil ou Mains, M6).

**Décisions validées par Frédéric avant implémentation** (ambiguïté PRD, `CLAUDE.md` §2.8) :
- Priorité Below Normal appliquée **inconditionnellement** au démarrage plutôt que seulement "pendant le jeu" — `gr-winmon` (détection de fenêtre Win32) reste vide, une détection en direct aurait élargi le périmètre au-delà de cette story.
- Fermer la fenêtre **masque par défaut** (garde le watcher actif) ; une bascule cochable dans le menu tray lui-même ("Garder actif en arrière-plan à la fermeture") permet de repasser en fermeture réelle — pas d'écran Paramètres dédié (prévu seulement en M8), le tray sert de surface d'option pour ce seul réglage.

**Complément du 29/09 (retour de Frédéric après test manuel) :** rien n'indiquait à l'utilisateur, au premier masquage, que l'app continuait de tourner. Tentative initiale avec `tauri-plugin-notification` abandonnée : la crate exige `tauri >= 2.12` alors que le projet est épinglé en `2.11.6`, ce qui aurait forcé une montée de version de `tauri` (et du paquet npm `@tauri-apps/api` assorti, cf. contrainte connue depuis M3-1) pour une simple notification — bien plus large que prévu. Implémenté sans nouvelle dépendance : au premier masquage, la fenêtre reste visible et `src-tauri::tray` émet `tray://first-hide-notice` ; le composant React `TrayFirstHideNotice` affiche un mot d'explication (i18n, R-I18N respecté) et ne masque la fenêtre qu'une fois acquitté (`mark_tray_notice_shown`, flag `settings` définitif). Plus jamais réaffiché ensuite.

**Réalisé :**
- `gr-store::status` — `count_hero_hands_since`/`latest_hero_hand_played_at` (sur `hands.hero_player_id`/`played_at`, déjà en base depuis M1/M2, jamais interrogés jusqu'ici).
- `src-tauri::watch` — `WatcherRunState` (Idle/Active/Paused) suivi à côté de la poignée du watcher ; `pause`/`resume` réutilisent `WatcherHandle::stop`/`spawn_watcher` de M3-2 sans rien dupliquer.
- `src-tauri::status::get_status_snapshot` — instantané barre d'état (mains aujourd'hui, dernière main, statut watcher) ; `since_ms` (minuit local) calculé côté UI, le backend ne connaît que l'UTC (R-MONEY). Bug ts-rs évité : `i64` est mappé en `bigint` par défaut alors que l'IPC Tauri sérialise en JSON (donc en `number` côté JS) — annoté `#[ts(type = "number")]`.
- `src-tauri::tray` — icône système (id fixe pour pouvoir reconstruire le menu après chaque action), menu Ouvrir/Pause-Reprendre/bascule "garder actif"/Quitter, clic gauche sur l'icône = Ouvrir. Interception `WindowEvent::CloseRequested` → masque la fenêtre au lieu de la fermer tant que la bascule est active.
- `src-tauri::priority::set_below_normal` — `SetPriorityClass`/`BELOW_NORMAL_PRIORITY_CLASS` via `windows-sys` (nouvelle dépendance ciblée `cfg(windows)` uniquement, réutilise une version déjà présente transitivement).
- `ui/app/AppShell.tsx` — barre d'état enfin branchée sur de vraies données (`statusBar.handsToday`/`lastHand`/`importIdle`|`importActive`|`importPaused` n'affichaient que des placeholders statiques depuis M0-5) ; rafraîchie sur `hands://new` (TanStack Query, `invalidateQueries`) et toutes les heures en secours.
- Tests : `gr-store::status` (2), `AppShell.test.tsx` corrigé (manquait `QueryClientProvider`, révélé par le nouveau `useQuery`) + fix d'une rejection non attrapée de `onHandsNew` hors contexte Tauri (tests/aperçu navigateur).

### M3-4 · Sessions · S · `DONE` (§9.3, H3) — le 29/09.
**CA :** regroupement avec seuil paramétrable ; recalcul correct lors d'un import hors ordre. ✅

**Réalisé :**
- `gr-store::sessions::recompute_hero_sessions` — recalcul **intégral** (pas incrémental) des sessions d'Hero à chaque appel : plus simple, et garantit "recalcul correct lors d'un import hors ordre" par construction (le regroupement ne dépend que de l'ordre chronologique final des mains via l'index déjà présent `ix_hands_played_at`, jamais de leur ordre d'insertion). Seuil `settings.session_gap_minutes` (30 par défaut, paramétrable dès maintenant même sans écran Réglages — M8) ; "plus de N minutes" (PRD) : un écart de exactement N minutes ne casse pas la session.
- Rattachées au profil Hero par défaut au moment de M3-4 ; **revu en M3-5** pour boucler sur chaque profil et filtrer via `hero_accounts` (voir plus bas), sans changer l'algorithme de regroupement lui-même.
- `hands`/`tournaments` (distinct) par session : triviaux, sous-produit du regroupement. `max_tables` : **approximation documentée** — nombre de tables distinctes vues pendant la session, pas un vrai calcul de chevauchement temporel (on ne connaît que l'instant de chaque main, pas sa durée). `profit`/`EV` cités par le PRD comme métriques de session ne sont **pas stockés** (cohérent avec le schéma existant depuis M2-1, qui ne les prévoyait déjà pas) : à calculer en jointure au moment de l'affichage (écran Sessions, M6), pas dénormalisés ici.
- Appelé une fois par lot après `run_import` (import en masse, M2-3) et après chaque passage du watcher ayant inséré au moins une main (M3-2) — jamais par fichier, pour ne pas payer le recalcul plusieurs fois inutilement.
- Perf (R-PERF, la story touche l'import) : le recalcul ajoute un `UPDATE ... WHERE id IN (...)` par lot de 500 mains plutôt qu'un par main (optimisation appliquée après une première mesure ligne-par-ligne). Débit mesuré sur 100k mains synthétiques (`gr-synth`, avec profil Hero pour que le recalcul s'exécute réellement) : **~1220-1330 mains/s** selon les runs (bruit de mesure sur cette machine sous charge répétée), contre 1430 mains/s sans recalcul de sessions (M2-4). Reste largement au-dessus de la cible PRD/BACKLOG (≥1000 mains/s).
- Tests : `gr-store::sessions` (9, dont les deux scénarios hors-ordre explicites — backfill isolé et backfill comblant un écart entre deux sessions existantes, qui doit les fusionner), `gr-ingest` (1, câblage bout-en-bout de `run_import`).

### M3-5 · Profils Hero multi-pseudos · M · `DONE` (D19) — le 29/09. **M3 est intégralement terminé.**
**CA :**
- Création et modification de profils. ✅
- Toutes les requêtes Hero filtrent par `hero_accounts` du profil sélectionné. ✅

**Décisions validées par Frédéric avant implémentation** (ambiguïté PRD — aucun écran Paramètres/Filtres réel n'existait encore, `CLAUDE.md` §2.8) :
- Interface minimale dans l'écran **Paramètres** (remplace son placeholder depuis M0-5) plutôt que d'attendre M6 (panneau de filtres complet, §11) ou M8 (reste de Paramètres) : cohérent avec le PRD (§13.10 place déjà "profils Hero et pseudos" sous Paramètres).
- Un seul profil suffit en pratique aujourd'hui (confirmé par Frédéric) : pas de sélecteur multi-profils proéminent construit, mais le filtrage par `hero_accounts` est correctement implémenté dès maintenant pour que plusieurs profils fonctionnent sans retouche future.

**Réalisé :**
- `gr-store::hero` — `rename_hero_profile`, `unlink_hero_account` (détache un pseudo, sans toucher aux mains déjà importées ni à leur `hero_player_id`), `list_hero_account_pseudos`. `create_hero_profile`/`link_hero_account`/`list_hero_profiles` inchangés (M3-1).
- `gr-store::status` et `gr-store::sessions` **revus** pour filtrer par profil via `hero_accounts` plutôt que par "profil par défaut" (M3-3/M3-4) ou "n'importe quel siège Hero" global : `count_hero_hands_since`/`latest_hero_hand_played_at` prennent désormais un `profile_id` explicite ; `recompute_hero_sessions` boucle sur **tous** les profils existants et donne à chacun son propre découpage en sessions indépendant (une main d'un autre profil n'y compte jamais). Comportement inchangé pour l'usage réel actuel (un seul profil, tous ses pseudos déjà rattachés via l'assistant M3-1).
- `src-tauri::hero_profiles` — nouveau module : `list_hero_profiles_cmd` (profils + pseudos), `get_active_hero_profile_id`/`set_active_hero_profile_id` (nouveau réglage `settings.active_hero_profile_id`, retombe sur le profil par défaut si absent/invalide), `create_hero_profile_cmd`, `rename_hero_profile_cmd`, `add_hero_pseudo_cmd`, `remove_hero_pseudo_cmd`. `status::get_status_snapshot` (M3-3) résout désormais le profil actif en interne au lieu de scanner tous les profils sans distinction.
- `ui/screens/Settings.tsx` — remplace le placeholder par une vraie section "Profils Hero" : liste des profils avec leurs pseudos, sélection du profil actif (bouton radio), renommage, ajout/retrait de pseudo, création d'un nouveau profil. Pas de test dédié (même limite que `Setup.tsx` : pas d'infrastructure de mock IPC dans le projet) — CA à valider par Frédéric via `just dev`.
- Perf (R-PERF, la story touche les requêtes utilisées par l'import) : la jointure `hero_accounts` ajoutée au recalcul de sessions n'a pas d'impact mesurable au-delà du bruit déjà observé en M3-4 — ~1185 mains/s sur 100k mains synthétiques (contre ~1220-1330 en M3-4), toujours au-dessus de la cible ≥1000 mains/s.

---

## M4 — Moteur de stats (§10)

### M4-1 · Positions et profondeurs · M · `DONE` (§10.3, §10.4) — le 29/09. M4 démarré.
**CA :**
- Positions correctes de 2 à 10 joueurs (tests table-driven). ✅ (2 à 9, voir décision ci-dessous)
- bb joueur et effectifs. ✅
- Tranches configurables. ✅

**Décision validée par Frédéric avant implémentation** (le CA mentionne "10 joueurs", mais l'enum `Position` posée en M1-1 n'a que 9 étiquettes distinctes — BTN/SB/BB + UTG/UTG+1/UTG+2/LJ/HJ/CO — et le corpus de fixtures ne dépasse pas 7-max) : confirmé que 9-max est le vrai plafond sur Winamax, jamais 10. Implémenté rigoureusement pour 2 à 9 joueurs ; au-delà, `assign_positions` renvoie une map vide plutôt que d'inventer une 10ᵉ position (R-NOPANIC : pas de panique, juste un résultat vide).

**Réalisé (nouveau crate `gr-stats`, premier code réel) :**
- `gr-stats::position::assign_positions` — rotation des sièges **distribués** (`SeatInfo::dealt_in`) à partir du bouton (sens horaire = numéros de siège croissants, confirmé sur deux vrais fixtures : 6-max `mtt/classic-itm/` bouton 3→SB 4→BB 5→UTG 6, et 7-max `mtt/final-table-heads-up/` bouton 4→SB 5→BB 6→UTG 7). Convention de réduction du nombre de positions nommées quand la table rétrécit de 9 à 4 joueurs (aucune règle explicite dans le PRD au-delà du cas 3-max) déduite et vérifiée cohérente avec les deux seuls points d'ancrage donnés : le cas **3-max** explicite du PRD (aucune position du milieu) et la convention **6-max** universellement admise (UTG, HJ, CO) — ordre de retrait : LJ, UTG+2, UTG+1, HJ, CO. 11 tests table-driven (2 à 9 joueurs, sièges non distribués exclus, tables >9 dégradées proprement).
- `gr-stats::depth::depth_bb`/`depth_bracket_label` — bb joueur (tapis / bb) et bb effectifs (min(tapis du joueur, plus gros tapis adverse **distribué**) / bb, PRD §10.3 littéral). Tranches par défaut du PRD (`<10`, `10-15`, ... `100+`, bornes basses incluses/hautes exclues) exposées via `DEFAULT_DEPTH_BRACKETS`, mais la fonction accepte n'importe quelle liste de bornes triée (CA "tranches configurables"). 8 tests.
- Limite connue non traitée (jamais observée dans le corpus, PRD la mentionne comme cas particulier) : le **bouton mort** (`button_seat` absent des sièges distribués) — `assign_positions` renvoie une map vide dans ce cas plutôt que d'inventer un comportement ; à traiter si un exemple réel apparaît (même pattern que les autres inconnues du corpus, cf. « Idées / à trier »).
- Portée volontairement limitée à des fonctions pures dans `gr-stats` : le câblage dans `hand_players` (colonnes `position`/`stack_bb`/`eff_stack_bb`/`depth_bucket`/...) est explicitement le périmètre de **M4-3**, pas de M4-1 (déjà indiqué par son propre CA "Colonnes hand_players remplies à l'import").

### M4-2 · DSL de test `hand!{}` · M · `DONE` — le 29/09.
Construction concise de mains de test pour `gr-stats`.

**Réalisé :**
- `gr-stats/tests/support/mod.rs` — macro `hand!{}` (`#[macro_export]` depuis un module `tests/` : visible sans import dans tout fichier de `gr-stats/tests/*.rs` faisant `mod support;`, vérifié empiriquement). Postes d'ante/blindes **générés automatiquement** à partir de `button`/`sb`/`bb`/`ante` et des sièges (réutilise `gr_stats::sb_bb_seats`, nouvelle fonction extraite de M4-1) : seules les actions volontaires (fold/call/raise/check/shove) sont à fournir dans `preflop`, exactement ce dont les flags préflop (M4-3) ont besoin.
- Portée volontairement limitée au préflop pour l'instant (M4-3 n'en a besoin que pour ça) ; à étendre avec des rues postflop quand M4-4 en aura besoin.
- Tests du DSL lui-même (6, `gr-stats/tests/hand_macro.rs`) : ordre des postes générés, ante, détection de `hero_pseudo`, montants des actions volontaires, `shove` marqué all-in, `check` de la BB.

### M4-3 · Flags préflop · M · `DONE` — le 29/09.
VPIP, PFR, RFI, LIMP, OSHOVE, 3B, F3B, 4B, ATS, FSTEAL (SB/BB), RSTEAL, et la ligne préflop synthétique.

**CA :**
- 3 tests minimum par stat (positif, négatif, pas d'opportunité).
- Colonnes `hand_players` remplies à l'import (migration `0002`).

**Réalisé :**
- `gr-stats/src/preflop.rs` — une fonction pure par stat (CLAUDE.md §6), `compute_<code>(hand: &HandRecord, pseudo: &str) -> StatFlag` (`{opp, act}`) : `compute_vpip`, `compute_pfr`, `compute_rfi`, `compute_limp`, `compute_oshove`, `compute_3b`, `compute_f3b`, `compute_4b`, `compute_ats`, `compute_fsteal`, `compute_rsteal`. Reconstruit en interne une chronologie des décisions volontaires préflop (`raises_before`/`entrants_before` par décision) partagée par toutes les fonctions, sans jamais l'exposer publiquement. `preflop_line(hand, pseudo) -> Option<String>` pour la ligne synthétique (ex. `RFI`, `RFI-F3B`, `CALL-OPEN`, `SQZ` — les 4 exemples de l'annexe schéma valident exactement l'algorithme retenu). 31 tests (`gr-stats/tests/preflop_flags.rs`, DSL `hand!{}` de M4-2), ≥ 3 par stat.
- `gr-stats::position::position_group` (EP/MP/LP/Blinds, PRD §10.4), 9 tests.
- **Décision technique (pas d'ambiguïté nécessitant validation, cf. CLAUDE.md §2.8) :** pas de migration `0002` — les colonnes de `hand_players` visées par cette story (`position`, `position_group`, `stack_bb`, `eff_stack_bb`, `depth_bucket`, `eff_depth_bucket`, `preflop_line`, et les paires `vpip_opp/vpip`, `pfr`, `rfi_opp/rfi`, `limp`, `oshove`, `tb_opp/tb`, `f3b_opp/f3b`, `fb_opp/fb`, `ats_opp/ats`, `fsteal_opp/fsteal`, `rsteal`) existent déjà intégralement dans `0001_init.sql` (schéma initial complet, PRD annexe). Seul `insert_hand_players` (`gr-store/src/repo.rs`) les laissait `NULL` faute de moteur de stats disponible (M2-2). Le CA visait donc le *remplissage*, pas une modification de schéma (R-SCHEMA ne s'applique pas ici) ; la mention « migration 0002 » dans le CA d'origine anticipait un besoin qui s'avère déjà couvert.
- `gr-store/src/repo.rs::insert_hand_players` calcule et insère désormais toutes ces colonnes à l'import, via `gr-stats` (un seul `assign_positions` par main, pas par siège). Profondeur : mode effectif ET joueur stockés tous les deux (`DepthMode::Player`/`Effective`, tranches `DEFAULT_DEPTH_BRACKETS` du PRD) — rendre le mode/les tranches configurables en Paramètres est explicitement hors périmètre (aucune UI de stats n'existe encore, cf. M3-3/M3-5).
- Nouveau test d'intégration `repo::tests::hand_players_have_preflop_stats_computed_at_insertion` (fixture réelle OBELISK 3-max) : vérifie que `position`/`position_group`/`vpip_opp`/`stack_bb` ne sont plus `NULL` après import.
- Perf (R-PERF, import touché) : avant M4-3 (M3-5) ~1185–1330 mains/s (bruit machine documenté) ; après M4-3, mesuré 1245 mains/s sur le corpus synthétique 100k, recalcul de sessions + flags préflop inclus — dans la même fourchette de bruit, toujours très au-dessus de la cible ≥ 1000 mains/s (M2-3).

### M4-4 · Flags postflop · M · `DONE` — le 29/09.
CBF, CBT, FCBF, WTSD, W$SD, WWSF, compteurs AF/AFQ.

**CA :** idem M4-3.

**Réalisé :**
- `gr-stats/src/postflop.rs` — `compute_cbf`, `compute_cbt`, `compute_fcbf`, `compute_wtsd`, `compute_wsd`, `compute_wwsf` (`StatFlag`), plus `compute_postflop_counts` (compteurs bruts `bets/raises/calls/folds/checks`, PRD "compteurs AF/AFQ" — le ratio lui-même s'agrège sur un échantillon de mains, pas sur une seule, donc hors périmètre d'une fonction par main). 22 tests (`gr-stats/tests/postflop_flags.rs`), ≥ 3 par stat.
- **Comment savoir si une main est allée à l'abattage**, déduit du format (`docs/formats/winamax.md` §4.4/§4.5) plutôt que supposé : la section `*** SHOW DOWN ***` du texte brut n'existe que si la main est réellement révélée ; ses actions (`Shows`/`Collected`) sont les seules taguées `Street::Showdown` par le parser (confirmé en lisant `crates/gr-parser-winamax/src/hand.rs`) — un pot remporté sans abattage garde le `Street` de la dernière rue jouée. Donc : main allée à l'abattage ⇔ `hand.actions` contient une action `Street::Showdown`. Fonctionne même sur un runout all-in sans aucune action postflop enregistrée (`*** FLOP/TURN/RIVER ***` listées sans action, §4.4) : testé explicitement (`wtsd_applies_to_an_all_in_runout_with_no_postflop_actions`).
- DSL `hand!{}` étendu au postflop (M4-2 avait prévu cette extension) : `gr-stats/tests/support/postflop.rs`, un module **frère** de `support/mod.rs` (inclus uniquement par `postflop_flags.rs` via `#[path]`), pas une extension de la grammaire de la macro `hand!{}` elle-même (qui reste preflop-only) — `with_postflop(hand, board, actions, winners)` complète une main déjà construite. Décision technique motivée par `clippy::dead_code` (`-D warnings`) : chaque fichier de test est son propre binaire ; des helpers postflop dans `support/mod.rs` partagé auraient été signalés "jamais utilisés" par `hand_macro.rs`/`preflop_flags.rs`, qui ne testent que le préflop.
- `gr-store/src/repo.rs::insert_hand_players` remplit désormais aussi `saw_flop/saw_turn/saw_river`, `went_sd/won_sd/won_hand`, `cbf_opp/cbf`, `cbt_opp/cbt`, `fcbf_opp/fcbf`, `pf_bets/pf_raises/pf_calls/pf_folds/pf_checks` — même décision que M4-3 (pas de migration, colonnes déjà présentes dans `0001_init.sql`). Extraction d'un `SeatStats`/`compute_seat_stats` (`clippy::too_many_lines` sur `insert_hand_players` sinon, 48 colonnes au total désormais). Nouveau test d'intégration `hand_players_have_postflop_stats_computed_at_insertion` : valeurs exactes (pas seulement non-`NULL`, ces colonnes sont des booléens/compteurs toujours renseignés) vérifiées à la main sur la fixture réelle OBELISK (Hero relance preflop, checke le flop après un check adverse, checke le turn, suit la rivière, abattage perdu).
- Perf (R-PERF, import touché) : 1166 mains/s mesurées sur le corpus synthétique 100k (contre 1166–1330 sur les mesures M4-3/M3-5 précédentes, bruit machine), toujours au-dessus de la cible ≥1000/s. **Mise en garde documentée dans le test lui-même** : `gr-synth` ne génère que le motif "tout le monde se couche sauf la BB" (jamais de flop, PRD/BACKLOG M2-4) — cette mesure ne couvre donc que le coût des flags préflop et le chemin de sortie rapide des flags postflop (`saw_street`/`last_preflop_raiser` retournent tôt sans board), pas un parcours réel de rues postflop avec board/actions. Étendre `gr-synth` à des mains multi-rues est hors périmètre de cette story (proposé dans « Idées / à trier » si un bench plus réaliste est jugé utile avant M4-4+).

### M4-5 · Phases du tournoi · M · `BLOCKED(différé en V2, décision Frédéric le 29/09)` (§10.5)
Méthode A (profils de structure) basée sur la structure `Levels` du summary, et méthode B (estimation par interpolation, car les places payées et les joueurs restants sont absents) ; choix dans les paramètres.

**CA :**
- Les deux colonnes sont remplies.
- La méthode B est marquée « estimation » si elle est interpolée.

**Décision (29/09) :** différé en V2. En creusant la story avant de l'implémenter, deux points non triviaux sont apparus : (1) la méthode A suppose la structure `Levels` du summary parsée en données structurées (durée par niveau) — elle ne l'est pas encore aujourd'hui (`gr-parser-winamax::summary` ignore actuellement cette ligne, `_levels_line` jamais exploitée) ; (2) le PRD ne donne aucune borne par défaut pour Early/Middle/Late en méthode A (contrairement à la méthode B qui a 300 %/150 %/100 % explicites) — les inventer serait un choix produit, pas un détail d'implémentation. Frédéric préfère reporter les deux décisions (parser `Levels` maintenant ou pas, bornes par défaut) à la V2 plutôt que de les trancher maintenant. `hands.phase_by_level`/`phase_by_players`/`players_left_est` restent `NULL` jusque-là (colonnes déjà prévues dans `0001_init.sql`, comme M4-3/M4-4).

### M4-6 · Compteurs incrémentaux `player_stat_counters` · M · `BLOCKED(différé en V2, décision Frédéric le 29/09)` (§10.7)
**CA :**
- Compteurs = recalcul complet (test d'équivalence sur le corpus).
- Lecture de 10 joueurs < 20 ms.
- Job de reconstruction.

**Décision (29/09) :** différé en V2, comme M4-5. C'est explicitement de la préparation HUD (D16, PRD §10.7 : « préparation HUD ») — or le HUD lui-même est V2 (CLAUDE.md §1). La clé de contexte du PRD (`format|position_group|depth_bucket|phase|street_line`) dépend en partie de `phase`, déjà différé (M4-5), et le PRD ne précise pas quelle granularité matérialiser (agrégat global seul, contexte complet, ou aussi les combinaisons intermédiaires comme « VPIP par position seule ») — un vrai choix produit, pas un détail. Aucun écran V1 ne consomme ces compteurs aujourd'hui. M4-7 (`AnalyticsBackend`) agrège directement depuis `hand_players` sans dépendre de compteurs incrémentaux.

### M4-7 · `AnalyticsBackend` (implémentation SQLite) · M · `DONE` — le 29/09.
API de requête générique : dimensions, mesures (stats/KPIs), filtres → table de résultats.

**CA :**
- Toutes les stats par position × profondeur pour le Hero.
- Performances NFR-P6 (version SQLite) mesurées et consignées.

**Réalisé :**
- Nouveau crate `gr-analytics` (jusqu'ici vide) : trait `AnalyticsBackend::run_report(&ReportRequest) -> Vec<ReportRow>` (PRD §13.5/ADR-002), agnostique du backend (SQLite livré ici, DuckDB en M6 derrière le même trait). `Dimension` (`PositionGroup`, `DepthBucket`), `Measure` (17 stats action/opportunité de PRD §10.2, VPIP à WWSF), `StatCell{opportunities, actions}` (+ `percentage()`, `None` si `opportunities == 0` plutôt qu'un faux `0.0 %`).
- **Portée volontairement limitée** aux 17 stats à couple opportunité/action : AF/AFQ (ratios de compteurs, pas un couple opp/act, cf. `gr_stats::PostflopActionCounts`) et les dimensions phase/format/cartes/jour (aucun écran ne les consomme encore ; phase différée en V2, M4-5) sont hors périmètre.
- `SqliteAnalyticsBackend` agrège directement `hand_players`/`hands` par `SUM`/`GROUP BY` (pas de compteurs incrémentaux : `player_stat_counters` différé en V2, M4-6) — un seul profil Hero à la fois, scopé via `hero_accounts` (même motif que M3-3/M3-4/M3-5).
- **Bug trouvé et corrigé par le test de perf lui-même** : `SUM()` sur un ensemble vide (ou une colonne jamais renseignée) renvoie `NULL` en SQL, pas `0` — `InvalidColumnType` sans `COALESCE(SUM(...), 0)`. Un test de régression dédié (`report_with_no_matching_hands_returns_zero_not_null`) couvre le cas réel correspondant : un profil Hero fraîchement créé, sans aucune main encore importée.
- **Nouvelle migration `0002_hands_hero_player_index.sql`** (`CREATE INDEX ix_hands_hero_player ON hands(hero_player_id)`) — la première vraie migration `0002` du projet (contrairement à M4-3/M4-4 où les colonnes existaient déjà). Justifiée par la mesure : le rapport à 2 dimensions × 10 stats sur 2M mains passait de 7,99 s à 5,6-5,8 s (cible NFR-P6 SQLite : < 8 s), une marge quasi nulle avant l'index jugée trop fragile pour la garder telle quelle.
- Perf (R-PERF, NFR-P6) : jeu de 2M lignes `hand_players` semé directement en SQL (pas via `gr-store::insert_hands`, qui mesurerait le coût de l'import — déjà couvert par M2-3/M2-4/M4-3/M4-4 — pas celui du rapport). **5,6-5,8 s mesurés** (3 lancers), sous la cible de 8 s avec une marge confortable après l'index.
- 4 tests d'intégration (`gr-analytics/tests/report.rs`, mains réellement insérées via `gr-store` donc flags `gr-stats` réels, pas une fixture SQL à la main) + 1 test de perf différé (`--ignored`, `gr-analytics/tests/perf.rs`).

### M4-8 · Validation croisée des stats · S · `TODO`
Si Frédéric a accès à un autre tracker (essai PT4/HM3 ou Xeester), comparer les stats du Hero sur un même échantillon.

**CA :** écarts < 0,5 point, ou différences de définition documentées.

---

## M5 — Résultats MTT et EV all-in (§9, §10.6)

### M5-1 · Calcul des KPIs de résultats · M · `DONE` — le 29/09.
Coût, frais, prize/bounty, tickets (paramètre FACE_VALUE/ZERO), profit, ROI, ROI hors frais, ROI prize pool / bounties, ITM, ABI, freerolls.

**CA :** scénarios chiffrés au centime (fichier `tests/results_scenarios.rs`), y compris re-entry, ticket utilisé et gagné, et PKO.

**Réalisé :**
- `gr-analytics::kpis` — `compute_results_kpis(&[TournamentResult], TicketValuation) -> ResultsKpis`, fonction pure (pas d'I/O, cohérent avec `gr-core`/`gr-stats`) sur des résultats déjà agrégés (`TournamentResult` = 1..N `Bullet` par re-entry). Couvre exactement la liste du BACKLOG : coût, frais, gains prize/bounty/tickets, gains totaux, profit, ROI, ROI hors frais, ROI prize pool/bounties (KO), ITM %, ABI, freerolls.
- **Portée volontairement limitée** à cette liste (pas tout le tableau PRD §9.1) : place moyenne, % de tables finales, meilleur gain, plus gros tournoi et $/heure sont dans le PRD mais pas dans la description BACKLOG de M5-1 — différés à une story dédiée (écran Résultats, M6+). Pas d'intégration `gr-store` non plus (le CA ne demande que les scénarios chiffrés, pas un branchement sur `tournament_entries`/`tournament_bullets` réels) : à faire quand un écran en aura besoin.
- **Simplification documentée** (aucun exemple réel de ticket dans le corpus pour la valider, `docs/formats/winamax.md` §5.2/§7) : ROI prize pool/bounties (KO) se limite aux bullets payés cash — un ticket utilisé ne se décompose pas en part prize/bounty/fee, donc les bullets payés par ticket sont exclus de ces deux ratios spécifiquement (mais comptent normalement dans le coût/profit/ROI global).
- 7 scénarios chiffrés au centime (`gr-analytics/tests/results_scenarios.rs`) : tournoi simple, re-entry (2 bullets d'un même tournoi, ABI divisé par le nombre de tournois pas d'entrées), ticket utilisé (FACE_VALUE vs ZERO), ticket gagné (ITM dépend du statut "ticket gagné", pas de sa valorisation — vérifié explicitement), PKO (bounty seul ≠ ITM, ROI prize pool/bounties calculés séparément), freeroll seul (ROI exclu via `None`, pas un ratio infini) et freeroll mélangé à un tournoi payant.

### M5-2 · Tickets : métadonnées et types · M · `DONE` — le 29/09 (backend uniquement, voir Réalisé).
Table `ticket_types` ; édition « joué via ticket » et « valeur du ticket » par tournoi ; filtre Via ticket.

**Réalisé :**
- `gr-store::ticket_types` — `create_or_update_ticket_type`/`list_ticket_types` (table `ticket_types`, déjà prévue dans `0001_init.sql`, pas de migration) ; `set_tournament_entry_ticket_used`/`set_tournament_entry_ticket_won` éditent les colonnes ticket de `tournament_entries` (déjà prévues elles aussi). Exposé sur `Store` (`create_or_update_ticket_type`, `list_ticket_types`, `set_tournament_entry_ticket_used`, `set_tournament_entry_ticket_won`).
- Écrit pour fonctionner **avant** qu'un summary ne soit attaché (`ON CONFLICT ... DO UPDATE` qui ne touche que ses propres colonnes, jamais `entries_count`/`finish_position`/`prize_cents`/`bounty_cents`) : un `attach_summary` (M2-6) ultérieur ne peut pas écraser une édition manuelle antérieure, et inversement. Testé explicitement (5 tests).
- **Portée volontairement limitée au backend, même raisonnement que les commandes Tauri de M2-5 (Logs) prêtes avant leur consommateur UI :** ni le filtre « Via ticket » (PRD §11, panneau de filtres global qui n'existe pas encore dans l'UI) ni l'écran d'édition (`Tournaments.tsx` est toujours un placeholder depuis M0-5) ne sont câblés. Sans surprise vu l'absence totale d'exemple de ticket dans le corpus (§7, non résolu) : rien à valider côté UI/parsing tant qu'un exemple réel n'existe pas de toute façon.

### M5-3 · Évaluateur de mains et équité · M · `TODO` (§10.6)
**CA :**
- Valeurs de référence à ±0,1 % en exact.
- HU préflop < 60 ms sur le Celeron (sinon Monte Carlo et justification).
- Multiway en Monte Carlo à graine fixe.

### M5-4 · Événements all-in et EV · M · `TODO`
Calcul post-main, asynchrone, par pot (side pots compris).

**CA :**
- Conservation : Σ EV des joueurs = Σ pots.
- `hand_players.allin_ev_diff_chips` rempli.
- R-COMP-1 respectée (aucun calcul sur une main incomplète).

### M5-5 · Vérification face à Winamax · S · `TODO`
**CA :** sur un mois réel, le profit et les frais concordent au centime avec l'historique du compte Winamax (vérification manuelle par Frédéric ; écarts documentés).

---

## M6 — Écrans principaux (§11, §13.1–13.4)

### M6-1 · Panneau de filtres global et presets · M · `TODO` (§11)
Tous les filtres H4, y compris la grille 13×13 et la saisie texte de ranges ; presets sauvegardés ; état persistant par écran.

### M6-2 · Accueil / KPIs · M · `TODO` (§13.1)
**CA :** < 1 s à 1 M mains (NFR-P5) ; variations par rapport à la période précédente.

### M6-3 · Résultats : graphes G1 à G6 + pivot · M · `TODO` (§9.2, §13.2)
**CA :**
- G2 superpose le réel et l'EV.
- Échantillonnage LTTB au-delà de 5 000 points.
- Export CSV du pivot.

### M6-4 · Tournois : liste et détail · M · `TODO` (§13.3)
Détail : évolution du tapis, all-in, mains, adversaires avec badge de classification (après M7-5), édition des métadonnées.

### M6-5 · Mains : liste virtualisée · M · `TODO` (§13.4)
**CA :**
- Premier affichage < 500 ms avec 1 M mains en base.
- Tag en masse.
- Double-clic → Replayer (placeholder avant M7-3).

---

## M7 — Analyse (§12, §13.5–13.8, §14)

### M7-1 · Constructeur de rapports · M · `TODO` (§13.5)
1 à 2 dimensions × N mesures, filtres, tri, mise en forme par rapport aux benchmarks, sauvegarde, et les 5 rapports prédéfinis.

### M7-2 · Leaks et benchmarks · M · `TODO` (§12)
Table `benchmarks` + valeurs par défaut §12.2 ; matrice stat × profondeur ; Top 10 pondéré ; clic → mains filtrées ; import/export JSON.

### M7-3 · Replayer · M · `TODO` (§13.7)
Table monochrome, contrôles, raccourcis, historique brut surligné, pots/SPR, équité et EV aux all-in.

**CA :** rejoue correctement 100 % des mains du corpus (test de bout en bout : état final du replayer = résultat parsé).

### M7-4 · Tags et notes · M · `TODO` (D27)
9 tags prédéfinis + tags libres + note par main ; filtre par tag.

### M7-5 · Classification des joueurs · M · `TODO` (§14)
Moteur de règles (JSON), éditeur avec prévisualisation, recalcul en fond, badges dans le Replayer et dans le détail d'un tournoi ; notes et label couleur par joueur.

### M7-6 · Backend DuckDB · S · `TODO` (ADR-002)
Feature `analytics-duckdb`, synchronisation incrémentale par curseur, bouton « Reconstruire ».

**CA :**
- Mêmes résultats que le backend SQLite (tests d'équivalence).
- Gain ≥ ×3 sur NFR-P6 à 2 M mains ; sinon, rapport de benchmark et décision consignée dans l'ADR.

### M7-7 · Grille 13×13 des mains de départ · S · `TODO` (§13.5)

---

## M8 — Finitions et release v1.0

### M8-1 · Écran Paramètres complet · M · `TODO` (§13.9)
### M8-2 · Sauvegardes hebdomadaires et restauration · M · `TODO` (§17.2, D33)
**CA :**
- Backup à l'échéance ou au lancement suivant.
- `integrity_check` sur la copie.
- Rétention de 8.
- Restauration testée.
- Backup avant chaque migration.

### M8-3 · Traduction EN complète et relecture FR · M · `TODO` (§17.5)
### M8-4 · Campagne de performance · M · `TODO` (§17.1)
**CA :** tableau NFR-P1 à NFR-P10 mesuré sur le Celeron avec 2 M mains synthétiques, toutes les cibles atteintes ou dérogations validées par Frédéric.

### M8-5 · Spike `gr-winmon` (préparation V2) · C · `TODO` (§19)
Énumération des fenêtres Winamax, correspondance titre → tournoi/table, événements de déplacement. Pas d'overlay.

**CA :** outil de diagnostic (Paramètres > Maintenance) qui liste les tables ouvertes et leur tournoi associé.

### M8-6 · README utilisateur + release v1.0.0 · M · `TODO`
Guide d'installation portable (dézipper, lancer, assistant), FAQ (langue du client, dossier History, WebView2), notes de version.

**CA :** tag `v1.0.0` → zip portable testé sur le PC de Frédéric et sur le PC d'un ami.

---

## V1.1 et au-delà (non planifié)
- V1.1 : parsers iPoker (€) et 888 (€) ; simulateur de downswing (G7) ; affinage des benchmarks.
- V2 : recherche de joueur + fiche détaillée ; HUD overlay (statique puis contextuel), popups, notes ; tiling des tables. **Prérequis : confirmation écrite de Winamax (R-COMP-2).**
- V3+ : ICM et push/fold post-session ; export de mains ; diagnostic IA.

## Idées / à trier
_(Claude Code ajoute ici les idées hors périmètre rencontrées en cours de route.)_

- **Corpus tickets manquant (différé le 27/09, validé par Frédéric)** : aucun exemple de tournoi payé par ticket ni de satellite ayant rapporté un ticket dans les 620+ tournois du corpus M0-4. À refournir si Frédéric en rencontre un. Impact : M1-6 (parsing du ticket dans le summary) et M5-2 (métadonnées ticket) resteront limités à la saisie manuelle (PRD §8.6) tant qu'un exemple réel n'existe pas.
- **Fichier Winamax copié pendant un tournoi en cours** (M0-4, item 7) : nécessite une copie faite *avant* la fin du tournoi, donc impossible à obtenir après coup. À refaire la prochaine fois qu'un tournoi de Frédéric est en cours. Impact : le test de « lecture incrémentale d'un bloc de main incomplet » (M1-3) reste couvert par des troncatures synthétiques uniquement, pas encore par un vrai fichier partiel.
- **Fichier « Day 2 » de consolidation multi-flights** : jamais observé (Hero n'a pas atteint un Day 2 dans le corpus M0-4). Le comportement du parser sur un multi-flights reste donc non vérifié au-delà du Day 1 (`Type: flight`, `Flight ID` non nul).
- **Test UI flaky (M2-3, 28/09)** : `ui/src/app/AppShell.test.tsx` a timeout une fois en CI (`Test timed out in 5000ms`, jsdom `Window's scrollTo() method` non implémenté) puis est passé au rejeu sans aucun changement de code. Aucun fichier `ui/` n'a été touché dans M2-3 : non-reproductible localement, probablement un runner CI lent. À surveiller ; si ça se reproduit, augmenter `testTimeout` ou mocker `scrollTo` dans le setup de test.
- **Omaha/Expresso non rejetés proprement par `detect()`** (trouvé le 28/09 sur le premier import réel de Frédéric) : ~2765 mains Omaha (`_real_omaha_pot-limit`) et des mains Expresso échouent en `import_errors` (`UNKNOWN_LINE`, sans corruption de données — comportement sûr) au lieu d'être silencieusement ignorées. Le PRD §6 point 8 prévoyait déjà que `detect()` reconnaisse et rejette ces cas, jamais implémenté. Signal fiable pour Omaha : l'en-tête dit `Omaha pot limit` au lieu de `Holdem no limit` (déjà, de fait, ce qui fait échouer `parse_header_line` proprement). Pas de signal fiable dans le fichier de mains pour distinguer un Expresso d'un freeroll MTT normal (`Mode: sng` n'est que dans le summary) : nécessite soit d'attendre le rattachement du summary (M2-6, déjà fait) pour filtrer a posteriori, soit une story dédiée. `gr-ingest::import_one_file` n'appelle d'ailleurs pas `WinamaxParser::detect()` aujourd'hui (seul `parse_hand` est utilisé directement) : le brancher est un préalable si on veut un rejet au niveau fichier plutôt que par erreur individuelle.
- **`gr-synth` ne génère que des mains preflop-only** (constaté en M4-4, 29/09) : le seul motif implémenté (« tout le monde se couche sauf la BB », M2-4) n'atteint jamais le flop. Les mesures de perf différées (M2-3/M3-4/M4-3/M4-4) ne couvrent donc que le coût des flags préflop et le chemin de sortie rapide des flags postflop, jamais un parcours réel de rues postflop avec board/actions/abattage. À enrichir si un bench plus réaliste devient nécessaire (M4-4+ ou une story de perf dédiée).
