-- Migration initiale (PRD §15). Ne jamais modifier ce fichier une fois merge
-- (R-SCHEMA) : toute evolution passe par une nouvelle migration numerotee.
-- Les PRAGMA (WAL, foreign_keys, synchronous) sont appliques par le code a
-- chaque ouverture de connexion, pas ici.

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

CREATE TABLE tournament_entries (          -- resultat du Hero (un pseudo Hero par ligne)
  id INTEGER PRIMARY KEY, tournament_id INTEGER NOT NULL REFERENCES tournaments(id),
  player_id INTEGER NOT NULL REFERENCES players(id),
  entries_count INTEGER DEFAULT 1, finish_position INTEGER,
  prize_cents INTEGER DEFAULT 0, bounty_cents INTEGER DEFAULT 0,
  ticket_won_type_id INTEGER REFERENCES ticket_types(id), ticket_won_count INTEGER DEFAULT 0,
  paid_with_ticket INTEGER DEFAULT 0, ticket_used_type_id INTEGER REFERENCES ticket_types(id),
  summary_file_id INTEGER REFERENCES import_files(id),
  UNIQUE(tournament_id, player_id));

CREATE TABLE tournament_bullets (         -- une ligne par entree (bloc de summary)
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
  codec TEXT DEFAULT 'zstd', data BLOB NOT NULL);   -- texte brut compresse (replayer, reparse)

CREATE TABLE hand_players (                        -- table de faits principale
  hand_id INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,
  player_id INTEGER NOT NULL REFERENCES players(id),
  seat INTEGER, position TEXT, position_group TEXT, is_hero INTEGER DEFAULT 0,
  start_stack INTEGER, stack_bb REAL, eff_stack_bb REAL, depth_bucket TEXT, eff_depth_bucket TEXT,
  hole_cards TEXT, hand_class TEXT,               -- ex. 'AKs', 'TT', 'Q9o'
  net_chips INTEGER, net_bb REAL, bounty_won_cents INTEGER DEFAULT 0,
  saw_flop INTEGER, saw_turn INTEGER, saw_river INTEGER, went_sd INTEGER, won_sd INTEGER, won_hand INTEGER,
  preflop_line TEXT,                              -- ex. 'RFI', 'RFI-F3B', 'CALL-OPEN', 'SQZ'
  -- flags de stats (opportunite / action) -- un couple par stat §10.2
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
