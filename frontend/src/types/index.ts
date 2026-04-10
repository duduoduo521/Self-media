/**
 * 前端类型定义
 */

// ============ 用户相关 ============

export interface User {
  id: string
  username: string
  email?: string
  created_at: string
}

export interface LoginParams {
  username: string
  password: string
}

export interface RegisterParams {
  username: string
  password: string
  email?: string
}

// ============ 平台相关 ============

export type Platform = 'xiaohongshu' | 'douyin' | 'wechat' | 'bilibili' | 'weibo' | 'toutiao'

export interface PlatformConfig {
  platform: Platform
  name: string
  enabled: boolean
  credentials?: PlatformCredential
}

export interface PlatformCredential {
  cookies: string
  headers?: Record<string, string>
  token?: string
  expires_at?: string
}

// ============ 热点相关 ============

export type HotspotSource = 'weibo' | 'douyin' | 'xiaohongshu' | 'bilibili' | 'toutiao' | 'zhihu'

export interface HotspotItem {
  id: string
  platform: HotspotSource
  title: string
  url: string
  hot_count?: string
  rank?: number
  timestamp: string
}

// ============ 任务相关 ============

export type TaskMode = 'text' | 'video'

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed'

export interface Task {
  id: string
  user_id: string
  mode: TaskMode
  topic: string
  platforms: Platform[]
  status: TaskStatus
  current_step: number
  result?: TaskResult
  error?: string
  created_at: string
  updated_at: string
}

export interface TaskResult {
  posts: PostResult[]
}

export interface PostResult {
  platform: Platform
  success: boolean
  post_id?: string
  url?: string
  error?: string
}

// ============ AI 相关 ============

export type AIModelType = 'text' | 'image' | 'video'

export interface AIGenerateParams {
  model: AIModelType
  prompt: string
  options?: {
    image_count?: number
    duration?: number
    style?: string
  }
}

// ============ API 响应 ============

export interface ApiResponse<T = any> {
  success: boolean
  data?: T
  error?: string
  message?: string
}

export interface PaginatedResponse<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

// ============ SSE 事件 ============

export interface SSETextEvent {
  type: 'text'
  delta: {
    content: string
  }
}

export interface SSEStepEvent {
  type: 'step'
  step: number
  name: string
}

export interface SSEStatusEvent {
  type: 'status'
  status: TaskStatus
  message?: string
}

export type SSEEvent = SSETextEvent | SSEStepEvent | SSEStatusEvent
