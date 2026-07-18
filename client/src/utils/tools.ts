export const POSITIVE_COLOR = 'rgb(246, 98, 105)'
export const NEGATIVE_COLOR = 'rgb(67, 160, 71)'
export const DEFAULT_COLOR = 'rgb(31, 34, 37)'

export function formatPercent(value: number): string {
  return (value * 100).toFixed(2) + '%'
}

export function rateColor(value: number): string {
  if (value > 0) return POSITIVE_COLOR
  if (value < 0) return NEGATIVE_COLOR
  return DEFAULT_COLOR
}
