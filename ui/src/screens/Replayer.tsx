import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, Outlet, useNavigate, useParams } from '@tanstack/react-router'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ReplayerTable } from '@/components/ReplayerTable'
import { getHandReplay, listTags, tagHands } from '@/lib/api'
import { formatCount, formatDate, formatPercent } from '@/lib/format'

const SPEEDS = [0.5, 1, 2, 4] as const
const BASE_STEP_MS = 1200
const STREET_ORDER = ['preflop', 'flop', 'turn', 'river', 'showdown']

function streetRank(street: string): number {
  const i = STREET_ORDER.indexOf(street)
  return i === -1 ? 0 : i
}

const ACTION_LABEL_KEYS: Record<string, string> = {
  postAnte: 'replayer.actions.postAnte',
  postSmallBlind: 'replayer.actions.postSmallBlind',
  postBigBlind: 'replayer.actions.postBigBlind',
  fold: 'replayer.actions.fold',
  check: 'replayer.actions.check',
  call: 'replayer.actions.call',
  bet: 'replayer.actions.bet',
  raise: 'replayer.actions.raise',
  shows: 'replayer.actions.shows',
  collected: 'replayer.actions.collected',
}

/** Route parente `/replayer` (layout) : ne rend que ses enfants
 * (`/replayer/` -> [[ReplayerEmpty]], `/replayer/$handId` -> [[Replayer]]).
 * `routes/replayer.tsx` et `routes/replayer.$handId.tsx` sont imbriquees par
 * la convention TanStack Router (meme prefixe de nom de fichier) : sans cet
 * `<Outlet />`, le routeur ne rend jamais l'enfant, meme quand son URL est
 * bien celle qui matche (bug trouve via `just dev`, 30/09 : double-clic sur
 * une main naviguait bien vers `/replayer/$handId` mais affichait toujours
 * l'etat "aucune main selectionnee" de la route parente). */
export function ReplayerLayout() {
  return <Outlet />
}

/** Route index `/replayer` (sans `handId`) : le Replayer n'a de sens que
 * pour une main precise, redirige l'utilisateur vers l'ecran Mains plutot
 * que d'afficher un ecran vide. */
export function ReplayerEmpty() {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-3 p-6">
      <p className="text-xs text-[var(--color-text-secondary)]">{t('replayer.noHandSelected')}</p>
      <Link to="/hands" className="text-xs underline">
        {t('nav.hands')}
      </Link>
    </div>
  )
}

/** Ecran Replayer (M7-3, PRD §13.7) : rejeu pas-a-pas d'une main, table
 * monochrome ovale, controles de lecture (debut/precedent/lecture-pause/
 * suivant/fin, vitesse 0.5x-4x, raccourcis clavier ←/→/espace), panneau
 * historique brut avec ligne courante surlignee, equite/EV a l'all-in,
 * pots finaux, SPR/mise en % du pot, tags et navigation main precedente/
 * suivante. */
export function Replayer() {
  const { handId } = useParams({ from: '/replayer/$handId' })
  // `key` force un remontage complet a chaque changement de main (navigation
  // precedente/suivante) : plus simple et plus sur qu'un effet qui
  // reinitialiserait `stepIndex`/`isPlaying` en reaction au changement de
  // parametre (`react-hooks/set-state-in-effect`).
  return <ReplayerForHand key={handId} numericHandId={Number(handId)} />
}

