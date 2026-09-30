import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  HandListRowPayload,
  HandReplayPayload,
  HandsNewPayload,
  HeroProfilePayload,
  HomeSnapshotPayload,
  ImportProgressPayload,
  ImportSummaryPayload,
  ReportRowPayload,
  ResultsSnapshotPayload,
  StatusSnapshotPayload,
  TagPayload,
  TournamentDetailPayload,
  TournamentListRowPayload,
  WinamaxAccountPayload,
} from '@/bindings'

/** M3-1 : comptes Winamax locaux detectes sous accounts\<pseudo>\history\. */
export function detectWinamaxAccounts() {
  return invoke<WinamaxAccountPayload[]>('detect_winamax_accounts')
}

/** M3-1/M3-5 : cree le profil Hero par defaut et y rattache les pseudos choisis. */
export function createHeroProfileAndAccounts(name: string, pseudos: string[]) {
  return invoke<number>('create_hero_profile_and_accounts', { name, pseudos })
}

/** M3-1 : l'assistant de premier lancement a-t-il deja ete complete ? */
export function isFirstLaunchComplete() {
  return invoke<boolean>('is_first_launch_complete')
}

/** M3-1 : marque l'assistant de premier lancement comme termine. */
export function markFirstLaunchComplete() {
  return invoke<void>('mark_first_launch_complete')
}

/** M2-3 : lance l'import sur les dossiers/fichiers donnes. */
export function importPaths(paths: string[]) {
  return invoke<ImportSummaryPayload>('import_paths', { paths })
}

/** M2-3 : demande l'arret d'un import en cours. */
export function cancelImport() {
  return invoke<void>('cancel_import')
}

/** M2-3 : ecoute la progression d'un import (`import://progress`). */
export function onImportProgress(callback: (progress: ImportProgressPayload) => void) {
  return listen<ImportProgressPayload>('import://progress', (event) => callback(event.payload))
}

/** M3-2 : persiste les dossiers a surveiller en temps reel et (re)demarre le watcher. */
export function setWatchedRoots(roots: string[]) {
  return invoke<void>('set_watched_roots', { roots })
}

/** M3-2 : ecoute l'arrivee de nouvelles mains detectees par le watcher (`hands://new`). */
export function onHandsNew(callback: (payload: HandsNewPayload) => void) {
  return listen<HandsNewPayload>('hands://new', (event) => callback(event.payload))
}

/** M3-3 : instantane pour la barre d'etat (mains aujourd'hui, derniere main, statut watcher). */
export function getStatusSnapshot(sinceMs: number) {
  return invoke<StatusSnapshotPayload>('get_status_snapshot', { sinceMs })
}

/** M3-3 : premiere fermeture de la fenetre (masquage en tray) a acquitter. */
export function onTrayFirstHideNotice(callback: () => void) {
  return listen('tray://first-hide-notice', () => callback())
}

/** M3-3 : n'affiche plus jamais le mot d'explication du masquage en tray. */
export function markTrayNoticeShown() {
  return invoke<void>('mark_tray_notice_shown')
}

/** M3-5 : liste tous les profils Hero avec leurs pseudos rattaches. */
export function listHeroProfiles() {
  return invoke<HeroProfilePayload[]>('list_hero_profiles_cmd')
}

/** M3-5 : id du profil Hero actif, `null` si aucun profil n'existe encore. */
export function getActiveHeroProfileId() {
  return invoke<number | null>('get_active_hero_profile_id')
}

/** M3-5 : change le profil Hero actif (filtre toutes les requetes Hero). */
export function setActiveHeroProfileId(profileId: number) {
  return invoke<void>('set_active_hero_profile_id', { profileId })
}

/** M3-5 : cree un nouveau profil Hero avec les pseudos donnes. */
export function createHeroProfile(name: string, pseudos: string[]) {
  return invoke<number>('create_hero_profile_cmd', { name, pseudos })
}

/** M3-5 : renomme un profil Hero. */
export function renameHeroProfile(profileId: number, newName: string) {
  return invoke<void>('rename_hero_profile_cmd', { profileId, newName })
}

/** M3-5 : rattache un pseudo supplementaire a un profil Hero. */
export function addHeroPseudo(profileId: number, pseudo: string) {
  return invoke<void>('add_hero_pseudo_cmd', { profileId, pseudo })
}

/** M3-5 : detache un pseudo d'un profil Hero. */
export function removeHeroPseudo(profileId: number, pseudo: string) {
  return invoke<void>('remove_hero_pseudo_cmd', { profileId, pseudo })
}

/** Plage de dates resolue (M6-1) : `null`/`null` = tout l'historique. Les
 * commandes de snapshot/liste ci-dessous l'acceptent toutes en parametres
 * optionnels, resolus cote UI (`ui/src/lib/filters.ts`) a partir du preset
 * choisi dans le panneau de filtres global. */
export interface DateRangeParams {
  sinceMs: number | null
  untilMs: number | null
}

