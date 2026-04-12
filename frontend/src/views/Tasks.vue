<template>
  <n-space vertical :size="16">
    <n-card title="任务列表">
      <template #header-extra>
        <n-button size="small" @click="fetchTasks">刷新</n-button>
      </template>
      <n-data-table :columns="columns" :data="tasks" :bordered="false" :loading="loading" />
      <n-empty v-if="!loading && tasks.length === 0" description="暂无任务记录" style="margin-top: 40px" />
    </n-card>

    <n-modal v-model:show="showDetail" preset="card" title="任务详情" style="width: 600px">
      <n-descriptions v-if="currentTask" label-placement="top" :column="1">
        <n-descriptions-item label="任务ID">{{ currentTask.id }}</n-descriptions-item>
        <n-descriptions-item label="主题">{{ currentTask.topic }}</n-descriptions-item>
        <n-descriptions-item label="模式">{{ currentTask.mode }}</n-descriptions-item>
        <n-descriptions-item label="状态">
          <n-tag :type="statusTagType[currentTask.status]">{{ statusText[currentTask.status] }}</n-tag>
        </n-descriptions-item>
        <n-descriptions-item label="进度">{{ currentTask.progress }} / {{ currentTask.total_steps }}</n-descriptions-item>
        <n-descriptions-item label="当前步骤">{{ currentTask.current_step || '-' }}</n-descriptions-item>
        <n-descriptions-item label="错误信息" v-if="currentTask.error">
          <n-alert type="error">{{ currentTask.error }}</n-alert>
        </n-descriptions-item>
        <n-descriptions-item label="生成结果" v-if="currentTask.result">
          <pre style="max-height: 300px; overflow: auto; font-size: 12px;">{{ formatResult(currentTask.result) }}</pre>
        </n-descriptions-item>
        <n-descriptions-item label="创建时间">{{ currentTask.created_at }}</n-descriptions-item>
        <n-descriptions-item label="更新时间">{{ currentTask.updated_at }}</n-descriptions-item>
      </n-descriptions>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { NCard, NButton, NSpace, NDataTable, NTag, NEmpty, NModal, NDescriptions, NDescriptionsItem, NAlert, useMessage } from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import { taskApi, type Task } from '@/api/task'

const message = useMessage()
const tasks = ref<Task[]>([])
const loading = ref(false)
const showDetail = ref(false)
const currentTask = ref<Task | null>(null)

function formatResult(result: string): string {
  try {
    return JSON.stringify(JSON.parse(result), null, 2)
  } catch {
    return result
  }
}

const statusTagType: Record<string, any> = {
  Pending: 'default',
  Running: 'warning',
  Completed: 'success',
  Failed: 'error',
  Cancelled: 'default',
}

const statusText: Record<string, string> = {
  Pending: '等待中',
  Running: '执行中',
  Completed: '已完成',
  Failed: '失败',
  Cancelled: '已取消',
}

const columns: DataTableColumns<Task> = [
  { title: '主题', key: 'topic', ellipsis: { tooltip: true } },
  { title: '模式', key: 'mode', width: 100 },
  {
    title: '状态', key: 'status', width: 100,
    render: (row) => h(NTag, { type: statusTagType[row.status] || 'default', size: 'small' }, { default: () => statusText[row.status] || row.status }),
  },
  { title: '进度', key: 'progress', width: 120, render: (row) => `${row.progress}/${row.total_steps}` },
  { title: '创建时间', key: 'created_at', width: 180 },
  {
    title: '操作', key: 'actions', width: 240,
    render: (row) => {
      const actions = [
        h(NButton, { size: 'tiny', type: 'primary', onClick: () => handleView(row) }, { default: () => '查看' }),
        h(NButton, { size: 'tiny', type: 'warning', style: 'margin-left: 8px', onClick: () => handleExecute(row.id) }, { default: () => '执行' }),
        h(NButton, { size: 'tiny', type: 'error', style: 'margin-left: 8px', onClick: () => handleDelete(row.id) }, { default: () => '删除' }),
      ]
      return h(NSpace, { size: 0 }, { default: () => actions })
    },
  },
]

async function handleView(task: Task) {
  currentTask.value = task
  showDetail.value = true
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

async function handleDelete(id: string) {
  try {
    await taskApi.cancel(id)
    message.success('任务已删除')
    await fetchTasks()
  } catch (e: any) {
    message.error(e.message || '删除失败')
  }
}

async function fetchTasks() {
  loading.value = true
  try {
    const { data } = await taskApi.list()
    tasks.value = data.tasks
  } catch {
    message.error('获取任务列表失败')
  } finally {
    loading.value = false
  }
}

onMounted(fetchTasks)
</script>
