import { createRouter, createWebHistory } from 'vue-router'

import FactorDashboard from '@/views/FactorDashboard.vue'
import Mode1Preview from '@/views/Mode1Preview.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      redirect: '/mode1',
    },
    {
      path: '/mode1',
      name: 'mode1',
      component: FactorDashboard,
    },
    {
      path: '/mode1/:id',
      name: 'mode1-preview',
      component: Mode1Preview,
    },
  ],
})

export default router
