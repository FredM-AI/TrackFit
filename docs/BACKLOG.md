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

### M5-3 · Évaluateur de mains et équité · M · `DONE` — le 29/09.
**CA :**
- Valeurs de référence à ±0,1 % en exact.
- HU préflop < 60 ms sur le Celeron (sinon Monte Carlo et justification).
- Multiway en Monte Carlo à graine fixe.

**Réalisé :**
- **Dépendance étudiée avant d'écrire du code** (CLAUDE.md §2.8) : plutôt que réécrire un évaluateur de mains (source classique de bugs subtils sur les égalités/kickers), recherche des crates existants sur crates.io. `rs_poker` (le plus utilisé, 95k+ téléchargements, maintenu activement, Apache-2.0) retenu avec `default-features = false` : ses features par défaut (`arena`, `omaha`, `open-hand-history`) tirent tokio/serde/chrono pour un sous-système CFR/TUI non nécessaire ; en les désactivant, les seules dépendances restantes sont `rand` et `thiserror` (déjà présent ailleurs dans le workspace), aucun appel réseau (le crate alternatif `poker` a une feature optionnelle `static_lookup` qui télécharge une table au build — jamais activée). `rs_poker` reste un détail d'implémentation : `gr-equity` n'expose que des `gr_core::Card` en API publique (`crates/gr-equity/src/convert.rs`), converties par correspondance explicite rang/couleur (les deux crates n'ordonnent pas les couleurs pareil).
- `gr_equity::calculate_equity(&[[Card;2]], &[Card]) -> Result<EquityCalculation, EquityError>` — choisit automatiquement la méthode : énumération exacte pour 2-3 joueurs, Monte Carlo (200 000 tirages, graine fixe) au-delà (PRD §10.6). Fonction pure (pas d'I/O), comme `gr-core`/`gr-stats` ; R-COMP-1 documentée comme responsabilité de l'appelant (M5-4), ce crate ne connaît pas la notion de "main en cours".
- **Recherche de perf substantielle, avec une vraie erreur de mesure corrigée en cours de route** (transparence totale, R-PERF) : la 1ʳᵉ estimation (annoncée à Frédéric avant l'implémentation réelle) était de ~18-20ms pour l'énumération exacte HU preflop, mesurée par erreur *après* l'appel à `.collect()` du générateur de combinaisons — en réalité cette étape à elle seule coûtait ~72-126ms sur la machine de recherche. Cause trouvée : `rs_poker::core::CardIter` s'appuie sur l'instruction matérielle PDEP (BMI2), absente du CPU de la machine de recherche (`is_x86_feature_detected!("bmi2") == false`), donc repliée sur une émulation logicielle lente. Remplacée par un générateur de combinaisons maison à boucles imbriquées (`gr-equity/src/combinations.rs`, indices croissants, indépendant du jeu d'instructions), réparti entre threads par plage d'indices de premier élément (round-robin, pas de double-comptage). Un `#[inline]` manquant sur les deux fonctions les plus chaudes faisait une différence de ~3x (96ms → ~30ms) — trouvé en instrumentant le code phase par phase plutôt qu'en devinant.
- Perf finale (R-PERF) : énumération exacte HU preflop mesurée seule à ~30-50ms sur la machine de recherche (sans BMI2 — probablement pas représentative de la vitesse par cœur du Celeron cible, dans un sens comme dans l'autre ; à confirmer par Frédéric via `just bench` sur la machine réelle). Lancée en même temps que le reste de la suite de tests (qui tourne ses propres threads en parallèle), la contention a fait monter la mesure jusqu'à 84ms — un artefact du harness de test (`#[test]` tourne dans son propre thread, et cette fonction en crée elle-même 4), pas de l'usage réel (un seul calcul d'équité à la fois). Test de perf marqué `#[ignore]` pour éviter un flake CI (même motif que les tests de perf de M4-7/gr-analytics), à lancer seul.
- Valeurs de référence (±0,1 % en exact) : AA vs KK HU preflop calculé à 81,26 % (couleurs totalement disjointes) — la valeur "~81,9%" généralement citée dépend en réalité de la configuration de couleurs choisie (vérifié empiriquement : 82,64 % avec les deux paires partageant leurs deux couleurs). Un test de symétrie parfaite (AKo vs AKo, aucune carte commune) vérifie une égalité exacte 50/50 mathématiquement garantie, indépendante de toute mémoire de référence externe. 14 tests (`gr-equity/tests/equity_scenarios.rs`) : référence exacte, égalité/split, HU preflop, 3 joueurs (conservation de l'équité), Monte Carlo 4-5 joueurs (déterminisme, conservation), validations.
- Multiway Monte Carlo (4+ joueurs) : graine fixe (`0x4772_6170_6869_7465`, jamais destinée à changer — un changement romprait la reproductibilité de calculs déjà affichés), 200 000 tirages (PRD §10.6). Testé déterministe (deux appels identiques → résultat bit-à-bit identique).

### M5-4 · Événements all-in et EV · M · `DONE` — le 29/09.
Calcul post-main, asynchrone, par pot (side pots compris).

**CA :**
- Conservation : Σ EV des joueurs = Σ pots.
- `hand_players.allin_ev_diff_chips` rempli.
- R-COMP-1 respectée (aucun calcul sur une main incomplète).

**Décisions de périmètre validées par Frédéric (29/09), avant implémentation :**
- **Pas de calcul par pot.** Les pots/gagnants sont déjà connus (`HandRecord.pots`, alimenté par les lignes `collected … from main/side pot K` déjà parsées) : pas besoin de reconstruire l'éligibilité aux side pots. L'équité se calcule **une seule fois pour l'ensemble des joueurs impliqués**, contre le pot total (`Σ hand.pots[].amount`) — pas séparément par pot. Limite assumée : sur un all-in multiway à tapis inégaux (vrais side pots), l'équité "pot principal" d'un joueur à tapis court peut différer de son équité "contre tout le monde" ; pour un affichage replayer (pas un solveur), c'est un compromis acceptable. En contrepartie, la conservation Σ EV = Σ pots est garantie par construction (les équités d'un seul calcul somment à 1), sans logique de sommation supplémentaire.
- **Synchrone, pas de file de jobs.** Les évenements all-in sont rares (la plupart des mains n'en ont aucun) et `gr-equity` est déjà rapide (M5-3). R-COMP-1 est satisfaite par construction : le pipeline d'import ne traite que des mains complètes (Winamax n'écrit une main sur disque qu'une fois terminée).

**Réalisé :**
- `gr_equity::detect_all_in_event(&HandRecord) -> Option<AllInEvent>` (nouveau module `gr-equity/src/allin.rs`) : au plus un événement par main (une fois qu'aucune décision supplémentaire n'est possible, la main se conclut automatiquement — pas de second point de décision possible ensuite). Détection purement structurelle à partir de `HandRecord` déjà existant, sans changement du parser : dernière rue avec une décision volontaire enregistrée (hors abattage) < riviere, ET board entièrement distribué (5 cartes) — ce qui ne peut arriver que si les rues restantes ont été distribuées sans action (`docs/formats/winamax.md` §4.4), la signature exacte d'un all-in payé avant la riviere. `None` si un joueur encore en jeu n'a jamais montré ses cartes (PRD : cartes connues requises).
- `allin_ev_diff_chips = gains réels − (équité × pot total)` par joueur impliqué (`gr-store::repo::insert_hand_players`, colonne déjà prévue dans `0001_init.sql`, pas de migration). `detect_all_in_event` appelé une fois par main (pas par siège), comme `assign_positions`/`gr-stats`.
- 6 tests (`gr-equity/tests/allin_events.rs`) : all-in preflop HU (conservation Σ diffs ≈ 0), pas d'évenement sur un showdown normal, pas d'évenement sur un fold non contesté, pas d'évenement si un joueur ne montre jamais ses cartes, pas d'évenement si l'all-in survient sur la riviere elle-même (rien à tirer), all-in à 3 joueurs au turn (conservation). Nouveau test d'intégration `hand_players_get_an_allin_ev_diff_when_the_hand_has_a_real_all_in` sur le corpus réel OBELISK (confirme qu'il contient de vrais all-in, pas seulement que la colonne existe).
- **Régression de perf découverte et corrigée pendant l'implémentation elle-même** (R-PERF) : un test `gr-store` existant important tout le corpus OBELISK (328 mains réelles, dont plusieurs all-in) est passé de <1s à ~16s en debug après ce câblage — `[profile.dev.package."*"]` (ADR-004) ne s'applique **pas** aux membres du workspace (uniquement aux dépendances externes, comportement Cargo documenté mais facile à oublier), donc le code d'énumération exacte de `gr-equity` tournait à `opt-level=0` en `cargo test` normal. Override nommé explicitement ajouté (`[profile.dev.package.gr-equity]`, `opt-level = 2`) : ramène ce test à ~1,7s (×9,5), sans toucher le profil dev du reste du workspace.
- Perf import (R-PERF) : 1168 mains/s sur le corpus synthétique 100k (inchangé par rapport à M4-4) — `gr-synth` ne génère jamais d'all-in (motif "tout se couche sauf la BB"), donc cette mesure ne couvre que le chemin de sortie rapide de `detect_all_in_event`, pas le coût d'un vrai calcul d'équité (déjà mesuré séparément en M5-3).

### M5-5 · Vérification face à Winamax · S · `TODO`
**CA :** sur un mois réel, le profit et les frais concordent au centime avec l'historique du compte Winamax (vérification manuelle par Frédéric ; écarts documentés).

---

## M6 — Écrans principaux (§11, §13.1–13.4)

### M6-1 · Panneau de filtres global et presets · M · `DONE (phase 1)` — le 30/09 (§11)
Tous les filtres H4, y compris la grille 13×13 et la saisie texte de ranges ; presets sauvegardés ; état persistant par écran.

**Décision de périmètre validée par Frédéric (30/09), avant implémentation :** le PRD §11 liste 16 dimensions de filtre (profil Hero, dates, room, buy-in, format, ticket, vitesse, taille de table, position, profondeur, phase, grille 13×13, situations préflop, all-in, tags, résultat de main) qui devraient en plus se brancher sur les 4 écrans déjà construits (Accueil, Résultats, Tournois, Mains) — story disproportionnée par rapport aux précédentes. **Phase 1 : dates (presets + plage libre) + profil Hero + presets nommés + état persistant par écran**, branchée sur les 4 écrans. Les 14 autres dimensions restent `TODO` (phase 2 ou V2, à trier — beaucoup nécessitent une capacité backend qui n'existe pas encore : cartes, situations préflop, phase, all-in).

**Réutilisation plutôt que nouveauté :** le filtre "Profil Hero / pseudos" (D19) ne réintroduit rien — il réutilise tel quel le mécanisme `hero_profiles`/`hero_accounts` déjà construit (M3-5) via un sélecteur dans la barre de filtres, qui appelle `set_active_hero_profile_id` déjà existant. Aucun nouveau concept de "profil différent par écran" : c'est un réglage global, comme avant M6-1.

**Persistance :** état de filtre par écran + presets nommés stockés dans `settings` (`Store::get_setting`/`set_setting`, déjà public, même mécanisme que `watched_roots` depuis M3-2) — contenu JSON opaque côté Rust (jamais relu/validé), les types vivent côté TypeScript (`ui/src/lib/filters.ts`). Pas de nouvelle table ni de migration nécessaire.

**Réalisé :**
- `fetch_hero_tournament_results` (M6-2/M6-3) réutilisé tel quel pour Résultats/Tournois (avait déjà `since_ms`/`until_ms`). Deux nouvelles fonctions filtrables ajoutées à `gr_analytics::results` : `fetch_hero_chip_history` (G2) et `fetch_hero_tournament_volume_by_day` (G6), qui ne l'étaient pas encore. `gr_analytics::hands::{count_hero_hands, fetch_hero_hands_page}` (M6-5) gagnent aussi `since_ms`/`until_ms` (littéraux liés directement, même précaution qu'en M6-5 pour ne pas réintroduire un tri complet — revérifié par `perf_hands`, 182 ms à 1M mains après ajout du filtre, toujours largement sous la cible NFR-P7 de 500 ms).
- `src-tauri::home::get_home_snapshot` : la période fixe "30 derniers jours vs 30 jours précédents" (`PERIOD_MS`) est remplacée par `since_ms`/`until_ms` explicites, résolus côté UI. `previous_period` devient `Option<PeriodKpis>` : `None` pour une période non bornée ("tout"), pas de comparaison bien définie dans ce cas. La courbe G1 de l'Accueil reste volontairement **non filtrée** (vue d'ensemble complète, même décision que pour Résultats en M6-2/M6-3 avant que M6-1 n'existe).
- `src-tauri::{results::get_results_snapshot, tournaments::get_tournaments_list, hands::{get_hands_count, get_hands_page}}` gagnent tous `since_ms`/`until_ms`.
- `src-tauri::filters` (nouveau module) — 4 commandes passe-plat vers `settings` (`get_filter_state`/`set_filter_state` par écran, `get_filter_presets`/`set_filter_presets`).
- `ui/src/lib/filters.ts` — store Zustand (première vraie utilisation du package, présent depuis M0-2 mais jamais utilisé jusqu'ici), `resolveDateRange` (presets → bornes concrètes, horloge locale car le backend ne connaît que l'UTC, R-MONEY), hydratation depuis `settings` au montage.
- `FilterBar.tsx` (nouveau), rendu dans l'en-tête partagé de `AppShell` (placeholder vide depuis M0-5) : sélecteur de profil Hero (visible seulement si plusieurs profils existent), boutons de preset de date + plage libre, application/sauvegarde de presets nommés — visible uniquement sur les 4 écrans branchés (pas sur le détail d'un tournoi ni les écrans sans données filtrables).
- **Piège de purity React Compiler trouvé et corrigé avant merge** : `Date.now()` appelé directement dans le corps de rendu (pour résoudre le preset et calculer la clé de requête) est rejeté par la règle ESLint `react-hooks/purity` ("Cannot call impure function during render"). Corrigé en utilisant l'objet `DateRangeState` (le preset choisi, pas les bornes résolues) comme clé de requête TanStack Query, et en ne résolvant les bornes concrètes (`resolveDateRange(range, Date.now())`) qu'à l'intérieur de `queryFn`, jamais pendant le rendu — même principe que `AppShell.tsx` le faisait déjà pour `startOfTodayMs()`.
- Home/Results/Tournaments/Hands mis à jour pour lire leur propre plage depuis le store et la passer aux commandes IPC.
- **CA (comportement visuel du panneau, persistance entre sessions)** à valider par Frédéric via `just dev` : test d'usage humain, pas vérifiable automatiquement depuis cet environnement.

### M6-2 · Accueil / KPIs · M · `DONE` — le 29/09.
**CA :** < 1 s à 1 M mains (NFR-P5) ; variations par rapport à la période précédente.

**Décision de périmètre validée par Frédéric (29/09), avant implémentation :** cartes KPI (valeur + variation vs période précédente, **sans** sparkline) + graphe G1 en format réduit (premier usage réel d'ECharts) + « dernière session » + « état de l'import ». Différés faute d'infrastructure : sparklines (nécessitent un historique de KPI par période, pas encore stocké/conçu) et « top 3 leaks » (nécessite le moteur de benchmarks/leaks, M7-2, pas encore construit).

**Réalisé :**
- `gr-analytics::home` (nouveau module) — `fetch_hero_tournament_results` (jointure `tournament_entries`×`tournament_bullets`, un `TournamentResult`/`Bullet` par tournoi réutilisant directement `compute_results_kpis`, M5-1) et `hero_allin_ev_diff_bb` (somme de `hand_players.allin_ev_diff_chips` en bb, M5-4), toutes deux filtrées `[since_ms, until_ms)`. **Bug trouvé et corrigé par les tests avant merge** : le ticket gagné (`tournament_entries.ticket_won_type_id`, colonne au niveau de l'entrée, pas du bullet) s'appliquait à *tous* les bullets d'une re-entry au lieu du dernier seul, faute d'effacer l'affectation précédente à chaque nouvelle ligne — un test dédié (re-entry + ticket utilisé + ticket gagné, valeurs chiffrées) l'a fait échouer immédiatement.
- `gr-store::sessions` gagne `latest_session` (dernière session + tournois qu'elle a touchés, via `hands.session_id`) et `total_session_ms_in_range` (durée totale de session par période, pour le KPI "$/h" — une session est comptée entièrement dans la période où elle a commencé, pas répartie au prorata si elle la chevauche, approximation documentée dans le même esprit que M5-4). `gr-store::status` gagne `count_all_hands` (mains importées, tous profils confondus — indicateur de santé globale de l'import, pas scopé Hero).
- `src-tauri::home::get_home_snapshot` orchestre les deux crates (direction de dépendance respectée, CLAUDE.md §5) : périodes courante/précédente (30 derniers jours vs les 30 jours précédents — aucun panneau de filtres/dates réglable tant que M6-1 n'existe pas, valeur par défaut assumée), courbe de profit cumulé (G1, tout l'historique, non filtrée par période) avec et sans bounties (réutilise `compute_results_kpis` avec les bounties mises à zéro plutôt que de dupliquer la logique de coût), meilleur/pire tournoi de la dernière session. 3 tests unitaires sur les fonctions pures (cumul chronologique, exclusion des tournois sans `started_at` connu, sélection meilleur/pire).
- Écran `Home.tsx` : 8 cartes KPI (Profit, ROI, ITM, ABI, Tournois, Frais payés, Écart EV all-in en bb, $/h), signe visible via `Intl.NumberFormat({signDisplay:'exceptZero'})` plutôt que la couleur seule (D15, mode monochrome par défaut) ; `ProfitCurveChart` (ECharts, courbe G1) ; cartes « Dernière session »/« État de l'import ». Nouveau `ui/src/lib/format.ts` (formatage centimes/pourcentage/bb/durée, réutilisable par les futurs écrans M6). `echarts` importé **dynamiquement** (`import('echarts')` dans l'effet de montage, pas en tête de fichier) après avoir observé que l'import statique alourdissait le chunk de la route Accueil à >1 Mo et faisait échouer `AppShell.test.tsx` par timeout (le rendu du graphe ne se déclenche que si des tournois existent, donc `echarts` n'est plus du tout chargé dans ce test) — chunk `routes-*.js` revenu de 1,12 Mo à 7,2 Ko après correction.
- **Perf (NFR-P5), avec une vraie marge insuffisante trouvée et corrigée avant merge** : `crates/gr-analytics/tests/perf_home.rs` (`#[ignore]`, sème 1M mains + 5000 tournois) mesure `fetch_hero_tournament_results` + `hero_allin_ev_diff_bb` + `total_session_ms_in_range` (périodes courante et précédente) + la courbe G1 non filtrée. 1ʳᵉ mesure : **969 ms** — sous la cible d'1 s mais avec une marge de ~3 % seulement, jugée trop fragile pour la machine cible réelle (Celeron N5095A, plus faible que la machine de recherche, CLAUDE.md §4). Décomposition par requête : `hero_allin_ev_diff_bb` à elle seule coûtait ~470-480 ms par appel (×2), les deux requêtes tournoi restant sous 15 ms. Cause : l'index existant `ix_hp_hero (is_hero, player_id)` scanne *toutes* les mains d'Hero (1M lignes) alors que les événements all-in sont rares (M5-4). **Nouvelle migration `0003_hand_players_hero_allin_index.sql`** — index partiel `ix_hp_hero_allin ON hand_players(hand_id) WHERE is_hero = 1 AND allin_ev_diff_chips IS NOT NULL` — mais SQLite ne le choisissait pas spontanément sans statistiques `ANALYZE` (vérifié via `EXPLAIN QUERY PLAN` sur un binaire de diagnostic jetable) ; forcé explicitement via `INDEXED BY ix_hp_hero_allin` dans `hero_allin_ev_diff_bb`. Mesure finale (2 runs) : **717 ms et 725 ms**, marge ramenée à ~28 %.

### M6-3 · Résultats : graphes G1 à G6 + pivot · M · `DONE` — phase 1 le 29/09, phase 2 le 30/09 (§9.2, §13.2)
**CA :**
- G2 superpose le réel et l'EV.
- Échantillonnage LTTB au-delà de 5 000 points.
- Export CSV du pivot.

**Décision de périmètre validée par Frédéric (29/09), avant implémentation — « Phase 1 » :** G1 (repris de M6-2) + G2 (réel vs EV all-in) + G6 (volume, version « par jour ») + pivot réduit (buy-in × KO/non-KO, avec les KPI §9.1 déjà calculés) + export CSV. Différés à une phase 2 dédiée :
- **G3** (ROI par buy-in) et **G5** (distribution des places) : pas de blocage technique, juste hors du premier lot.
- **G4** (ROI par format complet KO/PKO/Mystery/Space) : bloqué par une limite déjà documentée depuis M2-6 — le summary Winamax ne permet de fiabiliser que « bounty ou pas », pas le sous-type précis.
- **Pivot complet** (dimensions vitesse/jour de semaine/heure/mois) et les KPI §9.1 qui lui manquent encore (place moyenne, % tables finales, meilleur gain, plus gros tournoi) : pas encore calculés.

**Phase 2 (30/09), à la suite de M6-2 dans l'ordre choisi par Frédéric (phase 2 avant M6-4/M6-5, elles-mêmes avant M6-1) :** G3, G5, le pivot complet (vitesse/jour de semaine/heure/mois) et les 4 KPI §9.1 restants (place moyenne, % tables finales, meilleur gain, plus gros tournoi), exactement le périmètre laissé en attente par la phase 1 ci-dessus — pas de nouvelle négociation de périmètre nécessaire, celle de la phase 1 couvrait déjà ce lot.
- **Nouveau blocage découvert pendant cette phase, même catégorie que G4** : la mise en évidence de « la bulle » (PRD §9.2, sortie juste avant les places payées) sur G5 demanderait `tournaments.paid_places`, colonne du schéma jamais renseignée. Vérification faite dans `docs/formats/winamax.md` §5.2 (ligne « ❓ places payées | absentes des blocs | ✅ absence ») : le nombre de places payées est confirmé **absent** du format de summary Winamax, pas seulement non-observé. G5 est donc livré sans la mise en évidence de la bulle — distribution de percentile de sortie seule, comme G4 c'est une limite du format source, pas un report.
- `TournamentResultRow` (`gr-analytics::home`) gagne `speed`/`entrants`/`finish_position` (déjà en base depuis M2-6, jamais exposés jusqu'ici) et `weekday_utc`/`hour_utc`/`month_utc` : ces 3 derniers sont calculés côté SQL (`strftime('%w'|'%H'|'%m', t.started_at/1000, 'unixepoch')`) plutôt qu'en Rust — SQLite connaît déjà le calendrier grégorien correct (mois de longueur variable, années bissextiles), pas besoin d'ajouter `chrono` pour ce seul besoin ni de réimplémenter l'algorithme à la main. Vérifié par un test d'intégration sur epoch 0 (jeudi 1ᵉʳ janvier 1970 UTC, fait connu) contre un vrai `Store`, pas seulement des valeurs de confiance.
- `gr_analytics::results` gagne `roi_by_buyin` (G3, mêmes tranches que le pivot phase 1 mais sans le croisement KO), `finish_percentile_distribution` (G5, `finish_position / entrants` bucketé par dizaine, tournois `PROVISIONAL` sans summary exclus), `compute_additional_kpis` (les 4 KPI §9.1 restants — place moyenne, % tables finales avec un seuil fixe de 9 sièges faute d'écran de paramètres, meilleur gain, plus gros tournoi par nombre d'inscrits) et 4 nouvelles dimensions de pivot (`pivot_by_speed`, `pivot_by_day_of_week`, `pivot_by_hour`, `pivot_by_month`), chacune avec son export CSV. Le pivot buy-in × KO/non-KO de la phase 1 n'a pas été généralisé (pas de refactoring hors story) : les nouvelles dimensions vivent à côté, en dupliquant sciemment le petit gabarit regroupement + CSV plutôt que d'introduire un type générique qui aurait touché du code déjà testé.
- Écran `Results.tsx` : sélecteur de dimension (5 boutons) pour le pivot, un seul tableau générique de rendu ; nouveaux `RoiByBuyinChart` (G3, barres ROI %) et `FinishDistributionChart` (G5, barres par tranche de percentile) ; 4 cartes KPI supplémentaires.
- Perf : `fetch_hero_tournament_results` a gagné 6 colonnes SQL (3 `strftime` en plus des 3 déjà prévues) sans repasser sous la marge du NFR-P5 de M6-2 — reesuré avec `perf_home` (`--release --ignored`) : 651 ms sur 1M mains (vs 717-725 ms avant cette phase), toujours sous la cible d'1 s.

**Prérequis découvert et comblé avant de commencer G2 (décision validée avec Frédéric, 29/09) :** `hand_players.net_chips`/`net_bb` existent depuis `0001_init.sql` (M2-1) mais n'avaient jamais été calculés — reporté « à M4 » par la note de M2-2, jamais repris par aucune story M4. G2 (« cumul des jetons gagnés... par main ») en avait besoin de façon bloquante.
- `gr_stats::compute_net_chips`/`compute_net_bb` (nouveau module `gr-stats/src/chips.rs`) : identité comptable `gains (hand.pots[].winners) − mises (hand.actions)`, réutilisable sans reconstruire les mises street par street — le parser normalise déjà `raises X to Y` en un `amount` incrémental (`gr-parser-winamax::parse_action_line`), donc une simple somme des montants d'action suffit. 7 tests, dont une propriété de conservation (Σ net_chips = −rake) vérifiée sur les 328 vraies mains du corpus OBELISK, pas seulement des mains construites à la main.
- Câblé dans `gr-store::repo::insert_hand_players` (colonnes déjà prévues, aucune migration). **Rétro-remplissage** pour les mains déjà importées : `gr-store::backfill_net_chips` relit `hand_raw` (seule source du détail des pots gagnants — `PotResult` ne vit qu'en mémoire pendant le parsing, jamais persisté ailleurs) et reparse (Winamax uniquement). Lancé dans un thread séparé au démarrage de `src-tauri` (pas dans `.setup()`, qui bloquerait le lancement de l'app le temps de reparser tout un corpus existant) ; sans effet (quasi instantané) une fois le rattrapage initial fait.

**Réalisé :**
- `gr-analytics::results` (nouveau module) — `fetch_hero_chip_history` (historique par main, réel + ajusté EV all-in via `hand_players.allin_ev_diff_chips`, M5-4), `fetch_hero_tournament_volume_by_day` (groupé par jour calendaire UTC, basé sur `hands`/`tournaments` — pas `tournament_entries`, qui n'existe qu'une fois le summary attaché), `pivot_by_buyin_and_ko` + `pivot_to_csv` (fonctions pures, réutilisent `compute_results_kpis` de M5-1 sans dupliquer sa logique de coût/ticket).
- `gr_analytics::lttb` (nouveau module) — Largest-Triangle-Three-Buckets écrit à la main (aucun crate LTTB dans l'écosystème testé/disponible) : garde toujours le premier/dernier point, teste par une main construite exprès (un pic isolé dans une série plate doit survivre à l'échantillonnage — contrairement à un sous-échantillonnage uniforme). Pour les graphes à deux séries superposées (G1 avec/sans bounties, G2 réel/EV), LTTB tourne sur des **indices** (pas les abscisses réelles) pour choisir quelles lignes garder, puis les deux séries sont relues aux mêmes indices — sinon, exécuté séparément par série, LTTB choisirait des points différents et casserait la superposition.
- `src-tauri::results::get_results_snapshot` — assemble les quatre morceaux, applique le downsampling LTTB (>5000 points) à G1 et G2.
- Écran `Results.tsx` : réutilise `ProfitCurveChart` (M6-2) pour G1 ; nouveaux `ChipCurveChart` (G2, ECharts, mêmes conventions que G1 — import dynamique, couleurs via tokens CSS) et `VolumeChart` (G6, barres) ; tableau du pivot ; bouton d'export CSV (téléchargement navigateur via Blob, pas de nouvelle dépendance Tauri).

### M6-4 · Tournois : liste et détail · M · `DONE (phase 1)` — le 30/09 (§13.3)
Détail : évolution du tapis, all-in, mains, adversaires avec badge de classification (après M7-5), édition des métadonnées.

**Décision de périmètre validée par Frédéric (30/09), avant implémentation :** Phase 1 = liste virtualisée + détail d'un tournoi, **en lecture seule**. Différé, cette fois **entièrement à la V2** (pas une phase 2 rapprochée comme M6-2/M6-3) :
- **Édition des métadonnées** (via ticket, vitesse, valeur du ticket gagné) — la partie ticket a son backend prêt depuis M5-2 (jamais câblé), mais l'ajout d'une correction manuelle de vitesse toucherait aussi `attach_summary` (protéger une édition manuelle d'un futur re-rattachement de summary), jugé trop risqué pour être fait rapidement à la suite.
- Le badge de classification des adversaires (UC10) reste différé après M7-5 (moteur de classification pas encore construit), comme déjà noté avant cette story.

**Nouvelle dépendance ajoutée, confirmée avec Frédéric avant implémentation (CLAUDE.md §2.8) :** `@tanstack/react-virtual` (même famille que `@tanstack/react-table` déjà utilisé) — CLAUDE.md §6 impose un tableau virtualisé au-delà de 200 lignes, et le corpus réel de Frédéric compte des milliers de tournois.

**Réalisé :**
- `gr-analytics::home::fetch_hero_tournament_results` (M6-2/M6-3) gagne un 5ᵉ paramètre `tournament_id: Option<i64>` (filtre optionnel sur un seul tournoi, `None` = tous — réutilisé tel quel pour l'en-tête du détail, pas de requête dupliquée) ainsi que `status`/`total_played_seconds` sur `TournamentResultRow` (2 colonnes SQL de plus, `t.status` et somme de `tournament_bullets.played_seconds`).
- `gr-analytics::tournaments` (nouveau module) — `fetch_hero_tournament_aggregates` (mains jouées + écart EV all-in cumulé en bb, groupés par tournoi, un seul passage sur `hand_players`/`hands`) ; `fetch_tournament_hero_hands` (tapis en début de main via `hand_players.start_stack`/`stack_bb`, déjà calculés à l'import — pas de reconstruction depuis `net_chips` comme G2 a dû le faire faute d'alternative) ; `fetch_tournament_opponents` (adversaires distincts rencontrés, sans badge de classification).
- `src-tauri::tournaments::{get_tournaments_list, get_tournament_detail}` — assemblent les morceaux ci-dessus ; les KPI par tournoi (coût/gains/profit/tickets) réutilisent directement `compute_results_kpis` (M5-1) sur un slice d'un seul élément, sans nouvelle logique de calcul.
- Écran `Tournaments.tsx` : tableau virtualisé (`@tanstack/react-virtual`, grille CSS plutôt qu'un `<table>` HTML — incompatible avec le positionnement `absolute` des lignes virtualisées), toutes les colonnes PRD §13.3, clic sur une ligne → détail. Écran `TournamentDetail.tsx` (nouvelle route `/tournaments/$tournamentId`) : en-tête (buy-in, place, gains, profit), `TournamentStackChart` (nouveau, ECharts, mêmes conventions que G1/G2 — import dynamique), tableau des all-in, tableau des adversaires, liste des mains (non virtualisée : une seule tournoi tient toujours en quelques dizaines/centaines de mains, pas besoin).
- **CA (comportement visuel du tableau virtualisé, navigation liste→détail)** à valider par Frédéric via `just dev` : test d'usage humain sur une vraie fenêtre Tauri, pas vérifiable automatiquement depuis cet environnement (même limite que M3-1/M3-3).

### M6-5 · Mains : liste virtualisée · M · `DONE` — le 30/09 (§13.4)
**CA :**
- Premier affichage < 500 ms avec 1 M mains en base.
- Tag en masse.
- Double-clic → Replayer (placeholder avant M7-3).

**Décision de périmètre :** contrairement à M6-2/M6-3/M6-4, aucune négociation de périmètre nécessaire ici — les 3 CA ci-dessus couvraient déjà exactement ce qui a été livré. Les 9 tags prédéfinis (PRD §13.7/D27 : « Bad beat, Cooler, Hero call, Bluff, Erreur préflop, Erreur postflop, À revoir, Spot ICM, Question coach ») sont déjà entièrement spécifiés par le PRD (pas une donnée à inventer, R-FORMAT ne s'applique pas ici) : semés directement plutôt que d'attendre M7-4. **Tags libres, note texte par main et filtre par tag restent le périmètre de M7-4** (Tags et notes) — non touchés ici.

**Nouvelle dépendance :** aucune (`@tanstack/react-virtual`, ajoutée pour M6-4, est réutilisée).

**Architecture — pagination côté backend (différent de M6-2/M6-3/M6-4) :** contrairement aux écrans précédents (quelques milliers de tournois, tout l'historique tient en mémoire), le volume de mains vise 1 M+ (NFR-P7). Le frontend ne charge donc que les pages visibles du tableau virtualisé (`@tanstack/react-query` `useQueries`, une requête par page de 100 lignes, mise en cache), pas un instantané complet.

**Perf (NFR-P7), avec un vrai problème d'index trouvé et corrigé avant merge, en 3 temps :**
1. 1ʳᵉ mesure (requête telle qu'écrite au départ, avec un `LEFT JOIN`/`GROUP_CONCAT` des tags avant la `LIMIT`) : **4,6 s** à 1M mains — le `GROUP BY` sur l'ensemble filtré forçait SQLite à tout matérialiser et trier avant de ne garder que la page. Corrigé en sortant les tags de la requête paginée (page d'abord, tags de cette seule page ensuite, 2ᵉ requête ciblée sur au plus 100 `hand_id`).
2. 2ᵉ mesure (nouvel index composé `ix_hands_hero_player_played_at (hero_player_id, played_at DESC)`, migration `0005`, forcé via `INDEXED BY` comme en M6-2) : toujours **2,4 s**. Diagnostiqué via `EXPLAIN QUERY PLAN` (déjà la méthode de M6-2) : `USE TEMP B-TREE FOR ORDER BY` restait présent malgré l'index composé et malgré `INDEXED BY`.
3. **Cause racine trouvée** : le filtre `hero_player_id IN (SELECT player_id FROM hero_accounts WHERE profile_id = ?1)` est une sous-requête — SQLite ne peut pas prouver à la compilation qu'elle ne renverra qu'une seule valeur, donc il ne peut pas garantir que le résultat reste déjà trié par l'index et rajoute systématiquement un tri complet. Fixé en résolvant les `player_id` du profil **côté Rust d'abord** (`resolve_hero_player_ids`), puis en liant des entiers littéraux : SQLite reconnaît alors le cas courant (un seul pseudo Hero) comme une simple égalité et n'a plus besoin de trier. Mesure finale : **85,6 ms** (page : 1,1 ms, `COUNT` : 84 ms — inévitable, doit visiter les ~1M entrées d'index correspondantes), marge ~83 % sur la cible de 500 ms. Cette limite (sous-requête empêchant l'exploitation de l'ordre d'un index) n'était pas apparue en M6-2/M6-3 : leurs requêtes filtraient par plage de dates ou par tournoi, jamais par un ensemble de `player_id` potentiellement multi-valeurs.
- Cas multi-pseudos (plusieurs `hero_accounts` pour un même profil, rare) : reste sur le chemin `IN (littéraux multiples)`, donc un tri reste possible — limite acceptée et documentée dans le code, pas mesurée spécifiquement (cas non prioritaire).

**Réalisé :**
- `gr_analytics::hands` (nouveau module) — `count_hero_hands`, `fetch_hero_hands_page` (colonnes déjà calculées à l'import : position, `eff_stack_bb`, `preflop_line`, `net_bb`, écart EV all-in en bb — aucun nouveau calcul).
- `gr-store::tags` (nouveau module) — `list_tags`, `tag_hands` (application en masse, idempotente par `INSERT OR IGNORE` sur `(hand_id, tag_id)`). **Nouvelle migration `0004_predefined_tags.sql`** (seed des 9 tags, `label_key` = clé i18n) et **`0005_hands_hero_player_played_at_index.sql`** (index de perf ci-dessus).
- `src-tauri::hands::{get_hands_count, get_hands_page, list_tags, tag_hands}` — `get_hands_count` séparée de `get_hands_page` pour ne pas repayer le `COUNT` (~84 ms) à chaque page demandée pendant le défilement.
- Écran `Hands.tsx` : tableau virtualisé (grille CSS, mêmes conventions que M6-4), pages chargées à la demande selon la plage visible du virtualizer (`useQueries`), sélection multiple (cases à cocher) + barre d'action « tag en masse » (les 9 tags prédéfinis), double-clic → `/replayer` (placeholder).
- **CA de perf et comportement visuel** à valider par Frédéric via `just dev` (fenêtre Tauri réelle) : la mesure automatisée ci-dessus couvre la requête, pas le rendu React lui-même.

---

## M7 — Analyse (§12, §13.5–13.8, §14)

**Décisions de périmètre pour tout le jalon, validées par Frédéric (30/09) avant de commencer :** M7-2, M7-4 et M7-5 sont reportées en V2 ; M7-1 est réduite aux rapports prédéfinis (pas le constructeur générique) ; M7-3, M7-6, M7-7 gardent leur périmètre PRD tel quel.

### M7-1 · Constructeur de rapports · M · `DONE (phase 1)` — le 30/09 (§13.5)
1 à 2 dimensions × N mesures, filtres, tri, mise en forme par rapport aux benchmarks, sauvegarde, et les 5 rapports prédéfinis.

**Décision de périmètre (30/09) :** **uniquement les rapports prédéfinis** — pas le constructeur générique (choix libre de dimensions/mesures, filtres, tri, sauvegarde/duplication/renommage), différé. Des 5 rapports prédéfinis du PRD §13.5, seuls 4 sont construits maintenant : « Préflop par position », « Défense de BB par profondeur », « Open-shove par profondeur × position », « Postflop (c-bet) ». **« Résultats par phase » est bloqué** : il a besoin de la phase du tournoi (`hands.phase_by_level`/`phase_by_players`), et M4-5 (qui les calcule) a déjà été reportée en V2 le 29/09 — ce rapport la rejoint en V2 plutôt que d'afficher un unique groupe « phase inconnue » sans intérêt fonctionnel.

**Réalisé :**
- 3 des 4 rapports réutilisent directement `AnalyticsBackend::run_report` (M4-7, déjà construit et testé) : « Préflop par position » (dimension `PositionGroup`, 9 mesures préflop), « Open-shove par profondeur × position » (dimensions `DepthBucket`×`PositionGroup`, mesure OSHOVE), « Postflop (c-bet) » (dimension `PositionGroup`, mesures CBF/CBT/FCBF) — aucune nouvelle requête, juste la bonne combinaison dimension(s)/mesures.
- « Défense de BB par profondeur » (PRD §12.2 : « Fold BB to steal ») a besoin d'une requête dédiée (nouveau `gr_analytics::fetch_bb_defense_by_depth`) : filtre sur la position brute `BB` (`hand_players.position`), pas le groupe `"Blinds"` qui fusionne SB et BB (PRD §10.4) — la `AnalyticsBackend` générique ne fait pas de filtre (portée de M4-7).
- `src-tauri::reports` (nouveau module) — 4 commandes dédiées (pas un dispatcher générique par id de rapport), chacune scoping au profil Hero actif.
- Écran `Reports.tsx` : 4 boutons de sélection, un tableau générique pour les 4 formes de rapport (1 ou 2 dimensions, N mesures), valeurs sous le seuil d'échantillon (30 par défaut, PRD §10.1, pas d'écran de paramètres pour le régler) grisées plutôt que masquées.
- Pas de filtre de période (M6-1) pour l'instant : `FilterBar` ne reconnaît pas encore l'écran Rapports — à étendre si besoin dans une story dédiée plutôt que d'élargir M7-1.

### M7-2 · Leaks et benchmarks · M · `BLOCKED(différé en V2, décision Frédéric le 30/09)` (§12)
Table `benchmarks` + valeurs par défaut §12.2 ; matrice stat × profondeur ; Top 10 pondéré ; clic → mains filtrées ; import/export JSON.

### M7-3 · Replayer · M · `DONE` — le 30/09 (§13.7)
Table monochrome, contrôles, raccourcis, historique brut surligné, pots/SPR, équité et EV aux all-in.

**CA :** rejoue correctement 100 % des mains du corpus (test de bout en bout : état final du replayer = résultat parsé). ✅ `every_action_of_the_real_corpus_finds_a_raw_line` (`gr-store`) rejoue chaque main de `fixtures/winamax/**/*_real_holdem_no-limit.txt` et vérifie que chaque action trouve sa ligne brute.

**Réalisé :**
- `gr-store::replay` (nouveau module) — reparse `hand_raw` (même motif `backfill::backfill_net_chips`) puis reconstruit un `ReplayStep` par action (tapis/pot/cartes visibles/board juste après l'action). Pot affiché = pot unique cumulé (Σ contributions − Σ collectes), pas un découpage main pot/side pots pendant le jeu — même simplification documentée que `gr_equity::allin` (side pots réels visibles au moment de leur collecte et dans `final_pots`). SPR = tapis de l'acteur (pas le "tapis effectif" plafonné par le plus gros adversaire, différent de `gr_stats::depth`), documenté dans le code.
- **Correspondance ligne brute** : chaque action associée à sa ligne 0-indexée dans le texte brut via un marcheur séquentiel (pseudo entier inclus dans le préfixe recherché, jamais ambigu même sur un pseudo contenant un mot d'action, cf. fixture `edge-cases/pseudo-contains-action-verb`).
- `gr-equity::allin` étendu avec `compute_all_in_details`/`AllInDetails`/`AllInPlayerDetail` (équité + EV par joueur, pas seulement l'écart) — `detect_all_in_event` réécrit par-dessus pour préserver exactement son comportement existant (callers `gr-store::repo`/`gr-ingest::import` inchangés).
- Navigation main précédente/suivante : simplification documentée — chronologique simple sur `hands.hero_player_id`, pas la liste filtrée d'origine (aucun contexte de filtre courant transmis par le routing actuellement).
- `src-tauri::replayer::get_hand_replay` — une seule commande (pas de scope par profil Hero actif, le `hand_id` suffit).
- Écran `Replayer.tsx` (route `/replayer/$handId`) : table ovale monochrome (disposition trigonométrique CSS, `ReplayerTable.tsx`, aucune dépendance graphique), contrôles début/précédent/lecture-pause/suivant/fin + vitesse 0,5×–4×, raccourcis clavier ←/→/espace, panneau historique brut avec ligne courante surlignée (auto-scroll), panneau équité/EV à l'all-in (révélé une fois la rue de l'all-in atteinte en lecture), pots finaux, SPR/mise en % du pot par décision, tags (réutilise l'API M6-5). `Hands.tsx` : double-clic navigue désormais vers `/replayer/$handId` avec le vrai `hand_id` (route `/replayer` sans id redirige vers Mains).

**Bug trouvé via `just dev` et corrigé avant merge (30/09) :** `routes/replayer.$handId.tsx` est imbriquée sous `routes/replayer.tsx` par la convention TanStack Router (même préfixe de nom de fichier — même mécanisme que `tournaments.$tournamentId.tsx`/`tournaments.tsx`, M6-4, potentiellement affecté par le même bug, pas vérifié). Sans `<Outlet />` dans le composant parent, le routeur ne rend jamais l'enfant même quand son URL matche : le double-clic naviguait bien vers `/replayer/$handId` mais affichait toujours l'état "aucune main sélectionnée" de la route parente. Corrigé en convertissant `replayer.tsx` en route de layout (`ReplayerLayout`, ne rend que `<Outlet />`) et en déplaçant l'état "aucune main sélectionnée" vers une nouvelle route index `replayer.index.tsx`.

### M7-4 · Tags et notes · M · `BLOCKED(différé en V2, décision Frédéric le 30/09)` (D27)
9 tags prédéfinis + tags libres + note par main ; filtre par tag.

**Note :** les 9 tags prédéfinis et l'application en masse depuis l'écran Mains sont déjà livrés (M6-5, 30/09) — seuls les tags libres, la note texte par main et le filtre par tag (dans le panneau de filtres global, M6-1) restent différés ici.

### M7-5 · Classification des joueurs · M · `BLOCKED(différé en V2, décision Frédéric le 30/09)` (§14)
Moteur de règles (JSON), éditeur avec prévisualisation, recalcul en fond, badges dans le Replayer et dans le détail d'un tournoi ; notes et label couleur par joueur.

### M7-6 · Backend DuckDB · S · `DONE` — le 30/09 (ADR-002)
Feature `analytics-duckdb`, synchronisation incrémentale par curseur, bouton « Reconstruire ».

**CA :**
- ✅ Mêmes résultats que le backend SQLite (tests d'équivalence) : `crates/gr-analytics/tests/duckdb_equivalence.rs`, 4 tests (1 dimension, 2 dimensions × 17 mesures, agrégat sans dimension, reconstruction), sur de vraies mains insérées via `gr-store`.
- ✅ Gain ≥ ×3 sur NFR-P6 à 2 M mains — **mesuré : ×44,3** (SQLite 5,39 s, DuckDB 122 ms ; SQLite lui-même sous sa propre cible < 8 s). `cargo test -p gr-analytics --release --features analytics-duckdb -- --ignored --nocapture perf_duckdb`. Gain largement au-dessus du seuil : pas de rapport de benchmark/ADR nécessaire (la CA ne l'exige qu'en cas d'échec de la porte ×3).

**Décision de périmètre (validée par Frédéric, 30/09) :** `DuckDbAnalyticsBackend` implémente uniquement le trait générique `AnalyticsBackend` (les 4 rapports prédéfinis de l'écran Rapports, M7-1) — c'est tout ce que ce trait sert aujourd'hui. Étendre DuckDB aux pivots/KPIs de Résultats et Accueil (requêtes SQL dédiées, jamais passées par ce trait) est **différé en V2** (`docs/V2.md`).

**Réalisé :**
- `gr-analytics::duckdb_backend` (nouveau module, `#[cfg(feature = "analytics-duckdb")]`) — `open`/`sync_incremental`/`rebuild`/`DuckDbAnalyticsBackend`. Deux tables dénormalisées **hero-only** (`f_hand_player`, `hero_accounts` — simplification documentée : c'est tout ce que `SqliteAnalyticsBackend` lit déjà) synchronisées par curseur `hands.id` (`sync_state`, persisté dans `analytics.duckdb` lui-même, pas dans `gr-store::settings` — le fichier n'est jamais sauvegardé donc le curseur repart naturellement à zéro s'il est supprimé). Résolution `hero_player_id -> profile_id` à la requête (pas au sync, via `hero_accounts` resynchronisée en entier à chaque fois) pour éviter qu'un rattachement de pseudo après le sync ne rende le cache silencieusement faux.
- `src-tauri::analytics_duckdb` (nouveau module, compile toujours) — thread dédié, seul propriétaire de la `duckdb::Connection` (`Send`, pas besoin de `Mutex`/`Sync`), reçoit les demandes par canal `mpsc` : notification (débounce 5 s, PRD §7.3 point 8, coalescée) ou reconstruction (immédiate, bloque sur une réponse). Déclencheurs : fin d'import manuel (`import::import_paths`) et watcher temps réel (`watch.rs`, tick `hands://new`). Commande `reconstruct_analytics_index` toujours enregistrée — renvoie une erreur explicite (pas un branchement conditionnel côté frontend) si le build n'a pas la feature ou si l'ouverture a échoué au démarrage.
- Section « Analytique » minimale dans l'écran Paramètres (bouton « Reconstruire l'index analytique ») — ajout autonome en attendant la refonte complète M8-1, décision validée par Frédéric.
- Nouvelle dépendance `duckdb` (crate, feature `bundled`) : déjà nommée par le PRD (§7.3/ADR-002), pas une dépendance à reconfirmer. Coût de compilation réel et notable (C++ vendoré) confirmé en pratique (~35-40 min la première fois, `cargo build`/`cargo clippy` ont des caches de compilation séparés donc chacun recompile une fois) — déjà anticipé par ADR-004 (`just dev` sans la feature, `just dev-full` avec).

### M7-7 · Grille 13×13 des mains de départ · S · `TODO` (§13.5)

**Vérifié avant de commencer (30/09, demande de Frédéric) :** rien n'existe encore — ni composant UI, ni donnée backend. `hand_players.hand_class` (ex. `'AKs'`, `'TT'`, `'Q9o'`) est une colonne prévue depuis `0001_init.sql` (M2-1) mais **jamais calculée**, même situation que `net_chips` avant sa découverte en M6-3. Une vraie story à construire de zéro (calcul de `hand_class` depuis `hole_cards` + rétro-remplissage des mains déjà importées, puis la grille elle-même), pas une simple UI à brancher sur une donnée existante.

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
- **Amélioration UX du panneau de filtres, notée pour la V1.1** (retour de Frédéric après test manuel de M6-1 via `just dev`, 30/09) : remplacer les boutons de preset de date + le formulaire de sauvegarde séparé par une **liste déroulante unique**. « Période libre » serait la dernière option de la liste ; la sélectionner ouvrirait une pop-up avec date de début, date de fin et un nom, à sauvegarder — la nouvelle période nommée réapparaîtrait alors directement dans la liste déroulante (plus besoin d'un second contrôle « enregistrer un preset » à côté). Périmètre UX à part entière, pas une correction de bug ; `FilterBar.tsx` (M6-1) resterait la base à modifier.
- **Bug de double barre de scroll horizontal** (retour de Frédéric après test manuel de M6-4/M6-5 via `just dev`, 30/09) : les tableaux virtualisés Tournois (`Tournaments.tsx`) et Mains (`Hands.tsx`) ont chacun deux barres de défilement horizontal (probablement le conteneur `overflow-x-auto` externe et le conteneur interne du virtualizer qui ont tous les deux une largeur `minWidth: 'max-content'` et peuvent tous les deux devenir scrollables), ce qui produit un « comportement étrange » signalé par Frédéric. Noté pour la V1.1 plutôt que corrigé dans l'immédiat (retour groupé avec l'item ci-dessus) — à investiguer : un seul des deux conteneurs devrait porter le scroll horizontal, probablement en retirant `overflow-x-auto`/`minWidth` du conteneur externe et en ne le gardant que sur celui qui contient l'en-tête + le corps virtualisé ensemble.
- **`/tournaments/$tournamentId` confirmée affectée par le même bug de route imbriquée que M7-3, noté pour la V1.1** (confirmé par Frédéric via `just dev`, 30/09) : `tournaments.$tournamentId.tsx` est imbriquée sous `tournaments.tsx` par la même convention TanStack Router (même préfixe de nom de fichier), et `Tournaments.tsx` (le composant de la route parente) ne rend pas de `<Outlet />` — exactement la construction qui empêchait le Replayer de s'afficher (corrigé le 30/09). Le clic sur un tournoi ne fait rien : l'URL change bien vers `/tournaments/$tournamentId` mais la liste reste affichée. **Correctif différé à la V1.1** (décision de Frédéric, regroupé avec les autres retours `just dev` ci-dessus) plutôt que corrigé dans l'immédiat. Même correctif que le Replayer à appliquer alors : `tournaments.tsx` en route de layout (`<Outlet />` seul) + nouvelle route index `tournaments.index.tsx` pour la liste actuelle.
