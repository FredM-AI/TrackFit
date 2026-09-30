import { useTranslation } from 'react-i18next'
import type { HandClassCellPayload } from '@/bindings'
import { formatBb, formatCount, formatPercent } from '@/lib/format'

/** Meme seuil que `Reports.tsx` (PRD §10.1 : "en dessous d'un seuil
 * d'echantillon (parametrable, 30 par defaut)"). */
const MIN_SAMPLE_SIZE = 30

/** Rangs dans l'ordre standard d'une grille de mains de depart : A en haut
 * a gauche, 2 en bas a droite. */
const RANKS = ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2'] as const

/** Classe de la cellule (ligne, colonne) : diagonale = paire, triangle
 * superieur (ligne < colonne, rang de ligne plus fort) = suited, triangle
 * inferieur = offsuit — convention standard d'une grille de mains de
 * depart (toujours le rang le plus fort en premier dans le libelle, meme
 * ordre que `gr_stats::compute_hand_class`). */
function cellHandClass(rowIndex: number, colIndex: number): string {
  const rowRank = RANKS[rowIndex]
  const colRank = RANKS[colIndex]
  if (rowIndex === colIndex) return `${rowRank}${colRank}`
  if (rowIndex < colIndex) return `${rowRank}${colRank}s`
  return `${colRank}${rowRank}o`
}

interface HandClassGridProps {
  cells: HandClassCellPayload[]
}

/** Grille 13×13 des mains de depart (M7-7, PRD §13.5) : frequence, VPIP/
 * PFR et resultat moyen en bb/main par `hand_class`, en niveaux de gris
 * (teinte proportionnelle a la frequence, relative au maximum de la
 * grille). Sous le seuil d'echantillon, grisee comme les autres rapports. */
export function HandClassGrid({ cells }: HandClassGridProps) {
  const { t } = useTranslation()
  const byHandClass = new Map(cells.map((cell) => [cell.hand_class, cell]))
  const maxHandsPlayed = cells.reduce((max, cell) => Math.max(max, cell.hands_played), 0)

  return (
    <div
      className="grid gap-0.5 [font-variant-numeric:tabular-nums]"
      style={{ gridTemplateColumns: 'repeat(13, minmax(0, 1fr))' }}
    >
      {RANKS.map((_, rowIndex) =>
        RANKS.map((_, colIndex) => {
          const handClass = cellHandClass(rowIndex, colIndex)
          const cell = byHandClass.get(handClass)
          const belowSampleSize = !cell || cell.hands_played < MIN_SAMPLE_SIZE
          const intensity = cell && maxHandsPlayed > 0 ? cell.hands_played / maxHandsPlayed : 0
          const tooltip = cell
            ? t('reports.startingHands.tooltip', {
                handClass,
                hands: formatCount(cell.hands_played),
                vpip: formatPercent(cell.vpip.percentage),
                pfr: formatPercent(cell.pfr.percentage),
                bb: formatBb(cell.avg_net_bb),
              })
            : t('reports.startingHands.tooltipEmpty', { handClass })

          return (
            <div
              key={handClass}
              title={tooltip}
              className={`flex aspect-square flex-col items-center justify-center rounded-[2px] border border-[var(--color-border)] text-[9px] leading-tight ${
                belowSampleSize ? 'text-[var(--color-text-disabled)]' : 'text-[var(--color-bg)]'
              }`}
              style={{
                backgroundColor: belowSampleSize
                  ? 'var(--color-surface-2)'
                  : `color-mix(in srgb, var(--color-accent) ${Math.round(15 + intensity * 85)}%, var(--color-surface-2))`,
              }}
            >
              <span className="font-medium">{handClass}</span>
              {cell && <span>{formatCount(cell.hands_played)}</span>}
            </div>
          )
        }),
      )}
    </div>
  )
}
