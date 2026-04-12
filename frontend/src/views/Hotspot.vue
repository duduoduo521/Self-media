<template>
  <n-space vertical :size="16">
    <n-tabs v-model:value="activeTab" type="line" @update:value="handleTabChange">
      <n-tab name="search">关键词搜索</n-tab>
      <n-tab name="Weibo">微博</n-tab>
      <n-tab name="Douyin">抖音</n-tab>
      <n-tab name="Toutiao">头条</n-tab>
    </n-tabs>

    <div v-if="activeTab === 'search'">
      <n-card>
        <n-space vertical>
          <n-space align="center">
            <n-input v-model:value="searchKeyword" placeholder="输入搜索关键词，如：AI、科技、美食" style="width: 300px" @keyup.enter="handleSearch" />
            <n-button type="primary" :loading="loading" @click="handleSearch">搜索热点</n-button>
          </n-space>
          <n-text depth="3" style="font-size: 12px">输入您的垂直领域关键词，系统将获取相关的最新热点</n-text>
        </n-space>
      </n-card>

      <n-spin :show="loading">
        <n-grid :cols="1" :y-gap="12" style="margin-top: 16px">
          <n-gi v-for="(item, idx) in hotspots" :key="idx">
            <n-card hoverable size="small">
              <template #header>
                <n-space align="center">
                  <n-tag type="info" size="small">{{ idx + 1 }}</n-tag>
                  <span>{{ item.title }}</span>
                </n-space>
              </template>
              <template #header-extra>
                <n-space>
                  <n-text depth="3">{{ item.source }}</n-text>
                  <n-button type="primary" size="tiny" @click="createFromHotspot(item)">立即创作</n-button>
                </n-space>
              </template>
              <n-space v-if="item.snippet" :size="8">
                <n-text depth="2" style="font-size: 13px">{{ item.snippet }}</n-text>
              </n-space>
            </n-card>
          </n-gi>
        </n-grid>

        <n-empty v-if="!loading && hotspots.length === 0 && hasSearched" description="暂无热点数据，请尝试其他关键词" style="margin-top: 40px" />
        <n-empty v-if="!loading && hotspots.length === 0 && !hasSearched" description="输入关键词搜索热点" style="margin-top: 40px" />
      </n-spin>
    </div>

    <div v-else>
      <n-space justify="end" style="margin-bottom: 12px">
        <n-button size="small" @click="loadHotspots(true)">刷新</n-button>
      </n-space>

      <n-spin :show="loading">
        <n-grid :cols="1" :y-gap="12">
          <n-gi v-for="(item, idx) in platformHotspots" :key="idx">
            <n-card hoverable size="small">
              <template #header>
                <n-space align="center">
                  <n-tag type="warning" size="small">{{ idx + 1 }}</n-tag>
                  <span>{{ item.title }}</span>
                </n-space>
              </template>
              <template #header-extra>
                <n-space>
                  <n-text depth="3">{{ item.hot_score }}</n-text>
                  <n-button type="primary" size="tiny" @click="createFromHotspot(item)">立即创作</n-button>
                </n-space>
              </template>
              <n-space v-if="item.category" :size="8">
                <n-text depth="2" style="font-size: 13px">{{ item.category }}</n-text>
              </n-space>
            </n-card>
          </n-gi>
        </n-grid>

        <n-empty v-if="!loading && platformHotspots.length === 0" description="暂无热点数据" style="margin-top: 40px" />
      </n-spin>
    </div>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { NSpin, NCard, NSpace, NInput, NButton, NTag, NText, NEmpty, NGrid, NGi, NTabs, NTab } from 'naive-ui'
import { hotspotApi } from '@/api/hotspot'

const router = useRouter()

interface Hotspot {
  title: string
  snippet?: string
  hot_score?: number
  source: string
  url?: string
  category?: string
  event_date?: string
}

const activeTab = ref('search')
const searchKeyword = ref('')
const hotspots = ref<Hotspot[]>([])
const platformHotspots = ref<Hotspot[]>([])
const loading = ref(false)
const hasSearched = ref(false)

async function handleSearch() {
  if (!searchKeyword.value.trim()) return

  loading.value = true
  hasSearched.value = true
  try {
    const { data } = await hotspotApi.searchByKeyword(searchKeyword.value.trim())
    hotspots.value = data.hotspots
  } catch {
    hotspots.value = []
  } finally {
    loading.value = false
  }
}

async function handleTabChange(tab: string) {
  if (tab !== 'search') {
    loadPlatformHotspots(tab)
  }
}

async function loadPlatformHotspots(source: string) {
  loading.value = true
  platformHotspots.value = []
  try {
    const { data } = await hotspotApi.fetchBySource(source)
    platformHotspots.value = data.hotspots
  } catch {
    platformHotspots.value = []
  } finally {
    loading.value = false
  }
}

async function loadHotspots(forceRefresh = false) {
  if (!activeTab.value || activeTab.value === 'search') return
  await loadPlatformHotspots(activeTab.value)
}

onMounted(() => {
  loadHotspots()
})

function createFromHotspot(item: Hotspot) {
  const topic = item.snippet ? `${item.title}\n${item.snippet}` : item.title
  const query: Record<string, string> = { topic }
  if (item.event_date) {
    query.date = item.event_date
  }
  router.push({ name: 'Create', query })
}
</script>
