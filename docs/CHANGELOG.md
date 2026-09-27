# Changelog

## [Unreleased]
- docs: PRD, backlog, spécification du format Winamax, 2 fixtures anonymisés.
- chore(ci): repo GitHub `TrackFit` initialisé, premier commit poussé, branche `main` protégée (M0-1).
- chore: squelette du workspace (10 crates Rust vides, `src-tauri` Tauri 2, `ui` Vite/React/TS, `justfile`) ; `just dev` et `just check` validés (M0-2).
- chore(ci): CI GitHub Actions (jobs linux/windows) et release automatisée validées de bout en bout sur le tag `v0.0.1` (M0-3). M0 est terminé.
- docs: corpus réel élargi (620+ tournois) anonymisé ; 5 nouveaux fixtures (classique, Mystery KO, freeroll, table finale/heads-up, multi-flights) + 1 edge-case ; `docs/formats/winamax.md` mis à jour (Type/Mode/Flight ID confirmés, tickets toujours non observés) (M0-4).
- docs: M0-4 clôturé — Frédéric valide le report des 3 cas encore manquants (tickets, fichier en cours) ; notés dans « Idées / à trier ». M0 est intégralement terminé.
- feat(ui): shell UI (barre latérale, filtres, barre d'état), tokens de design monochrome (§16), TanStack Router (9 écrans + Logs), i18next FR/EN, règle ESLint i18next/no-literal-string active (M0-5).
- feat(gr-core): types de domaine (Card/Rank/Suit, Money/Chips, Street, Position, ActionKind/ActionRecord, SeatInfo, HandRecord, TournamentSummary/KoType), sérialisation serde, parsing/affichage des cartes, 14 tests (M1-1). M1 démarré.
