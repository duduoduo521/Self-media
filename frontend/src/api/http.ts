import axios from 'axios'

const http = axios.create({
  baseURL: '/api',
  timeout: 30000,
  withCredentials: true,
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
