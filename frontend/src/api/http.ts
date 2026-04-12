import axios from 'axios'

const http = axios.create({
  baseURL: '/api',
  timeout: 120000,
  withCredentials: true,
})

// 从 Cookie 中读取 CSRF token
function getCsrfToken(): string | null {
  const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/)
  return match ? decodeURIComponent(match[1]) : null
}

// 请求拦截：非 GET 请求添加 CSRF token header
http.interceptors.request.use((config) => {
  if (config.method && !['get', 'head', 'options'].includes(config.method.toLowerCase())) {
    const csrfToken = getCsrfToken()
    if (csrfToken) {
      config.headers['X-CSRF-Token'] = csrfToken
    }
  }
  return config
})

// 响应拦截：统一错误处理
http.interceptors.response.use(
  (response) => response,
  (error) => {
    const status = error.response?.status
    if (status === 401) {
      // 跳转登录
      window.location.hash = '#/login'
    }
    return Promise.reject(error.response?.data || error)
  },
)

export default http
