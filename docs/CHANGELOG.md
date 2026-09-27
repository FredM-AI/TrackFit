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
- feat(parser): trait `RoomParser` (`gr-parser-api`) et détection Winamax (`gr-parser-winamax`) ; 100% des fixtures committés détectés (M1-2).
- feat(parser): découpage des mains (`split_hand_blocks`, séparateur `\n\n\n`) avec gestion des blocs incomplets et offset de reprise ; testé par troncature ligne par ligne sur un vrai fixture (M1-3).
- feat(parser): parsing de l'en-tête/table/sièges (`WinamaxParser::parse_hand`), conversion de date sans dépendance validée contre le HandId réel, 9 snapshots `insta` (1679 mains) ; edge-case synthétique pour les pseudos à espaces/points/tirets (M1-4).
- feat(parser): actions/streets/board/pots complets, invariant de conservation PAR-11 vérifié sur tout le corpus, `uncalled_excess` calculé par une formule arithmétique générale (pas de détection de texte « side pot ») ; bug de reset des mises preflop trouvé et corrigé (M1-5).
- feat(parser): `parse_summary` (buy-in prize/bounty/fee, multi-blocs, Late Registration, 3 formes de `You won`) ; `RoomParser` entièrement implémenté pour `WinamaxParser`. Test chiffré OBELISK exact (M1-6).
