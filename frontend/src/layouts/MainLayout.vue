<template>
  <n-layout has-sider style="height: 100vh">
    <n-layout-sider bordered :width="220" :collapsed-width="64" collapse-mode="width" :collapsed="collapsed" show-trigger @collapse="collapsed = true" @expand="collapsed = false">
      <div class="logo" :class="{ collapsed }">
        <span v-if="!collapsed">Self-Media</span>
        <span v-else>SM</span>
      </div>
      <n-menu :collapsed="collapsed" :collapsed-width="64" :collapsed-icon-size="22" :options="menuOptions" :value="activeKey" @update:value="handleMenuClick" />
    </n-layout-sider>
    <n-layout>
      <n-layout-header bordered style="height: 56px; display: flex; align-items: center; justify-content: space-between; padding: 0 24px;">
        <span style="font-size: 16px; font-weight: 500">{{ pageTitle }}</span>
        <n-space>
          <span style="color: #999; font-size: 14px">{{ userStore.user?.username }}</span>
          <n-button size="small" quaternary @click="handleLogout">退出</n-button>
        </n-space>
      </n-layout-header>
      <n-layout-content style="padding: 24px; overflow: auto">
        <router-view />
      </n-layout-content>
    </n-layout>
  </n-layout>
</template>

<script setup lang="ts">
import { ref, computed, h } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { NLayout, NLayoutSider, NLayoutHeader, NLayoutContent, NMenu, NButton, NSpace } from 'naive-ui'
import type { MenuOption } from 'naive-ui'
import {
  HomeOutline,
  FlameOutline,
  ListOutline,
  SettingsOutline,
  GridOutline,
  CreateOutline,
} from '@vicons/ionicons5'
import { useUserStore } from '@/stores/user'

const router = useRouter()
const route = useRoute()
const userStore = useUserStore()
const collapsed = ref(false)

const menuOptions: MenuOption[] = [
  { label: '仪表盘', key: 'Dashboard', icon: () => h(HomeOutline) },
  { label: '热点发现', key: 'Hotspot', icon: () => h(FlameOutline) },
  { label: '创作中心', key: 'Create', icon: () => h(CreateOutline) },
  { label: '任务管理', key: 'Tasks', icon: () => h(ListOutline) },
  { label: '平台管理', key: 'Platforms', icon: () => h(GridOutline) },
  { label: '系统设置', key: 'Settings', icon: () => h(SettingsOutline) },
]

const activeKey = computed(() => route.name as string)
const pageTitle = computed(() => {
  const item = menuOptions.find((m) => m.key === activeKey.value)
  return item?.label as string || ''
})

function handleMenuClick(key: string) {
  router.push({ name: key })
}

async function handleLogout() {
  await userStore.logout()
  router.push({ name: 'Login' })
}
</script>

<style scoped>
.logo {
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  font-weight: 700;
  color: #63e2b7;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.logo.collapsed {
  font-size: 16px;
}
</style>
