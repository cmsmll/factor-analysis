import { ref } from 'vue'
import { defineStore } from 'pinia'

export type GlobalMessageType = 'info' | 'success' | 'error' | 'warning'

export interface GlobalMessageItem {
  id: number
  type: GlobalMessageType
  message: string
}

let nextMessageId = 1
const messageTimers = new Map<number, number>()

export const useGlobalMessageStore = defineStore('global-message', () => {
  const items = ref<GlobalMessageItem[]>([])

  function show(message: string, type: GlobalMessageType = 'info', duration = 5000): number {
    const id = nextMessageId++
    items.value.push({ id, type, message })

    if (duration > 0) {
      const timer = window.setTimeout(() => remove(id), duration)
      messageTimers.set(id, timer)
    }

    return id
  }

  function info(message: string, duration = 5000): number {
    return show(message, 'info', duration)
  }

  function success(message: string, duration = 5000): number {
    return show(message, 'success', duration)
  }

  function error(message: string, duration = 5000): number {
    return show(message, 'error', duration)
  }

  function warning(message: string, duration = 5000): number {
    return show(message, 'warning', duration)
  }

  function remove(id: number): void {
    const timer = messageTimers.get(id)
    if (timer !== undefined) {
      window.clearTimeout(timer)
      messageTimers.delete(id)
    }
    items.value = items.value.filter((item) => item.id !== id)
  }

  function clear(): void {
    for (const timer of messageTimers.values()) window.clearTimeout(timer)
    messageTimers.clear()
    items.value = []
  }

  return { items, show, info, success, error, warning, remove, clear }
})
