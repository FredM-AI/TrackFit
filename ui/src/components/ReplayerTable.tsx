import { useTranslation } from 'react-i18next'
import type { ReplaySeatStatePayload } from '@/bindings'
import { formatCount } from '@/lib/format'

interface ReplayerTableProps {
  tableMaxSeats: number
  buttonSeat: number
  seats: ReplaySeatStatePayload[]
  board: string[]
  potTotal: number
}

/** Position en pourcentage d'un siege autour d'une ellipse (PRD §13.7 : pas
 * de bibliotheque graphique, disposition trigonometrique simple en CSS,
 * meme esprit "aucune dependance non justifiee" que le reste du projet). Le
 * siege garde toujours la meme position visuelle d'une main a l'autre
 * (calculee sur `tableMaxSeats`, pas sur le nombre de joueurs encore en
 * lice) pour ne pas "sauter" pendant le rejeu. */
function seatPosition(seat: number, tableMaxSeats: number): { left: string; top: string } {
  const angle = -Math.PI / 2 + (2 * Math.PI * (seat - 1)) / Math.max(tableMaxSeats, 1)
  const left = 50 + 43 * Math.cos(angle)
  const top = 50 + 40 * Math.sin(angle)
  return { left: `${left}%`, top: `${top}%` }
}

function CardBadge({ card }: { card: string }) {
  return (
    <span className="rounded border border-[var(--color-border)] bg-[var(--color-surface-3)] px-1 text-[10px] font-semibold [font-variant-numeric:tabular-nums]">
      {card}
    </span>
  )
}

/** Table monochrome ovale (PRD §13.7) : sieges, tapis, cartes connues,
 * board et pot courant. Purement presentation — l'etat (quel pas courant)
 * vient du parent. */
export function ReplayerTable({ tableMaxSeats, buttonSeat, seats, board, potTotal }: ReplayerTableProps) {
  const { t } = useTranslation()

  return (
    <div className="relative mx-auto aspect-[2/1] w-full max-w-2xl rounded-[50%] border border-[var(--color-border)] bg-[var(--color-surface-2)]">
      <div className="absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-1">
        <div className="flex gap-1">
          {board.length > 0 ? (
            board.map((card, i) => <CardBadge key={`${card}-${i}`} card={card} />)
          ) : (
            <span className="text-[10px] text-[var(--color-text-secondary)]">
              {t('replayer.table.noBoard')}
            </span>
          )}
        </div>
        <div className="text-[10px] text-[var(--color-text-secondary)] [font-variant-numeric:tabular-nums]">
          {t('replayer.table.pot', { value: formatCount(potTotal) })}
        </div>
      </div>

      {seats.map((seat) => {
        const pos = seatPosition(seat.seat, tableMaxSeats)
        return (
          <div
            key={seat.pseudo}
            style={pos}
            className={`absolute flex w-24 -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-0.5 rounded border bg-[var(--color-bg)] p-1 text-[10px] [font-variant-numeric:tabular-nums] ${
              seat.folded
                ? 'border-[var(--color-border)] text-[var(--color-text-disabled)] opacity-60'
                : seat.is_hero
                  ? 'border-[var(--color-accent)] text-[var(--color-text-primary)]'
                  : 'border-[var(--color-border)] text-[var(--color-text-primary)]'
            }`}
          >
            {seat.seat === buttonSeat && (
              <span className="rounded-full border border-[var(--color-accent)] px-1 text-[9px] leading-tight">
                {t('replayer.table.button')}
              </span>
            )}
            <span className="w-full truncate text-center font-medium">{seat.pseudo}</span>
            <span>{formatCount(seat.stack)}</span>
            {seat.all_in && (
              <span className="text-[9px] text-[var(--color-text-secondary)]">
                {t('replayer.table.allIn')}
              </span>
            )}
            {seat.cards && (
              <div className="flex gap-0.5">
                {seat.cards.map((card, i) => (
                  <CardBadge key={`${card}-${i}`} card={card} />
                ))}
              </div>
            )}
          </div>
        )
      })}
    </div>
  )
}
