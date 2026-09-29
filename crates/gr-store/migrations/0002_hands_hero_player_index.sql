-- M4-7 (AnalyticsBackend, NFR-P6) : le rapport agrege hand_players x hands
-- filtre par "h.hero_player_id IN (SELECT player_id FROM hero_accounts
-- WHERE profile_id = ?)". Sans index sur hands.hero_player_id, ce filtre
-- force un scan complet de hands (mesure : 7.99s pour 2M mains, quasiment
-- au ras de la cible NFR-P6 de 8s). Jamais de modification de 0001_init.sql
-- (R-SCHEMA) : nouvelle migration.
CREATE INDEX ix_hands_hero_player ON hands(hero_player_id);
