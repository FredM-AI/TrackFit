import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ECharts } from 'echarts'
import type { ChipCurvePoint } from '@/bindings'
import { formatBb } from '@/lib/format'

interface ChipCurveChartProps {
  points: ChipCurvePoint[]
}

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

/** Graphe G2 (PRD §9.2) : cumul des jetons gagnes en bb, reel contre ajuste
 * a l'EV all-in (§10.6, M5-4) — l'equivalent MTT des lignes "rouge et
 * bleue". Meme approche que `ProfitCurveChart` (M6-2) : `echarts` importe
 * dynamiquement, couleurs resolues depuis les tokens CSS (D15). */
export function ChipCurveChart({ points }: ChipCurveChartProps) {
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
          formatter: (value: number) => formatBb(value),
        },
        splitLine: { lineStyle: { color: borderColor } },
      },
      tooltip: {
        trigger: 'axis',
        valueFormatter: (value: number | string) => formatBb(Number(value)),
      },
      legend: {
        top: 0,
        textStyle: { color: textColor, fontSize: 10 },
        data: [t('results.chipCurve.real'), t('results.chipCurve.evAdjusted')],
      },
      series: [
        {
          name: t('results.chipCurve.real'),
          type: 'line',
          showSymbol: false,
          lineStyle: { color: primaryColor, width: 2 },
          data: points.map((p) => [p.played_at, p.cumulative_net_bb]),
        },
        {
          name: t('results.chipCurve.evAdjusted'),
          type: 'line',
          showSymbol: false,
          lineStyle: { color: textColor, width: 1, type: 'dashed' },
          data: points.map((p) => [p.played_at, p.cumulative_ev_adjusted_net_bb]),
        },
      ],
    })
  }, [points, t, chartReady])

  if (points.length === 0) {
    return <p className="text-xs text-[var(--color-text-secondary)]">{t('results.chipCurve.empty')}</p>
  }

  return <div ref={containerRef} className="h-56 w-full" />
}
