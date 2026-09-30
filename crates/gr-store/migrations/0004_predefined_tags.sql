-- M6-5 (§13.4 : "selection multiple -> tag en masse") : la liste des 9 tags
-- predefinis est deja entierement specifiee par le PRD (§13.7, D27 : "Bad
-- beat, Cooler, Hero call, Bluff, Erreur preflop, Erreur postflop, A
-- revoir, Spot ICM, Question coach"), donc semee ici plutot que differee --
-- rien a inventer (R-FORMAT ne s'applique pas, ce n'est pas une donnee de
-- format Winamax). `label_key` est une cle i18n (traduite cote UI), pas le
-- libelle affiche directement -- coherent avec le nom de la colonne. Les
-- tags libres (creation a la volee) et les notes par main restent le
-- perimetre de M7-4, pas de cette migration.
INSERT INTO tags (label_key, is_predefined, shade) VALUES
  ('tags.badBeat', 1, NULL),
  ('tags.cooler', 1, NULL),
  ('tags.heroCall', 1, NULL),
  ('tags.bluff', 1, NULL),
  ('tags.preflopMistake', 1, NULL),
  ('tags.postflopMistake', 1, NULL),
  ('tags.toReview', 1, NULL),
  ('tags.icmSpot', 1, NULL),
  ('tags.coachQuestion', 1, NULL);
