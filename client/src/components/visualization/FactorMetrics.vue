<script setup lang="ts">
import { computed, ref } from 'vue'
import type { FactorMetric } from '@/features/mode1/detail'
import { formatPercent, rateColor } from '@/utils/tools'

const props = withDefaults(defineProps<{ metrics?: FactorMetric[] }>(), {
  metrics: () => [],
})

const sortKey = ref<string>('quantile')
const sortAsc = ref(true)

function toggleSort(key: string) {
  if (sortKey.value === key) {
    sortAsc.value = !sortAsc.value
  } else {
    sortKey.value = key
    sortAsc.value = false
  }
}

const sortedMetrics = computed(() => {
  const sorted = [...props.metrics]

  if (sortKey.value === 'quantile') {
    sorted.sort((a, b) => {
      const na = extractQuantileIndex(a.quantile)
      const nb = extractQuantileIndex(b.quantile)
      return sortAsc.value ? na - nb : nb - na
    })
    return sorted
  }

  sorted.sort((a, b) => {
    const va = a[sortKey.value as keyof FactorMetric]
    const vb = b[sortKey.value as keyof FactorMetric]
    const na = typeof va === 'number' ? va : 0
    const nb = typeof vb === 'number' ? vb : 0
    return sortAsc.value ? na - nb : nb - na
  })
  return sorted
})

function extractQuantileIndex(name: string): number {
  const match = name.match(/\d+/)
  return match ? Number(match[0]) : 0
}

function sortIcon(key: string): string {
  if (sortKey.value !== key) return ' ↕'
  return sortAsc.value ? ' ↑' : ' ↓'
}

function formatSharpeRatio(value: number): string {
  return value.toFixed(2)
}

function formatNav(value: number): string {
  if (value >= 1000000000000) {
    return (value / 1000000000000).toFixed(2) + '万亿'
  }
  if (value >= 100000000) {
    return (value / 100000000).toFixed(2) + '亿'
  }
  if (value >= 10000) {
    return (value / 10000).toFixed(2) + '万'
  }
  return value.toFixed(4)
}
</script>

<template>
  <div class="metrics-card">
    <div class="metrics-table">
      <div class="metrics-row metrics-header-row">
        <span class="metrics-label sortable" @click="toggleSort('quantile')">
          分位{{ sortIcon('quantile') }}
        </span>
        <span class="metrics-col sortable" @click="toggleSort('returnRate')">
          收益率{{ sortIcon('returnRate') }}
        </span>
        <span class="metrics-col sortable" @click="toggleSort('annualizedReturn')">
          年化收益{{ sortIcon('annualizedReturn') }}
        </span>
        <span class="metrics-col sortable" @click="toggleSort('nav')">
          净值{{ sortIcon('nav') }}
        </span>
        <span class="metrics-col sortable" @click="toggleSort('maxDrawdown')">
          最大回撤{{ sortIcon('maxDrawdown') }}
        </span>
        <span class="metrics-col sortable" @click="toggleSort('maxDrawdownDate')">
          回撤日期{{ sortIcon('maxDrawdownDate') }}
        </span>
        <span class="metrics-col sortable" @click="toggleSort('sharpeRatio')">
          夏普比率{{ sortIcon('sharpeRatio') }}
        </span>
        <span class="metrics-col sortable" @click="toggleSort('factorValue')">
          因子值{{ sortIcon('factorValue') }}
        </span>
        <span class="metrics-col sortable" @click="toggleSort('turnoverRate')">
          换手率{{ sortIcon('turnoverRate') }}
        </span>
      </div>
      <div v-for="metric in sortedMetrics" :key="metric.quantile" class="metrics-row">
        <span class="metrics-label quantile-tag">{{ metric.quantile }}</span>
        <span class="metrics-col" :style="{ color: rateColor(metric.returnRate) }">
          {{ formatPercent(metric.returnRate) }}
        </span>
        <span class="metrics-col" :style="{ color: rateColor(metric.annualizedReturn) }">
          {{ formatPercent(metric.annualizedReturn) }}
        </span>
        <span class="metrics-col" :style="{ color: rateColor(metric.nav - 1) }">
          {{ formatNav(metric.nav) }}
        </span>
        <span class="metrics-col" :style="{ color: rateColor(metric.maxDrawdown) }">
          {{ formatPercent(metric.maxDrawdown) }}
        </span>
        <span class="metrics-col metrics-date">{{ metric.maxDrawdownDate }}</span>
        <span class="metrics-col">{{ formatSharpeRatio(metric.sharpeRatio) }}</span>
        <span class="metrics-col">
          {{ formatNav(metric.factorValue) }}
        </span>
        <span class="metrics-col">{{ formatPercent(metric.turnoverRate) }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.metrics-card {
  background: #fff;
  border-radius: 8px;
  padding: 16px 24px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  overflow-x: auto;
}

.metrics-table {
  display: flex;
  flex-direction: column;
  min-width: max-content;
}

.metrics-row {
  display: grid;
  grid-template-columns: 1fr repeat(8, minmax(80px, 110px));
  align-items: center;
  padding: 6px 0;
  font-size: 13px;
  column-gap: 6px;
}

.metrics-header-row {
  font-weight: 600;
  color: #666;
  border-bottom: 1px solid #efeff5;
  padding-bottom: 8px;
}

.metrics-row:not(.metrics-header-row):hover {
  background: #f5f7fa;
  border-radius: 4px;
}

.metrics-label {
  color: #666;
  padding-left: 20px;
}

.metrics-col {
  text-align: center;
}

.sortable {
  cursor: pointer;
  user-select: none;
}

.sortable:hover {
  color: #5470c6;
}

.metrics-date {
  font-size: 12px;
  color: #888;
}

.quantile-tag {
  font-weight: 500;
  font-size: 12px;
}
</style>
