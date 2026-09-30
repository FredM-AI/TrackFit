import { create } from 'zustand'
import {
  getFilterPresets,
  getFilterState,
  setFilterPresets as saveFilterPresets,
  setFilterState as saveFilterState,
} from '@/lib/api'

/** Panneau de filtres global (M6-1, PRD §11), perimetre phase 1 : dates +
 * profil Hero (reutilise `hero_profiles`, M3-5, rien de nouveau ici) +
 * presets nommes. Les 14 autres dimensions du PRD §11 restent hors
 * perimetre (voir docs/BACKLOG.md). */

export type DateRangePreset = 'today' | 'last7' | 'last30' | 'thisMonth' | 'thisYear' | 'allTime' | 'custom'

export interface DateRangeState {
  preset: DateRangePreset
  /** Uniquement quand `preset === 'custom'` (epoch ms). */
  customSinceMs?: number
  customUntilMs?: number
}

export interface FilterPreset {
  id: string
  name: string
  range: DateRangeState
}

export const FILTER_SCREENS = ['home', 'results', 'tournaments', 'hands'] as const
export type FilterScreen = (typeof FILTER_SCREENS)[number]

const DEFAULT_RANGE: DateRangeState = { preset: 'last30' }

const DAY_MS = 86_400_000

/** Resout un preset en bornes concretes (epoch ms), a partir de l'horloge
 * locale (le backend ne connait que l'UTC, R-MONEY — meme convention que
 * `now_ms` dans M6-2). `sinceMs`/`untilMs` a `null` = periode "tout" (pas
 * de borne, transmis tel quel au backend). */
export function resolveDateRange(
  range: DateRangeState,
  nowMs: number,
): { sinceMs: number | null; untilMs: number | null } {
  switch (range.preset) {
    case 'today': {
      const start = new Date(nowMs)
      start.setHours(0, 0, 0, 0)
      return { sinceMs: start.getTime(), untilMs: nowMs }
    }
    case 'last7':
      return { sinceMs: nowMs - 7 * DAY_MS, untilMs: nowMs }
    case 'last30':
      return { sinceMs: nowMs - 30 * DAY_MS, untilMs: nowMs }
    case 'thisMonth': {
      const start = new Date(nowMs)
      start.setDate(1)
      start.setHours(0, 0, 0, 0)
      return { sinceMs: start.getTime(), untilMs: nowMs }
    }
    case 'thisYear': {
      const start = new Date(nowMs)
      start.setMonth(0, 1)
      start.setHours(0, 0, 0, 0)
      return { sinceMs: start.getTime(), untilMs: nowMs }
    }
    case 'allTime':
      return { sinceMs: null, untilMs: null }
    case 'custom':
      return { sinceMs: range.customSinceMs ?? null, untilMs: range.customUntilMs ?? null }
    default:
      return { sinceMs: null, untilMs: null }
  }
}

interface FilterStoreState {
  perScreen: Record<FilterScreen, DateRangeState>
  presets: FilterPreset[]
  hydrated: boolean
  setRange: (screen: FilterScreen, range: DateRangeState) => void
  addPreset: (preset: FilterPreset) => void
  removePreset: (id: string) => void
  hydrate: () => Promise<void>
}

function isDateRangeState(value: unknown): value is DateRangeState {
  return typeof value === 'object' && value !== null && 'preset' in value
}

export const useFilterStore = create<FilterStoreState>((set, get) => ({
  perScreen: {
    home: DEFAULT_RANGE,
    results: DEFAULT_RANGE,
    tournaments: DEFAULT_RANGE,
    hands: DEFAULT_RANGE,
  },
  presets: [],
  hydrated: false,

  setRange: (screen, range) => {
    set((s) => ({ perScreen: { ...s.perScreen, [screen]: range } }))
    void saveFilterState(screen, JSON.stringify(range))
  },

  addPreset: (preset) => {
    const presets = [...get().presets, preset]
    set({ presets })
    void saveFilterPresets(JSON.stringify(presets))
  },

  removePreset: (id) => {
    const presets = get().presets.filter((p) => p.id !== id)
    set({ presets })
    void saveFilterPresets(JSON.stringify(presets))
  },

  hydrate: async () => {
    const perScreen = { ...get().perScreen }
    await Promise.all(
      FILTER_SCREENS.map(async (screen) => {
        try {
          const json = await getFilterState(screen)
          if (!json) return
          const parsed: unknown = JSON.parse(json)
          if (isDateRangeState(parsed)) perScreen[screen] = parsed
        } catch {
          // Etat de confort UI seulement : un JSON invalide/manquant garde
          // simplement le preset par defaut, pas d'erreur bloquante.
        }
      }),
    )

    let presets: FilterPreset[] = []
    try {
      const json = await getFilterPresets()
      if (json) {
        const parsed: unknown = JSON.parse(json)
        if (Array.isArray(parsed)) presets = parsed as FilterPreset[]
      }
    } catch {
      // Idem : reste a la liste vide en cas de probleme.
    }

    set({ perScreen, presets, hydrated: true })
  },
}))
