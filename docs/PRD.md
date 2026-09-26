# PRD — Graphite, tracker de poker MTT pour Windows

> **Nom de code :** Graphite (provisoire, modifiable)
> **Version du PRD :** 1.0 — 26/09/2026
> **Auteur / Product Owner :** Frédéric
> **Destinataire :** Claude Code (développement), Frédéric (validation)
> **Documents liés :** `CLAUDE.md` (conventions de dev), `docs/BACKLOG.md` (épics, user stories, jalons)

---

## 0. Résumé exécutif

Graphite est une application de bureau Windows, 100 % locale, qui :

1. lit **en temps réel** les historiques de mains (hand histories, HH) et les résumés de tournois écrits par le client **Winamax** (en français) ;
2. les stocke dans une **base locale** (SQLite = source de vérité, DuckDB = couche analytique) ;
3. calcule des **indicateurs MTT** sur le jeu du joueur (Hero) — résultats (ROI, ITM, ABI, frais, tickets, bounties), EV all-in, stats de jeu par profondeur de tapis et par phase du tournoi — et sur ses adversaires ;
4. aide à **détecter les leaks** (comparaison à des benchmarks modifiables, rapports personnalisables, replayer avec équité et tags).

La **phase 2 (V2)** ajoutera la fiche joueur et un **HUD** superposé aux tables. L'architecture V1 doit la préparer : compteurs de stats incrémentaux, détection des fenêtres, stats contextuelles, notes et tags joueurs.

Utilisateurs : Frédéric et quelques amis. Diffusion : exécutable portable. Échéance : V1 avant le **31/12/2026**.

---

## 1. Décisions de cadrage (validées)

| # | Sujet | Décision |
|---|---|---|
| D1 | Rooms | **Winamax** en V1. Architecture multi-room par plugins de parsing. iPoker / 888 en V1.1 (voir §6.4 : conflit devise sur ACR). |
| D2 | Formats | **MTT uniquement** : classiques, Space KO, Mystery KO, PKO, freerolls, tournois joués via tickets, satellites. Pas de cash game ni d'Expresso en V1. |
| D3 | Stack | **Tauri 2 (Rust) + React/TypeScript** |
| D4 | Stockage | **SQLite** (vérité, import, futur HUD) + **DuckDB** (rapports analytiques) — voir ADR-002 |
| D5 | Objectifs | Analyse de mon jeu et des leaks + suivi des résultats et de la bankroll |
| D6 | Utilisateurs | Moi + quelques amis, sans licence ni compte |
| D7 | Volumétrie | 500 000 à 5 000 000 de mains ; 5 à 12 tables simultanées |
| D8 | KPIs MTT | ROI / ITM / ABI / profit ; EV all-in ; stats par profondeur de tapis ; phases du tournoi |
| D9 | ICM | Hors périmètre ; prévu en V3+ (le modèle de données le permet) |
| D10 | PKO | Oui : bounties gagnés, ROI séparé prize pool / bounties |
| D11 | Import | Temps réel (watcher) + import en masse + tournament summaries. Pas de saisie manuelle. |
| D12 | Leaks | Benchmarks de référence + rapports personnalisables + replayer avec tags. Pas d'IA en V1. |
| D13 | Données | 100 % local, aucun appel réseau |
| D14 | Langues | Interface FR/EN (i18n). Client Winamax en français. **Constat sur le corpus réel : les fichiers HH et summary sont écrits en anglais même avec un client FR** (voir `docs/formats/winamax.md`). |
| D15 | UI | Dashboard moderne sombre, **noir / blanc / nuances de gris**, sobre |
| D16 | Prépa HUD | Cache de stats incrémental ; détection des fenêtres de tables ; stats contextuelles ; notes et tags joueurs |
| D17 | Distribution | **Exécutable portable** (.exe), sans installeur |
| D18 | Qualité | Niveau « pro » : tests unitaires sur un corpus réel, CI GitHub Actions, lint bloquant |
| D19 | Hero multi-pseudos | Plusieurs pseudos regroupables en un **profil Hero** sélectionnable |
| D20 | Tickets | Valorisation paramétrable (valeur faciale ou 0) ; les tournois joués via ticket sont distingués |
| D21 | Frais | Rake / frais suivis séparément dans le ROI |
| D22 | Devise | **€ uniquement** |
| D23 | Stats hero | VPIP, PFR, 3-bet, Fold to 3-bet, open-shove par profondeur, Steal, Fold to steal, Resteal, C-bet, Fold to C-bet, WTSD, W$SD, AF |
| D24 | Profondeurs | <10, 10–15, 15–25, 25–40, 40–60, 60–80, 80–100, 100+ bb |
| D25 | Phases | Méthode de détection **au choix de l'utilisateur** (niveaux de blindes / joueurs restants) |
| D26 | Export de mains | Hors périmètre pour l'instant |
| D27 | Tags de mains | Tags prédéfinis + tags libres |
| D28 | Équité all-in | Affichée dans le replayer |
| D29 | Classification joueurs | Automatique (fish/reg/nit/maniac…) avec seuils modifiables |
| D30 | Fiche joueur / recherche | En V2 |
| D31 | Écrans V1 | Accueil/KPIs, Résultats, Tournois, Mains, Rapports, Leaks, Replayer, Paramètres |
| D32 | Notifications | Aucune ; **gestion de logs** consultable dans l'application |
| D33 | Sauvegarde | Base locale ; sauvegarde locale **hebdomadaire** |
| D34 | Machine cible | **Intel Celeron N5095A (4 cœurs), 16 Go RAM, SSD 500 Go**, Windows 10/11 |
| D35 | Repo / CI | GitHub privé à créer + GitHub Actions (voir BACKLOG M0) |

### 1.1 Hypothèses prises par défaut (à confirmer, modifiables)

Ces points ont été repris dans les réponses sans être tranchés. Les valeurs ci-dessous s'appliquent tant qu'elles ne sont pas remises en cause :

- **H1 – Benchmarks :** valeurs MTT indicatives fournies par défaut (§12.2), toutes modifiables, avec import/export JSON.
- **H2 – Graphes de variance :** courbe de profit (€) et courbe « chips gagnés vs EV all-in » en V1. Simulateur de downswing en V1.1.
- **H3 – Sessions :** regroupement automatique quand il n'y a aucune main depuis plus de 30 min (seuil paramétrable).
- **H4 – Filtres :** tous les filtres proposés sont en V1 (§11).
- **H5 – Licence :** repo privé, licence propriétaire « All rights reserved » par défaut.

---

## 2. Contexte et vision

### 2.1 Problème
Les trackers du marché sont soit lourds et datés (PokerTracker 4 : PostgreSQL, interface ancienne), soit sur abonnement (Hand2Note), soit peu adaptés aux MTT Winamax en français (PT4 exige un client en anglais). Le HUD natif de Winamax (octobre 2025) ne garde que 60 jours de données et reste limité.

### 2.2 Vision
Un tracker **MTT-first**, rapide sur une machine modeste, lisible en noir et blanc, qui répond en un coup d'œil à trois questions :
1. *Est-ce que je gagne, et pourquoi ?* (résultats vs EV, par format, buy-in et phase)
2. *Où est-ce que je perds de l'argent ?* (leaks par position, profondeur de tapis et phase)
3. *Contre qui je joue ?* (V1 : classification ; V2 : fiche joueur + HUD)

### 2.3 Benchmark concurrentiel (synthèse)

