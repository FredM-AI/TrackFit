import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  HandsNewPayload,
  ImportProgressPayload,
  ImportSummaryPayload,
  StatusSnapshotPayload,
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
