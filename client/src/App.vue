<script setup lang="ts">
import { RouterView } from 'vue-router'

import GlobalFilterSelector from '@/components/common/GlobalFilterSelector.vue'
import GlobalLoading from '@/components/common/GlobalLoading.vue'
import GlobalMessage from '@/components/common/GlobalMessage.vue'
</script>

<template>
  <RouterView />
  <GlobalFilterSelector />
  <GlobalLoading />
  <GlobalMessage />
</template>
