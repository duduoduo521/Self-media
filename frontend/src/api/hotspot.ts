import http from './http'

export interface Hotspot {
  title: string
  hot_score: number
  source: string
  url: string | null
  category: string | null
  fetched_at: string
}

export const hotspotApi = {
  fetchAll(forceRefresh = false) {
    return http.get<{ hotspots: Hotspot[] }>('/hotspot', { params: { force_refresh: forceRefresh } })
  },
  fetchBySource(source: string) {
    return http.get<{ hotspots: Hotspot[] }>(`/hotspot/${source}`)
  },
}