function ReplayerForHand({ numericHandId }: { numericHandId: number }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const [stepIndex, setStepIndex] = useState(0)
  const [isPlaying, setIsPlaying] = useState(false)
  const [speed, setSpeed] = useState<(typeof SPEEDS)[number]>(1)
  const rawPanelRef = useRef<HTMLDivElement>(null)
  const currentLineRef = useRef<HTMLDivElement>(null)

  const replayQuery = useQuery({
    queryKey: ['replayer', numericHandId],
    queryFn: () => getHandReplay(numericHandId),
  })
  const replay = replayQuery.data

  const tagsQuery = useQuery({ queryKey: ['tags'], queryFn: listTags })
  const tagMutation = useMutation({
    mutationFn: (tagId: number) => tagHands([numericHandId], tagId, Date.now()),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['hands', 'page'] })
    },
  })

  const stepCount = replay?.steps.length ?? 0
  const step = replay && stepCount > 0 ? replay.steps[Math.min(stepIndex, stepCount - 1)] : undefined

  useEffect(() => {
    if (!isPlaying || stepCount === 0) return
    const interval = setInterval(() => {
      setStepIndex((i) => {
        if (i >= stepCount - 1) {
          setIsPlaying(false)
          return i
        }
        return i + 1
      })
    }, BASE_STEP_MS / speed)
    return () => clearInterval(interval)
  }, [isPlaying, speed, stepCount])

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'ArrowLeft') {
        e.preventDefault()
        setIsPlaying(false)
        setStepIndex((i) => Math.max(0, i - 1))
      } else if (e.key === 'ArrowRight') {
        e.preventDefault()
        setIsPlaying(false)
        setStepIndex((i) => Math.min(stepCount - 1, i + 1))
      } else if (e.key === ' ') {
        e.preventDefault()
        setIsPlaying((p) => !p)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [stepCount])

  useEffect(() => {
    currentLineRef.current?.scrollIntoView({ block: 'center' })
  }, [step?.raw_line])

  const rawLines = useMemo(() => replay?.raw_text.split('\n') ?? [], [replay])

  if (replayQuery.isLoading) {
    return <div className="p-6 text-xs text-[var(--color-text-secondary)]">{t('replayer.loading')}</div>
  }

  if (!replay || !step) {
    return (
      <div className="flex flex-col gap-3 p-6">
        <p className="text-xs text-[var(--color-text-secondary)]">{t('replayer.notFound')}</p>
        <Link to="/hands" className="text-xs underline">
          {t('nav.hands')}
        </Link>
      </div>
    )
  }

  const showAllIn = replay.all_in && streetRank(step.street) >= streetRank(replay.all_in.street)

  return (
    <div className="flex flex-col gap-4 p-6">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h1 className="text-sm font-medium">{replay.tournament_name}</h1>
          <p className="text-xs text-[var(--color-text-secondary)]">
            {formatDate(replay.played_at)} · {t('replayer.header.level', { level: replay.level })} ·{' '}
            {t('replayer.header.blinds', {
              sb: formatCount(replay.sb),
              bb: formatCount(replay.bb),
              ante: formatCount(replay.ante),
            })}
          </p>
        </div>
        <div className="flex gap-3">
          <button
            type="button"
            disabled={replay.prev_hand_id == null}
            onClick={() => {
              if (replay.prev_hand_id != null) {
                void navigate({
                  to: '/replayer/$handId',
                  params: { handId: String(replay.prev_hand_id) },
                })
              }
            }}
            className="text-xs underline disabled:no-underline disabled:text-[var(--color-text-disabled)]"
          >
            {t('replayer.nav.prevHand')}
          </button>
          <button
            type="button"
            disabled={replay.next_hand_id == null}
            onClick={() => {
              if (replay.next_hand_id != null) {
                void navigate({
                  to: '/replayer/$handId',
                  params: { handId: String(replay.next_hand_id) },
                })
              }
            }}
            className="text-xs underline disabled:no-underline disabled:text-[var(--color-text-disabled)]"
          >
            {t('replayer.nav.nextHand')}
          </button>
        </div>
      </div>

      <ReplayerTable
        tableMaxSeats={replay.table_max_seats}
        buttonSeat={replay.button_seat}
        seats={step.seats}
        board={step.board}
        potTotal={step.pot_total}
      />

      <div className="flex flex-wrap items-center justify-center gap-2">
        <button
          type="button"
          onClick={() => {
            setIsPlaying(false)
            setStepIndex(0)
          }}
          className="rounded bg-[var(--color-surface-3)] px-2 py-1 text-xs"
        >
          {t('replayer.controls.start')}
        </button>
        <button
          type="button"
          onClick={() => {
            setIsPlaying(false)
            setStepIndex((i) => Math.max(0, i - 1))
          }}
          className="rounded bg-[var(--color-surface-3)] px-2 py-1 text-xs"
        >
          {t('replayer.controls.prev')}
        </button>
        <button
          type="button"
          onClick={() => setIsPlaying((p) => !p)}
          className="rounded bg-[var(--color-surface-3)] px-3 py-1 text-xs"
        >
          {isPlaying ? t('replayer.controls.pause') : t('replayer.controls.play')}
        </button>
        <button
          type="button"
          onClick={() => {
            setIsPlaying(false)
            setStepIndex((i) => Math.min(stepCount - 1, i + 1))
          }}
          className="rounded bg-[var(--color-surface-3)] px-2 py-1 text-xs"
        >
          {t('replayer.controls.next')}
        </button>
        <button
          type="button"
          onClick={() => {
            setIsPlaying(false)
            setStepIndex(stepCount - 1)
          }}
          className="rounded bg-[var(--color-surface-3)] px-2 py-1 text-xs"
        >
          {t('replayer.controls.end')}
        </button>
        <select
          value={speed}
          onChange={(e) => setSpeed(Number(e.target.value) as (typeof SPEEDS)[number])}
          className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-2 py-1 text-xs"
        >
          {SPEEDS.map((s) => (
            <option key={s} value={s}>
              {`${s}x`}
            </option>
          ))}
        </select>
        <span className="text-xs text-[var(--color-text-secondary)] [font-variant-numeric:tabular-nums]">
          {stepIndex + 1} / {stepCount}
        </span>
      </div>

      <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3 text-xs [font-variant-numeric:tabular-nums]">
        <span className="font-medium">{step.pseudo}</span> · {t(ACTION_LABEL_KEYS[step.kind] ?? step.kind)}
        {step.amount != null && <> {formatCount(step.amount)}</>}
        {step.to_amount != null && <> ({t('replayer.actions.to')} {formatCount(step.to_amount)})</>}
        {step.is_all_in && <span className="ml-1 text-[var(--color-text-secondary)]">{t('replayer.table.allIn')}</span>}
        {step.shown_label && <span className="ml-1 text-[var(--color-text-secondary)]">{step.shown_label}</span>}
        {(step.spr_before != null || step.bet_pct_pot != null) && (
          <div className="mt-1 text-[var(--color-text-secondary)]">
            {step.spr_before != null && (
              <>{t('replayer.spr', { value: step.spr_before.toFixed(1) })} </>
            )}
            {step.bet_pct_pot != null && <>{t('replayer.betPctPot', { value: formatPercent(step.bet_pct_pot) })}</>}
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <div
          ref={rawPanelRef}
          className="max-h-64 overflow-auto rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3 font-mono text-[11px] leading-5"
        >
          <h2 className="mb-2 text-xs font-medium [font-family:inherit]">{t('replayer.raw.title')}</h2>
          {rawLines.map((line, i) => (
            <div
              key={i}
              ref={i === step.raw_line ? currentLineRef : undefined}
              className={i === step.raw_line ? 'bg-[var(--color-surface-3)] text-[var(--color-text-primary)]' : 'text-[var(--color-text-secondary)]'}
            >
              {line || ' '}
            </div>
          ))}
        </div>

        <div className="flex flex-col gap-4">
          {showAllIn && replay.all_in && (
            <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3 text-xs">
              <h2 className="mb-2 font-medium">
                {t('replayer.allIn.title')}
                {replay.all_in.is_estimated && (
                  <span className="ml-1 text-[var(--color-text-secondary)]">
                    ({t('replayer.allIn.estimated')})
                  </span>
                )}
              </h2>
              <table className="w-full [font-variant-numeric:tabular-nums]">
                <thead>
                  <tr className="text-left text-[var(--color-text-secondary)]">
                    <th className="py-1 pr-3" />
                    <th className="py-1 pr-3 text-right">{t('replayer.allIn.equity')}</th>
                    <th className="py-1 text-right">{t('replayer.allIn.ev')}</th>
                  </tr>
                </thead>
                <tbody>
                  {replay.all_in.players.map((p) => (
                    <tr key={p.pseudo} className="border-t border-[var(--color-border)]">
                      <td className="py-1 pr-3">{p.pseudo}</td>
                      <td className="py-1 pr-3 text-right">{formatPercent(p.equity)}</td>
                      <td className="py-1 text-right">{formatCount(Math.round(p.ev_chips))}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3 text-xs">
            <h2 className="mb-2 font-medium">{t('replayer.pots.title')}</h2>
            {replay.final_pots.length > 0 ? (
              <ul className="flex flex-col gap-1 [font-variant-numeric:tabular-nums]">
                {replay.final_pots.map((pot, i) => (
                  <li key={i}>
                    {pot.label} — {formatCount(pot.amount)} :{' '}
                    {pot.winners.map((w) => `${w.pseudo} (${formatCount(w.amount)})`).join(', ')}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-[var(--color-text-secondary)]">{t('replayer.pots.empty')}</p>
            )}
          </div>

          <div className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] p-3 text-xs">
            <h2 className="mb-2 font-medium">{t('replayer.tagsTitle')}</h2>
            <div className="flex flex-wrap gap-2">
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
          </div>
        </div>
      </div>
    </div>
  )
}
