# CLAUDE.md — Graphite (tracker poker MTT, Windows)

Ce fichier est lu automatiquement par Claude Code. Il fixe **comment** travailler sur ce repo. Le **quoi** est dans `docs/PRD.md`, le **dans quel ordre** dans `docs/BACKLOG.md`.

## 1. Le projet en 5 lignes
- Référence du format : `docs/formats/winamax.md` (relevé sur de vrais fichiers). À lire avant toute modification du parser.
- Application de bureau **Windows** (Tauri 2 + Rust + React/TS), **100 % locale, aucun appel réseau**.
- Elle lit en temps réel les historiques de mains **Winamax (client en français)**, MTT uniquement, et les stocke en **SQLite** (vérité), avec **DuckDB** comme couche analytique dérivée.
- Elle calcule les résultats MTT (ROI, ITM, ABI, frais, tickets, bounties), l'EV all-in, les stats par position, profondeur de tapis et phase, et les leaks par rapport à des benchmarks.
- Le HUD en overlay arrive en V2 : ne pas l'implémenter, mais respecter les points de préparation (PRD §19).
- L'utilisateur est francophone : **répondre en français**. Le code, les identifiants et les commentaires sont en anglais. Les messages de commit sont en anglais (Conventional Commits).

## 2. Méthode de travail (obligatoire)
1. Au début d'une session : lire `docs/BACKLOG.md`, prendre la **première story `TODO` non bloquée** du jalon courant et annoncer son ID (ex. `M2-3`).
2. Relire les sections du PRD citées par la story. **Ne pas élargir le périmètre.** Toute idée hors story va dans `docs/BACKLOG.md` › « Idées / à trier ».
3. **Tests d'abord** pour le parser, les stats, l'équité et les KPIs : écrire le test, le voir échouer, puis implémenter.
4. Avant de déclarer une story terminée, vérifier chaque critère d'acceptation et lancer `just check` (voir §4).
5. Mettre à jour le statut dans `BACKLOG.md` (`TODO` → `DOING` → `DONE`) et ajouter une ligne dans `docs/CHANGELOG.md`.
6. Faire un commit par story, ou plusieurs commits atomiques, au format `feat(parser): …`, `fix(stats): …`, `test: …`, `docs: …`, `chore(ci): …`.
7. Si une décision d'architecture est nécessaire, rédiger une ADR courte dans `docs/adr/NNNN-titre.md` et **demander validation** avant d'implémenter.
8. S'arrêter et **poser la question** à Frédéric en cas de :
   - donnée de format Winamax inconnue ;
   - contradiction PRD/BACKLOG ;
   - besoin d'une nouvelle dépendance lourde (> 1 Mo de binaire ou temps de compilation notable) ;
   - modification du schéma de données non prévue par la story.

## 3. Règles non négociables
- **R-FORMAT : ne jamais inventer un libellé ou un motif du format Winamax.** Chaque motif du parser doit exister dans un fichier de `fixtures/` et être documenté dans `docs/formats/winamax.md`. En cas de doute, demander un exemple de fichier.
- **R-COMP-1 : aucun calcul d'équité, d'ICM ou de conseil sur une main en cours.** Les calculs d'équité ne s'exécutent que sur des mains terminées, déjà importées. Aucune fonctionnalité de « range dynamique » ou d'aide à la décision en temps réel (règles Winamax, risque de ban).
- **R-NET : aucun appel réseau** : pas de télémétrie, pas de téléchargement d'extension DuckDB, pas de polices ou CDN distants. La CSP Tauri bloque tout.
- **R-MONEY : argent en centimes `i64`, jetons en `i64`, dates en UTC epoch ms `i64`.** Jamais de `f64` stocké pour de l'argent. Les montants en bb sont calculés.
- **R-SCHEMA : toute modification du schéma passe par une nouvelle migration** `crates/gr-store/migrations/NNNN_*.sql`. Ne jamais modifier une migration déjà mergée.
- **R-NOPANIC : aucune `unwrap()` ni `expect()` dans les crates `gr-*`** hors tests. Erreurs typées avec `thiserror` ; `anyhow` seulement dans `src-tauri`.
- **R-I18N : aucune chaîne visible en dur dans l'UI.** Tout passe par `t('…')` avec des clés dans `ui/src/locales/fr` **et** `en`.
- **R-PRIVACY : ne jamais committer de HH non anonymisées.** Tout nouveau fichier de fixture passe par `tools/anonymize`. Pas de contenu de main dans les logs au niveau INFO.
- **R-PERF : ne pas régresser les benchs** (`cargo bench -p gr-parser-winamax`, `-p gr-stats`). Si une story touche l'import ou les requêtes, indiquer le résultat avant/après dans le résumé.

## 4. Commandes
Pré-requis Windows : Rust stable (MSVC), Visual Studio Build Tools 2022 (C++), Node 22 LTS + pnpm, WebView2, `just` (`winget install Casey.Just`).

