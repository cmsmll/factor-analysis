<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { NSpin } from 'naive-ui'

import { useGlobalLoadingStore } from '@/stores/globalLoading'

const store = useGlobalLoadingStore()
const { visible } = storeToRefs(store)
const dialog = ref<HTMLElement | null>(null)

watch(visible, async (value) => {
  if (!value) return
  await nextTick()
  dialog.value?.focus()
})

function blockKeyboard(event: KeyboardEvent) {
  if (!visible.value) return
  event.preventDefault()
  event.stopImmediatePropagation()
}

onMounted(() => document.addEventListener('keydown', blockKeyboard, true))
onBeforeUnmount(() => document.removeEventListener('keydown', blockKeyboard, true))
</script>

<template>
  <Teleport to="body">
    <Transition name="global-loading-fade">
      <div v-if="visible" class="global-loading-mask" @click.stop @wheel.prevent @touchmove.prevent>
        <section
          ref="dialog"
          class="global-loading-dialog"
          role="status"
          aria-modal="true"
          aria-label="加载中"
          tabindex="-1"
        >
          <NSpin :size="34" stroke="#3949ab" />
          <span>加载中</span>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.global-loading-mask {
  position: fixed;
  z-index: 10000;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(16, 22, 52, 0.32);
  cursor: wait;
}

.global-loading-dialog {
  width: 108px;
  height: 96px;
  display: grid;
  place-items: center;
  align-content: center;
  gap: 12px;
  border: 1px solid rgba(255, 255, 255, 0.72);
  border-radius: 8px;
  outline: none;
  background: #fff;
  box-shadow: 0 12px 36px rgba(9, 15, 48, 0.22);
}

.global-loading-dialog span {
  color: #4b5263;
  font-size: 14px;
}

.global-loading-fade-enter-active,
.global-loading-fade-leave-active {
  transition: opacity 180ms ease;
}

.global-loading-fade-enter-active .global-loading-dialog,
.global-loading-fade-leave-active .global-loading-dialog {
  transition: transform 180ms ease;
}

.global-loading-fade-enter-from,
.global-loading-fade-leave-to {
  opacity: 0;
}

.global-loading-fade-enter-from .global-loading-dialog,
.global-loading-fade-leave-to .global-loading-dialog {
  transform: translateY(8px) scale(0.98);
}
</style>
