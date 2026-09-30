import { Link, Outlet, useRouterState } from '@tanstack/react-router'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { FilterBar } from '@/components/FilterBar'
import { LanguageSwitcher } from '@/components/LanguageSwitcher'
import { TrayFirstHideNotice } from '@/components/TrayFirstHideNotice'
import { navItems } from '@/app/nav'
import { getStatusSnapshot, onHandsNew } from '@/lib/api'

const STATUS_QUERY_KEY = ['status', 'snapshot']

/** Minuit local en epoch ms : le backend ne connait que l'UTC (R-MONEY), le
 * decoupage "aujourd'hui" se fait donc cote UI. */
function startOfTodayMs(): number {
  const now = new Date()
  now.setHours(0, 0, 0, 0)
  return now.getTime()
}

export function AppShell() {
  const { t } = useTranslation()
  const pathname = useRouterState({ select: (state) => state.location.pathname })
  const queryClient = useQueryClient()

  const statusQuery = useQuery({
    queryKey: STATUS_QUERY_KEY,
    queryFn: () => getStatusSnapshot(startOfTodayMs()),
    // Reste correct si l'app tourne a cheval sur minuit ; pas critique pour
    // une barre d'etat, un refetch une fois par heure suffit entre deux
    // `hands://new`.
    refetchInterval: 60 * 60 * 1000,
  })

  useEffect(() => {
    // Hors contexte Tauri (tests, apercu navigateur), `listen` rejette :
    // on degrade silencieusement plutot que de planter le montage.
    const unlisten = onHandsNew(() => {
      void queryClient.invalidateQueries({ queryKey: STATUS_QUERY_KEY })
    }).catch(() => undefined)
    return () => {
      void unlisten.then((fn) => fn?.())
    }
  }, [queryClient])

  const status = statusQuery.data
  const importStatusKey =
    status?.watcher_run_state === 'active'
      ? 'statusBar.importActive'
      : status?.watcher_run_state === 'paused'
        ? 'statusBar.importPaused'
        : 'statusBar.importIdle'
  const lastHandValue =
    status?.last_hand_at == null
      ? t('statusBar.lastHandNone')
      : new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(
          new Date(status.last_hand_at),
        )

  return (
    <div className="flex h-screen bg-[var(--color-bg)] text-[var(--color-text-primary)]">
      <TrayFirstHideNotice />
      <nav
        aria-label={t('app.name')}
        className="flex w-48 flex-col gap-1 border-r border-[var(--color-border)] bg-[var(--color-surface-1)] p-3"
      >
        <div className="mb-2 px-2 text-sm font-semibold tracking-wide">{t('app.name')}</div>
        {navItems.map(({ to, labelKey, icon: Icon }) => {
          const active = pathname === to
          return (
            <Link
              key={to}
              to={to}
              className="flex items-center gap-2 rounded px-2 py-1.5 text-sm"
              style={{
                backgroundColor: active ? 'var(--color-surface-3)' : undefined,
                color: active ? 'var(--color-text-primary)' : 'var(--color-text-secondary)',
              }}
            >
              <Icon size={16} aria-hidden="true" />
              {t(labelKey)}
            </Link>
          )
        })}
      </nav>

      <div className="flex min-w-0 flex-1 flex-col">
        <header
          aria-label="filters"
          className="flex h-10 items-center border-b border-[var(--color-border)] bg-[var(--color-surface-1)] px-3"
        >
          <FilterBar />
        </header>

        <main className="flex-1 overflow-auto">
          <Outlet />
        </main>

        <footer className="flex h-8 items-center justify-between border-t border-[var(--color-border)] bg-[var(--color-surface-1)] px-3 text-xs text-[var(--color-text-secondary)]">
          <span>{t(importStatusKey)}</span>
          <div className="flex items-center gap-4">
            <span>{t('statusBar.handsToday', { count: status?.hands_today ?? 0 })}</span>
            <span>{t('statusBar.lastHand', { value: lastHandValue })}</span>
            <LanguageSwitcher />
          </div>
        </footer>
      </div>
    </div>
  )
}
