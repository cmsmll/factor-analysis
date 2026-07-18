import { ref } from 'vue'
import { defineStore } from 'pinia'

export interface FilterSelectorOptions {
  title: string
  options: string[]
  selected: string[]
}

export const useGlobalFilterSelectorStore = defineStore('global-filter-selector', () => {
  const visible = ref(false)
  const title = ref('')
  const options = ref<string[]>([])
  const selected = ref<string[]>([])
  let resolveSelection: ((value: string[] | undefined) => void) | undefined

  function open(config: FilterSelectorOptions): Promise<string[] | undefined> {
    cancel()
    title.value = config.title
    options.value = [...config.options]
    selected.value = config.selected.filter((item) => config.options.includes(item))
    visible.value = true

    return new Promise((resolve) => {
      resolveSelection = resolve
    })
  }

  function confirm(): void {
    close([...selected.value])
  }

  function cancel(): void {
    close(undefined)
  }

  function close(value: string[] | undefined): void {
    if (!visible.value && !resolveSelection) return
    visible.value = false
    const resolve = resolveSelection
    resolveSelection = undefined
    resolve?.(value)
  }

  return { visible, title, options, selected, open, confirm, cancel }
})