/** M6-2 : instantane de l'ecran Accueil (KPIs periode courante vs
 * precedente, courbe G1 non filtree, derniere session, etat de l'import),
 * scope au profil Hero actif. Filtrable par periode depuis M6-1. */
export function getHomeSnapshot({ sinceMs, untilMs }: DateRangeParams) {
  return invoke<HomeSnapshotPayload>('get_home_snapshot', { sinceMs, untilMs })
}

/** M6-3 : instantane de l'ecran Resultats (G1, G2, G6, pivot buy-in x
 * KO/non-KO + CSV), scope au profil Hero actif. Filtrable par periode
 * depuis M6-1. */
export function getResultsSnapshot({ sinceMs, untilMs }: DateRangeParams) {
  return invoke<ResultsSnapshotPayload>('get_results_snapshot', { sinceMs, untilMs })
}

/** M6-4 : liste des tournois du profil Hero actif (tableau virtualise cote
 * UI). Filtrable par periode depuis M6-1. */
export function getTournamentsList({ sinceMs, untilMs }: DateRangeParams) {
  return invoke<TournamentListRowPayload[]>('get_tournaments_list', { sinceMs, untilMs })
}

/** M6-4 : detail d'un tournoi (tapis par main, all-in, adversaires),
 * `null` si aucun profil actif ou si ce tournoi n'a aucune main du Hero. */
export function getTournamentDetail(tournamentId: number) {
  return invoke<TournamentDetailPayload | null>('get_tournament_detail', { tournamentId })
}

/** M6-5 : nombre total de mains du profil Hero actif (dimensionne le
 * tableau virtualise), appele une seule fois par changement de filtre.
 * Filtrable par periode depuis M6-1. */
export function getHandsCount({ sinceMs, untilMs }: DateRangeParams) {
  return invoke<number>('get_hands_count', { sinceMs, untilMs })
}

/** M6-5 : une page de la liste des mains (la plus recente d'abord).
 * Filtrable par periode depuis M6-1. */
export function getHandsPage(
  limit: number,
  offset: number,
  { sinceMs, untilMs }: DateRangeParams,
) {
  return invoke<HandListRowPayload[]>('get_hands_page', { limit, offset, sinceMs, untilMs })
}

/** M6-5 : tags connus (predefinis d'abord). */
export function listTags() {
  return invoke<TagPayload[]>('list_tags')
}

/** M6-5 : applique `tagId` a chaque main de `handIds` ("selection multiple
 * -> tag en masse", PRD §13.4). */
export function tagHands(handIds: number[], tagId: number, nowMs: number) {
  return invoke<void>('tag_hands', { handIds, tagId, nowMs })
}

/** M6-1 : etat de filtre persiste pour `screen` (JSON brut, deserialise
 * cote appelant), `null` si jamais enregistre. */
export function getFilterState(screen: string) {
  return invoke<string | null>('get_filter_state', { screen })
}

/** M6-1 : enregistre l'etat de filtre de `screen` (JSON deja serialise). */
export function setFilterState(screen: string, json: string) {
  return invoke<void>('set_filter_state', { screen, json })
}

/** M6-1 : presets de dates nommes (JSON brut, un tableau), partages entre
 * tous les ecrans. */
export function getFilterPresets() {
  return invoke<string | null>('get_filter_presets')
}

/** M6-1 : enregistre la liste des presets de dates nommes. */
export function setFilterPresets(json: string) {
  return invoke<void>('set_filter_presets', { json })
}

/** M7-1 : rapports predefinis (perimetre reduit, pas de constructeur
 * generique). « Preflop par position » : VPIP/PFR/RFI/LIMP/OSHOVE/3-bet/
 * fold-to-3-bet/4-bet/ATS par groupe de position. */
export function getPreflopByPositionReport() {
  return invoke<ReportRowPayload[]>('get_preflop_by_position_report')
}

/** M7-1 : « Open-shove par profondeur × position ». */
export function getOshoveByDepthAndPositionReport() {
  return invoke<ReportRowPayload[]>('get_oshove_by_depth_and_position_report')
}

/** M7-1 : « Postflop (c-bet) » — c-bet flop/turn, fold au c-bet flop, par
 * groupe de position. */
export function getPostflopCbetReport() {
  return invoke<ReportRowPayload[]>('get_postflop_cbet_report')
}

/** M7-1 : « Defense de BB par profondeur » (PRD §12.2 : "Fold BB to
 * steal") — position brute BB uniquement, pas le groupe "Blinds". */
export function getBbDefenseByDepthReport() {
  return invoke<ReportRowPayload[]>('get_bb_defense_by_depth_report')
}

/** M7-3 : rejeu complet d'une main (table, pas, historique brut, pots,
 * all-in). `null` si la main est inconnue ou ne reparse plus. */
export function getHandReplay(handId: number) {
  return invoke<HandReplayPayload | null>('get_hand_replay', { handId })
}
