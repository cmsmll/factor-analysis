import type { ProfitMode, QuantileData } from '@/types/mode1'
import {
  annualizedReturn,
  average,
  cumulativeReturn,
  formatDate,
  sharpeRatio,
  sumMaxDrawdown,
  sumReturn,
} from '@/utils/factorSeries'

export interface FactorStat {
  label: string
  value: string
}

export interface FactorMetric {
  quantile: string
  returnRate: number
  annualizedReturn: number
  nav: number
  maxDrawdown: number
  maxDrawdownDate: string
  sharpeRatio: number
  turnoverRate: number
  factorValue: number
}

export interface FactorDetail {
  factorName: string
  datetimes: string[]
  changePercent: number[][]
  factor: number[][]
  turnoverRate: number[][]
  quantileNames: string[]
  stats: FactorStat[]
  metrics: FactorMetric[]
}

export const emptyDetail: FactorDetail = {
  factorName: '',
  datetimes: [],
  changePercent: [],
  factor: [],
  turnoverRate: [],
  quantileNames: [],
  stats: [],
  metrics: [],
}

export function buildDetail(
  data: QuantileData | undefined,
  startDate?: number | null,
  endDate?: number | null,
  profitMode: ProfitMode = 1,
): FactorDetail {
  if (!data || data.datetime.length === 0) return emptyDetail

  const start =
    startDate === null || startDate === undefined ? 0 : findFirstIndex(data.datetime, startDate)
  const end =
    endDate === null || endDate === undefined
      ? data.datetime.length - 1
      : findLastIndex(data.datetime, endDate)

  if (start < 0 || end < start) return { ...emptyDetail, factorName: data.name }

  const datetimes = data.datetime.slice(start, end + 1)
  const changePercent = sliceSeries(getProfitSeries(data, profitMode), start, end + 1)
  const factor = sliceSeries(data.factor, start, end + 1)
  const turnoverRate = sliceSeries(data.turnover_rate ?? [], start, end + 1)
  const quantileCount = Math.max(
    data.count,
    changePercent.length,
    factor.length,
    turnoverRate.length,
  )
  const quantileNames = Array.from({ length: quantileCount }, (_, index) => `分位${index + 1}`)

  return {
    factorName: data.name,
    datetimes,
    changePercent,
    factor,
    turnoverRate,
    quantileNames,
    stats: buildStats(datetimes, turnoverRate),
    metrics: buildMetrics(datetimes, changePercent, factor, turnoverRate, quantileNames),
  }
}

function getProfitSeries(data: QuantileData, mode: ProfitMode): number[][] {
  switch (mode) {
    case 2:
      return data.profit2.map((profit) => profit.source)
    case 3:
      return data.profit3.map((profit) => profit.source)
    case 4:
      return data.profit4.map((profit) => profit.source)
    default:
      return data.profit1.map((profit) => profit.source)
  }
}

function buildStats(datetimes: string[], turnoverRate: number[][]): FactorStat[] {
  const values = turnoverRate.flat()

  return [
    { label: '交易日数量', value: String(datetimes.length) },
    { label: '平均换手率', value: `${(average(values) * 100).toFixed(2)}%` },
  ]
}

function buildMetrics(
  datetimes: string[],
  returns: number[][],
  factor: number[][],
  turnoverRate: number[][],
  quantileNames: string[],
): FactorMetric[] {
  return quantileNames
    .map((quantile, index) => {
      const quantileReturns = returns[index] ?? []
      const drawdown = sumMaxDrawdown(quantileReturns, datetimes)

      return {
        quantile,
        returnRate: sumReturn(quantileReturns),
        annualizedReturn: annualizedReturn(quantileReturns, calendarDays(datetimes)),
        nav: cumulativeReturn(quantileReturns) + 1,
        maxDrawdown: drawdown.value,
        maxDrawdownDate: drawdown.date ? formatDate(drawdown.date) : '--',
        sharpeRatio: sharpeRatio(quantileReturns),
        turnoverRate: average(turnoverRate[index] ?? []),
        factorValue: average(factor[index] ?? []),
      }
    })
    .sort((left, right) => right.returnRate - left.returnRate)
}

function sliceSeries(series: number[][], start: number, end: number): number[][] {
  return series.map((values) => values.slice(start, end))
}

function findFirstIndex(datetimes: string[], timestamp: number): number {
  return datetimes.findIndex((datetime) => toTimestamp(datetime) >= timestamp)
}

function findLastIndex(datetimes: string[], timestamp: number): number {
  for (let index = datetimes.length - 1; index >= 0; index -= 1) {
    const datetime = datetimes[index]
    if (datetime && toTimestamp(datetime) <= timestamp) return index
  }
  return -1
}

function calendarDays(datetimes: string[]): number {
  const first = datetimes[0]
  const last = datetimes[datetimes.length - 1]
  if (!first || !last) return 0
  return (toTimestamp(last) - toTimestamp(first)) / 86_400_000
}

function toTimestamp(datetime: string): number {
  const timestamp = new Date(formatDate(datetime)).getTime()
  return Number.isFinite(timestamp) ? timestamp : 0
}
