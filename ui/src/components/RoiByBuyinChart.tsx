import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ECharts } from 'echarts'
import type { BuyinRoiRowPayload } from '@/bindings'
import { formatCents } from '@/lib/format'

interface RoiByBuyinChartProps {
  rows: BuyinRoiRowPayload[]
}

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

function bracketLabel(row: BuyinRoiRowPayload): string {
  const min = formatCents(row.buyin_min_cents)
  return row.buyin_max_cents == null ? `${min}+` : `${min}–${formatCents(row.buyin_max_cents)}`
}

/** Graphe G3 (PRD §9.2, M6-3 phase 2) : ROI par tranche de buy-in, tous
 * formats confondus (contrairement au pivot qui croise en plus KO/non-KO). */
export function RoiByBuyinChart({ rows }: RoiByBuyinChartProps) {
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
      grid: { left: 40, right: 12, top: 12, bottom: 32 },
      xAxis: {
        type: 'category',
        data: rows.map(bracketLabel),
        axisLine: { lineStyle: { color: borderColor } },
        axisLabel: { color: textColor, fontSize: 10 },
      },
      yAxis: {
        type: 'value',
        axisLine: { show: false },
        axisLabel: { color: textColor, fontSize: 10, formatter: '{value}%' },
        splitLine: { lineStyle: { color: borderColor } },
      },
      tooltip: { trigger: 'axis' },
      series: [
        {
          type: 'bar',
          itemStyle: { color: primaryColor },
          data: rows.map((row) => (row.kpis.roi == null ? null : row.kpis.roi * 100)),
        },
      ],
    })
  }, [rows, chartReady])

  if (rows.length === 0) {
    return <p className="text-xs text-[var(--color-text-secondary)]">{t('results.roiByBuyin.empty')}</p>
  }

  return <div ref={containerRef} className="h-40 w-full" />
}
