import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import type { PeriodKpis } from '@/bindings'
import { ProfitCurveChart } from '@/components/ProfitCurveChart'
import { getHomeSnapshot, onHandsNew } from '@/lib/api'
import {
  formatBb,
  formatCents,
  formatCentsPerHour,
  formatCount,
  formatDurationMs,
  formatPercent,
  signedValueClassName,
} from '@/lib/format'

const HOME_QUERY_KEY = ['home', 'snapshot']

function emptyPeriod(): PeriodKpis {
  return {
    tournaments_count: 0,
    cost_cents: 0,
    fees_cents: 0,
    profit_cents: 0,
    roi: null,
    itm_rate: null,
    abi_cents: null,
    allin_ev_diff_bb: 0,
    dollars_per_hour_cents: null,
  }
}

interface KpiCardProps {
  label: string
  valueText: string
  previousValueText: string
  signedValue?: number
}

function KpiCard({ label, valueText, previousValueText, signedValue }: KpiCardProps) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-1 rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
      <span className="text-xs text-[var(--color-text-secondary)]">{label}</span>
      <span
        className={`text-lg [font-variant-numeric:tabular-nums] ${
          signedValue == null ? '' : signedValueClassName(signedValue)
        }`}
      >
        {valueText}
      </span>
      <span className="text-xs text-[var(--color-text-secondary)] [font-variant-numeric:tabular-nums]">
        {t('home.previousPeriod', { value: previousValueText })}
      </span>
    </div>
  )
}

/** Ecran Accueil (M6-2, PRD §13.1) : cartes KPI (30 derniers jours vs les 30
 * jours precedents, pas de filtre de date reglable tant que M6-1 n'existe
 * pas), graphe G1 en format reduit, derniere session, etat de l'import.
 * Sparklines et "top 3 leaks" differes (perimetre convenu avec Frederic,
 * 29/09 : historique par periode et moteur de benchmarks/leaks, M7-2, pas
 * encore construits). */
export function Home() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()

  const snapshotQuery = useQuery({
    queryKey: HOME_QUERY_KEY,
    queryFn: () => getHomeSnapshot(Date.now()),
  })

  useEffect(() => {
    // Hors contexte Tauri (tests, apercu navigateur), `listen` rejette : on
    // degrade silencieusement, meme convention que AppShell.tsx.
    const unlisten = onHandsNew(() => {
      void queryClient.invalidateQueries({ queryKey: HOME_QUERY_KEY })
    }).catch(() => undefined)
    return () => {
      void unlisten.then((fn) => fn?.())
    }
  }, [queryClient])

  const snapshot = snapshotQuery.data
  const current = snapshot?.current_period ?? emptyPeriod()
  const previous = snapshot?.previous_period ?? emptyPeriod()

  const kpis: KpiCardProps[] = [
    {
      label: t('home.kpis.profit'),
      valueText: formatCents(current.profit_cents),
      previousValueText: formatCents(previous.profit_cents),
      signedValue: current.profit_cents,
    },
    {
      label: t('home.kpis.roi'),
      valueText: formatPercent(current.roi),
      previousValueText: formatPercent(previous.roi),
      signedValue: current.roi ?? undefined,
    },
    {
      label: t('home.kpis.itm'),
      valueText: formatPercent(current.itm_rate),
      previousValueText: formatPercent(previous.itm_rate),
    },
    {
      label: t('home.kpis.abi'),
      valueText: formatCents(current.abi_cents),
      previousValueText: formatCents(previous.abi_cents),
    },
    {
      label: t('home.kpis.tournamentsCount'),
      valueText: formatCount(current.tournaments_count),
      previousValueText: formatCount(previous.tournaments_count),
    },
    {
      label: t('home.kpis.fees'),
      valueText: formatCents(current.fees_cents),
      previousValueText: formatCents(previous.fees_cents),
    },
    {
      label: t('home.kpis.allinEvDiff'),
      valueText: formatBb(current.allin_ev_diff_bb),
      previousValueText: formatBb(previous.allin_ev_diff_bb),
      signedValue: current.allin_ev_diff_bb,
    },
    {
      label: t('home.kpis.dollarsPerHour'),
      valueText: formatCentsPerHour(current.dollars_per_hour_cents),
      previousValueText: formatCentsPerHour(previous.dollars_per_hour_cents),
      signedValue: current.dollars_per_hour_cents ?? undefined,
    },
  ]

  const lastSession = snapshot?.last_session
  const importStatus = snapshot?.import_status

  return (
    <div className="flex flex-col gap-6 p-6">
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {kpis.map((kpi) => (
          <KpiCard key={kpi.label} {...kpi} />
        ))}
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3 lg:col-span-2">
          <h2 className="mb-2 text-sm font-medium">{t('home.profitCurve.title')}</h2>
          <ProfitCurveChart points={snapshot?.profit_curve ?? []} />
        </div>

        <div className="flex flex-col gap-4">
          <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
            <h2 className="mb-2 text-sm font-medium">{t('home.lastSession.title')}</h2>
            {lastSession ? (
              <ul className="flex flex-col gap-1 text-xs text-[var(--color-text-secondary)] [font-variant-numeric:tabular-nums]">
                <li>
                  {t('home.lastSession.duration', {
                    value: formatDurationMs(lastSession.ended_at - lastSession.started_at),
                  })}
                </li>
                <li>
                  {t('home.lastSession.tournamentsCount', { count: lastSession.tournaments })}
                </li>
                <li>{t('home.lastSession.handsCount', { count: lastSession.hands })}</li>
                <li className={signedValueClassName(lastSession.profit_cents)}>
                  {t('home.lastSession.profit', { value: formatCents(lastSession.profit_cents) })}
                </li>
                {lastSession.best_tournament_name != null && (
                  <li>
                    {t('home.lastSession.best', {
                      name: lastSession.best_tournament_name,
                      value: formatCents(lastSession.best_tournament_profit_cents),
                    })}
                  </li>
                )}
                {lastSession.worst_tournament_name != null && (
                  <li>
                    {t('home.lastSession.worst', {
                      name: lastSession.worst_tournament_name,
                      value: formatCents(lastSession.worst_tournament_profit_cents),
                    })}
                  </li>
                )}
              </ul>
            ) : (
              <p className="text-xs text-[var(--color-text-secondary)]">
                {t('home.lastSession.none')}
              </p>
            )}
          </div>

          <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
            <h2 className="mb-2 text-sm font-medium">{t('home.importStatus.title')}</h2>
            <ul className="flex flex-col gap-1 text-xs text-[var(--color-text-secondary)] [font-variant-numeric:tabular-nums]">
              <li>
                {t('home.importStatus.watchedRoots', {
                  count: importStatus?.watched_roots_count ?? 0,
                })}
              </li>
              <li>
                {t('home.importStatus.totalHands', { count: importStatus?.total_hands ?? 0 })}
              </li>
              <li>
                {t('home.importStatus.unresolvedErrors', {
                  count: importStatus?.unresolved_errors_count ?? 0,
                })}
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  )
}
