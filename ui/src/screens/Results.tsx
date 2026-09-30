import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import type {
  DayOfWeekPivotRowPayload,
  HourPivotRowPayload,
  MonthPivotRowPayload,
  PivotKpisPayload,
  PivotRowPayload,
  SpeedPivotRowPayload,
} from '@/bindings'
import { ChipCurveChart } from '@/components/ChipCurveChart'
import { FinishDistributionChart } from '@/components/FinishDistributionChart'
import { ProfitCurveChart } from '@/components/ProfitCurveChart'
import { RoiByBuyinChart } from '@/components/RoiByBuyinChart'
import { VolumeChart } from '@/components/VolumeChart'
import { getResultsSnapshot } from '@/lib/api'
import { formatCents, formatCount, formatPercent } from '@/lib/format'

const RESULTS_QUERY_KEY = ['results', 'snapshot']

function downloadCsv(csv: string, filename: string) {
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}

type PivotDimension = 'buyinKo' | 'speed' | 'dayOfWeek' | 'hour' | 'month'

interface PivotDisplayRow {
  key: string
  label: string
  kpis: PivotKpisPayload
}

function buyinKoRows(rows: PivotRowPayload[], t: (key: string) => string): PivotDisplayRow[] {
  return rows.map((row) => ({
    key: `${row.buyin_min_cents}-${row.is_ko}`,
    label: `${formatCents(row.buyin_min_cents)}${row.buyin_max_cents == null ? '+' : `–${formatCents(row.buyin_max_cents)}`} · ${row.is_ko ? t('results.pivot.ko') : t('results.pivot.nonKo')}`,
    kpis: {
      tournaments_count: row.tournaments_count,
      cost_cents: row.cost_cents,
      fees_cents: row.fees_cents,
      profit_cents: row.profit_cents,
      roi: row.roi,
      itm_rate: row.itm_rate,
      abi_cents: row.abi_cents,
    },
  }))
}

function speedLabel(speed: string | null, t: (key: string) => string): string {
  switch (speed) {
    case 'turbo':
      return t('results.pivot.speedTurbo')
    case 'semiturbo':
      return t('results.pivot.speedSemiturbo')
    case 'normal':
      return t('results.pivot.speedNormal')
    default:
      return speed ?? t('results.pivot.speedUnknown')
  }
}

function speedRows(rows: SpeedPivotRowPayload[], t: (key: string) => string): PivotDisplayRow[] {
  return rows.map((row) => ({
    key: row.speed ?? 'unknown',
    label: speedLabel(row.speed, t),
    kpis: row.kpis,
  }))
}

function dayOfWeekRows(rows: DayOfWeekPivotRowPayload[], t: (key: string) => string): PivotDisplayRow[] {
  return rows.map((row) => ({
    key: `${row.weekday}`,
    label: t(`results.pivot.weekday.${row.weekday}`),
    kpis: row.kpis,
  }))
}

function hourRows(rows: HourPivotRowPayload[]): PivotDisplayRow[] {
  return rows.map((row) => ({
    key: `${row.hour}`,
    label: `${row.hour}h`,
    kpis: row.kpis,
  }))
}

function monthRows(rows: MonthPivotRowPayload[], t: (key: string) => string): PivotDisplayRow[] {
  return rows.map((row) => ({
    key: `${row.month}`,
    label: t(`results.pivot.month.${row.month}`),
    kpis: row.kpis,
  }))
}

/** Ecran Resultats (M6-3, PRD §9.2/§13.2) : G1 (repris de M6-2), G2 (reel vs
 * EV all-in), G3 (ROI par buy-in), G6 (volume par jour), G5 (distribution
 * des places, sans la mise en evidence de "la bulle" — bloquee, voir
 * docs/BACKLOG.md), pivot complet (buy-in x format, vitesse, jour de
 * semaine, heure, mois) + export CSV. G4 (ROI par format complet) reste
 * bloque en permanence (docs/BACKLOG.md). */
