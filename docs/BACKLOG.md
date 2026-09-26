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

### M0-2 · Toolchain et squelette du workspace · M · `TODO`
- Installer : Rust stable MSVC, VS Build Tools 2022 (C++), Node 20 LTS, pnpm, `just`, `sccache` (optionnel).
- Créer le workspace Cargo et les crates vides de la §7.3, `src-tauri` (Tauri 2) et `ui` (Vite + React 19 + TS strict + Tailwind + shadcn/ui).
- Créer le `justfile` avec les commandes de `CLAUDE.md` §4.

**CA :**
- `just dev` ouvre une fenêtre « Graphite ».
- `just check` passe.
- Profil `dev` configuré (`[profile.dev.package."*"] opt-level = 1`).

### M0-3 · CI et release GitHub Actions · M · `TODO`
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

### M0-4 · Corpus réel et documentation du format Winamax · M · `DOING` — 2 tournois reçus et documentés le 26/09
**Déjà fait :** `docs/formats/winamax.md` (445 mains, conservation vérifiée 445/445) ; fixtures anonymisés `fixtures/winamax/mtt/space-ko/` et `space-ko-3max-itm-reentry/` ; `tools/anonymize.py`. Constats : **fichiers en anglais** ; ITM, re-entry, 3-max et changements de table couverts. **Reste :** les 7 cas du §7 de `winamax.md` (classique, Mystery KO, tickets, satellite, freeroll, table finale, fichier en cours). M1-1 à M1-6 peuvent démarrer ; seules les parties tickets et formats non-KO de M1-6 attendent ces fichiers.

