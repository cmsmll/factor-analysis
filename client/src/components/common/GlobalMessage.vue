<script setup lang="ts">
import { storeToRefs } from 'pinia'

import { useGlobalMessageStore } from '@/stores/globalMessage'

const store = useGlobalMessageStore()
const { items } = storeToRefs(store)
</script>

<template>
  <Teleport to="body">
    <TransitionGroup name="global-message" tag="div" class="global-message-layer">
      <div
        v-for="item in items"
        :key="item.id"
        class="global-message"
        :class="`global-message-${item.type}`"
        role="status"
      >
        {{ item.message }}
      </div>
    </TransitionGroup>
  </Teleport>
</template>

<style scoped>
.global-message-layer {
  position: fixed;
  top: 50px;
  left: 50%;
  z-index: 11000;
  display: grid;
  gap: 8px;
  width: min(420px, calc(100vw - 32px));
  pointer-events: none;
  transform: translateX(-50%);
}

.global-message {
  min-height: 38px;
  padding: 8px 14px;
  border: 1px solid var(--message-border);
  border-radius: 6px;
  color: var(--message-text);
  background-color: var(--message-bg);
  font-size: 14px;
  font-weight: 700;
  line-height: 20px;
  text-align: center;
}

.global-message-info {
  --message-bg: rgb(64 158 255 / 0.12);
  --message-border: rgb(64 158 255 / 0.2);
  --message-text: #409eff;
}

.global-message-success {
  --message-bg: rgb(103 194 58 / 0.12);
  --message-border: rgb(103 194 58 / 0.2);
  --message-text: #67c23a;
}

.global-message-error {
  --message-bg: rgb(245 108 108 / 0.12);
  --message-border: rgb(245 108 108 / 0.2);
  --message-text: #f56c6c;
}

.global-message-warning {
  --message-bg: rgb(230 162 60 / 0.14);
  --message-border: rgb(230 162 60 / 0.22);
  --message-text: #e6a23c;
}

.global-message-enter-active,
.global-message-leave-active {
  transition:
    opacity 180ms ease,
    transform 180ms ease;
}

.global-message-enter-from,
.global-message-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
