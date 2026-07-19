<script setup lang="ts">
import { RouterView } from 'vue-router'

import GlobalFilterSelector from '@/components/common/GlobalFilterSelector.vue'
import GlobalLoading from '@/components/common/GlobalLoading.vue'
import GlobalMessage from '@/components/common/GlobalMessage.vue'
</script>

<template>
  <RouterView v-slot="{ Component, route }">
    <KeepAlive include="FactorDashboard">
      <component :is="Component" v-if="route.name === 'mode1'" />
    </KeepAlive>
    <component :is="Component" v-if="route.name !== 'mode1'" :key="route.fullPath" />
  </RouterView>
  <GlobalFilterSelector />
  <GlobalLoading />
  <GlobalMessage />
</template>
