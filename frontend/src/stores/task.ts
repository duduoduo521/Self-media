import { defineStore } from 'pinia'
import { ref } from 'vue'
import { taskApi, type Task } from '@/api/task'

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<Task[]>([])
  const currentTask = ref<Task | null>(null)

  async function createTask(params: { mode: string; topic: string; platforms: string[]; config?: Record<string, any> }) {
    const { data } = await taskApi.create(params)
    tasks.value.unshift(data.task)
    return data.task
  }

  async function fetchTasks() {
    const { data } = await taskApi.list()
    tasks.value = data.tasks
  }

  async function fetchTask(id: string) {
    const { data } = await taskApi.get(id)
    currentTask.value = data.task
    return data.task
  }

  async function cancelTask(id: string) {
    await taskApi.cancel(id)
    const task = tasks.value.find(t => t.id === id)
    if (task) {
      task.status = 'cancelled'
    }
  }

  async function executeTask(id: string) {
    await taskApi.execute(id)
  }

  function getTask(id: string) {
    return tasks.value.find(t => t.id === id) || currentTask.value
  }

  return {
    tasks,
    currentTask,
    createTask,
    fetchTasks,
    fetchTask,
    cancelTask,
    executeTask,
    getTask,
  }
})
