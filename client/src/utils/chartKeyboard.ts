import { nextTick, ref } from 'vue'

type ReadableRef<T> = { readonly value: T }

type KeyboardChartRef = {
  dispatchAction: (payload: Record<string, unknown>) => void
}

type ChartMouseEvent = {
  dataIndex?: number
}

export function useChartKeyboardPointer(dataLength: ReadableRef<number>) {
  const chartRef = ref<KeyboardChartRef | null>(null)
  const activeIndex = ref(0)

  function setChartRef(instance: unknown) {
    chartRef.value = isKeyboardChartRef(instance) ? instance : null
  }

  function clampIndex(index: number): number {
    const lastIndex = Math.max(dataLength.value - 1, 0)
    return Math.min(Math.max(index, 0), lastIndex)
  }

  function showPointer(index = activeIndex.value) {
    if (dataLength.value <= 0) return

    activeIndex.value = clampIndex(index)

    void nextTick(() => {
      chartRef.value?.dispatchAction({
        type: 'showTip',
        seriesIndex: 0,
        dataIndex: activeIndex.value,
      })
    })
  }

  function handleChartKeydown(event: KeyboardEvent) {
    const step = getArrowStep(event.key)
    if (step === 0) return

    event.preventDefault()
    event.stopPropagation()
    showPointer(activeIndex.value + step)
  }

  function handleChartFocus() {
    showPointer(activeIndex.value)
  }

  function handleChartMousemove(event: ChartMouseEvent) {
    if (typeof event.dataIndex === 'number') {
      activeIndex.value = clampIndex(event.dataIndex)
    }
  }

  return {
    setChartRef,
    handleChartFocus,
    handleChartKeydown,
    handleChartMousemove,
  }
}

function isKeyboardChartRef(value: unknown): value is KeyboardChartRef {
  return (
    typeof value === 'object' &&
    value !== null &&
    'dispatchAction' in value &&
    typeof (value as KeyboardChartRef).dispatchAction === 'function'
  )
}

function getArrowStep(key: string): number {
  if (key === 'ArrowLeft' || key === 'ArrowUp') return -1
  if (key === 'ArrowRight' || key === 'ArrowDown') return 1
  return 0
}