```
just setup          # pnpm install + cargo fetch
just dev            # tauri dev (DuckDB désactivé : feature par défaut coupée en dev)
just dev-full       # tauri dev avec --features analytics-duckdb
just test           # cargo test --workspace + pnpm -C ui test
just lint           # cargo fmt --check, cargo clippy -D warnings, pnpm lint, pnpm typecheck
just check          # lint + test (à lancer avant chaque commit)
just bench          # cargo bench (parser, stats, equity)
just synth N=1000000  # génère N mains synthétiques dans ./target/synth
just perf           # import des mains synthétiques + mesures NFR (tools/perf.ps1)
just snapshots      # cargo insta review (valider les nouveaux snapshots)
just build          # tauri build --no-bundle → exe portable
```

Machine de dev = Intel Celeron N5095A, 16 Go de RAM : la compilation est lente. Il faut donc :
- privilégier `cargo test -p <crate>` ciblé ;
- éviter `cargo clean` ;
- garder DuckDB désactivé tant qu'on ne travaille pas sur `gr-analytics` ;
- laisser la CI produire les builds de release.

## 5. Architecture (rappel, PRD §7)
```
crates/gr-core            types de domaine (sans I/O)
crates/gr-parser-api      trait RoomParser, ParseError, détection
crates/gr-parser-winamax  parser du format Winamax (fichiers en ANGLAIS même avec un client FR), libellés dans en.toml
crates/gr-equity          évaluateur, équité exacte / MC, EV all-in
crates/gr-stats           flags de stats par main (PRD §10.2), contextes, compteurs
crates/gr-store           SQLite : migrations, repositories, backups
crates/gr-analytics       trait AnalyticsBackend : impl SQLite (défaut) + DuckDB (feature)
crates/gr-ingest          watcher, offsets, pipeline, reparse
crates/gr-synth           générateur de mains synthétiques
crates/gr-winmon          (préparation V2) détection des fenêtres Win32, sans overlay
src-tauri                 commandes IPC, état, tray, logs, settings
ui                        React 19, TS strict, Tailwind + shadcn/ui, TanStack Query/Table, ECharts, i18next, Zustand
```
- Dépendances **descendantes uniquement** : `gr-core` ← parsers ← stats ← store ← ingest/analytics ← src-tauri.
- `gr-core`, `gr-parser-*`, `gr-stats` et `gr-equity` sont **purs** : pas d'I/O, testables sous Linux en CI.
- Un seul writer SQLite (tâche dédiée dans `gr-ingest`) ; les lectures passent par un pool.
- IPC : les commandes Tauri sont typées, et les types TS sont générés depuis Rust (`specta`/`ts-rs`) dans `ui/src/bindings.ts` (fichier généré, ne pas éditer).
- Événements backend → UI : `import://progress`, `hands://new`, `import://error`, `backup://done`.

## 6. Conventions de code
**Rust** : édition 2021, `rustfmt` par défaut, `clippy::pedantic` en warn sur les crates `gr-*`. Modules petits, avec docs `///` sur les fonctions publiques. Stats : une fonction pure par stat, `fn compute_<code>(hand: &HandRecord, player: PlayerIdx) -> StatFlag { opp: bool, act: bool }`.

**TypeScript** : `strict: true`, pas de `any`. Composants fonctionnels, hooks `useXxx`. Dossiers `ui/src/{app,screens,components,features,hooks,lib,locales,styles}`. Les appels IPC sont encapsulés dans `ui/src/lib/api.ts` via TanStack Query.

**UI** : utiliser uniquement les tokens de `ui/src/styles/tokens.css` (PRD §16), jamais de couleur en dur. Le monochrome strict est le mode par défaut. Chiffres en police tabulaire (`font-variant-numeric: tabular-nums`). Tableaux virtualisés au-delà de 200 lignes.

**SQL** : noms `snake_case`. Index créés dans la même migration que la table. Requêtes paramétrées uniquement.

## 7. Tests
- Parser : un snapshot `insta` par fichier de `fixtures/winamax/**`. Tout bug de parsing corrigé ⇒ un nouveau fixture minimal dans `fixtures/winamax/edge-cases/` avec un nom explicite.
- Stats : pour chaque stat, au moins un cas positif, un cas négatif et un cas « pas d'opportunité », avec le DSL `hand!{}` de `gr-stats/tests/support`.
- Équité : valeurs de référence (AA vs KK ≈ 81,9 %, etc.), tolérance ±0,1 % en exact.
- KPIs : scénarios chiffrés au centime (tickets FACE/ZERO, re-entries, PKO, freeroll).
- Propriétés : conservation des jetons ; aucune panique sur une entrée tronquée.
- Ne jamais désactiver ou supprimer un test pour faire passer la CI. Corriger la cause, ou signaler et demander.

## 8. Git et CI
- Branche `main` protégée. Travail sur des branches `feat/<story-id>-slug`, puis PR vers `main` (Frédéric peut merger lui-même).
- La CI (`.github/workflows/ci.yml`) doit être verte avant le merge. La release se fait sur tag `vX.Y.Z` (`release.yml`) et produit `graphite-vX.Y.Z-portable.zip`.
- SemVer : `0.x` jusqu'à la V1.0.

## 9. Ce qu'il ne faut PAS faire
- Implémenter le HUD overlay, la fiche joueur, l'ICM, l'export de mains, le cloud ou l'IA (hors V1).
- Ajouter une dépendance réseau ou un service externe.
- Supporter le cash game, l'Expresso ou l'Omaha « au passage ».
- Réécrire de gros blocs hors de la story en cours (« refactoring opportuniste »). Proposer plutôt une story dédiée.
