import http from './http'

export interface Task {
  id: string
  user_id: number
  task_type: string
  status: string
  mode: string
  topic: string
  platforms: string
  progress: number
  total_steps: number
  current_step: string | null
  result: string | null
  error: string | null
  retry_count: number
  created_at: string
  updated_at: string
}

export const taskApi = {
  create(data: { mode: string; topic: string; platforms: string[] }) {
    return http.post<{ task: Task }>('/tasks', data)
  },
  list() {
    return http.get<{ tasks: Task[] }>('/tasks')
  },
  get(id: string) {
    return http.get<{ task: Task }>(`/tasks/${id}`)
  },
  cancel(id: string) {
    return http.delete(`/tasks/${id}`)
  },
  execute(id: string) {
    return http.post(`/tasks/${id}/execute`)
  },
}

export async function createTask(data: { mode: string; topic: string; platforms: string[]; config?: Record<string, any> }) {
  const response = await http.post<{ task: Task }>('/tasks', data)
  return response.task
}
