-- M6-2 (hero_allin_ev_diff_bb, NFR-P5) : le KPI "ecart EV all-in" filtre
-- "hp.is_hero = 1 AND hp.allin_ev_diff_chips IS NOT NULL", puis joint sur
-- hands pour le profil/la periode. Sans index dedie, l'index existant
-- ix_hp_hero (is_hero, player_id) fait deja un scan cible sur is_hero = 1,
-- mais ca reste TOUTES les mains d'Hero (les evenements all-in sont rares,
-- M5-4) : mesure a 1M mains, ~470ms par appel (x2, periode courante et
-- precedente) sur ~968ms au total, au ras de la cible NFR-P5 de 1s.
-- Index partiel sur les seules lignes ayant reellement un evenement all-in
-- (une fraction bien plus petite) : jamais de modification de 0001_init.sql
-- ni de 0002 deja mergees (R-SCHEMA), nouvelle migration.
CREATE INDEX ix_hp_hero_allin ON hand_players(hand_id)
  WHERE is_hero = 1 AND allin_ev_diff_chips IS NOT NULL;
