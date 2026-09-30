import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { TournamentListRowPayload } from '@/bindings'
import { getTournamentsList } from '@/lib/api'
import { formatBb, formatCents, formatCount, formatDate, formatDurationMs } from '@/lib/format'

const TOURNAMENTS_QUERY_KEY = ['tournaments', 'list']
const ROW_HEIGHT_PX = 32

function formatLabel(row: TournamentListRowPayload, t: (key: string) => string): string {
  if (row.is_freeroll) return t('tournaments.format.freeroll')
  return row.is_ko ? t('tournaments.format.ko') : t('tournaments.format.regular')
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

/** Grille CSS partagee par l'en-tete et les lignes (PRD §13.3) : colonnes
 * fixes plutot qu'un vrai `<table>`, necessaire pour virtualiser les lignes
 * (react-virtual positionne chaque ligne en `absolute`, incompatible avec
 * l'algorithme de mise en page d'un `<table>` HTML). */
const GRID_COLUMNS =
  '140px minmax(160px,1fr) 90px 110px 70px 70px 70px 60px 90px 80px 70px 90px 70px 70px 80px 100px'

interface TournamentsTableProps {
  rows: TournamentListRowPayload[]
}

function TournamentsTable({ rows }: TournamentsTableProps) {
  const { t } = useTranslation()
  const parentRef = useRef<HTMLDivElement>(null)

  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT_PX,
    overscan: 12,
  })

  const columns: { key: string; labelKey: string; right?: boolean }[] = [
    { key: 'date', labelKey: 'tournaments.columns.date' },
    { key: 'name', labelKey: 'tournaments.columns.name' },
    { key: 'format', labelKey: 'tournaments.columns.format' },
    { key: 'buyin', labelKey: 'tournaments.columns.buyin', right: true },
    { key: 'ticket', labelKey: 'tournaments.columns.ticket' },
    { key: 'entries', labelKey: 'tournaments.columns.entries', right: true },
    { key: 'entrants', labelKey: 'tournaments.columns.entrants', right: true },
    { key: 'place', labelKey: 'tournaments.columns.place', right: true },
    { key: 'prize', labelKey: 'tournaments.columns.prize', right: true },
    { key: 'bounty', labelKey: 'tournaments.columns.bounty', right: true },
    { key: 'tickets', labelKey: 'tournaments.columns.ticketsWon', right: true },
    { key: 'profit', labelKey: 'tournaments.columns.profit', right: true },
    { key: 'hands', labelKey: 'tournaments.columns.hands', right: true },
    { key: 'duration', labelKey: 'tournaments.columns.duration', right: true },
    { key: 'evDiff', labelKey: 'tournaments.columns.evDiff', right: true },
    { key: 'status', labelKey: 'tournaments.columns.status' },
  ]

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-x-auto">
      <div
        className="grid gap-2 border-b border-[var(--color-border)] px-2 py-1 text-xs text-[var(--color-text-secondary)]"
        style={{ gridTemplateColumns: GRID_COLUMNS, minWidth: 'max-content' }}
      >
        {columns.map((col) => (
          <div key={col.key} className={col.right ? 'text-right' : undefined}>
            {t(col.labelKey)}
          </div>
        ))}
      </div>

      <div ref={parentRef} className="min-h-0 flex-1 overflow-auto" style={{ minWidth: 'max-content' }}>
        <div
          style={{
            height: rowVirtualizer.getTotalSize(),
            position: 'relative',
            minWidth: 'max-content',
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualRow) => {
            const row = rows[virtualRow.index]
            if (!row) return null
            return (
              <Link
                key={row.tournament_id}
                to="/tournaments/$tournamentId"
                params={{ tournamentId: String(row.tournament_id) }}
                className="grid gap-2 border-b border-[var(--color-border)] px-2 text-xs [font-variant-numeric:tabular-nums] hover:bg-[var(--color-surface-2)]"
                style={{
                  gridTemplateColumns: GRID_COLUMNS,
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  height: ROW_HEIGHT_PX,
                  transform: `translateY(${virtualRow.start}px)`,
                  alignItems: 'center',
                }}
              >
                <div className="truncate">{formatDate(row.started_at)}</div>
                <div className="truncate">{row.name}</div>
                <div className="truncate">{formatLabel(row, t)}</div>
                <div className="text-right">
                  {formatCents(row.buyin_prize_cents + row.buyin_bounty_cents + row.buyin_fee_cents)}
                </div>
                <div>{row.paid_with_ticket ? t('tournaments.yes') : t('tournaments.no')}</div>
                <div className="text-right">{formatCount(row.entries_count)}</div>
                <div className="text-right">{formatCount(row.entrants)}</div>
                <div className="text-right">{formatCount(row.finish_position)}</div>
                <div className="text-right">{formatCents(row.prize_cents)}</div>
                <div className="text-right">{formatCents(row.bounty_cents)}</div>
                <div className="text-right">{formatCents(row.tickets_won_value_cents)}</div>
                <div className="text-right">{formatCents(row.profit_cents)}</div>
                <div className="text-right">{formatCount(row.hands_played)}</div>
                <div className="text-right">{formatDurationMs(row.played_seconds * 1000)}</div>
                <div className="text-right">{formatBb(row.ev_diff_bb)}</div>
                <div className="truncate">{statusLabel(row.status, t)}</div>
              </Link>
            )
          })}
        </div>
      </div>
    </div>
  )
}

/** Ecran Tournois (M6-4, phase 1, PRD §13.3) : liste virtualisee en lecture
 * seule (colonnes PRD), tout l'historique du profil Hero actif — pas de
 * panneau de filtres reglable tant que M6-1 n'existe pas. L'edition des
 * metadonnees (via ticket, vitesse, valeur du ticket gagne) est differee a
 * la V2 (decision de Frederic, 30/09) : cette liste est en lecture seule. */
export function Tournaments() {
  const { t } = useTranslation()

  const listQuery = useQuery({
    queryKey: TOURNAMENTS_QUERY_KEY,
    queryFn: getTournamentsList,
  })

  const rows = listQuery.data ?? []

  return (
    <div className="flex h-full flex-col gap-3 p-6">
      <h1 className="text-sm font-medium">{t('nav.tournaments')}</h1>
      {rows.length > 0 ? (
        <TournamentsTable rows={rows} />
      ) : (
        <p className="text-xs text-[var(--color-text-secondary)]">{t('tournaments.empty')}</p>
      )}
    </div>
  )
}
