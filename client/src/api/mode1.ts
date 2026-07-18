import type {
  ApiResponse,
  ModeFilter,
  ModeListItem,
  ModeRequest,
  Period,
  QuantileData,
} from '@/types/mode1'

const apiBaseUrl = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/$/, '')

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${apiBaseUrl}${path}`, init)
  const payload = (await response.json()) as ApiResponse<T>

  if (!response.ok || payload.code >= 400) {
    throw new Error(payload.info || `请求失败（${response.status}）`)
  }

  return payload.data
}

export async function fetchPeriods(): Promise<Period[]> {
  const data = await request<Period[]>('/api/period')

  if (!Array.isArray(data)) {
    throw new Error('时间周期配置格式不正确')
  }

  return data
}

export async function fetchSectors(): Promise<string[]> {
  return fetchFilterOptions('/api/sector', '行业板块')
}

export async function fetchIndices(): Promise<string[]> {
  return fetchFilterOptions('/api/indice', '指数列表')
}

async function fetchFilterOptions(path: string, label: string): Promise<string[]> {
  const data = await request<string[]>(path)

  if (!Array.isArray(data)) {
    throw new Error(`${label}数据格式不正确`)
  }

  return [...new Set(data)]
    .filter(Boolean)
    .sort((left, right) => left.localeCompare(right, 'zh-CN'))
}

export async function fetchMode1List(filter: ModeFilter): Promise<ModeListItem[]> {
  const data = await request<ModeListItem[]>('/api/mode1/list', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(filter),
  })

  if (!Array.isArray(data)) {
    throw new Error('模式一列表数据格式不正确')
  }

  return data
}

export function fetchMode1Data(params: ModeRequest): Promise<QuantileData> {
  const id = encodeURIComponent(params.base.id)

  return request<QuantileData>(`/api/mode1/${id}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(params),
  })
}
