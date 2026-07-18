<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useRoute, useRouter } from 'vue-router'
import {
  NButton,
  NConfigProvider,
  NDatePicker,
  NForm,
  NFormItem,
  NRadioButton,
  NRadioGroup,
  NSelect,
  dateZhCN,
  zhCN,
} from 'naive-ui'

import RefreshIcon from '@/assets/icons/refresh.svg'
import { fetchIndices, fetchSectors } from '@/api/mode1'
import DecayChart from '@/components/visualization/DecayChart.vue'
import FactorMetrics from '@/components/visualization/FactorMetrics.vue'
import GroupNavChart from '@/components/visualization/GroupNavChart.vue'
import IcChart from '@/components/visualization/IcChart.vue'
import IndustryIcChart from '@/components/visualization/IndustryIcChart.vue'
import TurnoverChart from '@/components/visualization/TurnoverChart.vue'
import { buildDetail } from '@/features/mode1/detail'
import { useGlobalLoadingStore } from '@/stores/globalLoading'
import { useGlobalMessageStore } from '@/stores/globalMessage'
import { useGlobalFilterSelectorStore } from '@/stores/globalFilterSelector'
import { useMode1Store } from '@/stores/mode1'
import { useMode1PreviewStore } from '@/stores/mode1Preview'
import type { ModeRequest, ProfitMode, QuantileCount } from '@/types/mode1'
import { formatDate } from '@/utils/factorSeries'

const route = useRoute()
const router = useRouter()
const mode1Store = useMode1Store()
const previewStore = useMode1PreviewStore()
const globalLoading = useGlobalLoadingStore()
const globalMessage = useGlobalMessageStore()
const filterSelector = useGlobalFilterSelectorStore()
const { items, listError, periodError } = storeToRefs(mode1Store)
const { results, statuses, errors } = storeToRefs(previewStore)
const requestParams = ref<ModeRequest>()
const startDate = ref<number | null>(null)
const endDate = ref<number | null>(null)
const initialStartDate = ref<number | null>(null)
const initialEndDate = ref<number | null>(null)
const profitMode = ref<ProfitMode>(1)
const quantileCount = ref<number>(5)
const sectorOptions = ref<string[]>()
const indiceOptions = ref<string[]>()
let settingRequestDates = false
const profitModeOptions = [
  { label: '收益1：当天收盘买，第二天收盘卖', value: 1 },
  { label: '收益2：第二天开盘买，第二天收盘卖', value: 2 },
  { label: '收益3：第二天开盘买，第三天开盘卖', value: 3 },
  { label: '收益4：第二天开盘买，第三天收盘卖', value: 4 },
]

const modeId = computed(() => {
  const value = route.params.id
  return Array.isArray(value) ? (value[0] ?? '') : (value ?? '')
})
const factorData = computed(() => results.value[modeId.value])
const quantileLoading = computed(() => statuses.value[modeId.value] === 'loading')
const routeFactorName = computed(() => factorData.value?.name || '因子预览')
const factorDetail = computed(() => buildDetail(factorData.value, null, null, profitMode.value))
const stats = computed(() => factorDetail.value.stats)
const previewFilter = computed(() => requestParams.value?.base.filter)

watch(
  modeId,
  () => {
    requestParams.value = undefined
    startDate.value = null
    endDate.value = null
    initialStartDate.value = null
    initialEndDate.value = null
    quantileCount.value = 5
    void loadPreview()
  },
  { immediate: true },
)

watch(
  startDate,
  (value) => {
    if (settingRequestDates || value === null || endDate.value === null) return
    if (value > endDate.value) endDate.value = value
    syncRequestDates(value, endDate.value)
  },
  { flush: 'sync' },
)

watch(
  endDate,
  (value) => {
    if (settingRequestDates || value === null || startDate.value === null) return
    if (value < startDate.value) startDate.value = value
    syncRequestDates(startDate.value, value)
  },
  { flush: 'sync' },
)

async function loadPreview() {
  try {
    await globalLoading.run(async () => {
      await mode1Store.loadDefaultList()
      if (periodError.value) throw new Error(periodError.value)
      if (listError.value) throw new Error(listError.value)

      const source = items.value.find((item) => item.args.base.id === modeId.value)
      if (!source) throw new Error('没有找到对应的因子列表数据')

      const params = structuredClone(source.args)
      requestParams.value = params
      quantileCount.value = params.base.count
      setRequestDates(params)
      previewStore.initialize(modeId.value, source.data)
    })
  } catch (error) {
    globalMessage.error(errorMessage(error, '因子数据加载失败'))
  }
}

function isQuantileCount(value: number): value is QuantileCount {
  return value === 3 || value === 5 || value === 10
}

async function changeQuantileCount(value: number) {
  if (!isQuantileCount(value) || quantileLoading.value) return

  const params = requestParams.value
  if (!params || (factorData.value?.count === value && quantileCount.value === value)) return

  const previousCount = factorData.value?.count
  quantileCount.value = value
  params.base.count = value

  try {
    await globalLoading.run(async () => {
      await previewStore.loadMode(params, true)
      if (statuses.value[modeId.value] === 'error') {
        throw new Error(errors.value[modeId.value] ?? '分位数据计算失败')
      }
    })
  } catch (error) {
    if (previousCount && isQuantileCount(previousCount)) {
      quantileCount.value = previousCount
      params.base.count = previousCount
    }
    globalMessage.error(errorMessage(error, `${quantileLabel(value)}数据计算失败`))
  }
}

