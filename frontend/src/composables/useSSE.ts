/**
 * SSE 流式接收 Composable
 * 用于接收 AI 生成的流式文本
 */

import { ref, onUnmounted } from 'vue'

export interface SSEMessage {
  type: 'text' | 'step' | 'status' | 'error' | 'complete'
  data: any
}

export function useSSE() {
  const content = ref('')
  const isStreaming = ref(false)
  const error = ref<string | null>(null)
  const currentStep = ref(0)

  let eventSource: EventSource | null = null

  async function startStream(taskId: string) {
    // 清理之前的连接
    stopStream()

    isStreaming.value = true
    error.value = null
    content.value = ''
    currentStep.value = 0

    // Tauri 端通过事件监听
    if ('__TAURI_INTERNALS__' in window) {
      try {
        // @ts-ignore - Tauri API 类型问题
        const { listen } = await import('@tauri-apps/api/event')
        // @ts-ignore
        const unlisten = await listen<string>('text-stream', (event: any) => {
          try {
            const data = JSON.parse(event.payload)
            handleMessage(data)
          } catch (e) {
            content.value += event.payload
          }
        })

        // 保存 unlisten 函数以便后续清理
        ;(window as any).__sseUnlisten = unlisten
      } catch (e) {
        console.error('Tauri event listen error:', e)
        // 回退到 Web SSE
        startWebSSE(taskId)
      }
    } else {
      // Web 端通过 EventSource
      startWebSSE(taskId)
    }
  }

  function startWebSSE(taskId: string) {
    eventSource = new EventSource(`/api/tasks/${taskId}/stream`)

    eventSource.onopen = () => {
      isStreaming.value = true
    }

    eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        handleMessage(data)
      } catch (e) {
        // 原始文本数据
        content.value += event.data
      }
    }

    eventSource.onerror = (e) => {
      console.error('SSE error:', e)
      isStreaming.value = false
      if (eventSource?.readyState === EventSource.CLOSED) {
        error.value = '连接已关闭'
      }
    }
  }

  function handleMessage(data: any) {
    switch (data.type) {
      case 'text':
      case 'content':
        content.value += data.delta?.content || data.content || ''
        break

      case 'step':
        currentStep.value = data.step
        break

      case 'status':
        if (data.status === 'completed') {
          isStreaming.value = false
        } else if (data.status === 'failed') {
          isStreaming.value = false
          error.value = data.error || '任务失败'
        }
        break

      case 'complete':
        isStreaming.value = false
        break

      case 'error':
        isStreaming.value = false
        error.value = data.message || data.error || '未知错误'
        break
    }
  }

  function stopStream() {
    if (eventSource) {
      eventSource.close()
      eventSource = null
    }

    // Tauri unlisten
    if ((window as any).__sseUnlisten) {
      ;(window as any).__sseUnlisten()
      ;(window as any).__sseUnlisten = null
    }
  }

  function reset() {
    stopStream()
    content.value = ''
    error.value = null
    isStreaming.value = false
    currentStep.value = 0
  }

  onUnmounted(() => {
    stopStream()
  })

  return {
    content,
    isStreaming,
    error,
    currentStep,
    startStream,
    stopStream,
    reset,
  }
}
