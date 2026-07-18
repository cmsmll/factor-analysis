export const TRADING_DAYS_PER_YEAR = 252

export function formatDate(value: string): string {
  return value.slice(0, 10)
}

export function average(values: number[]): number {
  const finiteValues = values.filter((value) => Number.isFinite(value))
  if (finiteValues.length === 0) return 0

  return finiteValues.reduce((sum, value) => sum + value, 0) / finiteValues.length
}

export function round(value: number, digits = 4): number {
  const base = 10 ** digits
  return Math.round(value * base) / base
}

export function sumReturn(values: number[]): number {
  return values.filter((value) => Number.isFinite(value)).reduce((sum, value) => sum + value, 0)
}

export function cumulativeReturn(values: number[]): number {
  return (
    values.filter((value) => Number.isFinite(value)).reduce((nav, value) => nav * (1 + value), 1) -
    1
  )
}

export function annualizedReturn(values: number[], days?: number): number {
  const finiteValues = values.filter((value) => Number.isFinite(value))
  if (finiteValues.length === 0) return 0

  const R = sumReturn(finiteValues)
  const base = 1 + R
  if (base <= 0) return 0

  // 如果有日历天数，使用 (1+R)^(365/d) - 1 的复利年化公式，R 为算术累计收益率
  if (days !== undefined && days > 0) {
    return Math.pow(base, 365 / days) - 1
  }

  // 兜底：按交易日数量年化
  return Math.pow(base, TRADING_DAYS_PER_YEAR / finiteValues.length) - 1
}

export function standardDeviation(values: number[]): number {
  const finiteValues = values.filter((value) => Number.isFinite(value))
  if (finiteValues.length === 0) return 0

  const mean = average(finiteValues)
  const variance = average(finiteValues.map((value) => (value - mean) ** 2))

  return Math.sqrt(variance)
}

export function sharpeRatio(values: number[]): number {
  const finiteValues = values.filter((value) => Number.isFinite(value))
  if (finiteValues.length === 0) return 0

  const deviation = standardDeviation(finiteValues)

  return deviation === 0
    ? 0
    : (average(finiteValues) / deviation) * Math.sqrt(TRADING_DAYS_PER_YEAR)
}

export function maxDrawdown(
  values: number[],
  datetimes: string[] = [],
): { date: string; value: number } {
  let nav = 1
  let peak = 1
  let drawdownValue = 0
  let drawdownDate = ''

  values.forEach((value, index) => {
    if (!Number.isFinite(value)) return

    nav *= 1 + value
    if (nav > peak) peak = nav

    const drawdown = peak === 0 ? 0 : nav / peak - 1
    if (drawdown < drawdownValue) {
      drawdownValue = drawdown
      drawdownDate = datetimes[index] ?? ''
    }
  })

  return { date: drawdownDate, value: drawdownValue }
}

export function sumMaxDrawdown(
  values: number[],
  datetimes: string[] = [],
): { date: string; value: number } {
  let nav = 1
  let peak = 1
  let drawdownValue = 0
  let drawdownDate = ''

  values.forEach((value, index) => {
    if (!Number.isFinite(value)) return

    nav += value
    if (nav > peak) peak = nav

    const drawdown = peak === 0 ? 0 : nav / peak - 1
    if (drawdown < drawdownValue) {
      drawdownValue = drawdown
      drawdownDate = datetimes[index] ?? ''
    }
  })

  return { date: drawdownDate, value: drawdownValue }
}

export function percentSeries(values: number[]): number[] {
  return values.map((value) => round(value * 100, 4))
}

export function cumulativeReturnPercentSeries(values: number[]): number[] {
  let nav = 1

  return values.map((value) => {
    nav *= 1 + value
    return round((nav - 1) * 100, 4)
  })
}

export function sumReturnPercentSeries(values: number[]): number[] {
  let total = 0

  return values.map((value) => {
    total += value
    return round(total * 100, 4)
  })
}
