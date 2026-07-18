export interface ApiResponse<T> {
  info: string
  code: number
  data: T
}

export interface ModeFilter {
  start: string
  end: string
  filter_bz: boolean
  filter_st: boolean
  sector: string[]
  indice: string[]
}

export interface Period {
  name: string
  start: string
  end: string
}

export interface ModeBase {
  id: string
  count: number
  filter: ModeFilter
}

export interface ModeRequest {
  base: ModeBase
  [key: string]: unknown
}

export interface ModeListItem {
  args: ModeRequest
  data: QuantileData
}

export interface Profit {
  source: number[]
  total_profit: number
  total_net_value: number
  annualized_profit: number
}

export interface QuantileData {
  name: string
  info: string
  count: number
  factor: number[][]
  turnover_rate?: number[][]
  profit1: Profit[]
  profit2: Profit[]
  profit3: Profit[]
  profit4: Profit[]
  datetime: string[]
}

export type ProfitMode = 1 | 2 | 3 | 4
export type QuantileCount = 3 | 5 | 10
export type ModeLoadStatus = 'idle' | 'loading' | 'success' | 'error'
