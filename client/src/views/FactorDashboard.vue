<script setup lang="ts">
import { computed, h, onMounted, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import {
  NButton,
  NConfigProvider,
  NDataTable,
  NForm,
  NFormItem,
  NInput,
  NPagination,
  NSelect,
  type DataTableColumns,
} from 'naive-ui'

import { useGlobalLoadingStore } from '@/stores/globalLoading'
import { useGlobalMessageStore } from '@/stores/globalMessage'
import { createModeFilter, useMode1Store } from '@/stores/mode1'
import type { ModeRequest, Profit, ProfitMode, QuantileData } from '@/types/mode1'

defineOptions({ name: 'FactorDashboard' })

interface FactorRow {
  id: string
  index: number
  params: ModeRequest
  data?: QuantileData
}

const router = useRouter()
const store = useMode1Store()
const globalLoading = useGlobalLoadingStore()
const globalMessage = useGlobalMessageStore()
const { periods, items, periodLoading, listLoading, periodError, listError } = storeToRefs(store)
const { visible: globalLoadingVisible } = storeToRefs(globalLoading)
const searchKeyword = ref('')
const page = ref(1)
const pageSize = ref(10)
const sortKey = ref<string | null>(null)
const sortOrder = ref<'ascend' | 'descend' | false>(false)

let settingInitialPeriod = false
const filters = reactive({
  strategy: '纯多头组合',
  index: '沪深300',
  period: '',
  profitMode: 1 as ProfitMode,
})

const strategyOptions = [{ label: '纯多头组合', value: '纯多头组合' }]
const indexOptions = [
  { label: '沪深300', value: '沪深300' },
  { label: '中证500', value: '中证500' },
  { label: '中证1000', value: '中证1000' },
]
const periodOptions = computed(() =>
  periods.value.map((period) => ({ label: period.name, value: period.name })),
)
const profitModeOptions = [
  { label: '收益1：当天收盘买，第二天收盘卖', value: 1 },
  { label: '收益2：第二天开盘买，第二天收盘卖', value: 2 },
  { label: '收益3：第二天开盘买，第三天开盘卖', value: 3 },
  { label: '收益4：第二天开盘买，第三天收盘卖', value: 4 },
]
const localPeriodLoading = computed(() => periodLoading.value && !globalLoadingVisible.value)
const localListLoading = computed(() => listLoading.value && !globalLoadingVisible.value)

const itemCount = computed(() => items.value.length)
const pageSizeOptions = computed(() => {
  const max = Math.ceil(itemCount.value / 10) * 10
  return Array.from({ length: max / 10 }, (_, index) => (index + 1) * 10)
})

const visibleItems = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return items.value.slice(start, start + pageSize.value)
})

const rows = computed<FactorRow[]>(() => {
  const offset = (page.value - 1) * pageSize.value
  const keyword = searchKeyword.value.trim().toLowerCase()
  const data = visibleItems.value.map((item, index) => ({
    id: item.args.base.id,
    index: offset + index + 1,
    params: item.args,
    data: item.data,
  }))
  const filtered = keyword
    ? data.filter((row) => factorName(row).toLowerCase().includes(keyword))
    : data

  if (!sortKey.value || !sortOrder.value) return filtered

  return [...filtered].sort((left, right) => {
    const leftValue = sortableValue(left, sortKey.value!)
    const rightValue = sortableValue(right, sortKey.value!)
    return sortOrder.value === 'ascend' ? leftValue - rightValue : rightValue - leftValue
  })
})

watch(searchKeyword, () => {
  page.value = 1
})

watch(itemCount, (count) => {
  const lastPage = Math.max(1, Math.ceil(count / pageSize.value))
  if (page.value > lastPage) page.value = lastPage
})

watch(
  () => filters.period,
  (name) => {
    if (!name || settingInitialPeriod) return
    void loadPeriod(name)
  },
  { flush: 'sync' },
)

onMounted(() => void initializeMode1())

async function initializeMode1() {
  try {
    await globalLoading.run(async () => {
      await store.loadPeriods()
      if (periodError.value) throw new Error(periodError.value)

      const period = periods.value[0]
      if (!period) throw new Error('没有可用的时间周期配置')

      settingInitialPeriod = true
      filters.period = period.name
      settingInitialPeriod = false
      await store.loadList(createModeFilter(period))
      if (listError.value) throw new Error(listError.value)
    })
  } catch (error) {
    globalMessage.error(errorMessage(error, '获取模式一列表失败'))
  }
}

async function loadPeriod(name: string) {
  const period = periods.value.find((item) => item.name === name)
  if (!period) return

  page.value = 1
  try {
    await globalLoading.run(async () => {
      await store.loadList(createModeFilter(period))
      if (listError.value) throw new Error(listError.value)
    })
  } catch (error) {
    globalMessage.error(errorMessage(error, '获取模式一列表失败'))
  }
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback
}

