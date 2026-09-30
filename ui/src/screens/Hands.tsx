import { useMutation, useQueries, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { HandListRowPayload } from '@/bindings'
import { getHandsCount, getHandsPage, listTags, tagHands } from '@/lib/api'
import { resolveDateRange, useFilterStore } from '@/lib/filters'
import { formatBb, formatCount, formatDate, signedValueClassName } from '@/lib/format'

/** Taille de page cote frontend (pas liee a une valeur backend) : assez
 * grande pour couvrir plusieurs ecrans de defilement sans multiplier les
 * appels IPC, assez petite pour rester rapide a charger par page (M6-5,
 * NFR-P7 : "premier affichage < 500 ms"). */
const PAGE_SIZE = 100
const ROW_HEIGHT_PX = 32
const GRID_COLUMNS = '28px 150px 150px 60px 60px 80px 90px 90px 130px 80px 80px 1fr'

function pageIndexOf(rowIndex: number): number {
  return Math.floor(rowIndex / PAGE_SIZE)
}

/** Ecran Mains (M6-5, PRD §13.4) : tableau virtualise du profil Hero actif,
 * paginee cote backend (le volume vise 1M+ mains, NFR-P7 : "premier
 * affichage < 500 ms" — pas de fetch-all comme M6-3/M6-4, quelques
 * milliers de tournois tiennent en memoire mais pas des millions de
 * mains). Selection multiple -> tag en masse (les 9 tags predefinis,
 * `docs/BACKLOG.md` : creer un tag libre reste le perimetre de M7-4).
 * Double-clic -> Replayer (placeholder avant M7-3). */
export function Hands() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const parentRef = useRef<HTMLDivElement>(null)
  const [selected, setSelected] = useState<Set<number>>(new Set())

  const range = useFilterStore((s) => s.perScreen.hands)

  const countQuery = useQuery({
    queryKey: ['hands', 'count', range],
    queryFn: () => getHandsCount(resolveDateRange(range, Date.now())),
  })
  const total = countQuery.data ?? 0

  const tagsQuery = useQuery({ queryKey: ['tags'], queryFn: listTags })

  const rowVirtualizer = useVirtualizer({
    count: total,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT_PX,
    overscan: 20,
  })

  const virtualItems = rowVirtualizer.getVirtualItems()
  const neededPages = useMemo(() => {
    const pages = new Set<number>()
    for (const item of virtualItems) pages.add(pageIndexOf(item.index))
    return Array.from(pages).sort((a, b) => a - b)
  }, [virtualItems])

  const pageQueries = useQueries({
    queries: neededPages.map((pageIndex) => ({
      queryKey: ['hands', 'page', range, pageIndex],
      queryFn: () => getHandsPage(PAGE_SIZE, pageIndex * PAGE_SIZE, resolveDateRange(range, Date.now())),
      staleTime: 60_000,
    })),
  })

  const rowsByIndex = useMemo(() => {
    const map = new Map<number, HandListRowPayload>()
    neededPages.forEach((pageIndex, i) => {
      const rows = pageQueries[i]?.data
      if (!rows) return
      rows.forEach((row, offsetInPage) => {
        map.set(pageIndex * PAGE_SIZE + offsetInPage, row)
      })
    })
    return map
  }, [neededPages, pageQueries])

  const tagMutation = useMutation({
    mutationFn: (tagId: number) => tagHands(Array.from(selected), tagId, Date.now()),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['hands', 'page'] })
      setSelected(new Set())
    },
  })

  function toggleSelected(handId: number) {
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(handId)) {
        next.delete(handId)
      } else {
        next.add(handId)
      }
      return next
    })
  }

  return (
    <div className="flex h-full flex-col gap-3 p-6">
      <div className="flex items-center justify-between gap-3">
        <h1 className="text-sm font-medium">{t('nav.hands')}</h1>
        {selected.size > 0 && (
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-xs text-[var(--color-text-secondary)]">
              {t('hands.selectedCount', { count: selected.size })}
            </span>
            {(tagsQuery.data ?? []).map((tag) => (
              <button
                key={tag.id}
                type="button"
                disabled={tagMutation.isPending}
                onClick={() => tagMutation.mutate(tag.id)}
                className="rounded bg-[var(--color-surface-3)] px-2 py-1 text-xs disabled:opacity-40"
              >
                {t(tag.label_key)}
              </button>
            ))}
          </div>
        )}
      </div>

      {total === 0 ? (
        <p className="text-xs text-[var(--color-text-secondary)]">{t('hands.empty')}</p>
      ) : (
        <div className="flex min-h-0 flex-1 flex-col overflow-x-auto">
          <div
            className="grid gap-2 border-b border-[var(--color-border)] px-2 py-1 text-xs text-[var(--color-text-secondary)]"
            style={{ gridTemplateColumns: GRID_COLUMNS, minWidth: 'max-content' }}
          >
            <div />
            <div>{t('hands.columns.date')}</div>
            <div>{t('hands.columns.tournament')}</div>
            <div className="text-right">{t('hands.columns.level')}</div>
            <div>{t('hands.columns.position')}</div>
            <div className="text-right">{t('hands.columns.depth')}</div>
            <div>{t('hands.columns.cards')}</div>
            <div>{t('hands.columns.preflopLine')}</div>
            <div>{t('hands.columns.board')}</div>
            <div className="text-right">{t('hands.columns.result')}</div>
            <div className="text-right">{t('hands.columns.evDiff')}</div>
            <div>{t('hands.columns.tags')}</div>
          </div>

          <div
            ref={parentRef}
            className="min-h-0 flex-1 overflow-auto"
            style={{ minWidth: 'max-content' }}
          >
            <div
              style={{
                height: rowVirtualizer.getTotalSize(),
                position: 'relative',
                minWidth: 'max-content',
              }}
            >
              {virtualItems.map((virtualRow) => {
                const row = rowsByIndex.get(virtualRow.index)
                return (
                  <div
                    key={virtualRow.index}
                    onDoubleClick={() => {
                      if (row) void navigate({ to: '/replayer' })
                    }}
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
                    {row ? (
                      <>
                        <input
                          type="checkbox"
                          checked={selected.has(row.hand_id)}
                          onChange={() => toggleSelected(row.hand_id)}
                          onClick={(e) => e.stopPropagation()}
                        />
                        <div className="truncate">{formatDate(row.played_at)}</div>
                        <div className="truncate">{row.tournament_name ?? '—'}</div>
                        <div className="text-right">{formatCount(row.level)}</div>
                        <div>{row.position ?? '—'}</div>
                        <div className="text-right">{formatBb(row.eff_stack_bb)}</div>
                        <div className="truncate">{row.hole_cards ?? '—'}</div>
                        <div className="truncate">{row.preflop_line ?? '—'}</div>
                        <div className="truncate">{row.board || '—'}</div>
                        <div className={`text-right ${signedValueClassName(row.net_bb ?? 0)}`}>
                          {formatBb(row.net_bb)}
                        </div>
                        <div className="text-right">{formatBb(row.allin_ev_diff_bb)}</div>
                        <div className="truncate">
                          {row.tag_label_keys.map((key) => t(key)).join(', ')}
                        </div>
                      </>
                    ) : (
                      <div
                        className="text-[var(--color-text-secondary)]"
                        style={{ gridColumn: `1 / -1` }}
                      >
                        {t('hands.loadingRow')}
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
