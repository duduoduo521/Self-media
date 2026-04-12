<template>
  <n-checkbox-group v-model:value="selectedPlatforms" @update:value="handleChange">
    <n-space>
      <n-checkbox v-for="platform in platforms" :key="platform.id" :value="platform.id" :label="platform.name" />
    </n-space>
  </n-checkbox-group>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NCheckboxGroup, NCheckbox, NSpace } from 'naive-ui'

interface Platform {
  id: string
  name: string
  icon?: string
  enabled: boolean
}

interface Props {
  modelValue: string[]
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string[]): void
}>()

const platforms = ref<Platform[]>([
  { id: 'xiaohongshu', name: '小红书', enabled: true },
  { id: 'douyin', name: '抖音', enabled: true },
  { id: 'wechat', name: '公众号', enabled: true },
  { id: 'bilibili', name: 'B站', enabled: true },
  { id: 'weibo', name: '微博', enabled: true },
  { id: 'toutiao', name: '头条', enabled: true },
])

const selectedPlatforms = ref<string[]>(props.modelValue)

watch(selectedPlatforms, (val) => {
  emit('update:modelValue', val)
})

function handleChange(value: (string | number)[]) {
  emit('update:modelValue', value as string[])
}
</script>
