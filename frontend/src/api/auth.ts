import http from './http'

export interface LoginRequest {
  username: string
  password: string
}

export interface RegisterRequest {
  username: string
  password: string
}

export interface User {
  id: number
  username: string
  created_at: string
}

export interface Session {
  id: number
  user_id: number
  token: string
  expires_at: string
}

export const authApi = {
  register(data: RegisterRequest) {
    return http.post<{ user: User; session: Session }>('/auth/register', data)
  },
  login(data: LoginRequest) {
    return http.post<{ session: Session }>('/auth/login', data)
  },
  logout() {
    return http.delete('/auth/logout')
  },
  changePassword(data: { old_password: string; new_password: string }) {
    return http.put('/auth/password', data)
  },
}
