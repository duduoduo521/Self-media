/**
 * Tauri IPC 适配
 * 用于在 Tauri 桌面端和 Web 端之间切换
 */

import { ref, onMounted } from 'vue'

// 检测是否在 Tauri 环境
export function isTauri(): boolean {
  return '__TAURI_INTERNALS__' in window
}

// Tauri API (懒加载)
let tauriApi: any = null

async function getTauriApi() {
  if (!tauriApi) {
    try {
      const { invoke } = await import('@tauri-apps/api/tauri')
      tauriApi = { invoke }
    } catch (e) {
      console.warn('Tauri API not available')
      return null
    }
  }
  return tauriApi
}

export function useTauri() {
  const isDesktop = ref(false)
  const isLoading = ref(true)

  onMounted(async () => {
    isDesktop.value = isTauri()
    isLoading.value = false
  })

  async function invoke<T>(command: string, args?: Record<string, any>): Promise<T> {
    const api = await getTauriApi()
    if (!api) {
      throw new Error('Tauri API not available')
    }
    return api.invoke(command, args)
  }

  return {
    isDesktop,
    isLoading,
    invoke,
    isTauri,
  }
}
