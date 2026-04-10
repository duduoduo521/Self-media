<template>
  <div class="create-page">
    <n-card title="创作中心" size="large">
      <n-tabs type="segment" v-model:value="mode">
        <n-tab-pane name="text" tab="文本模式">
          <n-form :model="textForm" label-placement="top">
            <n-form-item label="主题">
              <n-input
                v-model:value="textForm.topic"
                type="textarea"
                placeholder="输入创作主题，或从热点广场选择"
                :rows="3"
              />
            </n-form-item>
            <n-form-item label="配图数量">
              <n-input-number v-model:value="textForm.imageCount" :min="1" :max="9" />
            </n-form-item>
          </n-form>
        </n-tab-pane>
        <n-tab-pane name="video" tab="视频模式">
          <n-form :model="videoForm" label-placement="top">
            <n-form-item label="主题">
              <n-input
                v-model:value="videoForm.topic"
                type="textarea"
                placeholder="输入视频主题"
                :rows="3"
              />
            </n-form-item>
            <n-form-item label="视频时长(秒)">
              <n-input-number v-model:value="videoForm.duration" :min="15" :max="300" :step="15" />
            </n-form-item>
          </n-form>
        </n-tab-pane>
      </n-tabs>

      <n-divider />

      <n-form-item label="发布平台">
        <PlatformSelector v-model="selectedPlatforms" />
      </n-form-item>

      <n-space vertical :size="16">
        <n-button type="primary" size="large" block @click="handleGenerate" :loading="generating">
          {{ generating ? '生成中...' : '一键生成并发布' }}
        </n-button>

        <TaskProgress v-if="currentTaskId" :task-id="currentTaskId" @complete="handleComplete" />
      </n-space>
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { NCard, NTabs, NTabPane, NForm, NFormItem, NInput, NInputNumber, NButton, NSpace, NDivider, useMessage } from 'naive-ui'
import PlatformSelector from '@/components/PlatformSelector.vue'
import TaskProgress from '@/components/TaskProgress.vue'
import { createTask } from '@/api/task'

const message = useMessage()
const mode = ref<'text' | 'video'>('text')
const generating = ref(false)
const currentTaskId = ref<string | null>(null)

const textForm = reactive({
  topic: '',
  imageCount: 3,
})

const videoForm = reactive({
  topic: '',
  duration: 30,
})

const selectedPlatforms = ref<string[]>([])

async function handleGenerate() {
  if (!textForm.topic && !videoForm.topic) {
    message.warning('请输入创作主题')
    return
  }
  if (selectedPlatforms.value.length === 0) {
    message.warning('请选择至少一个发布平台')
    return
  }

  generating.value = true
  try {
    const topic = mode.value === 'text' ? textForm.topic : videoForm.topic
    const task = await createTask({
      mode: mode.value,
      topic,
      platforms: selectedPlatforms.value,
      config: mode.value === 'text' 
        ? { image_count: textForm.imageCount }
        : { duration: videoForm.duration },
    })
    currentTaskId.value = task.id
    message.success('任务已创建')
  } catch (error) {
    message.error('创建任务失败')
  } finally {
    generating.value = false
  }
}

function handleComplete(result: any) {
  message.success('发布完成')
  currentTaskId.value = null
}
</script>

<style scoped>
.create-page {
  max-width: 800px;
  margin: 0 auto;
}
</style>