export function Results() {
  const { t } = useTranslation()
  const [pivotDimension, setPivotDimension] = useState<PivotDimension>('buyinKo')

  const snapshotQuery = useQuery({
    queryKey: RESULTS_QUERY_KEY,
    queryFn: getResultsSnapshot,
  })

  const snapshot = snapshotQuery.data

  const pivotRows = useMemo<PivotDisplayRow[]>(() => {
    if (!snapshot) return []
    switch (pivotDimension) {
      case 'buyinKo':
        return buyinKoRows(snapshot.pivot, t)
      case 'speed':
        return speedRows(snapshot.pivot_speed, t)
      case 'dayOfWeek':
        return dayOfWeekRows(snapshot.pivot_day_of_week, t)
      case 'hour':
        return hourRows(snapshot.pivot_hour)
      case 'month':
        return monthRows(snapshot.pivot_month, t)
      default:
        return []
    }
  }, [snapshot, pivotDimension, t])

  const pivotCsv = useMemo(() => {
    if (!snapshot) return ''
    switch (pivotDimension) {
      case 'buyinKo':
        return snapshot.pivot_csv
      case 'speed':
        return snapshot.pivot_speed_csv
      case 'dayOfWeek':
        return snapshot.pivot_day_of_week_csv
      case 'hour':
        return snapshot.pivot_hour_csv
      case 'month':
        return snapshot.pivot_month_csv
      default:
        return ''
    }
  }, [snapshot, pivotDimension])

  const dimensionButtons: { id: PivotDimension; labelKey: string }[] = [
    { id: 'buyinKo', labelKey: 'results.pivot.dimension.buyinKo' },
    { id: 'speed', labelKey: 'results.pivot.dimension.speed' },
    { id: 'dayOfWeek', labelKey: 'results.pivot.dimension.dayOfWeek' },
    { id: 'hour', labelKey: 'results.pivot.dimension.hour' },
    { id: 'month', labelKey: 'results.pivot.dimension.month' },
  ]

  return (
    <div className="flex flex-col gap-6 p-6">
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <h2 className="mb-2 text-sm font-medium">{t('results.profitCurveTitle')}</h2>
          <ProfitCurveChart points={snapshot?.profit_curve ?? []} />
        </div>

        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <h2 className="mb-2 text-sm font-medium">{t('results.chipCurve.title')}</h2>
          <ChipCurveChart points={snapshot?.chip_curve ?? []} />
        </div>
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <h2 className="mb-2 text-sm font-medium">{t('results.roiByBuyin.title')}</h2>
          <RoiByBuyinChart rows={snapshot?.roi_by_buyin ?? []} />
        </div>

        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <h2 className="mb-2 text-sm font-medium">{t('results.finishDistribution.title')}</h2>
          <FinishDistributionChart buckets={snapshot?.finish_percentile_distribution ?? []} />
        </div>
      </div>

      <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
        <h2 className="mb-2 text-sm font-medium">{t('results.volume.title')}</h2>
        <VolumeChart points={snapshot?.volume ?? []} />
      </div>

      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <p className="text-xs text-[var(--color-text-secondary)]">
            {t('results.additionalKpis.avgFinishPosition')}
          </p>
          <p className="mt-1 text-lg font-semibold [font-variant-numeric:tabular-nums]">
            {snapshot?.additional_kpis.avg_finish_position == null
              ? '—'
              : formatCount(Math.round(snapshot.additional_kpis.avg_finish_position))}
          </p>
        </div>
        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <p className="text-xs text-[var(--color-text-secondary)]">
            {t('results.additionalKpis.finalTableRate')}
          </p>
          <p className="mt-1 text-lg font-semibold [font-variant-numeric:tabular-nums]">
            {formatPercent(snapshot?.additional_kpis.final_table_rate)}
          </p>
        </div>
        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <p className="text-xs text-[var(--color-text-secondary)]">{t('results.additionalKpis.bestWin')}</p>
          <p className="mt-1 text-lg font-semibold [font-variant-numeric:tabular-nums]">
            {formatCents(snapshot?.additional_kpis.best_win_cents ?? 0)}
          </p>
        </div>
        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <p className="text-xs text-[var(--color-text-secondary)]">
            {t('results.additionalKpis.biggestTournament')}
          </p>
          <p className="mt-1 text-lg font-semibold [font-variant-numeric:tabular-nums]">
            {formatCount(snapshot?.additional_kpis.biggest_tournament_entrants)}
          </p>
        </div>
      </div>

      <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
        <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <h2 className="text-sm font-medium">{t('results.pivot.title')}</h2>
            <div className="flex gap-1">
              {dimensionButtons.map((dimension) => (
                <button
                  key={dimension.id}
                  type="button"
                  onClick={() => setPivotDimension(dimension.id)}
                  className={`rounded px-2 py-1 text-xs ${
                    pivotDimension === dimension.id
                      ? 'bg-[var(--color-accent)] text-[var(--color-bg)]'
                      : 'bg-[var(--color-surface-1)] text-[var(--color-text-secondary)]'
                  }`}
                >
                  {t(dimension.labelKey)}
                </button>
              ))}
            </div>
          </div>
          <button
            type="button"
            disabled={pivotRows.length === 0}
            onClick={() => downloadCsv(pivotCsv, `resultats-${pivotDimension}.csv`)}
            className="rounded bg-[var(--color-accent)] px-3 py-1 text-xs text-[var(--color-bg)] disabled:opacity-40"
          >
            {t('results.pivot.exportCsv')}
          </button>
        </div>

        {pivotRows.length > 0 ? (
          <table className="w-full text-xs [font-variant-numeric:tabular-nums]">
            <thead>
              <tr className="text-left text-[var(--color-text-secondary)]">
                <th className="py-1 pr-3">{t(dimensionButtons.find((d) => d.id === pivotDimension)?.labelKey ?? '')}</th>
                <th className="py-1 pr-3 text-right">{t('results.pivot.tournamentsColumn')}</th>
                <th className="py-1 pr-3 text-right">{t('results.pivot.costColumn')}</th>
                <th className="py-1 pr-3 text-right">{t('results.pivot.feesColumn')}</th>
                <th className="py-1 pr-3 text-right">{t('results.pivot.profitColumn')}</th>
                <th className="py-1 pr-3 text-right">{t('results.pivot.roiColumn')}</th>
                <th className="py-1 pr-3 text-right">{t('results.pivot.itmColumn')}</th>
                <th className="py-1 text-right">{t('results.pivot.abiColumn')}</th>
              </tr>
            </thead>
            <tbody>
              {pivotRows.map((row) => (
                <tr key={row.key} className="border-t border-[var(--color-border)]">
                  <td className="py-1 pr-3">{row.label}</td>
                  <td className="py-1 pr-3 text-right">{formatCount(row.kpis.tournaments_count)}</td>
                  <td className="py-1 pr-3 text-right">{formatCents(row.kpis.cost_cents)}</td>
                  <td className="py-1 pr-3 text-right">{formatCents(row.kpis.fees_cents)}</td>
                  <td className="py-1 pr-3 text-right">{formatCents(row.kpis.profit_cents)}</td>
                  <td className="py-1 pr-3 text-right">{formatPercent(row.kpis.roi)}</td>
                  <td className="py-1 pr-3 text-right">{formatPercent(row.kpis.itm_rate)}</td>
                  <td className="py-1 text-right">{formatCents(row.kpis.abi_cents)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className="text-xs text-[var(--color-text-secondary)]">{t('results.pivot.empty')}</p>
        )}
      </div>
    </div>
  )
}
