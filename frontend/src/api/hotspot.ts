import http from './http'

export interface Hotspot {
  title: string
  snippet?: string
  hot_score?: number
  source: string
  url?: string
  category?: string
}

export interface HotspotListResponse {
  hotspots: Hotspot[]
}

export const hotspotApi = {
  searchByKeyword(keyword: string) {
    return http.get<HotspotListResponse>('/hotspot/search', { params: { keyword } })
  },

  fetchAll(forceRefresh = false) {
    return http.get<HotspotListResponse>('/hotspot', { params: { force_refresh: forceRefresh } })
  },

  fetchBySource(source: string, forceRefresh = false) {
    return http.get<HotspotListResponse>(`/hotspot/${source}`, { params: { force_refresh: forceRefresh } })
  },
}
