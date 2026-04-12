import http from './http'

export interface Draft {
  id: string
  user_id: number
  task_id: string | null
  mode: string
  topic: string
  platforms: string
  original_content: string | null
  adapted_contents: string
  generated_images: string
  status: 'draft' | 'published' | 'partially_published'
  publish_results: any[]
  created_at: string
  updated_at: string
}

export const draftApi = {
  list() {
    return http.get<{ drafts: Draft[] }>('/drafts')
  },
  get(id: string) {
    return http.get<{ draft: Draft }>(`/drafts/${id}`)
  },
  delete(id: string) {
    return http.delete(`/drafts/${id}`)
  },
  publish(id: string, platform?: string) {
    return http.post(`/drafts/${id}/publish`, { platform })
  },
}