function switchKanban(step: number) {
  // 保留旧版切换按钮，等待后续看板接入。
  void step
}

function handleSorterChange(
  sorter: { columnKey?: string; order?: 'ascend' | 'descend' | false } | null,
) {
  if (!sorter?.columnKey || !sorter.order) {
    sortKey.value = null
    sortOrder.value = false
    return
  }

  sortKey.value = sorter.columnKey
  sortOrder.value = sorter.order
}

function handlePageChange(value: number) {
  page.value = value
}

function handlePageSizeChange(value: number) {
  pageSize.value = value
  page.value = 1
}

function factorName(row: FactorRow): string {
  if (row.data?.name) return row.data.name
  return row.id
}

function first<T>(values: readonly T[]): T | undefined {
  return values[0]
}

function last<T>(values: readonly T[]): T | undefined {
  return values.length > 0 ? values[values.length - 1] : undefined
}

function average(values: readonly number[] | undefined): number | undefined {
  if (!values || values.length === 0) return undefined
  const valid = values.filter(Number.isFinite)
  if (valid.length === 0) return undefined
  return valid.reduce((sum, value) => sum + value, 0) / valid.length
}

function profitGroups(row: FactorRow): Profit[] {
  if (!row.data) return []

  switch (filters.profitMode) {
    case 2:
      return row.data.profit2
    case 3:
      return row.data.profit3
    case 4:
      return row.data.profit4
    default:
      return row.data.profit1
  }
}

function minAnnualized(row: FactorRow): number | undefined {
  return first(profitGroups(row))?.annualized_profit
}

function maxAnnualized(row: FactorRow): number | undefined {
  return last(profitGroups(row))?.annualized_profit
}

function minTurnover(row: FactorRow): number | undefined {
  return average(first(row.data?.turnover_rate ?? []))
}

function maxTurnover(row: FactorRow): number | undefined {
  return average(last(row.data?.turnover_rate ?? []))
}

function sortableValue(row: FactorRow, key: string): number {
  const values: Record<string, number | undefined> = {
    min_year_rate: minAnnualized(row),
    max_year_rate: maxAnnualized(row),
    min_turnover_rate: minTurnover(row),
    max_turnover_rate: maxTurnover(row),
  }
  return values[key] ?? 0
}

function formatPercent(value: number | undefined): string {
  if (value === undefined || !Number.isFinite(value)) return '--'
  return `${(value * 100).toFixed(2)}%`
}

function rateColor(value: number | undefined): string {
  if (value === undefined || value === 0) return '#1f2225'
  return value > 0 ? '#d03050' : '#18a058'
}

function renderRateCell(value: number | undefined) {
  return h('span', { style: { color: rateColor(value) } }, formatPercent(value))
}

function renderTurnoverCell(value: number | undefined) {
  return h('span', formatPercent(value))
}

const columns: DataTableColumns<FactorRow> = [
  {
    title: '序号',
    key: 'index',
    align: 'center',
    width: 60,
  },
  {
    key: 'factor_name',
    title: '因子名称',
    align: 'center',
    width: 300,
    ellipsis: { tooltip: true },
    render(row) {
      return h(
        'span',
        {
          style: { color: 'rgb(30, 70, 125)', cursor: 'pointer' },
          onClick: () => {
            void router.push({ name: 'mode1-preview', params: { id: row.id } })
          },
        },
        factorName(row),
      )
    },
  },
  {
    key: 'min_year_rate',
    title: '最小分位数\n年化收益率',
    align: 'center',
    sorter: true,
    width: 200,
    render: (row) => renderRateCell(minAnnualized(row)),
  },
  {
    key: 'max_year_rate',
    title: '最大分位数\n年化收益率',
    align: 'center',
    sorter: true,
    width: 200,
    render: (row) => renderRateCell(maxAnnualized(row)),
  },
  {
    key: 'min_turnover_rate',
    title: '最小分位数换手率',
    align: 'center',
    sorter: true,
    width: 170,
    render: (row) => renderTurnoverCell(minTurnover(row)),
  },
  {
    key: 'max_turnover_rate',
    title: '最大分位数换手率',
    align: 'center',
    sorter: true,
    width: 170,
    render: (row) => renderTurnoverCell(maxTurnover(row)),
  },
]

const rowKey = (row: FactorRow) => row.id
</script>

