<template>
  <n-space vertical :size="16">
    <n-grid :cols="3" :x-gap="16" :y-gap="16">
      <n-gi v-for="p in platforms" :key="p.platform">
        <n-card>
          <template #header>
            <n-space align="center" justify="space-between">
              <n-space align="center">
                <n-avatar :size="32" round>
                  {{ getPlatformIcon(p.platform) }}
                </n-avatar>
                <span>{{ p.name }}</span>
              </n-space>
              <n-tag :type="getStatusType(p.status)" size="small">
                {{ getStatusText(p.status) }}
              </n-tag>
            </n-space>
          </template>
          
          <n-space vertical :size="8">
            <n-text depth="3" v-if="p.last_login">
              上次登录: {{ formatTime(p.last_login) }}
            </n-text>
            <n-space>
              <n-switch 
                :value="p.enabled" 
                @update:value="(v) => handleToggle(p.platform, v)"
              />
              <n-text depth="3">{{ p.enabled ? '已启用' : '已禁用' }}</n-text>
            </n-space>
          </n-space>
          
          <template #footer>
            <n-space justify="end">
              <n-button 
                v-if="p.status !== 'connected'" 
                size="small" 
                type="primary"
                @click="showQrModal(p.platform)"
              >
                扫码登录
              </n-button>
              <n-button 
                v-else 
                size="small"
                @click="handleRefresh(p.platform)"
              >
                刷新状态
              </n-button>
            </n-space>
          </template>
        </n-card>
      </n-gi>
    </n-grid>

    <!-- 二维码弹窗 -->
    <n-modal v-model:show="showQr" preset="card" title="扫码登录" style="max-width: 400px">
      <n-space vertical align="center" :size="16">
        <n-qr-code :value="qrUrl" :size="200" />
        <n-text>请使用对应 App 扫码登录</n-text>
        <n-progress 
          v-if="qrLoading" 
          type="line" 
          :percentage="qrProgress" 
          status="success"
        />
        <n-text v-if="qrStatus" depth="3">{{ qrStatus }}</n-text>
      </n-space>
      <template #footer>
        <n-space justify="center">
          <n-button @click="cancelQr">取消</n-button>
        </n-space>
      </template>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { NSpace, NGrid, NGi, NCard, NAvatar, NTag, NText, NSwitch, NButton, NModal, NQrCode, NProgress, useMessage } from 'naive-ui'
import { platformApi, type PlatformConfig } from '@/api/platform'

const message = useMessage()
const platforms = ref<PlatformConfig[]>([])
const showQr = ref(false)
const qrUrl = ref('')
const qrId = ref('')
const currentPlatform = ref('')
const qrLoading = ref(false)
const qrProgress = ref(0)
const qrStatus = ref('')
let pollTimer: number | null = null

const platformIcons: Record<string, string> = {
  weibo: '📧',
  bilibili: '📺',
  douyin: '🎵',
  xiaohongshu: '📕',
  toutiao: '📰',
  wechatofficial: '💬',
}

function getPlatformIcon(platform: string) {
  return platformIcons[platform.toLowerCase()] || '📱'
}

function getStatusType(status: string) {
  const map: Record<string, any> = {
    connected: 'success',
    disconnected: 'default',
    expired: 'warning',
  }
  return map[status] || 'default'
}

function getStatusText(status: string) {
  const map: Record<string, string> = {
    connected: '已连接',
    disconnected: '未连接',
    expired: '已过期',
  }
  return map[status] || status
}

function formatTime(time: string) {
  try {
    return new Date(time).toLocaleString('zh-CN')
  } catch {
    return time
  }
}

async function loadPlatforms() {
  try {
    const { data } = await platformApi.list()
    platforms.value = data.platforms
  } catch (e: any) {
    message.error('加载平台列表失败')
  }
}

async function handleToggle(platform: string, enabled: boolean) {
  try {
    await platformApi.update(platform, { enabled })
    await loadPlatforms()
    message.success(enabled ? '已启用' : '已禁用')
  } catch (e: any) {
    message.error('更新失败')
  }
}

async function showQrModal(platform: string) {
  currentPlatform.value = platform
  qrLoading.value = true
  qrStatus.value = '正在生成二维码...'
  showQr.value = true
  
  try {
    const { data } = await platformApi.generateQr(platform)
    qrUrl.value = data.qr_info.qr_url
    qrId.value = data.qr_info.qr_id
    
    // 开始轮询
    startPolling()
  } catch (e: any) {
    message.error('生成二维码失败')
    showQr.value = false
  } finally {
    qrLoading.value = false
  }
}

function startPolling() {
  qrProgress.value = 0
  const interval = 3000 // 3秒轮询
  
  pollTimer = window.setInterval(async () => {
    try {
      const { data } = await platformApi.queryQrStatus(currentPlatform.value, qrId.value)
      qrStatus.value = getQrStatusText(data.status)
      
      if (data.status === 'confirmed') {
        // 登录确认，查询结果
        await platformApi.waitQrConfirmation(currentPlatform.value, qrId.value)
        message.success('登录成功')
        cancelQr()
        await loadPlatforms()
      } else if (data.status === 'expired') {
        message.warning('二维码已过期')
        cancelQr()
      }
    } catch (e: any) {
      // 忽略轮询错误
    }
    
    // 更新进度条
    qrProgress.value = Math.min(qrProgress.value + 10, 95)
  }, interval)
}

function getQrStatusText(status: string) {
  const map: Record<string, string> = {
    pending: '等待扫码...',
    scanned: '已扫码，请确认',
    confirmed: '登录成功',
    expired: '已过期',
    cancelled: '已取消',
  }
  return map[status] || status
}

function cancelQr() {
  showQr.value = false
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
  qrProgress.value = 0
  qrStatus.value = ''
}

async function handleRefresh(platform: string) {
  try {
    await loadPlatforms()
    message.success('刷新成功')
  } catch {
    message.error('刷新失败')
  }
}

onMounted(() => {
  loadPlatforms()
})

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer)
  }
})
</script>
