import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ECharts } from 'echarts'
import type { ProfitCurvePoint } from '@/bindings'
import { formatCents } from '@/lib/format'

interface ProfitCurveChartProps {
  points: ProfitCurvePoint[]
}

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

/** Graphe G1 en format reduit (PRD §13.5/§13.1, M6-2) : courbe de profit
 * cumule par tournoi, avec en superposition le profit hors bounties. Pas de
 * theme ECharts integre (couleurs resolues depuis les tokens CSS, D15) :
 * les couleurs monochromes par defaut restent coherentes avec le reste de
 * l'app sans dupliquer la palette.
 *
 * `echarts` est importe dynamiquement (pas d'import statique en tete de
 * fichier) : c'est une librairie lourde, et un import statique la mettait
 * dans le meme chunk que l'ecran Accueil, ralentissant son chargement
 * meme quand ce composant ne rend rien (aucun tournoi, cf. le early return
 * plus bas). Seul le type `ECharts` est importe statiquement (erase par
 * TypeScript, sans cout d'execution). */
export function ProfitCurveChart({ points }: ProfitCurveChartProps) {
  const { t } = useTranslation()
  const containerRef = useRef<HTMLDivElement>(null)
  const chartRef = useRef<ECharts | null>(null)
  const [chartReady, setChartReady] = useState(false)

  useEffect(() => {
    if (!containerRef.current) return undefined
    let cancelled = false
    let chart: ECharts | undefined

    void import('echarts').then((echarts) => {
      if (cancelled || !containerRef.current) return
      chart = echarts.init(containerRef.current)
      chartRef.current = chart
      setChartReady(true)
    })

    const handleResize = () => chartRef.current?.resize()
    window.addEventListener('resize', handleResize)
    return () => {
      cancelled = true
      window.removeEventListener('resize', handleResize)
      chart?.dispose()
      chartRef.current = null
    }
  }, [])

  useEffect(() => {
    const chart = chartRef.current
    if (!chart) return
    const textColor = cssVar('--color-text-secondary')
    const primaryColor = cssVar('--color-text-primary')
    const borderColor = cssVar('--color-border')

    chart.setOption({
      backgroundColor: 'transparent',
      textStyle: { color: textColor, fontSize: 10 },
      grid: { left: 48, right: 12, top: 28, bottom: 24 },
      xAxis: {
        type: 'time',
        axisLine: { lineStyle: { color: borderColor } },
        axisLabel: { color: textColor, fontSize: 10 },
        splitLine: { show: false },
      },
      yAxis: {
        type: 'value',
        axisLine: { show: false },
        axisLabel: {
          color: textColor,
          fontSize: 10,
          formatter: (value: number) => formatCents(value),
        },
        splitLine: { lineStyle: { color: borderColor } },
      },
      tooltip: {
        trigger: 'axis',
        valueFormatter: (value: number | string) => formatCents(Number(value)),
      },
      legend: {
        top: 0,
        textStyle: { color: textColor, fontSize: 10 },
        data: [t('home.profitCurve.withBounty'), t('home.profitCurve.withoutBounty')],
      },
      series: [
        {
          name: t('home.profitCurve.withBounty'),
          type: 'line',
          showSymbol: false,
          lineStyle: { color: primaryColor, width: 2 },
          data: points.map((p) => [p.started_at, p.cumulative_profit_cents]),
        },
        {
          name: t('home.profitCurve.withoutBounty'),
          type: 'line',
          showSymbol: false,
          lineStyle: { color: textColor, width: 1, type: 'dashed' },
          data: points.map((p) => [p.started_at, p.cumulative_profit_excluding_bounty_cents]),
        },
      ],
    })
  }, [points, t, chartReady])

  if (points.length === 0) {
    return (
      <p className="text-xs text-[var(--color-text-secondary)]">{t('home.profitCurve.empty')}</p>
    )
  }

  return <div ref={containerRef} className="h-44 w-full" />
}