async function reloadPreview(): Promise<void> {
  const params = requestParams.value
  if (!params || quantileLoading.value) return

  try {
    await requestPreview(params, '因子数据重载失败')
  } catch (error) {
    globalMessage.error(errorMessage(error, '因子数据重载失败'))
  }
}

async function requestPreview(params: ModeRequest, fallback: string): Promise<void> {
  await globalLoading.run(async () => {
    await previewStore.loadMode(params, true)
    if (statuses.value[modeId.value] === 'error') {
      throw new Error(errors.value[modeId.value] ?? fallback)
    }
  })
}

async function selectSectors(): Promise<void> {
  const filter = previewFilter.value
  if (!filter) return

  try {
    if (!sectorOptions.value) {
      await globalLoading.run(async () => {
        sectorOptions.value = await fetchSectors()
      })
    }
    const result = await filterSelector.open({
      title: '行业板块',
      options: sectorOptions.value ?? [],
      selected: filter.sector,
    })
    if (result) filter.sector = result
  } catch (error) {
    globalMessage.error(errorMessage(error, '行业板块加载失败'))
  }
}

async function selectIndices(): Promise<void> {
  const filter = previewFilter.value
  if (!filter) return

  try {
    if (!indiceOptions.value) {
      await globalLoading.run(async () => {
        indiceOptions.value = await fetchIndices()
      })
    }
    const result = await filterSelector.open({
      title: '指数列表',
      options: indiceOptions.value ?? [],
      selected: filter.indice,
    })
    if (result) filter.indice = result
  } catch (error) {
    globalMessage.error(errorMessage(error, '指数列表加载失败'))
  }
}

function quantileLabel(value: QuantileCount): string {
  if (value === 3) return '三分位'
  if (value === 5) return '五分位'
  return '十分位'
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback
}
function toDateValue(value?: string): number | null {
  if (!value) return null
  const timestamp = new Date(formatDate(value)).getTime()
  return Number.isFinite(timestamp) ? timestamp : null
}

function resetStartDate() {
  resetRequestDates()
}

function resetEndDate() {
  resetRequestDates()
}

function setRequestDates(params: ModeRequest) {
  settingRequestDates = true
  startDate.value = toDateValue(params.base.filter.start)
  endDate.value = toDateValue(params.base.filter.end)
  initialStartDate.value = startDate.value
  initialEndDate.value = endDate.value
  settingRequestDates = false
}

function syncRequestDates(start: number, end: number): void {
  const filter = previewFilter.value
  if (!filter) return
  filter.start = toDateString(start)
  filter.end = toDateString(end)
}

