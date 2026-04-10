<template>
  <n-card title="执行进度" size="small">
    <n-steps :current="currentStep" size="small">
      <n-step title="生成内容" :status="getStepStatus(0)" />
      <n-step title="生成配图" :status="getStepStatus(1)" />
      <n-step title="适配平台" :status="getStepStatus(2)" />
      <n-step title="发布" :status="getStepStatus(3)" />
    </n-steps>

    <n-divider />

    <div class="log-container">
      <div v-for="(log, index) in logs" :key="index" class="log-item" :class="log.type">
        <span class="log-time">{{ log.time }}</span>
        <span class="log-message">{{ log.message }}</span>
      </div>
    </div>

    <template #footer v-if="status === 'completed'">
      <n-space>
        <n-button type="primary" @click="viewResult">查看结果</n-button>
        <n-button @click="$emit('complete', result)">完成</n-button>
      </n-space>
    </template>
  </n-card>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { NCard, NSteps, NStep, NDivider, NButton, NSpace, useMessage } from 'naive-ui'

interface Props {
  taskId: string
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'complete', result: any): void
}>()

const message = useMessage()
const status = ref<'pending' | 'running' | 'completed' | 'failed'>('pending')
const currentStep = ref(0)
const logs = ref<Array<{ time: string; message: string; type: 'info' | 'success' | 'error' }>>([])
const result = ref<any>(null)

let eventSource: EventSource | null = null

const stepNames = ['生成内容', '生成配图', '适配平台', '发布']

function getStepStatus(index: number): 'process' | 'finish' | 'wait' | 'error' {
  if (index < currentStep.value) return 'finish'
  if (index === currentStep.value) return status.value === 'failed' ? 'error' : 'process'
  return 'wait'
}

function addLog(message: string, type: 'info' | 'success' | 'error' = 'info') {
  const now = new Date()
  const time = `${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}:${now.getSeconds().toString().padStart(2, '0')}`
  logs.value.push({ time, message, type })
}

onMounted(() => {
  status.value = 'running'
  addLog('任务已启动', 'info')

  // SSE 连接
  eventSource = new EventSource(`/api/tasks/${props.taskId}/stream`)
  eventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data)
      if (data.step !== undefined) {
        currentStep.value = data.step
        addLog(stepNames[data.step] + '中...', 'info')
      }
      if (data.message) {
        addLog(data.message, data.success ? 'success' : 'error')
      }
      if (data.status === 'completed') {
        status.value = 'completed'
        result.value = data.result
        addLog('任务完成', 'success')
        eventSource?.close()
      }
      if (data.status === 'failed') {
        status.value = 'failed'
        addLog('任务失败: ' + data.error, 'error')
        eventSource?.close()
      }
    } catch (e) {
      console.error('SSE parse error:', e)
    }
  }

  eventSource.onerror = () => {
    status.value = 'failed'
    addLog('连接断开', 'error')
    eventSource?.close()
  }
})

onUnmounted(() => {
  eventSource?.close()
})

function viewResult() {
  message.info('查看结果功能开发中')
}
</script>

<style scoped>
.log-container {
  max-height: 200px;
  overflow-y: auto;
  font-family: monospace;
  font-size: 12px;
}

.log-item {
  padding: 4px 0;
  display: flex;
  gap: 8px;
}

.log-item.success {
  color: #18a058;
}

.log-item.error {
  color: #d03050;
}

.log-time {
  color: #999;
  flex-shrink: 0;
}
</style>
