import { useNavigate } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ImportProgressPayload, ImportSummaryPayload } from '@/bindings'
import {
  createHeroProfileAndAccounts,
  detectWinamaxAccounts,
  importPaths,
  markFirstLaunchComplete,
  onImportProgress,
  setWatchedRoots,
} from '@/lib/api'

type Step = 'choose' | 'importing' | 'done'

export function Setup() {
  const { t } = useTranslation()
  const navigate = useNavigate()

  const accountsQuery = useQuery({
    queryKey: ['setup', 'winamax-accounts'],
    queryFn: detectWinamaxAccounts,
  })
  const accounts = accountsQuery.data ?? []

  const [step, setStep] = useState<Step>('choose')
  // `null` = pas encore modifie par l'utilisateur : la valeur affichee
  // retombe alors sur les comptes detectes (derivee au rendu, pas via un
  // effet, pour eviter un cascading render inutile).
  const [profileNameOverride, setProfileNameOverride] = useState<string | null>(null)
  const [manualPseudo, setManualPseudo] = useState('')
  const [selectedOverride, setSelectedOverride] = useState<Set<string> | null>(null)
  const [progress, setProgress] = useState<ImportProgressPayload | null>(null)
  const [result, setResult] = useState<ImportSummaryPayload | null>(null)
  const [error, setError] = useState<string | null>(null)

  const selected = selectedOverride ?? new Set(accounts.map((a) => a.pseudo))
  const profileName = profileNameOverride ?? accounts[0]?.pseudo ?? 'Hero'
  const selectedAccounts = accounts.filter((a) => selected.has(a.pseudo))

  function toggle(pseudo: string) {
    const next = new Set(selected)
    if (next.has(pseudo)) {
      next.delete(pseudo)
    } else {
      next.add(pseudo)
    }
    setSelectedOverride(next)
  }

  async function handleSubmit() {
    setError(null)
    const pseudos =
      accounts.length > 0 ? selectedAccounts.map((a) => a.pseudo) : [manualPseudo.trim()]
    const name = profileName.trim() || pseudos[0] || 'Hero'

    try {
      await createHeroProfileAndAccounts(name, pseudos)

      const historyDirs = selectedAccounts.map((a) => a.history_dir)
      if (historyDirs.length > 0) {
        setStep('importing')
        const unlisten = await onImportProgress(setProgress)
        try {
          const summary = await importPaths(historyDirs)
          setResult(summary)
        } finally {
          unlisten()
        }
        // M3-2 : les memes dossiers passent en surveillance temps reel des
        // maintenant (sans attendre un redemarrage de l'app).
        await setWatchedRoots(historyDirs)
      }

      await markFirstLaunchComplete()
      setStep('done')
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      setStep('choose')
    }
  }

  const canSubmit =
    accounts.length > 0 ? selected.size > 0 : manualPseudo.trim().length > 0

  return (
    <div className="mx-auto max-w-lg p-6">
      <h1 className="mb-1 text-base font-semibold">{t('setup.title')}</h1>
      <p className="mb-6 text-sm text-[var(--color-text-secondary)]">{t('setup.intro')}</p>

      {step === 'choose' && (
        <div className="flex flex-col gap-4">
          {accountsQuery.isLoading && (
            <p className="text-sm text-[var(--color-text-secondary)]">{t('setup.detecting')}</p>
          )}

          {accountsQuery.isSuccess && accounts.length > 0 && (
            <fieldset className="flex flex-col gap-2">
              <legend className="mb-1 text-sm font-medium">{t('setup.accountsFound')}</legend>
              {accounts.map((account) => (
                <label
                  key={account.pseudo}
                  className="flex items-center gap-2 rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-3 py-2 text-sm"
                >
                  <input
                    type="checkbox"
                    name={`account-${account.pseudo}`}
                    checked={selected.has(account.pseudo)}
                    onChange={() => toggle(account.pseudo)}
                  />
                  <span className="flex-1">{account.pseudo}</span>
                  <span className="text-xs text-[var(--color-text-secondary)]">
                    {t('setup.handFileCount', { count: account.hand_file_count })}
                  </span>
                </label>
              ))}
            </fieldset>
          )}

          {accountsQuery.isSuccess && accounts.length === 0 && (
            <div className="flex flex-col gap-2">
              <p className="text-sm text-[var(--color-text-secondary)]">
                {t('setup.noAccountsFound')}
              </p>
              <label className="flex flex-col gap-1 text-sm">
                {t('setup.manualPseudo')}
                <input
                  type="text"
                  name="manual-pseudo"
                  autoComplete="off"
                  value={manualPseudo}
                  onChange={(event) => setManualPseudo(event.target.value)}
                  className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-2 py-1"
                />
              </label>
            </div>
          )}

          <label className="flex flex-col gap-1 text-sm">
            {t('setup.profileName')}
            <input
              type="text"
              name="profile-name"
              autoComplete="off"
              value={profileName}
              onChange={(event) => setProfileNameOverride(event.target.value)}
              className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-2 py-1"
            />
          </label>

          {error && <p className="text-sm text-[var(--color-negative)]">{error}</p>}

          <button
            type="button"
            disabled={!canSubmit}
            onClick={() => {
              void handleSubmit()
            }}
            className="self-start rounded bg-[var(--color-accent)] px-4 py-1.5 text-sm text-[var(--color-bg)] disabled:opacity-40"
          >
            {t('setup.start')}
          </button>
        </div>
      )}

      {step === 'importing' && (
        <div className="flex flex-col gap-2">
          <p className="text-sm">{t('setup.importing')}</p>
          {progress && (
            <p className="text-sm text-[var(--color-text-secondary)]">
              {t('setup.importProgress', {
                done: progress.files_done,
                total: progress.files_total,
                hands: progress.hands_inserted,
              })}
            </p>
          )}
        </div>
      )}

      {step === 'done' && (
        <div className="flex flex-col gap-3">
          <p className="text-sm">{t('setup.done')}</p>
          {result && (
            <p className="text-sm text-[var(--color-text-secondary)]">
              {t('setup.doneSummary', {
                hands: result.hands_inserted,
                tournaments: result.summaries_attached,
              })}
            </p>
          )}
          <button
            type="button"
            onClick={() => {
              void navigate({ to: '/' })
            }}
            className="self-start rounded bg-[var(--color-accent)] px-4 py-1.5 text-sm text-[var(--color-bg)]"
          >
            {t('setup.finish')}
          </button>
        </div>
      )}
    </div>
  )
}
