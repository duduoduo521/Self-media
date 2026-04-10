<template>
  <n-radio-group v-model:value="currentMode" name="mode-selector" @change="handleChange">
    <n-space>
      <n-radio-button value="text">
        <div class="mode-option">
          <span class="mode-icon">📝</span>
          <span class="mode-label">文本模式</span>
          <span class="mode-desc">文案 + 自动配图</span>
        </div>
      </n-radio-button>
      <n-radio-button value="video">
        <div class="mode-option">
          <span class="mode-icon">🎬</span>
          <span class="mode-label">视频模式</span>
          <span class="mode-desc">脚本 + 视频 + 配音</span>
        </div>
      </n-radio-button>
    </n-space>
  </n-radio-group>

  <div v-if="currentMode === 'text'" class="mode-config">
    <n-form-item label="配图数量">
      <n-input-number v-model:value="imageCount" :min="1" :max="9" size="small" />
    </n-form-item>
  </div>

  <div v-if="currentMode === 'video'" class="mode-config">
    <n-form-item label="视频时长">
      <n-input-number v-model:value="videoDuration" :min="15" :max="300" :step="15" size="small" />
      <span class="suffix">秒</span>
    </n-form-item>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NRadioGroup, NRadioButton, NSpace, NFormItem, NInputNumber } from 'naive-ui'

interface Props {
  modelValue: string
  imageCount?: number
  videoDuration?: number
}

const props = withDefaults(defineProps<Props>(), {
  imageCount: 3,
  videoDuration: 30,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'update:imageCount', value: number): void
  (e: 'update:videoDuration', value: number): void
}>()

const currentMode = ref(props.modelValue)
const imageCount = ref(props.imageCount)
const videoDuration = ref(props.videoDuration)

watch(currentMode, (val) => {
  emit('update:modelValue', val)
})

watch(imageCount, (val) => {
  emit('update:imageCount', val)
})

watch(videoDuration, (val) => {
  emit('update:videoDuration', val)
})

function handleChange(value: string) {
  emit('update:modelValue', value)
}
</script>

<style scoped>
.mode-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 0;
}

.mode-icon {
  font-size: 20px;
}

.mode-label {
  font-weight: 500;
}

.mode-desc {
  font-size: 12px;
  color: #999;
}

.mode-config {
  margin-top: 16px;
  padding: 16px;
  background: #f5f5f5;
  border-radius: 8px;
}

.suffix {
  margin-left: 8px;
  color: #666;
}
</style>
