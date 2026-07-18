<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'

import CloseIcon from '@/assets/icons/icon-close.svg'
import SearchIcon from '@/assets/icons/search.svg'
import { useGlobalFilterSelectorStore } from '@/stores/globalFilterSelector'

const store = useGlobalFilterSelectorStore()
const { visible, title, options, selected } = storeToRefs(store)
const keyword = ref('')

const filteredOptions = computed(() => {
  const value = keyword.value.trim().toLocaleLowerCase()
  if (!value) return options.value
  return options.value.filter((item) => item.toLocaleLowerCase().includes(value))
})
const selectedSet = computed(() => new Set(selected.value))

watch(visible, (value) => {
  if (value) keyword.value = ''
})

function toggle(option: string): void {
  const values = new Set(selected.value)
  if (values.has(option)) values.delete(option)
  else values.add(option)
  selected.value = [...values]
}

function selectAll(): void {
  selected.value = [...options.value]
}

function clear(): void {
  selected.value = []
}
</script>

<template>
  <Teleport to="body">
    <Transition name="filter-selector">
      <div v-if="visible" class="selector-mask" @click.self="store.cancel">
        <section class="selector-dialog" role="dialog" aria-modal="true" :aria-label="title">
          <header class="selector-header">
            <label class="search-box">
              <img :src="SearchIcon" class="search-icon" alt="" />
              <input v-model="keyword" type="search" :placeholder="`搜索${title}`" autofocus />
            </label>
            <button class="icon-button" type="button" aria-label="关闭" @click="store.cancel">
              <img :src="CloseIcon" alt="" />
            </button>
          </header>

          <div class="selector-summary">
            <strong>{{ title }}</strong>
            <span>已选择 {{ selected.length }} / {{ options.length }}</span>
          </div>

          <div class="selector-content">
            <button
              v-for="option in filteredOptions"
              :key="option"
              class="option-item"
              :class="{ selected: selectedSet.has(option) }"
              type="button"
              @click="toggle(option)"
            >
              <span class="option-check" aria-hidden="true">{{
                selectedSet.has(option) ? '✓' : ''
              }}</span>
              <span>{{ option }}</span>
            </button>
            <div v-if="filteredOptions.length === 0" class="empty-state">没有匹配的选项</div>
          </div>

          <footer class="selector-footer">
            <button type="button" class="footer-button select-all-button" @click="selectAll">
              全选
            </button>
            <button type="button" class="footer-button confirm-button" @click="store.confirm">
              确认
            </button>
            <button type="button" class="footer-button clear-button" @click="clear">清空</button>
          </footer>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.selector-mask {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgb(15 23 42 / 0.42);
  backdrop-filter: blur(2px);
}

.selector-dialog {
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr) auto;
  width: min(720px, 100%);
  height: min(620px, calc(100vh - 48px));
  overflow: hidden;
  border-radius: 12px;
  background: #fff;
  box-shadow: 0 24px 70px rgb(15 23 42 / 0.24);
}

.selector-header {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 18px 20px;
  border-bottom: 1px solid #ebeef5;
}

.search-box {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 10px;
  height: 38px;
  padding: 0 12px;
  border: 1px solid #dcdfe6;
  border-radius: 6px;
  color: #909399;
  transition: border-color 160ms ease;
}

.search-box:focus-within {
  border-color: #409eff;
}

.search-box input {
  width: 100%;
  border: 0;
  outline: 0;
  color: #303133;
  background: transparent;
  font: inherit;
}

.search-icon {
  width: 15px;
  height: 15px;
  flex-shrink: 0;
}

.icon-button {
  display: grid;
  width: 34px;
  height: 34px;
  place-items: center;
  border: 0;
  border-radius: 6px;
  color: #909399;
  background: transparent;
  cursor: pointer;
}

.icon-button:hover {
  color: #409eff;
  background: #ecf5ff;
}

.icon-button svg {
  width: 14px;
  height: 14px;
}

.selector-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px 8px;
  color: #303133;
}

.selector-summary span {
  color: #909399;
  font-size: 13px;
}

.selector-content {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  align-content: start;
  gap: 10px;
  margin: 8px 20px 18px;
  padding-right: 4px;
  overflow-y: auto;
}

.option-item {
  display: flex;
  align-items: center;
  gap: 9px;
  min-height: 38px;
  padding: 8px 10px;
  overflow: hidden;
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  color: #606266;
  background: #fff;
  cursor: pointer;
  text-align: left;
}

.option-item:hover,
.option-item.selected {
  border-color: #409eff;
  color: #409eff;
  background: #ecf5ff;
}

.option-check {
  display: grid;
  width: 16px;
  height: 16px;
  flex: 0 0 16px;
  place-items: center;
  border: 1px solid #c0c4cc;
  border-radius: 3px;
  color: #fff;
  font-size: 12px;
}

.selected .option-check {
  border-color: #409eff;
  background: #409eff;
}

.empty-state {
  grid-column: 1 / -1;
  padding: 72px 0;
  color: #909399;
  text-align: center;
}

.selector-footer {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  align-items: center;
  padding: 14px 20px;
  border-top: 1px solid #ebeef5;
}

.footer-button {
  min-width: 72px;
  height: 34px;
  border: 1px solid #dcdfe6;
  border-radius: 6px;
  color: #606266;
  background: #fff;
  cursor: pointer;
}

.select-all-button {
  justify-self: start;
}

.confirm-button {
  justify-self: center;
  border-color: #409eff;
  color: #fff;
  background: #409eff;
}

.clear-button {
  justify-self: end;
}

.filter-selector-enter-active,
.filter-selector-leave-active {
  transition: opacity 160ms ease;
}

.filter-selector-enter-active .selector-dialog,
.filter-selector-leave-active .selector-dialog {
  transition: transform 160ms ease;
}

.filter-selector-enter-from,
.filter-selector-leave-to {
  opacity: 0;
}

.filter-selector-enter-from .selector-dialog,
.filter-selector-leave-to .selector-dialog {
  transform: translateY(10px) scale(0.98);
}

@media (max-width: 640px) {
  .selector-mask {
    padding: 12px;
  }

  .selector-dialog {
    height: calc(100vh - 24px);
  }

  .selector-content {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
