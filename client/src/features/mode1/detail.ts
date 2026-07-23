import type { Profit, ProfitMode, QuantileData } from '@/types/mode1'
import { average, formatDate, sharpeRatio, sumMaxDrawdown } from '@/utils/factorSeries'

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

  const profits = getProfits(data, profitMode)
  const datetimes = data.datetime.slice(start, end + 1)
  const changePercent = sliceSeries(
    profits.map((profit) => profit.source),
    start,
    end + 1,
  )
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
    metrics: buildMetrics(datetimes, profits, changePercent, factor, turnoverRate, quantileNames),
  }
}

function getProfits(data: QuantileData, mode: ProfitMode): Profit[] {
  switch (mode) {
    case 2:
      return data.profit2
    case 3:
      return data.profit3
    case 4:
      return data.profit4
    default:
      return data.profit1
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
  profits: Profit[],
  returns: number[][],
  factor: number[][],
  turnoverRate: number[][],
  quantileNames: string[],
): FactorMetric[] {
  return quantileNames
    .map((quantile, index) => {
      const quantileReturns = returns[index] ?? []
      const profit = profits[index]
      const drawdown = sumMaxDrawdown(quantileReturns, datetimes)

      return {
        quantile,
        returnRate: profit?.total_profit ?? 0,
        annualizedReturn: profit?.annualized_profit ?? 0,
        nav: profit?.total_net_value ?? 1,
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

function toTimestamp(datetime: string): number {
  const timestamp = new Date(formatDate(datetime)).getTime()
  return Number.isFinite(timestamp) ? timestamp : 0
}
