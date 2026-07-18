<script setup lang="ts">
import { computed } from 'vue'
import { formatDate, percentSeries } from '@/utils/factorSeries'
import { useChartKeyboardPointer } from '@/utils/chartKeyboard'

const props = withDefaults(
  defineProps<{
    datetimes?: string[]
    turnoverRate?: number[][]
    quantileNames?: string[]
  }>(),
  {
    datetimes: () => [],
    turnoverRate: () => [],
    quantileNames: () => [],
  },
)

const COLORS = [
  '#91cc75',
  '#73c0de',
  '#5470c6',
  '#ee6666',
  '#fac858',
  '#e74c3c',
  '#2ecc71',
  '#3498db',
  '#9b59b6',
  '#34495e',
]

const seriesNames = computed(() => {
  return props.turnoverRate.map((_, index) => props.quantileNames[index] ?? `切片${index + 1}`)
})
const defaultLegendSelected = computed(() => {
  const lastIndex = seriesNames.value.length - 1
  const selected: Record<string, boolean> = {}

  seriesNames.value.forEach((name, index) => {
    selected[name] = index === 0 || index === lastIndex
  })

  return selected
})
const { setChartRef, handleChartFocus, handleChartKeydown, handleChartMousemove } =
  useChartKeyboardPointer(computed(() => props.datetimes.length))

const option = computed(() => ({
  tooltip: {
    trigger: 'axis' as const,
    axisPointer: { type: 'cross' as const, snap: true },
    valueFormatter: (value: number) => `${value.toFixed(2)}%`,
  },
  legend: {
    data: seriesNames.value,
    selected: defaultLegendSelected.value,
    type: 'scroll' as const,
    bottom: 0,
  },
  grid: { top: 20, left: 50, right: 30, bottom: 70 },
  xAxis: {
    type: 'category' as const,
    data: props.datetimes.map(formatDate),
  },
  yAxis: {
    type: 'value' as const,
    scale: true,
    axisLabel: { formatter: '{value}%' },
  },
  dataZoom: [
    { type: 'inside' as const, start: 0, end: 100 },
    {
      type: 'slider' as const,
      height: 8,
      bottom: 30,
      disabled: true,
      showDetail: false,
      showDataShadow: false,
      brushSelect: false,
      handleSize: 0,
      moveHandleSize: 0,
      borderColor: 'transparent',
      backgroundColor: '#edf1f7',
      fillerColor: '#8aa4d6',
      handleStyle: { opacity: 0 },
      moveHandleStyle: { opacity: 0 },
      emphasis: {
        handleStyle: { opacity: 0 },
        moveHandleStyle: { opacity: 0 },
      },
    },
  ],
  color: COLORS,
  series: props.turnoverRate.map((series, index) => ({
    name: seriesNames.value[index] ?? `切片${index + 1}`,
    type: 'line' as const,
    data: percentSeries(series),
    lineStyle: { width: index === 0 || index === props.turnoverRate.length - 1 ? 2 : 1.5 },
    showSymbol: false,
  })),
}))
</script>

<template>
  <div class="chart-card">
    <div class="chart-card-header">
      <span class="chart-title">切片换手率时序图</span>
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
