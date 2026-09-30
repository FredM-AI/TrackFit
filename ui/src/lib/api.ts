import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  HandListRowPayload,
  HandsNewPayload,
  HeroProfilePayload,
  HomeSnapshotPayload,
  ImportProgressPayload,
  ImportSummaryPayload,
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

/** M6-2 : instantane de l'ecran Accueil (KPIs 30j vs 30j precedents, courbe
 * G1, derniere session, etat de l'import), scope au profil Hero actif. */
export function getHomeSnapshot(nowMs: number) {
  return invoke<HomeSnapshotPayload>('get_home_snapshot', { nowMs })
}

/** M6-3 : instantane de l'ecran Resultats (G1, G2, G6, pivot buy-in x
 * KO/non-KO + CSV), scope au profil Hero actif. */
export function getResultsSnapshot() {
  return invoke<ResultsSnapshotPayload>('get_results_snapshot')
}

/** M6-4 : liste des tournois du profil Hero actif (tout l'historique,
 * tableau virtualise cote UI). */
export function getTournamentsList() {
  return invoke<TournamentListRowPayload[]>('get_tournaments_list')
}

/** M6-4 : detail d'un tournoi (tapis par main, all-in, adversaires),
 * `null` si aucun profil actif ou si ce tournoi n'a aucune main du Hero. */
export function getTournamentDetail(tournamentId: number) {
  return invoke<TournamentDetailPayload | null>('get_tournament_detail', { tournamentId })
}

/** M6-5 : nombre total de mains du profil Hero actif (dimensionne le
 * tableau virtualise), appele une seule fois au montage de l'ecran. */
export function getHandsCount() {
  return invoke<number>('get_hands_count')
}

/** M6-5 : une page de la liste des mains (la plus recente d'abord). */
export function getHandsPage(limit: number, offset: number) {
  return invoke<HandListRowPayload[]>('get_hands_page', { limit, offset })
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
