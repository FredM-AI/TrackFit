import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ECharts } from 'echarts'
import type { VolumePointPayload } from '@/bindings'

interface VolumeChartProps {
  points: VolumePointPayload[]
}

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

/** Graphe G6 (PRD §9.2), version "par jour" (M6-3 phase 1 — semaine/mois et
 * heatmap jour x heure differes) : nombre de tournois joues par jour. */
export function VolumeChart({ points }: VolumeChartProps) {
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
      grid: { left: 32, right: 12, top: 12, bottom: 24 },
      xAxis: {
        type: 'time',
        axisLine: { lineStyle: { color: borderColor } },
        axisLabel: { color: textColor, fontSize: 10 },
        splitLine: { show: false },
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
          data: points.map((p) => [p.day_epoch_ms, p.tournaments_count]),
        },
      ],
    })
  }, [points, chartReady])

  if (points.length === 0) {
    return <p className="text-xs text-[var(--color-text-secondary)]">{t('results.volume.empty')}</p>
  }

  return <div ref={containerRef} className="h-40 w-full" />
}
