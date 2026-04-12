<template>
  <n-modal v-model:show="showModal" preset="card" :title="`${platformName} 扫码登录`" style="width: 400px">
    <div class="qr-container">
      <div v-if="loading" class="loading">
        <n-spin size="large" />
        <p>正在生成二维码...</p>
      </div>
      <div v-else-if="qrUrl" class="qr-code">
        <img :src="qrUrl" alt="扫码登录" />
        <p class="hint">请使用{{ platformName }}扫码登录</p>
        <n-button size="small" @click="refreshQR">刷新二维码</n-button>
      </div>
      <div v-else-if="error" class="error">
        <n-result status="error" :title="error" />
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NModal, NSpin, NButton, NResult } from 'naive-ui'

interface Props {
  platform: string
  show: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
  (e: 'success', credential: any): void
  (e: 'error', error: string): void
}>()

const showModal = computed({
  get: () => props.show,
  set: (val) => emit('update:show', val),
})

const platformName = computed(() => {
  const names: Record<string, string> = {
    weibo: '微博',
    douyin: '抖音',
    xiaohongshu: '小红书',
    bilibili: 'B站',
    toutiao: '头条',
  }
  return names[props.platform] || props.platform
})

const loading = ref(false)
const qrUrl = ref<string | null>(null)
const error = ref<string | null>(null)
const qrId = ref<string | null>(null)

async function fetchQRCode() {
  loading.value = true
  error.value = null
  qrUrl.value = null

  try {
    const response = await fetch(`/api/platforms/${props.platform}/qr`, {
      method: 'POST',
      credentials: 'include',
    })
    const data = await response.json()
    if (data.qr_url) {
      qrUrl.value = data.qr_url
      qrId.value = data.qr_id
      startPolling()
    } else {
      error.value = '获取二维码失败'
    }
  } catch (e) {
    error.value = '网络错误'
  } finally {
    loading.value = false
  }
}

let pollTimer: number | null = null

function startPolling() {
  if (!qrId.value) return

  pollTimer = window.setInterval(async () => {
    try {
      const response = await fetch(`/api/platforms/${props.platform}/qr/${qrId.value}/status`, {
        credentials: 'include',
      })
      const data = await response.json()

      if (data.status === 'scanned') {
        // 已扫码，待确认
      } else if (data.status === 'confirmed') {
        stopPolling()
        emit('success', data.credential)
        showModal.value = false
      } else if (data.status === 'expired') {
        stopPolling()
        error.value = '二维码已过期，请刷新'
      }
    } catch (e) {
      console.error('Poll error:', e)
    }
  }, 2000)
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

function refreshQR() {
  stopPolling()
  fetchQRCode()
}

watch(() => props.show, (val) => {
  if (val) {
    fetchQRCode()
  } else {
    stopPolling()
  }
})
</script>

<style scoped>
.qr-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 300px;
}

.loading {
  text-align: center;
}

.qr-code {
  text-align: center;
}

.qr-code img {
  width: 200px;
  height: 200px;
  margin-bottom: 16px;
}

.hint {
  color: #666;
  margin-bottom: 16px;
}

.error {
  width: 100%;
}
</style>
