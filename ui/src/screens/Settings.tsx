import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  addHeroPseudo,
  createHeroProfile,
  getActiveHeroProfileId,
  listHeroProfiles,
  removeHeroPseudo,
  renameHeroProfile,
  setActiveHeroProfileId,
} from '@/lib/api'

const PROFILES_QUERY_KEY = ['hero-profiles']
const ACTIVE_PROFILE_QUERY_KEY = ['hero-profile-active']

/** M3-5 (D19) : section "Profils Hero" — creation/modification, selection
 * du profil dont les requetes Hero (barre d'etat, sessions...) sont
 * filtrees. Seule section reelle de l'ecran Parametres pour l'instant, le
 * reste (sauvegardes, perf...) arrive en M8. */
export function Settings() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()

  const profilesQuery = useQuery({ queryKey: PROFILES_QUERY_KEY, queryFn: listHeroProfiles })
  const activeProfileQuery = useQuery({
    queryKey: ACTIVE_PROFILE_QUERY_KEY,
    queryFn: getActiveHeroProfileId,
  })

  const [renamingId, setRenamingId] = useState<number | null>(null)
  const [renameValue, setRenameValue] = useState('')
  const [newPseudoByProfile, setNewPseudoByProfile] = useState<Record<number, string>>({})
  const [newProfileName, setNewProfileName] = useState('')
  const [newProfilePseudo, setNewProfilePseudo] = useState('')
  const [error, setError] = useState<string | null>(null)

  const profiles = profilesQuery.data ?? []
  const activeProfileId = activeProfileQuery.data ?? null

  function invalidateProfiles() {
    void queryClient.invalidateQueries({ queryKey: PROFILES_QUERY_KEY })
  }
  function invalidateActive() {
    void queryClient.invalidateQueries({ queryKey: ACTIVE_PROFILE_QUERY_KEY })
  }
  function reportError(err: unknown) {
    setError(err instanceof Error ? err.message : String(err))
  }

  async function handleSelectActive(profileId: number) {
    setError(null)
    try {
      await setActiveHeroProfileId(profileId)
      invalidateActive()
    } catch (err) {
      reportError(err)
    }
  }

  function startRename(profileId: number, currentName: string) {
    setRenamingId(profileId)
    setRenameValue(currentName)
  }

  async function confirmRename(profileId: number) {
    setError(null)
    const name = renameValue.trim()
    if (!name) {
      return
    }
    try {
      await renameHeroProfile(profileId, name)
      setRenamingId(null)
      invalidateProfiles()
    } catch (err) {
      reportError(err)
    }
  }

  async function handleAddPseudo(profileId: number) {
    setError(null)
    const pseudo = (newPseudoByProfile[profileId] ?? '').trim()
    if (!pseudo) {
      return
    }
    try {
      await addHeroPseudo(profileId, pseudo)
      setNewPseudoByProfile((prev) => ({ ...prev, [profileId]: '' }))
      invalidateProfiles()
    } catch (err) {
      reportError(err)
    }
  }

  async function handleRemovePseudo(profileId: number, pseudo: string) {
    setError(null)
    try {
      await removeHeroPseudo(profileId, pseudo)
      invalidateProfiles()
    } catch (err) {
      reportError(err)
    }
  }

  async function handleCreateProfile() {
    setError(null)
    const name = newProfileName.trim()
    if (!name) {
      return
    }
    try {
      await createHeroProfile(name, newProfilePseudo.trim() ? [newProfilePseudo.trim()] : [])
      setNewProfileName('')
      setNewProfilePseudo('')
      invalidateProfiles()
    } catch (err) {
      reportError(err)
    }
  }

  return (
    <div className="mx-auto max-w-lg p-6">
      <h1 className="mb-1 text-base font-semibold">{t('settings.heroProfiles.title')}</h1>
      <p className="mb-6 text-sm text-[var(--color-text-secondary)]">
        {t('settings.heroProfiles.intro')}
      </p>

      {error && <p className="mb-4 text-sm text-[var(--color-negative)]">{error}</p>}

      <div className="flex flex-col gap-4">
        {profiles.map((profile) => (
          <div
            key={profile.id}
            className="flex flex-col gap-2 rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3"
          >
            <div className="flex items-center gap-2">
              <input
                type="radio"
                name="active-hero-profile"
                id={`active-profile-${profile.id}`}
                checked={activeProfileId === profile.id}
                onChange={() => void handleSelectActive(profile.id)}
              />
              {renamingId === profile.id ? (
                <>
                  <input
                    type="text"
                    name={`rename-profile-${profile.id}`}
                    autoComplete="off"
                    value={renameValue}
                    onChange={(e) => setRenameValue(e.target.value)}
                    className="rounded border border-[var(--color-border)] bg-[var(--color-surface-1)] px-2 py-1 text-sm"
                  />
                  <button
                    type="button"
                    onClick={() => void confirmRename(profile.id)}
                    className="rounded bg-[var(--color-accent)] px-2 py-1 text-xs text-[var(--color-bg)]"
                  >
                    {t('settings.heroProfiles.save')}
                  </button>
                </>
              ) : (
                <>
                  <label htmlFor={`active-profile-${profile.id}`} className="flex-1 text-sm font-medium">
                    {profile.name}
                    {profile.is_default && (
                      <span className="ml-2 text-xs text-[var(--color-text-secondary)]">
                        {t('settings.heroProfiles.defaultBadge')}
                      </span>
                    )}
                  </label>
                  <button
                    type="button"
                    onClick={() => startRename(profile.id, profile.name)}
                    className="text-xs text-[var(--color-text-secondary)] underline"
                  >
                    {t('settings.heroProfiles.rename')}
                  </button>
                </>
              )}
            </div>

            <div className="flex flex-wrap items-center gap-2 pl-6 text-xs text-[var(--color-text-secondary)]">
              {profile.pseudos.length === 0 && <span>{t('settings.heroProfiles.noPseudos')}</span>}
              {profile.pseudos.map((pseudo) => (
                <span
                  key={pseudo}
                  className="flex items-center gap-1 rounded border border-[var(--color-border)] px-2 py-0.5"
                >
                  {pseudo}
                  <button
                    type="button"
                    onClick={() => void handleRemovePseudo(profile.id, pseudo)}
                    aria-label={t('settings.heroProfiles.removePseudo', { pseudo })}
                    className="text-[var(--color-negative)]"
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>

            <div className="flex items-center gap-2 pl-6">
              <input
                type="text"
                name={`add-pseudo-${profile.id}`}
                autoComplete="off"
                value={newPseudoByProfile[profile.id] ?? ''}
                onChange={(e) =>
                  setNewPseudoByProfile((prev) => ({ ...prev, [profile.id]: e.target.value }))
                }
                placeholder={t('settings.heroProfiles.addPseudoPlaceholder')}
                className="rounded border border-[var(--color-border)] bg-[var(--color-surface-1)] px-2 py-1 text-xs"
              />
              <button
                type="button"
                onClick={() => void handleAddPseudo(profile.id)}
                className="text-xs text-[var(--color-text-secondary)] underline"
              >
                {t('settings.heroProfiles.addPseudo')}
              </button>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-6 flex flex-col gap-2 rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
        <h2 className="text-sm font-medium">{t('settings.heroProfiles.createTitle')}</h2>
        <input
          type="text"
          name="new-profile-name"
          autoComplete="off"
          value={newProfileName}
          onChange={(e) => setNewProfileName(e.target.value)}
          placeholder={t('settings.heroProfiles.createNamePlaceholder')}
          className="rounded border border-[var(--color-border)] bg-[var(--color-surface-1)] px-2 py-1 text-sm"
        />
        <input
          type="text"
          name="new-profile-pseudo"
          autoComplete="off"
          value={newProfilePseudo}
          onChange={(e) => setNewProfilePseudo(e.target.value)}
          placeholder={t('settings.heroProfiles.createPseudoPlaceholder')}
          className="rounded border border-[var(--color-border)] bg-[var(--color-surface-1)] px-2 py-1 text-sm"
        />
        <button
          type="button"
          onClick={() => void handleCreateProfile()}
          disabled={!newProfileName.trim()}
          className="self-start rounded bg-[var(--color-accent)] px-4 py-1.5 text-sm text-[var(--color-bg)] disabled:opacity-40"
        >
          {t('settings.heroProfiles.create')}
        </button>
      </div>
    </div>
  )
}
