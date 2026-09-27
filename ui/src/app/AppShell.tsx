import { Link, Outlet, useRouterState } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { LanguageSwitcher } from '@/components/LanguageSwitcher'
import { navItems } from '@/app/nav'

export function AppShell() {
  const { t } = useTranslation()
  const pathname = useRouterState({ select: (state) => state.location.pathname })

  return (
    <div className="flex h-screen bg-[var(--color-bg)] text-[var(--color-text-primary)]">
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
        />

        <main className="flex-1 overflow-auto">
          <Outlet />
        </main>

        <footer className="flex h-8 items-center justify-between border-t border-[var(--color-border)] bg-[var(--color-surface-1)] px-3 text-xs text-[var(--color-text-secondary)]">
          <span>{t('statusBar.importIdle')}</span>
          <div className="flex items-center gap-4">
            <span>{t('statusBar.handsToday', { count: 0 })}</span>
            <span>{t('statusBar.lastHand', { value: t('statusBar.lastHandNone') })}</span>
            <LanguageSwitcher />
          </div>
        </footer>
      </div>
    </div>
  )
}