| Produit | À reprendre | À éviter |
|---|---|---|
| PokerTracker 4 | LeakTracker (benchmarks), replayer avec équité, profils de HUD, rapports personnalisables | PostgreSQL, interface dense et datée, client EN imposé |
| Holdem Manager 3 | Moteur de base intégré, Situational Views, filtres avec autocomplétion | Windows-only accepté, mais pas la complexité |
| Hand2Note 4 | Stats contextuelles (position, profondeur, séquence d'actions), import très rapide, stockage compact | Courbe d'apprentissage, abonnement |
| Xeester | Tracker FR, stats sur 5 axes (mains de départ, positions, zones, mises, mises adverses), replayer avec HUD contextuel, notes | — |
| Pokstats | Orientation MTT, dashboards de résultats, tiling des tables | — |
| Winamax HUD natif | Couleurs par stat, stats adaptées au format | 60 jours de rétention, peu de stats |

---

## 3. Objectifs, non-objectifs et mesures de succès

### 3.1 Objectifs V1
- O1 : importer 100 % des mains et tournois MTT Winamax FR du corpus de test sans erreur de parsing.
- O2 : rendre une nouvelle main visible dans l'interface en **moins de 2 s (p95)** après son écriture sur disque, en jouant 12 tables.
- O3 : fournir tous les KPIs de résultats (§9) et les stats de la §10, filtrables (§11).
- O4 : afficher les leaks du Hero face aux benchmarks, par position, profondeur de tapis et phase.
- O5 : tenir les cibles de performance de la §17 sur la machine cible (Celeron N5095A).

### 3.2 Non-objectifs V1
Pas de cash game, Expresso/Spin ni Omaha. Pas de HUD en overlay (V2). Pas de fiche joueur détaillée (V2). Pas d'ICM, pas de push/fold (V3+). Pas de cloud ni de synchronisation. Pas d'export de mains. Pas de saisie manuelle de tournois. Pas d'IA. Pas de notifications.

### 3.3 Mesures de succès
- Taux de parsing ≥ 99,9 % sur le corpus réel ; 0 main perdue (chaque échec est tracé dans `import_errors`).
- Les chiffres de ROI et de profit concordent au centime avec le relevé Winamax (« Mon compte > historique ») sur un mois test.
- Frédéric utilise Graphite à chaque session pendant 4 semaines consécutives.

---

## 4. Personas et cas d'usage

**P1 — Frédéric (Hero, grinder MTT)** : joue 5 à 12 tables en soirée sur Winamax, en français. Il revoit ses sessions le lendemain.
**P2 — Ami joueur** : reçoit l'exe portable, configure son pseudo et son dossier d'historiques en moins de 5 minutes.

| ID | Cas d'usage |
|---|---|
| UC1 | Au premier lancement, l'assistant détecte le dossier Winamax, propose les pseudos trouvés et importe l'existant. |
| UC2 | Pendant le jeu, Graphite tourne en arrière-plan et importe chaque main en continu (tray icon, indicateur d'état). |
| UC3 | Après une session : « Combien j'ai gagné, en réel et en EV, par tournoi ? » |
| UC4 | « Quel est mon ROI sur les Mystery KO entre 5 et 20 € de buy-in ce trimestre, frais séparés ? » |
| UC5 | « Est-ce que je sur-folde ma BB face aux steals à 15–25 bb ? » (Leaks) |
| UC6 | Revoir les all-in perdus en étant devant (replayer + équité), les taguer « bad beat » ou « erreur ». |
| UC7 | Regrouper les pseudos `Fred_W` et `FredMTT` sous le profil « Frédéric ». |
| UC8 | Créer un rapport « 3-bet par position × profondeur » et le sauvegarder. |
| UC9 | Consulter les logs d'import et relancer l'import d'une main en erreur après une mise à jour du parser. |
| UC10 | Voir la classification des adversaires d'un tournoi (liste simple ; la fiche détaillée arrive en V2). |

---

## 5. Périmètre par version

| Version | Contenu |
|---|---|
| **V1.0** (fin 2026) | Winamax FR MTT ; import temps réel et en masse ; summaries ; résultats ; stats Hero ; phases ; profondeurs ; EV all-in ; leaks ; rapports ; replayer + tags ; classification des adversaires (liste) ; logs ; sauvegardes ; i18n FR/EN ; exe portable |
| **V1.1** | Parsers iPoker et 888 (en €) ; simulateur de downswing ; ajustements des benchmarks |
| **V2** | Recherche de joueur et fiche détaillée ; **HUD overlay** (stats statiques puis contextuelles, popups, notes) ; tiling des tables en option |
| **V3+** | ICM (calcul hors session) ; analyse push/fold post-session ; export de mains ; IA (diagnostic par LLM) |

---

## 6. Contraintes

### 6.1 Réglementaires (rooms)
- **Winamax** tolère les trackers et HUD tiers (PT4, HM3, Xeester). Il **interdit** les ranges « dynamiques » et les solveurs ouverts pendant le jeu (PioSolver, Equilab, ICMizer), sous peine de suspension.
  - ⇒ **Règle produit R-COMP-1 :** Graphite n'affiche **jamais** de conseil de décision, de range recommandée ni de calcul d'équité ou d'ICM **pendant qu'une table est ouverte**. Les calculs d'équité ne servent qu'à l'analyse **a posteriori** d'une main terminée.
  - ⇒ **R-COMP-2 :** le futur HUD (V2) n'affiche que des statistiques descriptives. Avant la V2, relire les CGU Winamax et demander confirmation écrite à `support@winamax.fr`.
- GGPoker et partypoker interdisent les HUD tiers : ils ne seront pas supportés.

### 6.2 Matérielles
Celeron N5095A : 4 cœurs / 4 threads, 2,0–2,9 GHz, faible puissance. Conséquences :
- l'import et les calculs s'exécutent sur un pool limité (par défaut 2 threads de travail) pour ne pas ralentir le client Winamax pendant le jeu ;
- priorité de processus **Below Normal** pendant le jeu ;
- les compilations Rust seront lentes sur cette machine : les builds de release se font sur la CI (voir ADR-004).

### 6.3 Légales et données
Données personnelles du joueur et pseudos d'adversaires stockés localement uniquement. Aucune télémétrie.

### 6.4 Conflit identifié : ACR et devise
ACR joue en USD alors que D22 impose « € uniquement ». **ACR est donc exclu.** iPoker et 888 ne sont supportés que pour les tournois en €. Un tournoi dans une autre devise est importé mais **exclu des agrégats**, et un avertissement est journalisé.

---

## 7. Architecture

### 7.1 Vue d'ensemble

```
┌──────────────────────────── Processus Graphite (Tauri 2) ────────────────────────────┐
│                                                                                       │
│  ┌─────────────┐   événements    ┌──────────────┐   HandRecord   ┌─────────────────┐  │
│  │  Watcher    │ ──────────────▶ │  Ingest      │ ─────────────▶ │  Store (SQLite) │  │
│  │ (notify +   │  fichier modifié│  pipeline    │  + erreurs     │  WAL, migrations│  │
│  │  polling)   │                 │  (offsets,   │                │  = VÉRITÉ       │  │
│  └─────────────┘                 │  découpage,  │                └───────┬─────────┘  │
│        ▲                         │  dédoublon.) │                        │ sync incr. │
│        │ dossiers History        └──────┬───────┘                        ▼            │
│        │                                │ appelle          ┌─────────────────────┐    │
│   Winamax client                ┌───────▼────────┐         │ Analytics (DuckDB)  │    │
│   écrit *.txt                   │ Parsers (trait │         │ tables dérivées,    │    │
│                                 │ RoomParser)    │         │ reconstructible     │    │
│                                 │ winamax_fr ... │         └─────────┬───────────┘    │
│                                 └───────┬────────┘                   │                │
│                                         ▼                            │                │
│                                 ┌────────────────┐                   │                │
│                                 │ Stats engine   │──▶ compteurs HUD  │                │
│                                 │ (flags/main,   │    (SQLite)       │                │
│                                 │ équité, EV)    │                   │                │
│                                 └────────────────┘                   │                │
│                                                                      │                │
│  ┌──────────────────────── Commandes Tauri (IPC typé) ◀──────────────┘                │
│  │                                                                                    │
│  ▼                                                                                    │
│  UI React (WebView2) : Accueil · Résultats · Tournois · Mains · Rapports · Leaks ·    │
│                        Replayer · Paramètres · Logs                                   │
└───────────────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Stack technique

| Couche | Choix | Justification |
|---|---|---|
| Shell | Tauri 2.x | Léger (WebView2), backend Rust, bon support des overlays pour la V2 |
| Backend | Rust stable (édition 2021), Tokio | Performance sur un CPU faible |
| Watcher | `notify` + polling de secours (2 s) | Les événements FS Windows peuvent se perdre |
| SQLite | `rusqlite` (feature `bundled`), WAL, `r2d2`/pool | Vérité, écritures rapides, lectures concurrentes |
| DuckDB | crate `duckdb` (feature `bundled`) derrière la feature cargo `analytics-duckdb` | Rapports agrégés rapides (ADR-002) |
| Évaluateur de mains | module interne (lookup 7 cartes) ou crate éprouvée (`rs_poker`, à évaluer en M5) | Équité all-in |
| Logs | `tracing` + `tracing-subscriber` + `tracing-appender` (rotation quotidienne) | Logs consultables (D32) |
| Erreurs | `thiserror` (crates lib), `anyhow` (couche app) | |
| Sérialisation | `serde`, `serde_json`; types TS générés via `ts-rs` ou `specta` | IPC typé de bout en bout |
| Front | React 19 + TypeScript strict + Vite | |
| UI kit | Tailwind CSS + shadcn/ui (Radix) | Composants accessibles, thème monochrome facile |
| Données front | TanStack Query (cache IPC), TanStack Table + virtualisation | Tableaux de 100 000+ lignes |
| Graphiques | Apache ECharts (échantillonnage LTTB pour les longues séries) | Performant, thème monochrome personnalisable |
| i18n | i18next + react-i18next ; ICU pour les pluriels | D14 |
| État UI | Zustand (filtres globaux, préférences) | |
| Tests | `cargo test`, `insta` (snapshots), `proptest`, `criterion` ; Vitest + Testing Library | D18 |

### 7.3 Organisation du code (workspace Cargo)

```
graphite/
├─ CLAUDE.md
├─ README.md
├─ docs/ PRD.md · BACKLOG.md · adr/ · formats/winamax.md
├─ fixtures/                      # corpus réel anonymisé (voir §15.2)
│   └─ winamax/mtt/{space-ko,classic,mystery-ko,…}/ + winamax/edge-cases/
├─ crates/
│   ├─ gr-core/        # types de domaine : Card, Action, Street, Position, Money, Chips, HandRecord, TournamentRecord
│   ├─ gr-parser-api/  # trait RoomParser, erreurs de parsing, détection de room/langue
│   ├─ gr-parser-winamax/  # parser Winamax (dictionnaires FR et EN, voir §8.3)
│   ├─ gr-equity/      # évaluateur de mains, équité exacte ou Monte Carlo, EV all-in
│   ├─ gr-stats/       # calcul des flags de stats par main et par joueur, définitions (§10)
│   ├─ gr-store/       # SQLite : migrations, repositories, sauvegardes
│   ├─ gr-analytics/   # trait AnalyticsBackend ; implémentations SQLite et DuckDB
│   ├─ gr-ingest/      # watcher, suivi des offsets, pipeline d'import, reparse
│   ├─ gr-synth/       # générateur de mains synthétiques au format Winamax (tests de performance)
│   └─ gr-winmon/      # (V2, mais amorcé en V1) détection des fenêtres de tables Win32
├─ src-tauri/          # app Tauri : commandes IPC, état, tray, config
└─ ui/                 # React
```

### 7.4 Décisions d'architecture (ADR)

- **ADR-001 — SQLite source de vérité.** Toutes les écritures passent par SQLite en mode WAL. Un seul writer (tâche dédiée), plusieurs lecteurs.
- **ADR-002 — DuckDB comme couche analytique dérivée.**
  - DuckDB est alimenté par **synchronisation incrémentale** depuis SQLite (curseur `hands.id`) après chaque lot d'import. Il n'y a pas d'extension `sqlite_scanner`, car elle se télécharge à l'exécution et Graphite n'accède pas au réseau.
  - Le fichier `analytics.duckdb` est **reconstructible** à tout moment (bouton « Reconstruire l'index analytique ») et n'est pas sauvegardé.
  - Les requêtes de rapports passent par le trait `AnalyticsBackend`. L'implémentation SQLite est livrée en premier (M4), l'implémentation DuckDB en M6.
  - **Porte de décision :** DuckDB n'est activé par défaut que s'il divise par 3 ou plus le temps des rapports sur le jeu de 2 M mains synthétiques. Sinon, il reste désactivable pour garder un exe plus léger.
- **ADR-003 — Stats précalculées par main (style PT4).** À l'import, chaque ligne `hand_players` reçoit des colonnes booléennes « opportunité / action » pour chaque stat (§10). Les agrégats deviennent de simples `SUM()/SUM()`, sans rejouer les actions. La même logique alimente les compteurs incrémentaux du futur HUD (`player_stat_counters`).
- **ADR-004 — Builds.** Les builds de release se font sur GitHub Actions (`windows-latest`). En local, le profil `dev` utilise `opt-level=1` pour les dépendances et une compilation incrémentale. DuckDB peut être désactivé en dev (`--no-default-features`) pour accélérer la compilation sur le Celeron.
- **ADR-005 (révisée le 26/09 après analyse du corpus) — Parser au format anglais, libellés externalisés.** Les fichiers Winamax sont en anglais même avec un client FR. Le parser est une machine à états, et ses libellés sont regroupés dans un seul dictionnaire `en.toml`. Si une variante est un jour observée, il suffira d'ajouter un dictionnaire, sans toucher à la logique. Aucun dictionnaire FR n'est construit par anticipation.
- **ADR-006 — Argent et jetons.** Montants en € stockés en **centimes (i64)**. Jetons en **i64**. Montants en bb calculés à la volée (f64) ; jamais de flottant stocké pour l'argent. Horodatages en **UTC** (i64 epoch ms), affichés en Europe/Paris par défaut.
- **ADR-007 — Mode portable.** L'exe stocke ses données dans `./graphite-data/` à côté de lui si ce dossier existe ou si l'exe est lancé depuis une clé. Sinon, dans `%LOCALAPPDATA%\Graphite\`. Le chemin est affiché dans Paramètres.

---

## 8. Ingestion des données

### 8.1 Sources Winamax
- **Dossier des comptes** (confirmé par Frédéric) : `C:\Users\<user>\AppData\Roaming\winamax\documents\accounts\`, soit `%APPDATA%\winamax\documents\accounts\`. Chaque sous-dossier `accounts\<PSEUDO>\` correspond à un pseudo candidat pour le Hero (UC1, D19). Les historiques sont dans son sous-dossier `history` (casse à confirmer).
- **Fichier de mains** : `AAAAMMJJ_<NOM>(<ID>)_real_holdem_no-limit.txt`. Un fichier par tournoi, complété au fil de l'eau.
- **Fichier summary** : même nom suivi de `_summary.txt`. Il est écrit à la fin du tournoi pour le Hero.
- **La spécification de référence est `docs/formats/winamax.md`.** Elle a été relevée sur 2 tournois réels (445 mains), anonymisés dans `fixtures/winamax/mtt/` : un Space KO 6-max, et un Space KO **3-max ITM avec re-entry et 9 changements de table**. L'invariant de conservation des jetons y est vérifié sur 445/445 mains. Les points non encore observés (tickets, Mystery KO, classique, freeroll, satellite, table finale) y sont listés ❓. Claude Code ne doit **jamais inventer** un libellé pour ces cas : il attend le fixture correspondant.

### 8.2 Particularités du format à retenir (détail dans `docs/formats/winamax.md`)
1. Les fichiers sont en **anglais**, en UTF-8 sans BOM, avec des fins de ligne LF. Les mains sont séparées par `\n\n\n`. Certaines lignes ont un espace final : il faut appliquer `trim_end()` sur chaque ligne.
2. L'en-tête des blindes vaut `(ante/SB/BB)` quand il y a une ante. Le `HandId` a la forme `#<id de TABLE>-<n° main sur la table>-<timestamp Unix>`. Seule la chaîne complète est unique.
3. **L'ID du tournoi** n'apparaît dans le fichier de mains que dans le nom de table : `'<NOM>(<ID>)#<n° table>'`.
4. `buyIn: A€ + F€` dans l'en-tête de main : A = prize + bounty **cumulés**. Le découpage exact prize / bounty / fee vient **du summary** (`Buy-In : 0.80€ + 1€ + 0.20€`).
5. La ligne de siège porte le **bounty courant** du joueur : `Seat 2: X (20000, 1€ bounty)`. Les pseudos peuvent contenir des espaces, des points et des tirets.
6. Un **joueur assis mais non distribué** (aucune ligne posts ou action) n'est pas dans la main : il est exclu des positions et des stats.
7. **Pas de ligne « uncalled bet returned »** : l'excédent non suivi reste dans `Total pot` et revient via `collected` (éventuellement sous la forme `side pot K`). Le pot disputé doit donc être recalculé pour l'EV.
8. **Aucune ligne de KO** : le KO se déduit (joueur all-in à 0, absent de la main suivante, bounty du vainqueur en hausse). Le montant en € ne vient que du summary (`You won Bounty 1€`).
9. **Formats de table** : 6-max et **3-max** (Trident), avec des heads-up à une table 3-max (rôle `(small blind) (button)`). **Pots partagés** = plusieurs lignes `collected … from pot`.
10. **Re-entry** : toutes les entrées sont dans les mêmes fichiers. Le summary contient **un bloc par entrée**, séparés par une ligne vide ; l'entrée éliminée pendant les inscriptions tardives est suffixée ` - Late Registration`. Dans le fichier de mains, une nouvelle entrée = le Hero réapparaît (nouvelle table, tapis de départ) après avoir été éliminé.
11. Résultat : `You won X€ + Bounty Y€` (ITM), `You won Bounty Y€` (hors ITM avec KO), ligne absente sinon. `Registered players` et `Prizepool` sont des instantanés : on prend le dernier bloc, et `Prizepool` n'est pas utilisé pour le ROI.
12. Le summary fournit aussi : `Registered players`, `Type`, `Speed` (ex. `semiturbo`), **la structure complète `Levels`** (SB-BB:ante:durée en s), `Prizepool`, `You played <durée>`, `You finished in <N>th place`.

### 8.3 Parser — exigences

| ID | Exigence |
|---|---|
| PAR-1 | Trait `RoomParser { fn detect(&[u8]) -> Option<Detection>; fn parse_hand(&str) -> Result<HandRecord, ParseError>; fn parse_summary(&str) -> Result<TournamentSummary, ParseError>; }` |
| PAR-2 | Encodage : UTF-8 sans BOM et LF (observé). Tolérer un BOM ou du CRLF par robustesse. `trim_end()` sur chaque ligne (espaces finaux observés). |
| PAR-3 | Découpage des mains : un bloc commence à la ligne `Winamax Poker - ` et n'est **complet** que s'il est suivi du séparateur `\n\n\n` (observé). Un bloc incomplet (fichier en cours d'écriture) est ignoré jusqu'au passage suivant. |
| PAR-4 | Extraction de l'en-tête : type (Tournament), nom, **ID tournoi**, buy-in décomposé (prize / bounty / fee), niveau, **HandId**, variante, blindes SB/BB et ante, date UTC |
| PAR-5 | Table : nom, max de sièges (2 à 10), siège du bouton |
| PAR-6 | Sièges : pseudo (espaces, points, tirets observés ; accents et parenthèses à tolérer), tapis de départ, **bounty courant** `, X€ bounty`. Joueur assis sans aucune ligne posts ou action = **non distribué** (exclu de la main). |
| PAR-7 | Actions observées : `posts ante`, `posts small/big blind`, `folds`, `checks`, `calls N`, `bets N`, `raises X to Y` (Y = total de la street), suffixe `and is all-in`, `shows [..] (..)`, `collected N from pot / main pot / side pot K`. Actions **non encore observées** (mucks, sit out, uncalled bet, ties) : aucune implémentation spéculative. Une ligne inconnue produit une `ParseError UNKNOWN_LINE` ; il faut ajouter un fixture puis le support. |
| PAR-8 | Streets : preflop, flop, turn, river, showdown ; board ; runs multiples (run it twice) si rencontrés |
| PAR-9 | Cartes du Hero (`Dealt to`) et cartes montrées ou muckées des adversaires |
| PAR-10 | Pots multiples (main pot + side pots), rake, gains par joueur |
| PAR-11 | **Invariant de conservation :** Σ(contributions) = Σ(`collected`) = `Total pot` (vérifié 117/117 ; les mises non suivies restent dans le pot, voir §8.2-7). Si l'invariant est violé, la main passe dans `import_errors` avec le code `CHIP_MISMATCH`. |
| PAR-12 | Summary = **N blocs (un par entrée)**, séparés par une ligne vide, suffixe ` - Late Registration` possible. Lignes observées : `Tournament summary : NOM(ID)`, `Player`, `Buy-In : P€ + B€ + F€`, `Registered players`, `Mode`, `Type`, `Speed`, `Flight ID`, `Levels : Levels : [SB-BB:ante:durée:jeu,…]`, `Prizepool`, `Tournament started`, `You played`, `You finished in Nth place`, `You won X€ + Bounty Y€` / `You won Bounty Y€`. **À observer avant d'implémenter** : ITM hors KO, tickets gagnés ou utilisés, places payées, valeurs de `Type` et `Speed` des autres formats. |
| PAR-13 | Chaque objet parsé porte `parser_version` (semver). Un changement de version permet de **reparser** les mains en erreur ou toutes les mains (UC9). |
| PAR-14 | Performance du parser seul : ≥ 5 000 mains/s sur un cœur du Celeron (bench `criterion`) |
| PAR-15 | Aucune panique : toute entrée inattendue produit une `ParseError { code, line_no, line, context }` |

### 8.4 Pipeline d'import

1. **Découverte** : au démarrage, scan des dossiers configurés et comparaison de `(path, size, mtime)` avec la table `import_files`.
2. **Lecture incrémentale** : lecture à partir de `last_offset` ; seuls les blocs complets sont traités ; l'offset avance jusqu'à la fin du dernier bloc complet.
3. **Parsing** dans un pool de 2 workers (paramétrable 1–4).
4. **Dédoublonnage** : `UNIQUE(room_id, room_hand_id)`. Une main déjà présente est ignorée (compteur `duplicates`).
5. **Enrichissement** : positions, profondeurs en bb, flags de stats (ADR-003), équité et EV all-in si les cartes sont connues (§10.6), phase du tournoi (§10.5).
6. **Écriture** en transaction par lot (500 mains max ou 250 ms) : `hands`, `hand_players`, `actions`, `allin_events`, puis mise à jour de `player_stat_counters` et des `sessions`.
7. **Événement IPC** `import://progress` et `hands://new` pour rafraîchir l'interface.
8. **Synchronisation analytique** DuckDB en tâche de fond (débounce de 5 s).

**Modes :**
- **Temps réel** : watcher `notify` sur les dossiers, plus un polling de sécurité toutes les 2 s sur les fichiers modifiés depuis moins de 12 h.
- **Import en masse** : sélection de dossiers ou de fichiers, avec barre de progression, vitesse (mains/s), estimation du temps restant, bouton annuler (l'annulation garde les lots déjà validés), rapport final (importées / doublons / erreurs).
- **Reparse** : relance sur les fichiers ou les mains en erreur, ou sur tout le corpus après une mise à jour du parser.

**Robustesse :** fichier verrouillé par Winamax → réessai avec backoff (100 ms → 2 s) ; fichier tronqué ou réécrit (taille < offset) → relecture complète avec dédoublonnage ; renommage ou suppression → marquage `missing`, les données restent en base.

### 8.5 Rattachement tournoi ↔ mains ↔ summary
- Clé : `(room, room_tournament_id)`. Les mains créent un tournoi « provisoire » ; le summary le complète (buy-in exact, place, gains).
- **Re-entries** (observé) : un bloc de summary par entrée → une ligne `tournament_bullets` par entrée (n°, place, durée, gains, bounties, flag late reg). `entries_count` = nombre de blocs. Le coût vaut `entries_count × buy-in`. Les gains sont la somme de tous les blocs. La place du tournoi est celle du dernier bloc. Chaque main porte son `entry_no`, déduit de la réapparition du Hero après son élimination. Cas validé : OBELISK, 2 entrées, profit +48,19 € (`docs/formats/winamax.md` §5.3).
- Un tournoi sans summary est marqué `incomplete`. Il est affiché, mais exclu du ROI par défaut (option « inclure les tournois incomplets »).

### 8.6 Tickets (D20)
- **Ticket utilisé comme buy-in** : si le summary l'indique, `paid_with_ticket = true` automatiquement. Sinon, le tournoi peut être marqué « joué via ticket » dans l'écran Tournois (édition de métadonnée, pas de saisie de résultat).
- **Ticket gagné** (satellites) : libellé et valeur (valeur faciale lue ou saisie une fois par type de ticket, dans une table `ticket_types`).
- **Paramètre global** `ticket_valuation = FACE_VALUE | ZERO` :
  - FACE_VALUE : un ticket utilisé coûte sa valeur faciale ; un ticket gagné rapporte sa valeur faciale.
  - ZERO : un ticket utilisé coûte 0 € ; un ticket gagné rapporte 0 €.
- Filtre dédié : « Via ticket : tous / oui / non ».

---

## 9. Résultats MTT (écran Résultats + Accueil)

### 9.1 Définitions (toutes en €, centimes en base)

| KPI | Formule |
|---|---|
| Buy-in brut | prize + bounty + fee |
| Frais (rake) | Σ fee × entries |
| Coût | Σ (prize + bounty + fee) × entries (ajusté selon la valorisation des tickets) |
| Gains prize pool | Σ prize gagnés (hors bounties) |
| Gains bounties | Σ bounties gagnés (PKO, Mystery, Space) |
| Tickets gagnés | Σ valeur des tickets gagnés (selon le paramètre) |
| Gains totaux | prize pool + bounties + tickets |
| **Profit** | Gains totaux − Coût |
| **ROI** | Profit / Coût |
| **ROI hors frais** | (Gains totaux − Coût hors fee) / Coût hors fee |
| ROI prize pool / ROI bounties | Pour les KO : (gains prize pool − part prize du coût) / part prize du coût, et idem pour les bounties |
| **ITM %** | tournois avec gains prize pool > 0 ou ticket gagné / tournois joués |
| **ABI** | coût moyen par tournoi (inscription + re-entries), en € |
| Nombre de tournois, entrées, re-entries | |
| Place moyenne, % de tables finales | Place ≤ nombre de sièges de la table finale (9 par défaut, paramétrable par format) |
| Meilleur gain, plus gros tournoi | |
| $/heure | profit / durée cumulée des sessions |
| Freerolls | exclus du ROI (coût 0) mais comptés dans le volume et les gains ; filtre dédié |

### 9.2 Graphes (Résultats)
- **G1 — Courbe de profit cumulé (€)** par tournoi, avec en superposition le profit sans bounties. Axe X au choix : n° de tournoi ou date.
- **G2 — Chips : réel vs EV all-in** (MTT) : cumul des jetons gagnés, en bb, par main, contre le cumul ajusté à l'EV all-in (§10.6). C'est l'équivalent MTT des lignes « rouge et bleue ».
- **G3 — ROI par buy-in** (histogramme par tranches : <2 €, 2–5, 5–10, 10–25, 25–50, 50–100, 100+ ; tranches modifiables).
- **G4 — ROI par format** (Classique, KO, Mystery KO, Space KO, Satellite, Freeroll).
- **G5 — Distribution des places** (percentile de sortie), avec la bulle en évidence.
- **G6 — Volume** : tournois par jour/semaine/mois, heatmap jour × heure.
- (V1.1) **G7 — Simulateur de downswing** basé sur l'ABI, le ROI et la distribution observée.

### 9.3 Sessions (H3)
Une session regroupe les mains du profil Hero sans interruption de plus de N minutes (30 par défaut). Métriques : durée, tournois, mains, profit, EV, tables maximum simultanées.

---

## 10. Moteur de statistiques

### 10.1 Principes
- Chaque stat = **action / opportunité**, calculée par joueur et par main (ADR-003).
- Les stats sont calculables pour le Hero **et** pour tout adversaire (même moteur).
- Affichage : pourcentage avec 1 décimale + échantillon « (n) ». En dessous d'un seuil d'échantillon (paramétrable, 30 par défaut), la valeur est grisée.
- Toutes les stats sont déclinables par **position**, **profondeur de tapis** (§10.3), **phase** (§10.5), **format** et **nombre de joueurs à la table**.

### 10.2 Définitions (V1)

Notation : « folded to » = tous les joueurs ayant parlé avant se sont couchés. Le préflop exclut les blindes et antes postées.

| Code | Stat | Opportunité | Action |
|---|---|---|---|
| VPIP | Voluntarily Put $ In Pot | Toute main distribuée où le joueur a eu la parole préflop (walks exclus) | Call ou raise préflop (le check de la BB ne compte pas) |
| PFR | Preflop Raise | Idem VPIP | Au moins une relance préflop (open-shove compris) |
| RFI | Raise First In, par position | « Folded to » le joueur préflop | Relance (y compris all-in) |
| LIMP | Open limp | « Folded to » le joueur | Complete ou call de la BB |
| OSHOVE | Open-shove | « Folded to » le joueur | Relance all-in. **Ventilé par profondeur de tapis** |
| 3B | 3-bet | Le joueur fait face à exactement une relance préflop (avec ou sans callers) et a encore la parole | Relance |
| F3B | Fold to 3-bet | Le joueur a fait le premier raise préflop et fait face à une 3-bet | Fold |
| 4B | 4-bet (complémentaire) | Face à une 3-bet | Relance |
| ATS | Attempt To Steal | « Folded to » le joueur au CO, BTN ou SB | Relance |
| FSTEAL | Fold to steal (SB, BB séparés) | Joueur en SB/BB face à une tentative de steal, seul le stealer est entré | Fold |
| RSTEAL | Resteal (3-bet vs steal) | Même opportunité que FSTEAL | Relance |
| CBF | C-bet flop | Joueur dernier relanceur préflop, qui voit le flop et à qui c'est la première action ou pour qui il y a eu check jusqu'à lui | Mise |
| CBT | C-bet turn (complémentaire) | Idem sur turn, après un c-bet au flop | Mise |
| FCBF | Fold to c-bet flop | Face à un c-bet au flop (première mise, faite par le relanceur préflop) | Fold |
| WTSD | Went To ShowDown | Le joueur a vu le flop | Arrivé à l'abattage |
| WSD | Won $ at ShowDown (W$SD) | Arrivé à l'abattage | A gagné tout ou partie d'un pot |
| WWSF | Won When Saw Flop (complémentaire) | A vu le flop | A gagné tout ou partie d'un pot |
| AF | Aggression Factor postflop | — | (bets + raises) / calls, postflop. Affiché comme un ratio (1 décimale), pas en % |
| AFQ | Aggression Frequency (complémentaire) | Actions postflop hors check | (bets + raises) / (bets + raises + calls + folds) |

Chaque définition est implémentée dans `gr-stats` avec **au moins 3 tests unitaires** sur des mains construites à la main : un cas positif, un cas négatif et un cas « pas d'opportunité ».

### 10.3 Profondeur de tapis (D24)
- **bb du joueur** = tapis de départ de la main / big blind.
- **bb effectifs** = min(tapis du joueur, plus gros tapis parmi les adversaires encore en jeu). Le mode (joueur ou effectif) se règle dans Paramètres ; par défaut, **effectif**.
- Tranches : `<10`, `10–15`, `15–25`, `25–40`, `40–60`, `60–80`, `80–100`, `100+` (bornes basses incluses, hautes exclues). Les tranches sont stockées en configuration et modifiables.

### 10.4 Positions
BTN, SB, BB, UTG, UTG+1, UTG+2, LJ, HJ, CO, calculées à partir du bouton et des joueurs **distribués** (pas des sièges théoriques ni des joueurs assis mais non distribués). Cas particuliers observés : tables **3-max** (BTN, SB, BB) et **heads-up** (BTN = SB). Bouton mort si rencontré. Les stats sont aussi filtrables par format de table (3-max, 6-max, 9-max), car les fréquences de référence diffèrent fortement. Regroupements : EP (UTG à UTG+2), MP (LJ, HJ), LP (CO, BTN), Blinds.

### 10.5 Phases du tournoi (D25)
L'utilisateur choisit la méthode dans Paramètres. Les deux restent calculées et stockées.

- **Méthode A — Niveaux de blindes (fiable).** La structure réelle du tournoi est lue dans le summary (`Levels`), et le niveau de chaque main dans l'en-tête (`level: N`). Les bornes *Early*, *Middle* et *Late* sont exprimées en **% de la durée théorique** ou en **niveau absolu**, selon le paramétrage, avec des profils génériques par vitesse en secours si le summary est absent. Les phases *Bulle*, *ITM* et *Table finale* s'appliquent par-dessus quand elles sont détectables (voir ci-dessous).
- **Méthode B — Joueurs restants.** Phase selon le ratio joueurs restants / places payées :
  - *Early* > 300 %
  - *Middle* 150–300 %
  - *Pré-bulle* 100–150 %
  - *Bulle* ≈ 100 % (± une table)
  - *ITM*
  - *Table finale*

  Toutes les bornes sont paramétrables.
- ⚠️ **Limite connue :** les HH ne contiennent pas le nombre de joueurs restants, et le summary observé ne donne pas les places payées (confirmé sur le corpus). En conséquence, la méthode B l'**estime par interpolation** entre le début (inscrits) et la fin du tournoi (place du Hero à sa sortie, horodatée par sa dernière main), et l'interface l'indique comme « estimation ». La table finale est détectée si elle est identifiable (nom de table, nombre de joueurs + summary). À valider sur le corpus.

### 10.6 Équité et EV all-in
- **Événement all-in** : au moins un joueur est all-in et est payé avant la river, et les cartes de tous les joueurs impliqués sont connues (montrées).
- **Équité** : énumération exacte des boards restants pour 2 ou 3 joueurs ; Monte Carlo à 200 000 tirages au-delà, avec une graine déterministe. Chaque pot (main pot et side pots) est calculé séparément.
- **EV (jetons)** du joueur = Σ(équité × taille du pot) − contribution. **Écart EV** = gain réel − EV.
- Stockage : table `allin_events` (hand_id, player_id, street, equity, pot_share_ev, actual, ev_diff).
- **R-COMP-1** : ce calcul ne se fait qu'**après** la fin de la main, jamais sur une main en cours.
- Performance : HU préflop exact < 60 ms sur le Celeron (sinon Monte Carlo à 100 000 tirages). Calcul asynchrone, qui ne bloque pas l'import.

### 10.7 Compteurs incrémentaux (préparation HUD, D16)
- Table `player_stat_counters(player_id, context_key, stat_code, opp, act, updated_at)`.
- `context_key` = combinaison encodée : `format|position_group|depth_bucket|phase|street_line`. Par exemple `MTT|LP|15-25|MID|*`. `*` désigne l'agrégat.
- Mise à jour dans la même transaction que l'insertion de la main. Lecture en O(1) pour le futur HUD, cible < 20 ms pour 10 joueurs.
- Un job de reconstruction complète est disponible (Paramètres > Maintenance).

---

## 11. Filtres (H4)

Un panneau de filtres global et persistant par écran. Chaque écran peut aussi « épingler » ses propres filtres.

| Filtre | Détail |
|---|---|
| Profil Hero / pseudos | Sélection multiple (D19) |
| Dates | Presets (aujourd'hui, 7 j, 30 j, mois, année, tout) + plage libre + sessions |
| Room | Winamax (V1), autres en V1.1 |
| Buy-in | Plage € + tranches |
| Format | Classique, KO/PKO, Mystery KO, Space KO, Satellite, Freeroll |
| Via ticket | Tous / oui / non |
| Vitesse | Valeur `Speed` du summary (ex. `semiturbo`) ; correction manuelle par tournoi si le summary est absent |
| Taille de table | HU, 3-max, 5/6-max, 7/8-max, 9/10-max ; nombre de joueurs présents à la main |
| Position | Positions individuelles et groupes |
| Profondeur de tapis | Tranches D24 (bb joueur ou effectifs) |
| Phase | Selon la méthode choisie |
| Cartes | Grille 13×13 cliquable + saisie texte (« AKs, TT+, A5s-A2s ») |
| Actions | Situations préflop : RFI, face à un open, face à une 3-bet, squeeze, vs steal… |
| All-in | Mains avec all-in, Hero devant ou derrière, écart EV < / > X |
| Tags | Inclure / exclure des tags |
| Résultat de la main | Gagnée / perdue / taille du pot en bb |

Les combinaisons de filtres peuvent être **sauvegardées en presets nommés**.

---

## 12. Leaks et benchmarks

### 12.1 Fonctionnement
- Pour chaque stat (§10.2) × contexte (position ou groupe, tranche de profondeur, phase), l'écran compare la valeur du Hero à une **fourchette min–max**.
- Statuts : **OK**, **Trop bas**, **Trop haut**, **Échantillon insuffisant** (grisé). Sévérité = écart à la fourchette / largeur de la fourchette.
- Vue « Top leaks » : les 10 plus gros écarts, pondérés par la fréquence de la situation (les situations fréquentes passent devant).
- Clic sur un leak → liste des mains concernées, préfiltrée → replayer.

### 12.2 Benchmarks par défaut (H1) — valeurs MTT 8–9 max, **indicatives et modifiables**

| Stat | Fourchette par défaut | Remarque |
|---|---|---|
| VPIP | 18–26 % | 9-max, profondeur > 25 bb |
| PFR | 14–21 % | |
| 3-bet | 6–10 % | |
| Fold to 3-bet | 40–60 % | dépend fortement de la profondeur |
| ATS | 32–48 % | |
| Fold BB to steal | 30–45 % | antes présentes |
| Fold SB to steal | 60–80 % | |
| Resteal | 8–14 % | |
| C-bet flop | 50–70 % | |
| Fold to c-bet flop | 35–50 % | |
| WTSD | 24–32 % | |
| W$SD | 48–56 % | |
| AF | 2,0–3,5 | |

- Les benchmarks sont stockés dans la table `benchmarks(stat_code, context_key, min, max, source)`. Import/export en JSON. Bouton « Restaurer les valeurs par défaut ».
- Des profils de benchmark par format sont possibles (ex. « KO », « Classique »). **Des profils par taille de table sont indispensables** : tes tournois incluent des formats **3-max (Trident)** et 6-max, où les fréquences normales (VPIP, ATS, défense de BB) sont bien plus élevées qu'en 9-max. La V1 livre des profils par défaut 9-max, 6-max et 3-max ; les valeurs 6-max et 3-max sont à calibrer (question Q2).

---

## 13. Écrans (D31) — spécifications fonctionnelles

Mise en page : barre latérale de navigation à gauche (icônes + libellés), barre de filtres en haut, zone de contenu. Barre d'état en bas : statut d'import (● actif / en pause / erreur), mains importées aujourd'hui, dernière main.

### 13.1 Accueil / KPIs
- Cartes KPI : Profit, ROI, ITM, ABI, Tournois, Frais payés, Écart EV all-in (jetons → bb), $/h. Chaque carte affiche la valeur, la variation par rapport à la période précédente et une sparkline.
- Graphe G1 en format réduit.
- « Dernière session » : durée, tournois, profit, meilleurs et pires tournois.
- « Top 3 leaks » (lien vers Leaks).
- État de l'import : dossiers surveillés, nombre de mains, erreurs non résolues (lien vers Logs).

### 13.2 Résultats
- Graphes G1 à G6 ; tableau pivot Résultats × dimension (buy-in, format, vitesse, jour de semaine, heure, mois) avec les colonnes KPI de la §9.1.
- Export CSV du tableau. Il s'agit de données agrégées, pas de mains, donc compatible avec D26.

### 13.3 Tournois
- Tableau virtualisé : date, nom, format, buy-in (prize/bounty/fee), via ticket, entrées, inscrits, place, gains prize pool, bounties, tickets, profit, mains jouées, durée, EV diff, statut (complet/incomplet).
- Détail d'un tournoi : chronologie du tapis du Hero (jetons et bb par main), all-in, liste des mains, adversaires rencontrés avec leur classification (UC10), édition des métadonnées (via ticket, vitesse, valeur du ticket gagné).

### 13.4 Mains
- Tableau virtualisé : date, tournoi, niveau, position, profondeur (bb), cartes, action préflop synthétique (ex. « RFI → call 3B »), board, résultat en bb, EV diff, tags.
- Sélection multiple → tag en masse. Double-clic → Replayer.

### 13.5 Rapports (D12, UC8)
- **Constructeur de rapports** :
  - lignes = 1 ou 2 dimensions (position, profondeur, phase, format, buy-in, cartes, main de départ en grille 13×13, jour…) ;
  - colonnes = stats et KPIs à sélectionner ;
  - filtres ;
  - tri ;
  - mise en forme conditionnelle en niveaux de gris, comparée aux benchmarks.
- Vue **grille 13×13** des mains de départ (fréquence, VPIP/PFR, résultat en bb/main), en niveaux de gris.
- Rapports enregistrés, dupliqués, renommés. Quelques rapports prédéfinis :
  - « Préflop par position » ;
  - « Défense de BB par profondeur » ;
  - « Open-shove par profondeur × position » ;
  - « Postflop (c-bet) » ;
  - « Résultats par phase ».
- Backend : `AnalyticsBackend` (SQLite puis DuckDB).

### 13.6 Leaks
Voir §12. Vue en matrice : stat × tranche de profondeur, avec des cellules grises, blanches ou encadrées selon le statut.

### 13.7 Replayer (D27, D28)
- Table ovale stylisée en monochrome, sièges avec pseudo, tapis (jetons et bb) et bounty. Les cartes sont visibles pour le Hero et pour les joueurs qui les ont montrées.
- Contrôles : début, précédent, lecture/pause (vitesse 0,5× à 4×), suivant, fin ; navigation par street ; raccourcis clavier (← → espace).
- Panneau latéral : texte de l'historique brut, avec la ligne courante surlignée.
- **Équité** : à chaque all-in, affichage de l'équité de chaque joueur et de l'EV (post-main uniquement, R-COMP-1).
- Pot et side pots, SPR, taille des mises en bb et en % du pot.
- **Tags** : prédéfinis (*Bad beat, Cooler, Hero call, Bluff, Erreur préflop, Erreur postflop, À revoir, Spot ICM, Question coach*) + tags libres, avec couleur en niveaux de gris et note texte par main.
- Navigation « main suivante / précédente » dans la liste filtrée d'origine.

### 13.8 Adversaires (V1 minimal, UC10)
Pas d'écran de fiche (V2). La classification (§14) apparaît dans le détail d'un tournoi et dans le replayer (badge à côté du pseudo). Des notes et un label couleur par joueur peuvent être saisis depuis le replayer (D16).

### 13.9 Paramètres
- **Comptes** : dossiers surveillés (ajout, suppression, détection automatique) ; profils Hero et pseudos (D19).
- **Import** : temps réel on/off, workers, priorité, reparse, « inclure les tournois incomplets ».
- **Stats** : mode de profondeur (joueur/effectif), tranches, seuil d'échantillon, méthode de phase et bornes, profils de structure, seuil de session.
- **Résultats** : valorisation des tickets (D20), tranches de buy-in, taille de la table finale.
- **Benchmarks** et **règles de classification** (éditeurs).
- **Affichage** : langue FR/EN, fuseau horaire, format des nombres, unité (bb / jetons), « monochrome strict » (voir §16).
- **Sauvegardes** : jour et heure, dossier, rétention, « Sauvegarder maintenant », restaurer.
- **Maintenance** : chemin des données, taille de la base, VACUUM, reconstruction des compteurs, reconstruction de DuckDB, logs.

### 13.10 Logs (D32)
- Visionneuse des logs applicatifs : niveau (ERROR, WARN, INFO, DEBUG), recherche plein texte, filtre par module (ingest, parser, store, analytics, ui), période.
- Onglet **Erreurs d'import** : table `import_errors` avec fichier, ligne, code, message et extrait brut. Actions : « Reparser », « Ignorer », « Copier pour rapport de bug ».
- Rotation quotidienne des fichiers, rétention 30 jours (paramétrable). Niveau par défaut : INFO. DEBUG activable à chaud.
- Le contenu des mains n'est **jamais** écrit dans les logs au niveau INFO (seulement les HandId).

---

## 14. Classification des joueurs (D29)

- Règles ordonnées, évaluées sur les stats globales du joueur (échantillon ≥ `min_hands`, 30 par défaut) ; la première règle vérifiée l'emporte. En dessous du seuil : « Inconnu ».
- Règles par défaut (modifiables : seuils, ordre, libellés, nuance de gris) :

| Ordre | Label | Condition par défaut |
|---|---|---|
| 1 | Maniac | VPIP ≥ 45 et PFR ≥ 30 |
| 2 | Fish (loose passif) | VPIP ≥ 35 et PFR/VPIP < 0,4 |
| 3 | Calling station | VPIP ≥ 30 et AF < 1,2 et WTSD ≥ 35 |
| 4 | LAG | VPIP 26–45 et PFR/VPIP ≥ 0,65 |
| 5 | Nit | VPIP < 14 et PFR < 11 |
| 6 | Reg (TAG) | VPIP 14–26 et PFR/VPIP ≥ 0,7 |
| 7 | Rec (autre) | par défaut |

- Éditeur de règles avec prévisualisation du nombre de joueurs par label.
- Préparation de la V2 : les labels sont stockés (`players.auto_label`) et recalculés en fond après chaque lot d'import.

---

## 15. Modèle de données (SQLite — source de vérité)

> DDL indicatif. Claude Code l'implémente sous forme de **migrations numérotées** (`gr-store/migrations/0001_init.sql`, …). Toute évolution passe par une nouvelle migration, et une sauvegarde est faite automatiquement avant chaque migration.

```sql
PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;

CREATE TABLE rooms (id INTEGER PRIMARY KEY, code TEXT UNIQUE NOT NULL, name TEXT NOT NULL);

CREATE TABLE players (
  id INTEGER PRIMARY KEY, room_id INTEGER NOT NULL REFERENCES rooms(id),
  screen_name TEXT NOT NULL, first_seen_at INTEGER, last_seen_at INTEGER,
  auto_label TEXT, manual_label TEXT, color_label TEXT, note TEXT,
  UNIQUE(room_id, screen_name));

CREATE TABLE hero_profiles (id INTEGER PRIMARY KEY, name TEXT UNIQUE NOT NULL, is_default INTEGER DEFAULT 0);
CREATE TABLE hero_accounts (profile_id INTEGER REFERENCES hero_profiles(id) ON DELETE CASCADE,
  player_id INTEGER REFERENCES players(id), PRIMARY KEY(profile_id, player_id));

CREATE TABLE ticket_types (id INTEGER PRIMARY KEY, room_id INTEGER, label TEXT NOT NULL,
  face_value_cents INTEGER, UNIQUE(room_id, label));

CREATE TABLE tournaments (
  id INTEGER PRIMARY KEY, room_id INTEGER NOT NULL, room_tournament_id TEXT NOT NULL,
  name TEXT, started_at INTEGER, ended_at INTEGER,
  buyin_prize_cents INTEGER, buyin_bounty_cents INTEGER, buyin_fee_cents INTEGER,
  currency TEXT DEFAULT 'EUR',
  ko_type TEXT CHECK(ko_type IN ('NONE','PKO','KO','MYSTERY','SPACE')) DEFAULT 'NONE',
  is_satellite INTEGER DEFAULT 0, is_freeroll INTEGER DEFAULT 0,
  speed TEXT, speed_overridden INTEGER DEFAULT 0,
  table_size INTEGER, entrants INTEGER, prize_pool_cents INTEGER, paid_places INTEGER,
  status TEXT CHECK(status IN ('PROVISIONAL','COMPLETE','INCOMPLETE')) DEFAULT 'PROVISIONAL',
  UNIQUE(room_id, room_tournament_id));

CREATE TABLE tournament_entries (          -- résultat du Hero (un pseudo Hero par ligne)
  id INTEGER PRIMARY KEY, tournament_id INTEGER NOT NULL REFERENCES tournaments(id),
  player_id INTEGER NOT NULL REFERENCES players(id),
  entries_count INTEGER DEFAULT 1, finish_position INTEGER,
  prize_cents INTEGER DEFAULT 0, bounty_cents INTEGER DEFAULT 0,
  ticket_won_type_id INTEGER REFERENCES ticket_types(id), ticket_won_count INTEGER DEFAULT 0,
  paid_with_ticket INTEGER DEFAULT 0, ticket_used_type_id INTEGER REFERENCES ticket_types(id),
  summary_file_id INTEGER REFERENCES import_files(id),
  UNIQUE(tournament_id, player_id));

CREATE TABLE tournament_bullets (         -- une ligne par entrée (bloc de summary)
  entry_id INTEGER NOT NULL REFERENCES tournament_entries(id) ON DELETE CASCADE,
  entry_no INTEGER NOT NULL, late_reg_bust INTEGER DEFAULT 0,
  finish_position INTEGER, played_seconds INTEGER,
  prize_cents INTEGER DEFAULT 0, bounty_cents INTEGER DEFAULT 0,
  registered_snapshot INTEGER, prizepool_snapshot_cents INTEGER,
  PRIMARY KEY(entry_id, entry_no));

CREATE TABLE hands (
  id INTEGER PRIMARY KEY, room_id INTEGER NOT NULL, room_hand_id TEXT NOT NULL,
  tournament_id INTEGER REFERENCES tournaments(id), table_name TEXT, table_max_seats INTEGER,
  players_dealt INTEGER, button_seat INTEGER, level INTEGER,
  sb INTEGER, bb INTEGER, ante INTEGER, played_at INTEGER NOT NULL,
  board TEXT, total_pot INTEGER, rake INTEGER DEFAULT 0,
  hero_player_id INTEGER REFERENCES players(id), hero_entry_no INTEGER,
  phase_by_level TEXT, phase_by_players TEXT, players_left_est INTEGER,
  session_id INTEGER REFERENCES sessions(id),
  file_id INTEGER REFERENCES import_files(id), file_offset INTEGER,
  parser_version TEXT NOT NULL,
  UNIQUE(room_id, room_hand_id));
CREATE INDEX ix_hands_played_at ON hands(played_at);
CREATE INDEX ix_hands_tournament ON hands(tournament_id);

CREATE TABLE hand_raw (hand_id INTEGER PRIMARY KEY REFERENCES hands(id) ON DELETE CASCADE,
  codec TEXT DEFAULT 'zstd', data BLOB NOT NULL);   -- texte brut compressé (replayer, reparse)

CREATE TABLE hand_players (                        -- table de faits principale
  hand_id INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,
  player_id INTEGER NOT NULL REFERENCES players(id),
  seat INTEGER, position TEXT, position_group TEXT, is_hero INTEGER DEFAULT 0,
  start_stack INTEGER, stack_bb REAL, eff_stack_bb REAL, depth_bucket TEXT, eff_depth_bucket TEXT,
  hole_cards TEXT, hand_class TEXT,               -- ex. 'AKs', 'TT', 'Q9o'
  net_chips INTEGER, net_bb REAL, bounty_won_cents INTEGER DEFAULT 0,
  saw_flop INTEGER, saw_turn INTEGER, saw_river INTEGER, went_sd INTEGER, won_sd INTEGER, won_hand INTEGER,
  preflop_line TEXT,                              -- ex. 'RFI', 'RFI-F3B', 'CALL-OPEN', 'SQZ'
  -- flags de stats (opportunité / action) — un couple par stat §10.2
  vpip_opp INTEGER, vpip INTEGER, pfr INTEGER,
  rfi_opp INTEGER, rfi INTEGER, limp INTEGER, oshove INTEGER,
  tb_opp INTEGER, tb INTEGER, f3b_opp INTEGER, f3b INTEGER, fb_opp INTEGER, fb INTEGER,
  ats_opp INTEGER, ats INTEGER, fsteal_opp INTEGER, fsteal INTEGER, rsteal INTEGER,
  cbf_opp INTEGER, cbf INTEGER, cbt_opp INTEGER, cbt INTEGER, fcbf_opp INTEGER, fcbf INTEGER,
  pf_bets INTEGER, pf_raises INTEGER, pf_calls INTEGER, pf_folds INTEGER, pf_checks INTEGER,
  allin_ev_diff_chips REAL,
  PRIMARY KEY(hand_id, player_id));
CREATE INDEX ix_hp_player ON hand_players(player_id);
CREATE INDEX ix_hp_hero ON hand_players(is_hero, player_id);

CREATE TABLE actions (hand_id INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL, street TEXT NOT NULL, player_id INTEGER REFERENCES players(id),
  kind TEXT NOT NULL, amount INTEGER, to_amount INTEGER, is_allin INTEGER DEFAULT 0,
  PRIMARY KEY(hand_id, seq));

CREATE TABLE allin_events (hand_id INTEGER REFERENCES hands(id) ON DELETE CASCADE,
  player_id INTEGER REFERENCES players(id), street TEXT, pot_index INTEGER,
  equity REAL, ev_chips REAL, actual_chips INTEGER, method TEXT,  -- 'EXACT' | 'MC200K'
  PRIMARY KEY(hand_id, player_id, pot_index));

CREATE TABLE tags (id INTEGER PRIMARY KEY, label_key TEXT UNIQUE NOT NULL, is_predefined INTEGER, shade TEXT);
CREATE TABLE hand_tags (hand_id INTEGER REFERENCES hands(id) ON DELETE CASCADE,
  tag_id INTEGER REFERENCES tags(id), note TEXT, created_at INTEGER, PRIMARY KEY(hand_id, tag_id));

CREATE TABLE sessions (id INTEGER PRIMARY KEY, hero_profile_id INTEGER, started_at INTEGER, ended_at INTEGER,
  hands INTEGER, tournaments INTEGER, max_tables INTEGER);

CREATE TABLE player_stat_counters (player_id INTEGER, context_key TEXT, stat_code TEXT,
  opp INTEGER DEFAULT 0, act INTEGER DEFAULT 0, updated_at INTEGER,
  PRIMARY KEY(player_id, context_key, stat_code)) WITHOUT ROWID;

CREATE TABLE import_files (id INTEGER PRIMARY KEY, path TEXT UNIQUE NOT NULL, kind TEXT,  -- 'HANDS'|'SUMMARY'
  size INTEGER, mtime INTEGER, last_offset INTEGER DEFAULT 0, language TEXT, status TEXT, updated_at INTEGER);

CREATE TABLE import_errors (id INTEGER PRIMARY KEY, file_id INTEGER REFERENCES import_files(id),
  file_offset INTEGER, line_no INTEGER, code TEXT, message TEXT, raw_excerpt TEXT,
  parser_version TEXT, status TEXT DEFAULT 'OPEN', created_at INTEGER);

CREATE TABLE benchmarks (stat_code TEXT, context_key TEXT, profile TEXT DEFAULT 'default',
  min REAL, max REAL, source TEXT, PRIMARY KEY(profile, stat_code, context_key));
CREATE TABLE classification_rules (id INTEGER PRIMARY KEY, ord INTEGER, label TEXT, shade TEXT,
  expr_json TEXT NOT NULL, enabled INTEGER DEFAULT 1);
CREATE TABLE saved_reports (id INTEGER PRIMARY KEY, name TEXT, definition_json TEXT, is_builtin INTEGER);
CREATE TABLE filter_presets (id INTEGER PRIMARY KEY, screen TEXT, name TEXT, definition_json TEXT);
CREATE TABLE settings (key TEXT PRIMARY KEY, value_json TEXT NOT NULL);
```

**Couche DuckDB** : des tables dénormalisées `f_hand_player` (jointure hands × hand_players × tournaments) et `f_tournament_entry`, synchronisées par curseur. Seules les requêtes des Rapports, des Leaks et des Résultats agrégés y sont lues.

### 15.1 Volumétrie cible
≤ **1,8 Ko par main** en SQLite, raw zstd et index compris. 5 M mains ≈ 9 Go : acceptable sur un SSD de 500 Go, et la taille est affichée dans Paramètres.

### 15.2 Corpus de test
- `fixtures/winamax/mtt/<format>/` : fichiers réels fournis par Frédéric, **anonymisés** par `tools/anonymize.py` (implémentation de référence livrée) (pseudos remplacés de façon déterministe, sauf le Hero, renommé `Hero`).
- Couverture actuelle : Space KO 6-max ; Space KO 3-max ITM avec re-entry et changements de table.
- Couverture exigée : classique, PKO, Mystery KO, Space KO, freeroll, satellite avec ticket gagné, tournoi joué via ticket, re-entry, heads-up final, all-in multiway avec side pots, main en cours d'écriture (fichier tronqué), pseudos avec espaces et caractères spéciaux.

---

## 16. Design system (D15)

- **Palette (tokens)** :
  - fond `#0A0A0A` ;
  - surfaces `#121212` / `#1A1A1A` / `#222222` ;
  - bordures `#2A2A2A` ;
  - texte principal `#F5F5F5`, secondaire `#A3A3A3`, désactivé `#5C5C5C` ;
  - accent = blanc pur `#FFFFFF`.
- **Sémantique** : les gains et pertes sont signalés d'abord par **le signe et la graisse** (+ en blanc gras, − en gris). Une option « Couleurs sémantiques discrètes » ajoute un vert et un rouge désaturés (`#7FA88A` / `#B07A7A`). Elle est désactivée par défaut, ce qui correspond au mode « monochrome strict ».
- **Graphiques** : séries en blanc, gris clair, gris moyen et pointillés. Réel en trait plein, EV en pointillés. Grilles très discrètes.
- **Typographie** : Inter (UI) et JetBrains Mono (chiffres tabulaires, historiques bruts), embarquées localement. Taille de base 13 px (densité d'un outil pro).
- **Densité** : tableaux compacts (28 px par ligne), sticky headers, colonnes redimensionnables et réordonnables, persistance par écran.
- **Accessibilité** : contraste AA minimum, navigation clavier complète, focus visible.
- Thème clair : hors périmètre V1, mais les tokens le permettent.

---

## 17. Exigences non fonctionnelles

### 17.1 Performance (machine cible D34)

| ID | Exigence | Cible |
|---|---|---|
| NFR-P1 | Latence temps réel (écriture du fichier → main visible dans l'interface), 12 tables | p95 < 2 s, p50 < 1 s |
| NFR-P2 | Import en masse (parse + enrichissement + écriture) | ≥ 1 000 mains/s |
| NFR-P3 | Parser seul, 1 cœur | ≥ 5 000 mains/s |
| NFR-P4 | Démarrage à froid de l'application | < 3 s |
| NFR-P5 | Accueil / KPIs à 1 M mains | < 1 s |
| NFR-P6 | Rapport personnalisé (2 dimensions, 10 stats) à 2 M mains | < 3 s (DuckDB) / < 8 s (SQLite) |
| NFR-P7 | Liste des mains filtrée (virtualisée), premier affichage | < 500 ms |
| NFR-P8 | RAM au repos / pendant un import en masse | < 350 Mo / < 1 Go |
| NFR-P9 | CPU pendant le jeu (import temps réel, 12 tables) | < 10 % en moyenne |
| NFR-P10 | Lecture des compteurs HUD, 10 joueurs (préparation V2) | < 20 ms |

Les mesures se font avec `gr-synth` (1 M et 2 M mains générées) et un banc reproductible (`cargo bench`, script `tools/perf.ps1`).

### 17.2 Fiabilité
- Aucune perte de données en cas de crash : WAL + transactions. L'import reprend depuis `last_offset`.
- Sauvegarde **hebdomadaire** automatique (D33), par défaut le dimanche à 12:00 ou au premier lancement suivant si l'app était fermée :
  - API de backup en ligne SQLite, compression zstd ;
  - vérification `PRAGMA integrity_check` sur la copie ;
  - rétention de 8 sauvegardes, paramétrable ;
  - sauvegarde supplémentaire avant chaque migration.
- Restauration depuis Paramètres, avec confirmation et redémarrage.

### 17.3 Sécurité et confidentialité
Aucun appel réseau. La CSP Tauri interdit toute connexion externe. Les capabilities Tauri 2 sont restreintes aux commandes nécessaires : accès au système de fichiers limité aux dossiers configurés et au dossier de données.

### 17.4 Compatibilité
Windows 10 22H2+ et Windows 11, x64. WebView2 (présent par défaut sur Windows 10/11 à jour ; message d'aide sinon). Écrans 1366×768 minimum, DPI 100 à 200 %.

### 17.5 Internationalisation (D14)
- Toutes les chaînes de l'interface sont dans `ui/src/locales/{fr,en}/*.json`. **Aucune chaîne en dur** (règle lint).
- Nombres, dates et devises formatés via `Intl` selon la locale. La devise reste €.
- Les codes de stat (VPIP, PFR…) restent identiques dans les deux langues. Les descriptions (infobulles) sont traduites.

### 17.6 Maintenabilité
Couverture de tests ≥ 85 % sur `gr-parser-winamax`, `gr-stats` et `gr-equity` ; ≥ 60 % sur le reste du Rust. Clippy `-D warnings`, rustfmt, ESLint et Prettier bloquants en CI.

---

## 18. Qualité, tests et CI (D18)

- **Parser** : un test snapshot (`insta`) par fichier du corpus : le `HandRecord` sérialisé en JSON est comparé à la référence validée. Tests de propriétés (`proptest`) : conservation des jetons, positions cohérentes, pas de panique sur des entrées tronquées ou aléatoires (fuzz léger).
- **Stats** : mains construites à la main (DSL de test `hand!{…}`) → flags attendus, pour chaque stat §10.2.
- **Équité** : valeurs de référence connues (AA vs KK ≈ 81,9 % ; AKo vs 22 ≈ 47,6 % ; coin-flip, etc.) avec une tolérance de ±0,1 % en exact.
- **Résultats** : scénarios de tournois (tickets FACE/ZERO, re-entries, PKO, freeroll) → KPIs attendus au centime.
- **Intégration** : import de bout en bout du corpus dans une base temporaire → contrôles de comptage.
- **Front** : Vitest + Testing Library pour les composants critiques (filtres, constructeur de rapports, replayer).
- **CI** (`.github/workflows/ci.yml`) :
  - job **Linux** rapide (fmt, clippy, tests des crates sans dépendance Windows) ;
  - job **Windows** (build Tauri + tests complets) ;
  - cache `Swatinem/rust-cache` ;
  - déclenchement sur push et PR.
- **Release** (`.github/workflows/release.yml`) : sur tag `v*`, build Windows **portable** (exe + zip) attaché à une GitHub Release, avec les notes générées depuis les commits conventionnels.

---

## 19. Préparation de la V2 (HUD) dès la V1 (D16)

| Élément | Livré en V1 | Utilisé en V2 |
|---|---|---|
| `player_stat_counters` incrémentaux + contextes | Oui (§10.7) | Lecture HUD < 20 ms |
| Contextes (position × profondeur × phase × ligne) | Oui (clé de contexte) | Stats dynamiques à la Hand2Note |
| Notes et labels joueurs | Oui (replayer) | Popup HUD |
| `gr-winmon` : énumération des fenêtres Winamax, extraction du titre, correspondance titre → tournoi/table, événements `SetWinEventHook` (création, déplacement, redimensionnement, destruction) | **Spike + module testé**, sans overlay | Positionnement de l'overlay |
| Classification automatique | Oui | Badge HUD |
| Architecture d'overlay (fenêtre Tauri transparente, click-through, pool de fenêtres) | ADR rédigée seulement | Implémentation |

---

## 20. Risques et mitigations

| # | Risque | Proba | Impact | Mitigation |
|---|---|---|---|---|
| R1 | Format Winamax partiellement connu : le cœur est documenté (Space KO, 117 mains) ; ITM, tickets, re-entries et autres formats restent à observer | Moyenne | Haut | Compléter le corpus (liste dans `docs/formats/winamax.md` §7) ; `UNKNOWN_LINE` tracée ; aucun libellé inventé |
| R2 | Winamax modifie son format | Moyenne | Haut | `parser_version`, `import_errors`, reparse, tests snapshot pour détecter les régressions |
| R3 | Nombre de joueurs restants absent des HH → phase par joueurs imprécise | Haute | Moyen | Méthode B affichée comme estimation ; méthode A par défaut |
| R4 | Perf insuffisante sur le Celeron | Moyenne | Haut | Flags précalculés, DuckDB, banc `gr-synth` dès M2, pool limité |
| R5 | Compilation Rust très lente en local | Haute | Moyen | ADR-004 : CI pour les releases, feature DuckDB désactivable en dev, `sccache` |
| R6 | Évolution des CGU Winamax (HUD) | Faible | Haut | R-COMP-1/2 ; confirmation écrite avant la V2 |
| R7 | Tickets et bounties non présents dans les fichiers | Moyenne | Moyen | Édition des métadonnées d'un tournoi (pas une saisie de résultat) |
| R8 | Dérive du périmètre vs échéance fin 2026 | Moyenne | Moyen | Backlog priorisé (MoSCoW) ; V1.1 absorbe le « Could » |

---

## 21. Questions ouvertes (non bloquantes pour démarrer)

1. Q1 — Compléter le corpus avec les 9 cas listés dans `docs/formats/winamax.md` §7. Un premier tournoi (Space KO) a été reçu et documenté ; les stories M1-1 à M1-5 peuvent démarrer, et M1-6 (summaries ITM et tickets) attend ces fichiers.
2. Q2 — Les benchmarks §12.2 conviennent-ils, ou faut-il des valeurs de coach ?
3. Q3 — Nom définitif de l'application (« Graphite » par défaut).
4. Q4 — Valeurs faciales des tickets Winamax courants, si elles n'apparaissent pas dans les fichiers.
5. Q5 — Taille par défaut de la table finale par format (9 pour les classiques ; autres cas ?).

---

## 22. Glossaire

- **ABI** : Average Buy-In.
- **bb** : big blind.
- **EV all-in** : espérance de gain au moment de l'all-in.
- **HH** : hand history.
- **ITM** : In The Money.
- **PKO** : Progressive Knockout.
- **RFI** : Raise First In.
- **SPR** : Stack-to-Pot Ratio.
- **WAL** : Write-Ahead Logging (SQLite).
- **Hero** : le joueur utilisateur.
- **Villain** : un adversaire.
