import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useRouterState } from '@tanstack/react-router'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { getActiveHeroProfileId, listHeroProfiles, setActiveHeroProfileId } from '@/lib/api'
import { type DateRangePreset, type FilterScreen, useFilterStore } from '@/lib/filters'

const DATE_PRESETS: DateRangePreset[] = ['today', 'last7', 'last30', 'thisMonth', 'thisYear', 'allTime']

function screenFromPathname(pathname: string): FilterScreen | null {
  switch (pathname) {
    case '/':
      return 'home'
    case '/results':
      return 'results'
    case '/tournaments':
      return 'tournaments'
    case '/hands':
      return 'hands'
    default:
      return null
  }
}

function toDateInputValue(epochMs: number | undefined): string {
  if (epochMs == null) return ''
  return new Date(epochMs).toISOString().slice(0, 10)
}

function fromDateInputValue(value: string): number | undefined {
  if (!value) return undefined
  return new Date(`${value}T00:00:00`).getTime()
}

/** Panneau de filtres global (M6-1, PRD §11, phase 1 : dates + profil Hero
 * + presets), rendu dans l'en-tete partage de `AppShell`. Le selecteur de
 * profil Hero est toujours visible (concept global, reutilise M3-5) ; le
 * controle de periode n'apparait que sur les 4 ecrans branches (Accueil,
 * Resultats, Tournois, Mains — pas sur le detail d'un tournoi ni sur les
 * ecrans sans donnees filtrables comme Logs/Parametres). */
export function FilterBar() {
  const { t } = useTranslation()
  const pathname = useRouterState({ select: (state) => state.location.pathname })
  const queryClient = useQueryClient()
  const screen = screenFromPathname(pathname)

  const hydrated = useFilterStore((s) => s.hydrated)
  const hydrate = useFilterStore((s) => s.hydrate)
  const perScreen = useFilterStore((s) => s.perScreen)
  const setRange = useFilterStore((s) => s.setRange)
  const presets = useFilterStore((s) => s.presets)
  const addPreset = useFilterStore((s) => s.addPreset)

  useEffect(() => {
    if (!hydrated) void hydrate()
  }, [hydrated, hydrate])

  const profilesQuery = useQuery({ queryKey: ['heroProfiles'], queryFn: listHeroProfiles })
  const activeProfileQuery = useQuery({
    queryKey: ['activeHeroProfile'],
    queryFn: getActiveHeroProfileId,
  })

  const [presetName, setPresetName] = useState('')

  async function handleProfileChange(profileId: number) {
    await setActiveHeroProfileId(profileId)
    await queryClient.invalidateQueries({ queryKey: ['activeHeroProfile'] })
    await queryClient.invalidateQueries({ queryKey: ['home'] })
    await queryClient.invalidateQueries({ queryKey: ['results'] })
    await queryClient.invalidateQueries({ queryKey: ['tournaments'] })
    await queryClient.invalidateQueries({ queryKey: ['hands'] })
  }

  const profiles = profilesQuery.data ?? []
  const range = screen ? perScreen[screen] : undefined

  return (
    <div className="flex w-full items-center gap-4 overflow-x-auto px-1">
      {profiles.length > 1 && (
        <select
          aria-label={t('filters.heroProfile')}
          value={activeProfileQuery.data ?? ''}
          onChange={(e) => void handleProfileChange(Number(e.target.value))}
          className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-2 py-1 text-xs"
        >
          {profiles.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      )}

      {screen && range && (
        <>
          <div className="flex items-center gap-1">
            {DATE_PRESETS.map((preset) => (
              <button
                key={preset}
                type="button"
                onClick={() => setRange(screen, { preset })}
                className={`rounded px-2 py-1 text-xs ${
                  range.preset === preset
                    ? 'bg-[var(--color-accent)] text-[var(--color-bg)]'
                    : 'bg-[var(--color-surface-2)] text-[var(--color-text-secondary)]'
                }`}
              >
                {t(`filters.presets.${preset}`)}
              </button>
            ))}
          </div>

          {range.preset === 'custom' && (
            <div className="flex items-center gap-1 text-xs">
              <input
                type="date"
                aria-label={t('filters.customSince')}
                value={toDateInputValue(range.customSinceMs)}
                onChange={(e) =>
                  setRange(screen, {
                    preset: 'custom',
                    customSinceMs: fromDateInputValue(e.target.value),
                    customUntilMs: range.customUntilMs,
                  })
                }
                className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-1 py-0.5"
              />
              <span className="text-[var(--color-text-secondary)]">→</span>
              <input
                type="date"
                aria-label={t('filters.customUntil')}
                value={toDateInputValue(range.customUntilMs)}
                onChange={(e) =>
                  setRange(screen, {
                    preset: 'custom',
                    customSinceMs: range.customSinceMs,
                    customUntilMs: fromDateInputValue(e.target.value),
                  })
                }
                className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-1 py-0.5"
              />
            </div>
          )}
          {range.preset !== 'custom' && (
            <button
              type="button"
              onClick={() => setRange(screen, { preset: 'custom' })}
              className="text-xs text-[var(--color-text-secondary)] underline"
            >
              {t('filters.customRange')}
            </button>
          )}

          {presets.length > 0 && (
            <select
              aria-label={t('filters.applyPreset')}
              value=""
              onChange={(e) => {
                const preset = presets.find((p) => p.id === e.target.value)
                if (preset) setRange(screen, preset.range)
              }}
              className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-2 py-1 text-xs"
            >
              <option value="" disabled>
                {t('filters.applyPreset')}
              </option>
              {presets.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          )}

          <form
            className="flex items-center gap-1"
            onSubmit={(e) => {
              e.preventDefault()
              if (!presetName.trim()) return
              addPreset({ id: crypto.randomUUID(), name: presetName.trim(), range })
              setPresetName('')
            }}
          >
            <input
              type="text"
              value={presetName}
              onChange={(e) => setPresetName(e.target.value)}
              placeholder={t('filters.newPresetName')}
              className="w-28 rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-2 py-1 text-xs"
            />
            <button
              type="submit"
              disabled={!presetName.trim()}
              className="rounded bg-[var(--color-surface-3)] px-2 py-1 text-xs disabled:opacity-40"
            >
              {t('filters.savePreset')}
            </button>
          </form>
        </>
      )}
    </div>
  )
}
