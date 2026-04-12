<template>
  <div class="cookie-status">
    <n-tag :type="statusType" size="small">
      <template #icon>
        <n-icon :component="statusIcon" />
      </template>
      {{ statusText }}
    </n-tag>
    <n-button size="tiny" @click="$emit('check')" :loading="checking">
      检查状态
    </n-button>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NTag, NIcon, NButton } from 'naive-ui'
import { CheckmarkCircle, AlertCircle, CloseCircle } from '@vicons/ionicons5'

interface Props {
  platform: string
  cookieValid?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  cookieValid: false,
})

defineEmits<{
  (e: 'check'): void
}>()

const checking = ref(false)

const statusType = computed(() => {
  if (props.cookieValid === null) return 'default'
  return props.cookieValid ? 'success' : 'error'
})

const statusIcon = computed(() => {
  if (props.cookieValid === null) return AlertCircle
  return props.cookieValid ? CheckmarkCircle : CloseCircle
})

const statusText = computed(() => {
  if (props.cookieValid === null) return '未知'
  return props.cookieValid ? '已登录' : '未登录'
})
</script>

<style scoped>
.cookie-status {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
