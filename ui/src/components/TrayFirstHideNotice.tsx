import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { markTrayNoticeShown, onTrayFirstHideNotice } from '@/lib/api'

/**
 * Mot d'explication affiche une seule fois (M3-3, idee backlog du 29/09) au
 * tout premier masquage de la fenetre en tray : rien n'indiquait autrement
 * a l'utilisateur que l'app continuait de tourner. Le backend garde la
 * fenetre visible et emet `tray://first-hide-notice` au lieu de la masquer
 * directement tant que ce mot n'a pas ete acquitte (`mark_tray_notice_shown`).
 */
export function TrayFirstHideNotice() {
  const { t } = useTranslation()
  const [visible, setVisible] = useState(false)

  useEffect(() => {
    // Hors contexte Tauri (tests, apercu navigateur), `listen` rejette : on
    // degrade silencieusement plutot que de planter le montage.
    const unlisten = onTrayFirstHideNotice(() => setVisible(true)).catch(() => undefined)
    return () => {
      void unlisten.then((fn) => fn?.())
    }
  }, [])

  if (!visible) {
    return null
  }

  async function acknowledge() {
    setVisible(false)
    await markTrayNoticeShown().catch(() => undefined)
    await getCurrentWindow().hide()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="mx-4 max-w-sm rounded border border-[var(--color-border)] bg-[var(--color-surface-1)] p-4 text-[var(--color-text-primary)]">
        <h2 className="mb-2 text-sm font-semibold">{t('tray.firstHideTitle')}</h2>
        <p className="mb-4 text-sm text-[var(--color-text-secondary)]">{t('tray.firstHideBody')}</p>
        <button
          type="button"
          onClick={() => void acknowledge()}
          className="self-start rounded bg-[var(--color-accent)] px-4 py-1.5 text-sm text-[var(--color-bg)]"
        >
          {t('tray.firstHideConfirm')}
        </button>
      </div>
    </div>
  )
}
