<template>
  <n-grid :cols="4" :x-gap="16" :y-gap="16">
    <n-gi>
      <n-card>
        <n-statistic label="热点源" :value="6" />
      </n-card>
    </n-gi>
    <n-gi>
      <n-card>
        <n-statistic label="发布平台" :value="6" />
      </n-card>
    </n-gi>
    <n-gi>
      <n-card>
        <n-statistic label="今日任务" :value="tasks.length" />
      </n-card>
    </n-gi>
    <n-gi>
      <n-card>
        <n-statistic label="进行中" :value="runningCount" />
      </n-card>
    </n-gi>
  </n-grid>

  <n-card title="最近任务" style="margin-top: 16px">
    <n-data-table :columns="columns" :data="tasks" :bordered="false" />
  </n-card>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { NGrid, NGi, NCard, NStatistic, NDataTable } from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import { taskApi, type Task } from '@/api/task'

const tasks = ref<Task[]>([])
const runningCount = computed(() => tasks.value.filter((t) => t.status === 'Running').length)

const columns: DataTableColumns<Task> = [
  { title: '主题', key: 'topic', ellipsis: { tooltip: true } },
  { title: '模式', key: 'mode', width: 100 },
  { title: '状态', key: 'status', width: 120 },
  { title: '进度', key: 'progress', width: 100, render: (row) => `${row.progress}/${row.total_steps}` },
  { title: '创建时间', key: 'created_at', width: 180 },
]

onMounted(async () => {
  try {
    const { data } = await taskApi.list()
    tasks.value = data.tasks
  } catch {}
})
</script>
