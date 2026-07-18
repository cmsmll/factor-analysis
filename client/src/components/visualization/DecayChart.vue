<script setup lang="ts">
import { computed } from 'vue'
import { average, round } from '@/utils/factorSeries'
import { useChartKeyboardPointer } from '@/utils/chartKeyboard'

const props = withDefaults(
  defineProps<{
    changePercent?: number[][]
    quantileNames?: string[]
  }>(),
  {
    changePercent: () => [],
    quantileNames: () => [],
  },
)

const decayDays = 10
const bestSeries = computed(() => findExtremeSeries('max'))
const worstSeries = computed(() => findExtremeSeries('min'))
const { setChartRef, handleChartFocus, handleChartKeydown, handleChartMousemove } =
  useChartKeyboardPointer(computed(() => decayDays))

const option = computed(() => ({
  tooltip: {
    trigger: 'axis' as const,
    axisPointer: { type: 'cross' as const, snap: true },
    valueFormatter: (value: number) => `${value.toFixed(2)}%`,
  },
  legend: {
    data: [worstSeries.value.name, bestSeries.value.name],
    bottom: 0,
  },
  grid: { top: 20, left: 50, right: 30, bottom: 50 },
  xAxis: {
    type: 'category' as const,
    data: Array.from({ length: decayDays }, (_, index) => `T+${index + 1}`),
  },
  yAxis: {
    type: 'value' as const,
    axisLabel: { formatter: '{value}%' },
  },
  color: ['#91cc75', '#fac858'],
  series: [
    {
      name: worstSeries.value.name,
      type: 'line' as const,
      data: worstSeries.value.data,
      smooth: true,
      lineStyle: { width: 2 },
    },
    {
      name: bestSeries.value.name,
      type: 'line' as const,
      data: bestSeries.value.data,
      smooth: true,
      lineStyle: { width: 2, type: 'dashed' as const },
    },
  ],
}))

function findExtremeSeries(type: 'min' | 'max') {
  if (props.changePercent.length === 0) {
    return { name: '--', data: [] as number[] }
  }

  const averageReturns = props.changePercent.map(average)
  const index = averageReturns.reduce((bestIndex, value, currentIndex) => {
    const bestValue = averageReturns[bestIndex]
    if (typeof bestValue !== 'number') return currentIndex

    return type === 'max'
      ? value > bestValue
        ? currentIndex
        : bestIndex
      : value < bestValue
        ? currentIndex
        : bestIndex
  }, 0)

  const source = props.changePercent[index] ?? []

  return {
    name: props.quantileNames[index] ?? `切片${index + 1}`,
    data: Array.from({ length: decayDays }, (_, offset) => {
      const groupedReturns = source.slice(offset).filter((_, index) => index % decayDays === 0)

      return round(average(groupedReturns) * 100, 4)
    }),
  }
}
</script>

<template>
  <div class="chart-card">
    <div class="chart-card-header">
      <span class="chart-title">买入信号衰减分析</span>
    </div>
    <VChart
      :ref="setChartRef"
      class="chart-body"
      :option="option"
      autoresize
      tabindex="0"
      @focus="handleChartFocus"
      @keydown="handleChartKeydown"
      @mousemove="handleChartMousemove"
    />
  </div>
</template>

<style scoped>
.chart-card {
  background: #fff;
  border-radius: 8px;
  padding: 16px 20px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  flex: 1;
  min-width: 0;
}

.chart-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.chart-title {
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.chart-body {
  width: 100%;
  height: 450px;
}
</style>
