/**
 * 任务状态管理 Composable
 */

import { ref, computed } from 'vue'
import { useTaskStore } from '@/stores/task'

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed'

export interface TaskStep {
  name: string
  status: 'wait' | 'running' | 'completed' | 'error'
}

export function useTask(taskId?: string) {
  const store = useTaskStore()
  const currentTaskId = ref(taskId || null)
  const isLoading = ref(false)

  const task = computed(() => {
    if (!currentTaskId.value) return null
    return store.getTask(currentTaskId.value)
  })

  const status = computed(() => task.value?.status || 'pending')

  const steps = computed((): TaskStep[] => {
    if (!task.value) {
      return [
        { name: '生成内容', status: 'wait' },
        { name: '生成配图', status: 'wait' },
        { name: '适配平台', status: 'wait' },
        { name: '发布', status: 'wait' },
      ]
    }

    const currentStep = task.value.current_step || 0
    return [
      { name: '生成内容', status: getStepStatus(0, currentStep, task.value.status) },
      { name: '生成配图', status: getStepStatus(1, currentStep, task.value.status) },
      { name: '适配平台', status: getStepStatus(2, currentStep, task.value.status) },
      { name: '发布', status: getStepStatus(3, currentStep, task.value.status) },
    ]
  })

  function getStepStatus(index: number, currentStep: number, taskStatus: TaskStatus): TaskStep['status'] {
    if (taskStatus === 'failed' && index === currentStep) return 'error'
    if (taskStatus === 'completed' && index <= currentStep) return 'completed'
    if (index < currentStep) return 'completed'
    if (index === currentStep) return 'running'
    return 'wait'
  }

  async function loadTask(id: string) {
    currentTaskId.value = id
    isLoading.value = true
    try {
      await store.fetchTask(id)
    } finally {
      isLoading.value = false
    }
  }

  async function createTask(params: {
    mode: string
    topic: string
    platforms: string[]
    config?: Record<string, any>
  }) {
    isLoading.value = true
    try {
      const result = await store.createTask(params)
      currentTaskId.value = result.id
      return result
    } finally {
      isLoading.value = false
    }
  }

  async function cancelTask() {
    if (!currentTaskId.value) return
    await store.cancelTask(currentTaskId.value)
  }

  return {
    task,
    status,
    steps,
    isLoading,
    currentTaskId,
    loadTask,
    createTask,
    cancelTask,
  }
}
