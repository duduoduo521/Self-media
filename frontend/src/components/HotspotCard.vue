<template>
  <n-card class="hotspot-card" hoverable @click="handleClick">
    <template #header>
      <div class="card-header">
        <n-tag :type="platformType" size="small">{{ platformName }}</n-tag>
        <span class="rank" v-if="rank">#{{ rank }}</span>
      </div>
    </template>
    <template #default>
      <div class="content">
        <p class="title">{{ title }}</p>
        <div class="meta" v-if="hotCount || timestamp">
          <span v-if="hotCount" class="hot-count">🔥 {{ hotCount }}</span>
          <span v-if="timestamp" class="time">{{ formatTime(timestamp) }}</span>
        </div>
      </div>
    </template>
    <template #footer>
      <n-space size="small">
        <n-button size="tiny" @click.stop="useAsTopic">用作主题</n-button>
        <n-button size="tiny" @click.stop="share">分享</n-button>
      </n-space>
    </template>
  </n-card>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NCard, NTag, NSpace, NButton, useMessage } from 'naive-ui'
import { useRouter } from 'vue-router'

interface Props {
  id: string
  platform: string
  title: string
  hotCount?: string
  timestamp?: string
  rank?: number
  url?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'select', data: { platform: string; title: string }): void
}>()

const router = useRouter()
const message = useMessage()

const platformName = computed(() => {
  const names: Record<string, string> = {
    weibo: '微博',
    douyin: '抖音',
    xiaohongshu: '小红书',
    bilibili: 'B站',
    toutiao: '头条',
    zhihu: '知乎',
  }
  return names[props.platform] || props.platform
})

const platformType = computed(() => {
  const types: Record<string, 'error' | 'warning' | 'info' | 'success' | 'default'> = {
    weibo: 'error',
    douyin: 'default',
    xiaohongshu: 'error',
    bilibili: 'info',
    toutiao: 'warning',
    zhihu: 'info',
  }
  return types[props.platform] || 'default'
})

function formatTime(timestamp: string): string {
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const hours = Math.floor(diff / (1000 * 60 * 60))
  if (hours < 1) return '刚刚'
  if (hours < 24) return `${hours}小时前`
  const days = Math.floor(hours / 24)
  return `${days}天前`
}

function handleClick() {
  if (props.url) {
    window.open(props.url, '_blank')
  }
}

function useAsTopic() {
  emit('select', { platform: props.platform, title: props.title })
  message.success('已选择为创作主题')
}

function share() {
  navigator.clipboard.writeText(props.url || `${props.platform}: ${props.title}`)
  message.success('链接已复制')
}
</script>

<style scoped>
.hotspot-card {
  cursor: pointer;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.rank {
  font-size: 12px;
  color: #999;
}

.content .title {
  font-size: 14px;
  line-height: 1.5;
  margin-bottom: 8px;
}

.meta {
  font-size: 12px;
  color: #999;
  display: flex;
  gap: 12px;
}

.hot-count {
  color: #f56c6c;
}
</style>
