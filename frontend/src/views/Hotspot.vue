<template>
  <n-space vertical :size="16">
    <n-card>
      <n-space>
        <n-select v-model:value="selectedSource" :options="sourceOptions" placeholder="选择热点源" style="width: 200px" clearable />
        <n-button type="primary" :loading="loading" @click="fetchHotspots">刷新</n-button>
      </n-space>
    </n-card>

    <n-grid :cols="1" :y-gap="12">
      <n-gi v-for="item in hotspots" :key="item.title + item.source">
        <n-card hoverable size="small">
          <template #header>
            <n-space align="center">
              <n-tag :type="sourceTagType(item.source)" size="small">{{ item.source }}</n-tag>
              <span>{{ item.title }}</span>
            </n-space>
          </template>
          <template #header-extra>
            <n-text depth="3">{{ formatScore(item.hot_score) }}</n-text>
          </template>
          <n-space v-if="item.category" :size="8">
            <n-tag size="tiny">{{ item.category }}</n-tag>
          </n-space>
        </n-card>
      </n-gi>
    </n-grid>

    <n-empty v-if="!loading && hotspots.length === 0" description="暂无热点数据" />
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NCard, NSpace, NSelect, NButton, NGrid, NGi, NTag, NText, NEmpty } from 'naive-ui'
import { hotspotApi, type Hotspot } from '@/api/hotspot'

const hotspots = ref<Hotspot[]>([])
const loading = ref(false)
const selectedSource = ref<string | null>(null)

const sourceOptions = [
  { label: '全部', value: '' },
  { label: '微博', value: 'Weibo' },
  { label: 'B站', value: 'Bilibili' },
  { label: '抖音', value: 'Douyin' },
  { label: '小红书', value: 'Xiaohongshu' },
  { label: '今日头条', value: 'Toutiao' },
  { label: '知乎', value: 'Zhihu' },
]

function sourceTagType(source: string) {
  const map: Record<string, string> = {
    Weibo: 'error',
    Bilibili: 'info',
    Douyin: 'warning',
    Xiaohongshu: 'success',
    Toutiao: 'default',
    Zhihu: 'default',
  }
  return (map[source] || 'default') as any
}

function formatScore(score: number) {
  if (score >= 10000) return (score / 10000).toFixed(1) + '万'
  return String(score)
}

async function fetchHotspots() {
  loading.value = true
  try {
    if (selectedSource.value) {
      const { data } = await hotspotApi.fetchBySource(selectedSource.value)
      hotspots.value = data.hotspots
    } else {
      const { data } = await hotspotApi.fetchAll(true)
      hotspots.value = data.hotspots
    }
  } catch {} finally {
    loading.value = false
  }
}

onMounted(fetchHotspots)
</script>
