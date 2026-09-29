import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { ChipCurveChart } from '@/components/ChipCurveChart'
import { ProfitCurveChart } from '@/components/ProfitCurveChart'
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

/** Ecran Resultats (M6-3, PRD §9.2/§13.2), phase 1 validee avec Frederic :
 * G1 (repris de M6-2), G2 (reel vs EV all-in), G6 (volume par jour), pivot
 * reduit buy-in x KO/non-KO + export CSV. G3-G5 et le pivot complet
 * (format/vitesse/jour/heure/mois) sont differes (voir docs/BACKLOG.md). */
export function Results() {
  const { t } = useTranslation()

  const snapshotQuery = useQuery({
    queryKey: RESULTS_QUERY_KEY,
    queryFn: getResultsSnapshot,
  })

  const snapshot = snapshotQuery.data

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

      <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
        <h2 className="mb-2 text-sm font-medium">{t('results.volume.title')}</h2>
        <VolumeChart points={snapshot?.volume ?? []} />
      </div>

      <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
        <div className="mb-2 flex items-center justify-between">
          <h2 className="text-sm font-medium">{t('results.pivot.title')}</h2>
          <button
            type="button"
            disabled={!snapshot || snapshot.pivot.length === 0}
            onClick={() => {
              if (snapshot) downloadCsv(snapshot.pivot_csv, 'resultats.csv')
            }}
            className="rounded bg-[var(--color-accent)] px-3 py-1 text-xs text-[var(--color-bg)] disabled:opacity-40"
          >
            {t('results.pivot.exportCsv')}
          </button>
        </div>

        {snapshot && snapshot.pivot.length > 0 ? (
          <table className="w-full text-xs [font-variant-numeric:tabular-nums]">
            <thead>
              <tr className="text-left text-[var(--color-text-secondary)]">
                <th className="py-1 pr-3">{t('results.pivot.buyinColumn')}</th>
                <th className="py-1 pr-3">{t('results.pivot.formatColumn')}</th>
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
              {snapshot.pivot.map((row) => (
                <tr
                  key={`${row.buyin_min_cents}-${row.is_ko}`}
                  className="border-t border-[var(--color-border)]"
                >
                  <td className="py-1 pr-3">
                    {formatCents(row.buyin_min_cents)}
                    {row.buyin_max_cents != null ? `–${formatCents(row.buyin_max_cents)}` : '+'}
                  </td>
                  <td className="py-1 pr-3">
                    {row.is_ko ? t('results.pivot.ko') : t('results.pivot.nonKo')}
                  </td>
                  <td className="py-1 pr-3 text-right">{formatCount(row.tournaments_count)}</td>
                  <td className="py-1 pr-3 text-right">{formatCents(row.cost_cents)}</td>
                  <td className="py-1 pr-3 text-right">{formatCents(row.fees_cents)}</td>
                  <td className="py-1 pr-3 text-right">{formatCents(row.profit_cents)}</td>
                  <td className="py-1 pr-3 text-right">{formatPercent(row.roi)}</td>
                  <td className="py-1 pr-3 text-right">{formatPercent(row.itm_rate)}</td>
                  <td className="py-1 text-right">{formatCents(row.abi_cents)}</td>
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
