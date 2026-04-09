<template>
  <n-space vertical :size="16">
    <n-card title="创建任务">
      <n-form ref="formRef" :model="form" label-placement="left" label-width="80">
        <n-form-item label="主题">
          <n-input v-model:value="form.topic" type="textarea" :rows="3" placeholder="输入创作主题或热点关键词" />
        </n-form-item>
        <n-form-item label="模式">
          <n-radio-group v-model:value="form.mode">
            <n-radio value="Text">图文模式</n-radio>
            <n-radio value="Video">视频模式</n-radio>
          </n-radio-group>
        </n-form-item>
        <n-form-item label="平台">
          <n-checkbox-group v-model:value="form.platforms">
            <n-space>
              <n-checkbox v-for="p in platformOptions" :key="p.value" :value="p.value" :label="p.label" />
            </n-space>
          </n-checkbox-group>
        </n-form-item>
        <n-button type="primary" :loading="creating" @click="handleCreate">创建任务</n-button>
      </n-form>
    </n-card>

    <n-card title="任务列表">
      <n-data-table :columns="columns" :data="tasks" :bordered="false" />
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, h } from 'vue'
import { NCard, NForm, NFormItem, NInput, NRadioGroup, NRadio, NCheckboxGroup, NCheckbox, NButton, NSpace, NDataTable, NTag, useMessage } from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import { taskApi, type Task } from '@/api/task'

const message = useMessage()
const creating = ref(false)
const tasks = ref<Task[]>([])

const platformOptions = [
  { label: '微博', value: 'Weibo' },
  { label: '今日头条', value: 'Toutiao' },
  { label: '微信公众号', value: 'WeChatOfficial' },
  { label: 'B站', value: 'Bilibili' },
  { label: '小红书', value: 'Xiaohongshu' },
  { label: '抖音', value: 'Douyin' },
]

const form = reactive({
  topic: '',
  mode: 'Text' as string,
  platforms: ['Weibo'] as string[],
})

const statusTagType: Record<string, any> = {
  Pending: 'default',
  Running: 'warning',
  Completed: 'success',
  Failed: 'error',
  Cancelled: 'default',
}

const columns: DataTableColumns<Task> = [
  { title: '主题', key: 'topic', ellipsis: { tooltip: true } },
  { title: '模式', key: 'mode', width: 100 },
  {
    title: '状态', key: 'status', width: 120,
    render: (row) => h(NTag, { type: statusTagType[row.status] || 'default', size: 'small' }, { default: () => row.status }),
  },
  { title: '进度', key: 'progress', width: 100, render: (row) => `${row.progress}/${row.total_steps}` },
  { title: '创建时间', key: 'created_at', width: 180 },
  {
    title: '操作', key: 'actions', width: 160,
    render: (row) => {
      const actions = [
        h(NButton, { size: 'tiny', type: 'primary', onClick: () => handleExecute(row.id) }, { default: () => '执行' }),
      ]
      if (row.status === 'Pending' || row.status === 'Running') {
        actions.push(h(NButton, { size: 'tiny', type: 'error', style: 'margin-left: 8px', onClick: () => handleCancel(row.id) }, { default: () => '取消' }))
      }
      return h(NSpace, { size: 0 }, { default: () => actions })
    },
  },
]

async function handleCreate() {
  if (!form.topic.trim()) {
    message.warning('请输入主题')
    return
  }
  if (form.platforms.length === 0) {
    message.warning('请至少选择一个平台')
    return
  }
  creating.value = true
  try {
    await taskApi.create({ mode: form.mode, topic: form.topic, platforms: form.platforms })
    message.success('任务创建成功')
    form.topic = ''
    await fetchTasks()
  } catch (e: any) {
    message.error(e.message || '创建失败')
  } finally {
    creating.value = false
  }
}

async function handleExecute(id: string) {
  try {
    await taskApi.execute(id)
    message.success('任务开始执行')
    await fetchTasks()
  } catch (e: any) {
    message.error(e.message || '执行失败')
  }
}

async function handleCancel(id: string) {
  try {
    await taskApi.cancel(id)
    message.success('任务已取消')
    await fetchTasks()
  } catch (e: any) {
    message.error(e.message || '取消失败')
  }
}

async function fetchTasks() {
  try {
    const { data } = await taskApi.list()
    tasks.value = data.tasks
  } catch {}
}

onMounted(fetchTasks)
</script>