**Action de Frédéric :** copier dans `fixtures/raw/` (ignoré par git) le contenu de `%APPDATA%\winamax\documents\accounts\<pseudo>\History\`. Le corpus doit contenir au minimum 300 mains et 20 summaries couvrant : classique, PKO, Mystery KO, Space KO, freeroll, satellite avec ticket gagné, tournoi joué via ticket, re-entry, heads-up final, all-in multiway avec side pots.

**Claude Code :**
- écrire `tools/anonymize` (Rust bin ou script) : pseudos → `P0001…` de façon déterministe, Hero → `Hero` ;
- produire `fixtures/winamax/mtt/<format>/` ;
- compléter **`docs/formats/winamax.md`** : nommage des fichiers, encodage/BOM, en-têtes, toutes les lignes d'action, sections, summary, bounties, tickets, re-entries. Préciser si le nombre de joueurs restants ou les places payées apparaissent (impact §10.5), ainsi que les formats de titre de fenêtre si connus.

**CA :**
- Chaque type de ligne rencontré est listé avec un exemple réel anonymisé.
- La liste des inconnues restantes est rédigée et validée par Frédéric.

### M0-5 · Shell UI et design tokens · S · `TODO`
- `tokens.css` (§16), polices Inter et JetBrains Mono embarquées, layout : barre latérale, barre de filtres (vide), barre d'état.
- Routes vides pour les 8 écrans + Logs.
- i18next FR/EN branché, avec sélecteur de langue.

**CA :**
- Navigation fonctionnelle.
- Aucune chaîne en dur (règle ESLint `i18next/no-literal-string` active).

---

## M1 — Parser Winamax (§8.1–8.3, `docs/formats/winamax.md`)

### M1-1 · Types de domaine `gr-core` · M · `TODO`
Card, Rank, Suit, Street, ActionKind, Position, Money(cents), Chips, HandRecord, SeatInfo, ActionRecord, PotResult, TournamentSummary, KoType.

**CA :**
- Sérialisation serde.
- Parsing et affichage des cartes (`"Ah"`).
- Tests unitaires.

### M1-2 · Trait `RoomParser` et détection · M · `TODO`
Détection de room et de langue par signature d'en-tête ; gestion de l'encodage (BOM UTF-8, repli Windows-1252 si le corpus l'exige).

**CA :** les fichiers du corpus sont détectés `winamax` à 100 %.

### M1-3 · Découpage des mains et blocs incomplets · M · `TODO`
**CA :**
- Un fichier tronqué au milieu d'une main renvoie N mains complètes, plus un offset qui s'arrête avant la main incomplète.
- Tests avec des troncatures à chaque ligne d'une main (property test).

### M1-4 · En-tête, table, sièges, blindes/antes · M · `TODO` (PAR-4/5/6)
**CA :**
- Snapshots validés sur tout le corpus.
- Les pseudos avec espaces et caractères spéciaux sont couverts par un edge-case.

### M1-5 · Actions, streets, board, showdown, pots · M · `TODO` (PAR-7 à PAR-10)
**CA :**
- Invariant de conservation PAR-11 vérifié sur 100 % du corpus.
- Side pots corrects sur le fixture multiway (main 49 du fixture Space KO : `side pot 1` = excédent non suivi).
- Joueurs assis mais non distribués exclus (mains 1 et 2 du fixture Space KO).
- Pots partagés (7 cas dans le fixture OBELISK) et heads-up à une table 3-max (`(small blind) (button)`).
- Calcul de l'excédent non suivi par main (champ `uncalled_excess`), utilisé par l'EV all-in (M5-4).

### M1-6 · Parser de summaries · M · `TODO` (PAR-12, §8.6)
**CA :**
- Buy-in décomposé prize/bounty/fee.
- Summaries **multi-blocs** (un par entrée), suffixe ` - Late Registration`, `You won X€ + Bounty Y€` / `You won Bounty Y€` / ligne absente.
- Découpage des mains par `entry_no` (réapparition du Hero après élimination).
- Test chiffré : OBELISK → 2 entrées, coût 4,00 €, gains 52,19 €, profit +48,19 €, place 7.
- Tickets : uniquement quand le fixture sera fourni.
- Snapshots validés.

### M1-7 · Robustesse et benchs du parser · M · `TODO` (PAR-14/15)
**CA :**
- Fuzz léger (proptest) sans panique.
- Bench `criterion` ≥ 5 000 mains/s mesuré sur la machine de Frédéric.
- Chaque `ParseError` porte un code et une ligne.

---

## M2 — Stockage, import en masse, logs (§8.4, §15, §13.10)

### M2-1 · Migrations SQLite initiales · M · `TODO`
DDL §15 en `0001_init.sql` ; PRAGMA WAL ; pool de lecture et writer unique.

**CA :**
- Base créée au premier lancement dans le dossier de données (ADR-007, mode portable compris).
- Tests de migration.

### M2-2 · Repositories et insertion par lots · M · `TODO`
Insertion `hands`, `hand_players` (sans flags de stats pour l'instant), `actions`, `hand_raw` (zstd) et `tournaments` provisoires.

**CA :**
- Dédoublonnage `UNIQUE(room, hand_id)` testé.
- Transaction par lot de 500 mains.

### M2-3 · Import en masse · M · `TODO`
Sélection de dossiers ou de fichiers, progression (événement IPC), annulation, rapport final.

**CA :**
- Import du corpus complet sans erreur.
- Réimport = 0 insertion, N doublons.
- Premier jet de ≥ 1 000 mains/s sur 100 000 mains synthétiques.

### M2-4 · `gr-synth` : générateur de mains synthétiques · M · `TODO`
Générateur de tournois cohérents au format Winamax, basé **exclusivement** sur les motifs documentés en M0-4, avec une graine déterministe.

**CA :**
- `just synth N=1000000` génère environ 1 M de mains parsables à 100 %.

### M2-5 · Logs et erreurs d'import · M · `TODO`
`tracing` + rotation quotidienne, rétention 30 jours ; table `import_errors` ; écran Logs (visionneuse + onglet Erreurs d'import, avec les actions Reparser / Ignorer / Copier).

**CA :**
- Une main corrompue injectée apparaît dans l'onglet Erreurs.
- « Reparser » fonctionne après correction du parser.

### M2-6 · Rattachement des summaries et des tournois · M · `TODO` (§8.5)
**CA :**
- Un tournoi passe de `PROVISIONAL` à `COMPLETE` à l'import de son summary, quel que soit l'ordre d'import.
- Les re-entries sont comptées.
- Un tournoi sans summary passe `INCOMPLETE` après 24 h sans nouvelle main.

---

## M3 — Temps réel et sessions (§8.4)

### M3-1 · Assistant de premier lancement · M · `TODO` (UC1)
Détection de `%APPDATA%\winamax\documents\accounts\*\History`, choix des pseudos → profil Hero, import initial.

**CA :** un ami configure l'application en moins de 5 min sans aide.

### M3-2 · Watcher temps réel · M · `TODO`
`notify` + polling de secours (2 s) ; lecture incrémentale par offset ; backoff sur fichier verrouillé ; gestion de la troncature et du renommage.

**CA :**
- Test d'intégration qui simule 12 fichiers écrits en parallèle (1 main/3 s/table) pendant 10 min : p95 < 2 s, 0 perte, 0 doublon.
- CPU < 10 % en moyenne.

### M3-3 · Tray et barre d'état · S · `TODO`
Icône dans la zone de notification (Pause/Reprendre import, Ouvrir, Quitter) ; barre d'état avec le statut d'import, les mains du jour et la dernière main ; priorité processus Below Normal (§6.2).

**CA :** fermer la fenêtre garde l'import actif (option).

### M3-4 · Sessions · S · `TODO` (§9.3, H3)
**CA :** regroupement avec seuil paramétrable ; recalcul correct lors d'un import hors ordre.

### M3-5 · Profils Hero multi-pseudos · M · `TODO` (D19)
**CA :**
- Création et modification de profils.
- Toutes les requêtes Hero filtrent par `hero_accounts` du profil sélectionné.

---

## M4 — Moteur de stats (§10)

### M4-1 · Positions et profondeurs · M · `TODO` (§10.3, §10.4)
**CA :**
- Positions correctes de 2 à 10 joueurs (tests table-driven).
- bb joueur et effectifs.
- Tranches configurables.

### M4-2 · DSL de test `hand!{}` · M · `TODO`
Construction concise de mains de test pour `gr-stats`.

### M4-3 · Flags préflop · M · `TODO`
VPIP, PFR, RFI, LIMP, OSHOVE, 3B, F3B, 4B, ATS, FSTEAL (SB/BB), RSTEAL, et la ligne préflop synthétique.

**CA :**
- 3 tests minimum par stat (positif, négatif, pas d'opportunité).
- Colonnes `hand_players` remplies à l'import (migration `0002`).

### M4-4 · Flags postflop · M · `TODO`
CBF, CBT, FCBF, WTSD, W$SD, WWSF, compteurs AF/AFQ.

**CA :** idem M4-3.

### M4-5 · Phases du tournoi · M · `TODO` (§10.5)
Méthode A (profils de structure) basée sur la structure `Levels` du summary, et méthode B (estimation par interpolation, car les places payées et les joueurs restants sont absents) ; choix dans les paramètres.

**CA :**
- Les deux colonnes sont remplies.
- La méthode B est marquée « estimation » si elle est interpolée.

### M4-6 · Compteurs incrémentaux `player_stat_counters` · M · `TODO` (§10.7)
**CA :**
- Compteurs = recalcul complet (test d'équivalence sur le corpus).
- Lecture de 10 joueurs < 20 ms.
- Job de reconstruction.

### M4-7 · `AnalyticsBackend` (implémentation SQLite) · M · `TODO`
API de requête générique : dimensions, mesures (stats/KPIs), filtres → table de résultats.

**CA :**
- Toutes les stats par position × profondeur pour le Hero.
- Performances NFR-P6 (version SQLite) mesurées et consignées.

### M4-8 · Validation croisée des stats · S · `TODO`
Si Frédéric a accès à un autre tracker (essai PT4/HM3 ou Xeester), comparer les stats du Hero sur un même échantillon.

**CA :** écarts < 0,5 point, ou différences de définition documentées.

---

## M5 — Résultats MTT et EV all-in (§9, §10.6)

### M5-1 · Calcul des KPIs de résultats · M · `TODO` (§9.1)
Coût, frais, prize/bounty, tickets (paramètre FACE_VALUE/ZERO), profit, ROI, ROI hors frais, ROI prize pool / bounties, ITM, ABI, freerolls.

**CA :** scénarios chiffrés au centime (fichier `tests/results_scenarios.rs`), y compris re-entry, ticket utilisé et gagné, et PKO.

### M5-2 · Tickets : métadonnées et types · M · `TODO` (§8.6)
Table `ticket_types` ; édition « joué via ticket » et « valeur du ticket » par tournoi ; filtre Via ticket.

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
