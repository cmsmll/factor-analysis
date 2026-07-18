import { reactive } from 'vue'
import { defineStore } from 'pinia'

import { fetchMode1Data } from '@/api/mode1'
import type { ModeLoadStatus, ModeRequest, QuantileData } from '@/types/mode1'

export const useMode1PreviewStore = defineStore('mode1-preview', () => {
  const results = reactive<Record<string, QuantileData | undefined>>({})
  const statuses = reactive<Record<string, ModeLoadStatus | undefined>>({})
  const errors = reactive<Record<string, string | undefined>>({})
  const pending = new Map<string, Promise<void>>()
  const requestVersions = new Map<string, number>()

  function initialize(id: string, data: QuantileData): void {
    requestVersions.set(id, (requestVersions.get(id) ?? 0) + 1)
    results[id] = structuredClone(data)
    statuses[id] = 'success'
    errors[id] = undefined
  }

  function loadMode(params: ModeRequest, force = false): Promise<void> {
    const id = params.base.id

    if (!force && results[id]) return Promise.resolve()
    const current = pending.get(id)
    if (!force && current) return current

    const version = (requestVersions.get(id) ?? 0) + 1
    requestVersions.set(id, version)
    statuses[id] = 'loading'
    errors[id] = undefined

    const task = fetchMode1Data(params)
      .then((data) => {
        if (requestVersions.get(id) !== version) return
        results[id] = data
        statuses[id] = 'success'
      })
      .catch((error: unknown) => {
        if (requestVersions.get(id) !== version) return
        statuses[id] = 'error'
        errors[id] = error instanceof Error ? error.message : '获取因子预览数据失败'
      })
      .finally(() => {
        if (pending.get(id) === task) pending.delete(id)
      })

    pending.set(id, task)
    return task
  }

  return { results, statuses, errors, initialize, loadMode }
})
