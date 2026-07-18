<script setup lang="ts">
import { computed } from 'vue'
import { average, round } from '@/utils/factorSeries'
import { useChartKeyboardPointer } from '@/utils/chartKeyboard'

const props = withDefaults(
  defineProps<{
    factor?: number[][]
    quantileNames?: string[]
  }>(),
  {
    factor: () => [],
    quantileNames: () => [],
  },
)

const chartData = computed(() => {
  return props.factor.map((series, index) => {
    const value = round(average(series), 4)

    return {
      name: props.quantileNames[index] ?? `切片${index + 1}`,
      value,
      itemStyle: { color: value >= 0 ? '#5470c6' : '#91cc75' },
    }
  })
})

const axisLimit = computed(() => {
  const maxAbsoluteValue = Math.max(...chartData.value.map((item) => Math.abs(item.value)), 1)

  return Math.ceil(maxAbsoluteValue * 10) / 10
})
const { setChartRef, handleChartFocus, handleChartKeydown, handleChartMousemove } =
  useChartKeyboardPointer(computed(() => chartData.value.length))

const option = computed(() => ({
  tooltip: { trigger: 'axis' as const, axisPointer: { type: 'cross' as const, snap: true } },
  grid: { top: 10, left: 100, right: 40, bottom: 20 },
  xAxis: {
    type: 'value' as const,
    min: -axisLimit.value,
    max: axisLimit.value,
  },
  yAxis: {
    type: 'category' as const,
    data: chartData.value.map((item) => item.name),
    inverse: true,
  },
  series: [
    {
      type: 'bar' as const,
      data: chartData.value,
      barMaxWidth: 16,
    },
  ],
}))
</script>

<template>
  <div class="chart-card">
    <div class="chart-card-header">
      <span class="chart-title">各切片平均因子值</span>
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
