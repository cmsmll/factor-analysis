<script setup lang="ts">
import { computed, ref } from 'vue'
import { NButton, NRadioGroup, NRadio } from 'naive-ui'
import { sumReturnPercentSeries, formatDate } from '@/utils/factorSeries'
import { useChartKeyboardPointer } from '@/utils/chartKeyboard'

const props = withDefaults(
  defineProps<{
    datetimes?: string[]
    changePercent?: number[][]
    quantileNames?: string[]
    quantileCount?: number
    loading?: boolean
  }>(),
  {
    datetimes: () => [],
    changePercent: () => [],
    quantileNames: () => [],
    quantileCount: 5,
    loading: false,
  },
)

const emit = defineEmits<{
  'update:quantileCount': [value: number]
}>()
const sortMode = ref(0)
const showIndex = ref(false)

const sortLabel = computed(() => {
  if (sortMode.value === 1) return '↓ 降序'
  if (sortMode.value === 2) return '↑ 升序'
  return '默认'
})

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

const selectedSeries = computed(() => props.changePercent)

// 暂时保留旧版指数按钮，指数数据接入后再展示基准曲线。
const indexReturnSeries = computed<number[]>(() => [])

const selectedNames = computed(() => props.quantileNames)
const { setChartRef, handleChartFocus, handleChartKeydown, handleChartMousemove } =
  useChartKeyboardPointer(computed(() => props.datetimes.length))

const option = computed(() => ({
  tooltip: {
    trigger: 'axis' as const,
    axisPointer: { type: 'cross' as const, snap: true },
    valueFormatter: (value: number) => `${value.toFixed(2)}%`,
    ...(sortMode.value !== 0
      ? { order: sortMode.value === 1 ? ('valueDesc' as const) : ('valueAsc' as const) }
      : {}),
  },
  legend: {
    bottom: 0,
    type: 'scroll' as const,
    data: selectedNames.value,
  },
  grid: { top: 20, left: 60, right: 30, bottom: 70 },
  xAxis: {
    type: 'category' as const,
    data: props.datetimes.map(formatDate),
  },
  yAxis: {
    type: 'value' as const,
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
  series: [
    ...selectedSeries.value.map((series, index) => ({
      name: selectedNames.value[index] ?? `切片${index + 1}`,
      type: 'line' as const,
      data: sumReturnPercentSeries(series),
      smooth: true,
      showSymbol: false,
      lineStyle: { width: index === 0 || index === selectedSeries.value.length - 1 ? 2 : 1.5 },
    })),
    {
      name: 'A股指数',
      type: 'line' as const,
      data: showIndex.value ? indexReturnSeries.value : [],
      smooth: true,
      showSymbol: false,
      lineStyle: { color: '#333', width: 2 },
      itemStyle: { color: '#333' },
    },
  ],
}))
</script>

<template>
  <div class="chart-card">
    <div class="chart-card-header">
      <span class="chart-title">分组收益曲线</span>
      <div class="header-controls">
        <NButton
          size="small"
          :type="showIndex ? 'primary' : 'default'"
          secondary
          @click="showIndex = !showIndex"
        >
          A股指数
        </NButton>
        <NButton class="sort-btn" size="tiny" quaternary @click="sortMode = (sortMode + 1) % 3">
          {{ sortLabel }}
        </NButton>
        <NRadioGroup
          :value="quantileCount"
          :disabled="loading"
          size="small"
          @update:value="emit('update:quantileCount', Number($event))"
        >
          <NRadio :value="3">三分位</NRadio>
          <NRadio :value="5">五分位</NRadio>
          <NRadio :value="10">十分位</NRadio>
        </NRadioGroup>
      </div>
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
}

.chart-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.header-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.sort-btn {
  font-size: 14px !important;
  color: rgb(51, 54, 57) !important;
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
