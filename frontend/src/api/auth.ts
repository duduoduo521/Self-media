import http from './http'

export interface LoginRequest {
  username: string
  password: string
}

export interface RegisterRequest {
  username: string
  password: string
  confirmPassword: string
  email: string
  minimaxApiKey: string
  phone?: string
}

export interface User {
  id: number
  username: string
  email?: string
  phone?: string
  created_at: string
}

export interface Session {
  id: number
  user_id: number
  token: string
  expires_at: string
}

export interface CheckApiKeyResponse {
  valid: boolean
  message: string
}

export const authApi = {
  register(data: RegisterRequest) {
    return http.post<{ code: number; data: { user: User; session: Session } }>('/auth/register', {
      username: data.username,
      password: data.password,
      email: data.email,
      minimax_api_key: data.minimaxApiKey,
      phone: data.phone,
    })
  },
  login(data: LoginRequest) {
    return http.post<{ code: number; data: { session: Session; token: string } }>('/auth/login', data)
  },
  logout() {
    return http.delete('/auth/logout')
  },
  changePassword(data: { old_password: string; new_password: string }) {
    return http.put('/auth/password', data)
  },
  checkApiKey(apiKey: string) {
    return http.post<CheckApiKeyResponse>('/auth/check-apikey', { api_key: apiKey })
  },
}
