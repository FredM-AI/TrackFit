import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ECharts } from 'echarts'
import type { TournamentStackPointPayload } from '@/bindings'

interface TournamentStackChartProps {
  points: TournamentStackPointPayload[]
}

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

/** "Chronologie du tapis du Hero" (PRD §13.3, M6-4) : tapis en bb en debut
 * de chaque main du tournoi. Meme conventions ECharts que les graphes
 * Resultats (import dynamique, couleurs via tokens CSS). */
export function TournamentStackChart({ points }: TournamentStackChartProps) {
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
      grid: { left: 40, right: 12, top: 12, bottom: 24 },
      xAxis: {
        type: 'time',
        axisLine: { lineStyle: { color: borderColor } },
        axisLabel: { color: textColor, fontSize: 10 },
        splitLine: { show: false },
      },
      yAxis: {
        type: 'value',
        axisLine: { show: false },
        axisLabel: { color: textColor, fontSize: 10, formatter: '{value} bb' },
        splitLine: { lineStyle: { color: borderColor } },
      },
      tooltip: { trigger: 'axis' },
      series: [
        {
          type: 'line',
          showSymbol: false,
          lineStyle: { color: primaryColor },
          data: points.map((p) => [p.played_at, p.stack_bb]),
        },
      ],
    })
  }, [points, chartReady])

  if (points.length === 0) {
    return (
      <p className="text-xs text-[var(--color-text-secondary)]">{t('tournaments.detail.stackEmpty')}</p>
    )
  }

  return <div ref={containerRef} className="h-48 w-full" />
}