<template>
  <NConfigProvider>
    <div class="factorKanban-layout">
      <!-- 页首 -->
      <header class="page-header">
        <NButton text circle class="header-switch-btn" @click="switchKanban(-1)">
          <template #icon>
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
              <path
                fill="none"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="48"
                d="M328 112L184 256l144 144"
              ></path>
            </svg>
          </template>
        </NButton>
        <div class="header-content">
          <h1 class="page-title">因子看盘可视化显示</h1>
          <p class="page-subtitle">多因子量化分析平台</p>
        </div>
        <NButton text circle class="header-switch-btn" @click="switchKanban(1)">
          <template #icon>
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
              <path
                fill="none"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="48"
                d="M184 112l144 144-144 144"
              ></path>
            </svg>
          </template>
        </NButton>
      </header>

      <!-- 筛选区域 -->
      <div class="filter-bar">
        <NForm layout="inline" label-placement="left" size="small">
          <NFormItem label="因子搜索">
            <NInput
              v-model:value="searchKeyword"
              placeholder="输入因子名称搜索"
              clearable
              style="width: 180px"
            />
          </NFormItem>
          <NFormItem label="策略类型">
            <NSelect
              v-model:value="filters.strategy"
              :options="strategyOptions"
              style="width: 140px"
            />
          </NFormItem>
          <NFormItem label="指数范围">
            <NSelect v-model:value="filters.index" :options="indexOptions" style="width: 140px" />
          </NFormItem>
          <NFormItem label="时间周期">
            <NSelect
              v-model:value="filters.period"
              :options="periodOptions"
              :loading="localPeriodLoading"
              :disabled="periodLoading || listLoading"
              style="width: 140px"
            />
          </NFormItem>
          <NFormItem label="收益模式">
            <NSelect
              v-model:value="filters.profitMode"
              :options="profitModeOptions"
              :consistent-menu-width="false"
              style="width: 260px"
            />
          </NFormItem>
        </NForm>
      </div>

      <!-- 表格区域 -->
      <NDataTable
        :columns="columns"
        :data="rows"
        :row-key="rowKey"
        :loading="localListLoading"
        :bordered="true"
        :single-line="true"
        :pagination="false"
        :style="{
          'white-space': 'pre-wrap',
          '--n-font-size': '14px',
          '--n-th-padding': '12px',
          '--n-td-padding': '12px',
        }"
        size="small"
        class="factor-table"
        @update:sorter="handleSorterChange"
      />

      <!-- 分页 -->
      <div class="pagination-wrap">
        <NPagination
          :page="page"
          :page-size="pageSize"
          :item-count="itemCount"
          :page-sizes="pageSizeOptions"
          show-size-picker
          @update:page="handlePageChange"
          @update:page-size="handlePageSizeChange"
        />
      </div>

      <!-- 页尾 -->
      <footer class="page-footer">
        <span>因子看板 &copy; 2026</span>
      </footer>
    </div>
  </NConfigProvider>
</template>

<style scoped>
.factorKanban-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 32px;
  gap: 24px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: linear-gradient(135deg, #1a237e 0%, #283593 40%, #3949ab 100%);
  border-radius: 10px;
  box-shadow: 0 4px 20px rgba(26, 35, 126, 0.25);
  overflow: hidden;
  padding: 28px 32px;
}

.header-switch-btn {
  flex-shrink: 0;
  width: 40px;
  height: 40px;
  color: rgba(255, 255, 255, 0.88);
  background: rgba(255, 255, 255, 0.1);
}

.header-switch-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.18);
}

.header-switch-btn svg {
  width: 22px;
  height: 22px;
}

.header-content {
  flex: 1;
  text-align: center;
  position: relative;
}

.header-content::after {
  content: '';
  position: absolute;
  left: -80px;
  top: -50px;
  width: 140px;
  height: 140px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.04);
  pointer-events: none;
}

.header-content::before {
  content: '';
  position: absolute;
  right: -60px;
  bottom: -40px;
  width: 100px;
  height: 100px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.03);
  pointer-events: none;
}

.page-title {
  margin: 0;
  font-size: 30px;
  font-weight: 700;
  color: #fff;
  letter-spacing: 2px;
  position: relative;
}

.page-subtitle {
  margin: 6px 0 0;
  font-size: 13px;
  color: rgba(255, 255, 255, 0.65);
  letter-spacing: 4px;
}

.page-footer {
  display: flex;
  justify-content: center;
  padding: 12px 0;
  color: #999;
  font-size: 13px;
}

.filter-bar {
  background: #fff;
  padding: 16px 20px;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
}

.filter-bar :deep(.n-form) {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px 20px;
}

.filter-bar :deep(.n-form-item) {
  margin-bottom: 0;
}

.filter-bar :deep(.n-form-item-feedback-wrapper) {
  display: none;
}

.factor-table {
  background: #fff;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  padding: 0;
  flex: 1;
}

.pagination-wrap {
  display: flex;
  justify-content: center;
  padding: 4px 0;
}

.pagination-wrap :deep(.n-pagination-item--button) {
  --n-button-color: #fff;
}
</style>