function toDateString(timestamp: number): string {
  const date = new Date(timestamp)
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

function resetRequestDates() {
  settingRequestDates = true
  startDate.value = initialStartDate.value
  endDate.value = initialEndDate.value
  settingRequestDates = false
  if (startDate.value !== null && endDate.value !== null) {
    syncRequestDates(startDate.value, endDate.value)
  }
}

function goBack() {
  void router.push({ name: 'mode1' })
}
</script>

<template>
  <div class="visualization-layout">
    <!-- 页首 -->
    <div class="page-title-bar">
      <NButton text size="small" @click="goBack" class="back-btn">
        <template #icon>
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" class="back-icon">
            <path
              fill="none"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="48"
              d="M244 400L100 256l144-144"
            ></path>
            <path
              fill="none"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="48"
              d="M120 256h292"
            ></path>
          </svg>
        </template>
        返回
      </NButton>
      <h2 class="factor-title">{{ routeFactorName || factorDetail.factorName }}</h2>
      <NButton
        type="primary"
        color="#409eff"
        size="small"
        class="reload-btn"
        :loading="quantileLoading"
        @click="reloadPreview"
      >
        <template #icon><img :src="RefreshIcon" alt="" class="reload-icon" /></template>
        重载
      </NButton>
    </div>

    <div v-if="previewFilter" class="filter-bar">
      <NForm layout="inline" label-placement="left" size="small">
        <NFormItem label="过滤ST">
          <NRadioGroup v-model:value="previewFilter.filter_st" size="small">
            <NRadioButton :value="false">否</NRadioButton>
            <NRadioButton :value="true">是</NRadioButton>
          </NRadioGroup>
        </NFormItem>
        <NFormItem label="过滤北证">
          <NRadioGroup v-model:value="previewFilter.filter_bz" size="small">
            <NRadioButton :value="false">否</NRadioButton>
            <NRadioButton :value="true">是</NRadioButton>
          </NRadioGroup>
        </NFormItem>
        <NFormItem label="行业板块">
          <NButton size="small" class="selector-button" @click="selectSectors">
            {{
              previewFilter.sector.length ? `已选 ${previewFilter.sector.length} 项` : '全部行业'
            }}
          </NButton>
        </NFormItem>
        <NFormItem label="指数列表">
          <NButton size="small" class="selector-button" @click="selectIndices">
            {{
              previewFilter.indice.length ? `已选 ${previewFilter.indice.length} 项` : '全部指数'
            }}
          </NButton>
        </NFormItem>
      </NForm>
    </div>

    <div class="stats-header">
      <div class="stat-item date-range-item">
        <span class="stat-label">数据区间</span>
        <NConfigProvider :locale="zhCN" :date-locale="dateZhCN">
          <div class="date-picker-wrap">
            <NDatePicker
              v-model:value="startDate"
              type="date"
              size="small"
              :clearable="false"
              to=".visualization-layout"
            >
              <template #now>
                <NButton size="tiny" class="date-reset-btn" @click.stop.prevent="resetStartDate">
                  复位
                </NButton>
              </template>
            </NDatePicker>
            <NDatePicker
              v-model:value="endDate"
              type="date"
              size="small"
              :clearable="false"
              to=".visualization-layout"
            >
              <template #now>
                <NButton size="tiny" class="date-reset-btn" @click.stop.prevent="resetEndDate">
                  复位
                </NButton>
              </template>
            </NDatePicker>
          </div>
        </NConfigProvider>
      </div>
      <div v-for="stat in stats" :key="stat.label" class="stat-item">
        <span class="stat-label">{{ stat.label }}</span>
        <span class="stat-value">{{ stat.value }}</span>
      </div>
      <div class="stat-item profit-mode-item">
        <span class="stat-label">收益模式</span>
        <NSelect
          v-model:value="profitMode"
          :options="profitModeOptions"
          size="small"
          class="profit-mode-select"
          :consistent-menu-width="false"
        />
      </div>
    </div>

    <FactorMetrics :metrics="factorDetail.metrics" />
    <GroupNavChart
      :datetimes="factorDetail.datetimes"
      :change-percent="factorDetail.changePercent"
      :quantile-names="factorDetail.quantileNames"
      :quantile-count="quantileCount"
      :loading="quantileLoading"
      @update:quantile-count="changeQuantileCount"
    />

    <div class="chart-row">
      <IcChart
        :datetimes="factorDetail.datetimes"
        :factor="factorDetail.factor"
        :quantile-names="factorDetail.quantileNames"
      />
      <IndustryIcChart :factor="factorDetail.factor" :quantile-names="factorDetail.quantileNames" />
    </div>

    <div class="chart-row">
      <TurnoverChart
        :datetimes="factorDetail.datetimes"
        :turnover-rate="factorDetail.turnoverRate"
        :quantile-names="factorDetail.quantileNames"
      />
      <DecayChart
        :change-percent="factorDetail.changePercent"
        :quantile-names="factorDetail.quantileNames"
      />
    </div>
  </div>
</template>

<style scoped>
.visualization-layout {
  padding: 24px 32px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.page-title-bar {
  display: flex;
  align-items: center;
  background: #fff;
  padding: 14px 24px;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  gap: 16px;
}

.back-btn {
  flex-shrink: 0;
  color: var(--n-text-color);
  font-size: 13px;
}

.back-icon {
  width: 16px;
  height: 16px;
}

.factor-title {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  color: #1a237e;
  flex: 1;
  text-align: center;
}

.reload-btn {
  min-width: 76px;
  flex-shrink: 0;
}

.reload-icon {
  width: 14px;
  height: 14px;
  filter: brightness(0) invert(1);
}

.filter-bar {
  padding: 16px 20px;
  border-radius: 8px;
  background: #fff;
  box-shadow: 0 1px 3px rgb(0 0 0 / 0.06);
}

.filter-bar :deep(.n-form) {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px 28px;
}

.filter-bar :deep(.n-form-item) {
  margin-bottom: 0;
}

.filter-bar :deep(.n-form-item-feedback-wrapper) {
  display: none;
}

.selector-button {
  min-width: 112px;
  color: #409eff;
}

.stats-header {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
}

.stat-item {
  background: #fff;
  border-radius: 8px;
  padding: 16px 20px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.date-range-item {
  padding: 10px 12px;
}

.date-picker-wrap {
  display: flex;
  flex-direction: row;
  gap: 8px;
}

.date-picker-wrap :deep(.n-date-picker) {
  width: 156px;
}

.date-picker-wrap :deep(.n-date-panel-actions) {
  justify-content: center;
}

.date-picker-wrap :deep(.n-date-panel-actions__prefix) {
  display: none;
}

.date-picker-wrap :deep(.n-date-panel-actions__suffix) {
  align-self: center;
  margin-bottom: 0;
}

.date-picker-wrap :deep(.date-reset-btn) {
  min-width: 56px;
}

.profit-mode-item {
  min-width: 320px;
}

.profit-mode-select {
  min-width: 280px;
}

.stat-label {
  font-size: 14px;
  color: rgb(31, 34, 37);
}

.stat-value {
  font-size: 20px;
  font-weight: 700;
  color: rgb(31, 34, 37);
}

.chart-row {
  display: flex;
  gap: 16px;
}
</style>
