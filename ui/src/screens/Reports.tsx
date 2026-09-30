import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ReportRowPayload } from '@/bindings'
import { HandClassGrid } from '@/components/HandClassGrid'
import {
  getBbDefenseByDepthReport,
  getOshoveByDepthAndPositionReport,
  getPostflopCbetReport,
  getPreflopByPositionReport,
  getStartingHandsGridReport,
} from '@/lib/api'
import { formatCount, formatPercent } from '@/lib/format'

/** Sous le seuil d'echantillon, la valeur est grisee plutot que masquee
 * (PRD §10.1 : "en dessous d'un seuil d'echantillon (parametrable, 30 par
 * defaut)") — pas d'ecran de parametres pour regler ce seuil encore,
 * valeur par defaut assumee (meme principe que d'autres constantes PRD
 * deja assumees ailleurs, ex. FINAL_TABLE_SEATS_DEFAULT en M6-3). */
const MIN_SAMPLE_SIZE = 30

type TableReportId =
  | 'preflopByPosition'
  | 'oshoveByDepthAndPosition'
  | 'postflopCbet'
  | 'bbDefenseByDepth'
type ReportId = TableReportId | 'startingHands'

interface ReportConfig {
  id: TableReportId
  dimensionLabelKeys: string[]
  measureLabelKeys: string[]
  queryFn: () => Promise<ReportRowPayload[]>
}

const REPORT_IDS: ReportId[] = [
  'preflopByPosition',
  'oshoveByDepthAndPosition',
  'postflopCbet',
  'bbDefenseByDepth',
  'startingHands',
]

/** `Record` plutot qu'un tableau + `.find()` : garantit a la compilation
 * qu'un `TableReportId` valide donne toujours une config (pas de
 * `undefined` possible), sans avoir a re-verifier a l'usage. `startingHands`
 * (M7-7) n'a pas d'entree ici : sa forme (grille, pas dimensions/mesures)
 * est incompatible avec `ReportConfig`, rendue a part dans `Reports()`. */
const REPORTS: Record<TableReportId, ReportConfig> = {
  preflopByPosition: {
    id: 'preflopByPosition',
    dimensionLabelKeys: ['reports.dimensions.position'],
    measureLabelKeys: [
      'reports.measures.vpip',
      'reports.measures.pfr',
      'reports.measures.rfi',
      'reports.measures.limp',
      'reports.measures.oshove',
      'reports.measures.threeBet',
      'reports.measures.foldToThreeBet',
      'reports.measures.fourBet',
      'reports.measures.ats',
    ],
    queryFn: getPreflopByPositionReport,
  },
  oshoveByDepthAndPosition: {
    id: 'oshoveByDepthAndPosition',
    dimensionLabelKeys: ['reports.dimensions.depth', 'reports.dimensions.position'],
    measureLabelKeys: ['reports.measures.oshove'],
    queryFn: getOshoveByDepthAndPositionReport,
  },
  postflopCbet: {
    id: 'postflopCbet',
    dimensionLabelKeys: ['reports.dimensions.position'],
    measureLabelKeys: ['reports.measures.cbf', 'reports.measures.cbt', 'reports.measures.fcbf'],
    queryFn: getPostflopCbetReport,
  },
  bbDefenseByDepth: {
    id: 'bbDefenseByDepth',
    dimensionLabelKeys: ['reports.dimensions.depth'],
    measureLabelKeys: ['reports.measures.fsteal', 'reports.measures.rsteal'],
    queryFn: getBbDefenseByDepthReport,
  },
}

function TableReportSection({ report }: { report: ReportConfig }) {
  const { t } = useTranslation()
  const reportQuery = useQuery({
    queryKey: ['reports', report.id],
    queryFn: report.queryFn,
  })

  const rows = reportQuery.data ?? []
  const dimensionCount = report.dimensionLabelKeys.length

  return (
    <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
      {rows.length > 0 ? (
        <table className="w-full text-xs [font-variant-numeric:tabular-nums]">
          <thead>
            <tr className="text-left text-[var(--color-text-secondary)]">
              {report.dimensionLabelKeys.map((key) => (
                <th key={key} className="py-1 pr-3">
                  {t(key)}
                </th>
              ))}
              {report.measureLabelKeys.map((key) => (
                <th key={key} className="py-1 pr-3 text-right">
                  {t(key)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr
                key={row.dimension_values.join('|')}
                className="border-t border-[var(--color-border)]"
              >
                {row.dimension_values.slice(0, dimensionCount).map((value, i) => (
                  <td key={report.dimensionLabelKeys[i]} className="py-1 pr-3">
                    {value ?? '—'}
                  </td>
                ))}
                {row.cells.map((cell, i) => (
                  <td
                    key={report.measureLabelKeys[i]}
                    className={`py-1 pr-3 text-right ${
                      cell.opportunities < MIN_SAMPLE_SIZE ? 'text-[var(--color-text-secondary)]' : ''
                    }`}
                  >
                    {formatPercent(cell.percentage)}{' '}
                    <span className="text-[var(--color-text-secondary)]">
                      ({formatCount(cell.opportunities)})
                    </span>
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <p className="text-xs text-[var(--color-text-secondary)]">{t('reports.empty')}</p>
      )}
    </div>
  )
}

/** Grille 13×13 des mains de depart (M7-7, PRD §13.5) : une section a part
 * du tableau generique ci-dessus (forme differente, pas de dimensions/
 * mesures choisies). */
function StartingHandsGridSection() {
  const { t } = useTranslation()
  const gridQuery = useQuery({
    queryKey: ['reports', 'startingHands'],
    queryFn: getStartingHandsGridReport,
  })
  const cells = gridQuery.data ?? []

  return (
    <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3">
      {cells.length > 0 ? (
        <HandClassGrid cells={cells} />
      ) : (
        <p className="text-xs text-[var(--color-text-secondary)]">{t('reports.empty')}</p>
      )}
    </div>
  )
}

/** Ecran Rapports (M7-1, PRD §13.5), perimetre reduit aux rapports
 * predefinis (decision validee par Frederic, 30/09) : pas de constructeur
 * generique (choix libre de dimensions/mesures, filtres, tri, sauvegarde).
 * « Resultats par phase » (5e rapport du PRD) rejoint M4-5 en V2 (phase du
 * tournoi jamais calculee). Grille 13×13 des mains de depart ajoutee en
 * M7-7 — rendue a part (forme differente), pas via `REPORTS`. */
export function Reports() {
  const { t } = useTranslation()
  const [reportId, setReportId] = useState<ReportId>('preflopByPosition')

  return (
    <div className="flex flex-col gap-4 p-6">
      <div className="flex flex-wrap gap-1">
        {REPORT_IDS.map((id) => (
          <button
            key={id}
            type="button"
            onClick={() => setReportId(id)}
            className={`rounded px-3 py-1 text-xs ${
              id === reportId
                ? 'bg-[var(--color-accent)] text-[var(--color-bg)]'
                : 'bg-[var(--color-surface-2)] text-[var(--color-text-secondary)]'
            }`}
          >
            {t(`reports.titles.${id}`)}
          </button>
        ))}
      </div>

      {reportId === 'startingHands' ? (
        <StartingHandsGridSection />
      ) : (
        <TableReportSection report={REPORTS[reportId]} />
      )}
    </div>
  )
}
