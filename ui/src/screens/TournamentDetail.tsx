import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { TournamentStackChart } from '@/components/TournamentStackChart'
import { getTournamentDetail } from '@/lib/api'
import {
  formatBb,
  formatCents,
  formatCount,
  formatDate,
  formatDurationMs,
  signedValueClassName,
} from '@/lib/format'

function formatLabel(
  isKo: boolean,
  isFreeroll: boolean,
  t: (key: string) => string,
): string {
  if (isFreeroll) return t('tournaments.format.freeroll')
  return isKo ? t('tournaments.format.ko') : t('tournaments.format.regular')
}

function statusLabel(status: string, t: (key: string) => string): string {
  switch (status) {
    case 'COMPLETE':
      return t('tournaments.status.complete')
    case 'INCOMPLETE':
      return t('tournaments.status.incomplete')
    default:
      return t('tournaments.status.provisional')
  }
}

/** Detail d'un tournoi (M6-4, phase 1, PRD §13.3) : en-tete (buy-in, place,
 * gains, profit), chronologie du tapis (`TournamentStackChart`), all-in,
 * liste des mains, adversaires rencontres — en lecture seule (edition des
 * metadonnees et badge de classification des adversaires differes, voir
 * docs/BACKLOG.md). */
export function TournamentDetail() {
  const { t } = useTranslation()
  const { tournamentId } = useParams({ from: '/tournaments/$tournamentId' })

  const detailQuery = useQuery({
    queryKey: ['tournaments', 'detail', tournamentId],
    queryFn: () => getTournamentDetail(Number(tournamentId)),
  })

  const detail = detailQuery.data

  if (detailQuery.isLoading) {
    return <div className="p-6 text-xs text-[var(--color-text-secondary)]">{t('tournaments.detail.loading')}</div>
  }

  if (!detail) {
    return (
      <div className="flex flex-col gap-3 p-6">
        <p className="text-xs text-[var(--color-text-secondary)]">{t('tournaments.detail.notFound')}</p>
        <Link to="/tournaments" className="text-xs underline">
          {t('tournaments.detail.backToList')}
        </Link>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-4 p-6">
      <Link to="/tournaments" className="text-xs underline">
        {t('tournaments.detail.backToList')}
      </Link>

      <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
        <h1 className="mb-2 text-sm font-medium">{detail.name}</h1>
        <div className="grid grid-cols-2 gap-x-6 gap-y-1 text-xs [font-variant-numeric:tabular-nums] sm:grid-cols-4">
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.date')}</div>
            <div>{formatDate(detail.started_at)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.format')}</div>
            <div>{formatLabel(detail.is_ko, detail.is_freeroll, t)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.buyin')}</div>
            <div>
              {formatCents(detail.buyin_prize_cents + detail.buyin_bounty_cents + detail.buyin_fee_cents)}
            </div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.status')}</div>
            <div>{statusLabel(detail.status, t)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.entries')}</div>
            <div>{formatCount(detail.entries_count)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.entrants')}</div>
            <div>{formatCount(detail.entrants)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.place')}</div>
            <div>{formatCount(detail.finish_position)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.duration')}</div>
            <div>{formatDurationMs(detail.played_seconds * 1000)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.prize')}</div>
            <div>{formatCents(detail.prize_cents)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.bounty')}</div>
            <div>{formatCents(detail.bounty_cents)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.ticketsWon')}</div>
            <div>{formatCents(detail.tickets_won_value_cents)}</div>
          </div>
          <div>
            <div className="text-[var(--color-text-secondary)]">{t('tournaments.columns.profit')}</div>
            <div className={signedValueClassName(detail.profit_cents)}>
              {formatCents(detail.profit_cents)}
            </div>
          </div>
        </div>
      </div>

      <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
        <h2 className="mb-2 text-sm font-medium">{t('tournaments.detail.stackTitle')}</h2>
        <TournamentStackChart points={detail.stack_curve} />
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <h2 className="mb-2 text-sm font-medium">{t('tournaments.detail.allInsTitle')}</h2>
          {detail.all_ins.length > 0 ? (
            <table className="w-full text-xs [font-variant-numeric:tabular-nums]">
              <thead>
                <tr className="text-left text-[var(--color-text-secondary)]">
                  <th className="py-1 pr-3">{t('tournaments.columns.date')}</th>
                  <th className="py-1 pr-3 text-right">{t('tournaments.detail.level')}</th>
                  <th className="py-1 text-right">{t('tournaments.columns.evDiff')}</th>
                </tr>
              </thead>
              <tbody>
                {detail.all_ins.map((row) => (
                  <tr key={row.hand_id} className="border-t border-[var(--color-border)]">
                    <td className="py-1 pr-3">{formatDate(row.played_at)}</td>
                    <td className="py-1 pr-3 text-right">{formatCount(row.level)}</td>
                    <td className="py-1 text-right">{formatBb(row.allin_ev_diff_bb)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <p className="text-xs text-[var(--color-text-secondary)]">{t('tournaments.detail.allInsEmpty')}</p>
          )}
        </div>

        <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
          <h2 className="mb-2 text-sm font-medium">{t('tournaments.detail.opponentsTitle')}</h2>
          {detail.opponents.length > 0 ? (
            <table className="w-full text-xs [font-variant-numeric:tabular-nums]">
              <thead>
                <tr className="text-left text-[var(--color-text-secondary)]">
                  <th className="py-1 pr-3">{t('tournaments.detail.opponentName')}</th>
                  <th className="py-1 text-right">{t('tournaments.detail.handsTogether')}</th>
                </tr>
              </thead>
              <tbody>
                {detail.opponents.map((row) => (
                  <tr key={row.player_id} className="border-t border-[var(--color-border)]">
                    <td className="py-1 pr-3">{row.screen_name}</td>
                    <td className="py-1 text-right">{formatCount(row.hands_together)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <p className="text-xs text-[var(--color-text-secondary)]">
              {t('tournaments.detail.opponentsEmpty')}
            </p>
          )}
        </div>
      </div>

      <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
        <h2 className="mb-2 text-sm font-medium">{t('tournaments.detail.handsTitle')}</h2>
        {detail.stack_curve.length > 0 ? (
          <table className="w-full text-xs [font-variant-numeric:tabular-nums]">
            <thead>
              <tr className="text-left text-[var(--color-text-secondary)]">
                <th className="py-1 pr-3">{t('tournaments.columns.date')}</th>
                <th className="py-1 pr-3 text-right">{t('tournaments.detail.level')}</th>
                <th className="py-1 pr-3 text-right">{t('tournaments.detail.stack')}</th>
                <th className="py-1 text-right">{t('tournaments.detail.handResult')}</th>
              </tr>
            </thead>
            <tbody>
              {detail.stack_curve.map((row) => (
                <tr key={row.hand_id} className="border-t border-[var(--color-border)]">
                  <td className="py-1 pr-3">{formatDate(row.played_at)}</td>
                  <td className="py-1 pr-3 text-right">{formatCount(row.level)}</td>
                  <td className="py-1 pr-3 text-right">{formatBb(row.stack_bb)}</td>
                  <td className={`py-1 text-right ${signedValueClassName(row.net_bb ?? 0)}`}>
                    {formatBb(row.net_bb)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className="text-xs text-[var(--color-text-secondary)]">{t('tournaments.detail.handsEmpty')}</p>
        )}
      </div>
    </div>
  )
}
