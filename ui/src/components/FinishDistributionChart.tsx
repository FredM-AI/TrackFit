import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ECharts } from 'echarts'
import type { FinishPercentileBucketPayload } from '@/bindings'

interface FinishDistributionChartProps {
  buckets: FinishPercentileBucketPayload[]
}

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

/** Graphe G5 (PRD §9.2, M6-3 phase 2) : distribution des places de sortie
 * en percentile du nombre d'inscrits (0 % = victoire, 100 % = sortie la plus
 * precoce). Pas de mise en evidence de "la bulle" : bloquee, voir
 * `docs/BACKLOG.md` (M6-3) et `gr_analytics::results` (`paid_places` jamais
 * renseigne, absent du format de summary Winamax). */
export function FinishDistributionChart({ buckets }: FinishDistributionChartProps) {
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
      grid: { left: 32, right: 12, top: 12, bottom: 32 },
      xAxis: {
        type: 'category',
        data: buckets.map((b) => `${b.floor_percent}–${b.floor_percent + 10}%`),
        axisLine: { lineStyle: { color: borderColor } },
        axisLabel: { color: textColor, fontSize: 10 },
      },
      yAxis: {
        type: 'value',
        minInterval: 1,
        axisLine: { show: false },
        axisLabel: { color: textColor, fontSize: 10 },
        splitLine: { lineStyle: { color: borderColor } },
      },
      tooltip: { trigger: 'axis' },
      series: [
        {
          type: 'bar',
          itemStyle: { color: primaryColor },
          data: buckets.map((b) => b.tournaments_count),
        },
      ],
    })
  }, [buckets, chartReady])

  if (buckets.length === 0) {
    return (
      <p className="text-xs text-[var(--color-text-secondary)]">{t('results.finishDistribution.empty')}</p>
    )
  }

  return <div ref={containerRef} className="h-40 w-full" />
}
