import { ref } from 'vue'
import { defineStore } from 'pinia'

export const useGlobalLoadingStore = defineStore('global-loading', () => {
  const visible = ref(false)
  let pendingTasks = 0

  async function run<T>(task: () => Promise<T>): Promise<T> {
    pendingTasks += 1
    visible.value = true

    try {
      return await task()
    } finally {
      pendingTasks = Math.max(0, pendingTasks - 1)
      visible.value = pendingTasks > 0
    }
  }

  return { visible, run }
})
