import http from './http'

export interface UserPreferences {
  default_mode: string
  default_tags: string[]
  auto_publish: boolean
}

export interface ModelConfig {
  text_model: string
  image_model: string
  video_model: string
  speech_model: string
  music_model: string
}

export const MODEL_OPTIONS = {
  text: [
    { label: 'MiniMax-M2.7 (推荐)', value: 'MiniMax-M2.7' },
    { label: 'MiniMax-M2.7-highspeed', value: 'MiniMax-M2.7-highspeed' },
    { label: 'abab6.5s-chat', value: 'abab6.5s-chat' },
    { label: 'abab6.5-chat', value: 'abab6.5-chat' },
  ],
  image: [
    { label: 'image-01 (推荐)', value: 'image-01' },
    { label: 'image-01-highres', value: 'image-01-highres' },
  ],
  video: [
    { label: 'video-01 (推荐)', value: 'video-01' },
    { label: 'video-01-t2v', value: 'video-01-t2v' },
  ],
  speech: [
    { label: 'speech-02-hd (推荐)', value: 'speech-02-hd' },
    { label: 'speech-02', value: 'speech-02' },
    { label: 'speech-01', value: 'speech-01' },
  ],
  music: [
    { label: 'music-01 (推荐)', value: 'music-01' },
  ],
}

export const configApi = {
  setApiKey(data: { provider: string; key: string; region: string }) {
    return http.put('/api-key', data)
  },
  getApiKey() {
    return http.get<{ provider: string; key: string; region: string }>('/api-key')
  },
  getPlatforms() {
    return http.get('/platforms')
  },
  getPreferences() {
    return http.get<{ preferences: UserPreferences }>('/preferences')
  },
  setPreferences(data: Partial<UserPreferences>) {
    return http.put('/preferences', data)
  },
  getModelConfig() {
    return http.get<ModelConfig>('/models')
  },
  setModelConfig(data: ModelConfig) {
    return http.put('/models', data)
  },
}
