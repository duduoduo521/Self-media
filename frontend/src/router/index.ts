import { createRouter, createWebHashHistory } from 'vue-router'
import { useUserStore } from '@/stores/user'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/login',
      name: 'Login',
      component: () => import('@/views/Login.vue'),
    },
    {
      path: '/register',
      name: 'Register',
      component: () => import('@/views/Register.vue'),
    },
    {
      path: '/',
      component: () => import('@/layouts/MainLayout.vue'),
      meta: { requiresAuth: true },
      children: [
        { path: '', name: 'Dashboard', component: () => import('@/views/Dashboard.vue') },
        { path: 'hotspot', name: 'Hotspot', component: () => import('@/views/Hotspot.vue') },
        { path: 'create', name: 'Create', component: () => import('@/views/Create.vue') },
        { path: 'tasks', name: 'Tasks', component: () => import('@/views/Tasks.vue') },
        { path: 'drafts', name: 'Drafts', component: () => import('@/views/Drafts.vue') },
        { path: 'platforms', name: 'Platforms', component: () => import('@/views/Platforms.vue') },
        { path: 'settings', name: 'Settings', component: () => import('@/views/Settings.vue') },
      ],
    },
  ],
})

router.beforeEach((to) => {
  const userStore = useUserStore()
  if (to.meta.requiresAuth && !userStore.isLoggedIn) {
    return { name: 'Login' }
  }
})

export default router
