import http from './http'

export interface UserPreferences {
  default_mode: string
  default_tags: string[]
  auto_publish: boolean
}

export const configApi = {
  setApiKey(data: { provider: string; key: string; region: string }) {
    return http.put('/config/api-key', data)
  },
  getPlatforms() {
    return http.get('/config/platforms')
  },
  getPreferences() {
    return http.get<{ preferences: UserPreferences }>('/config/preferences')
  },
  setPreferences(data: Partial<UserPreferences>) {
    return http.put('/config/preferences', data)
  },
}
