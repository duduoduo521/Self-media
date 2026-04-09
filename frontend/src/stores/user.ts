import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { authApi, type User } from '@/api/auth'

export const useUserStore = defineStore('user', () => {
  const user = ref<User | null>(null)
  const token = ref<string | null>(localStorage.getItem('token'))

  const isLoggedIn = computed(() => !!token.value)

  async function register(username: string, password: string) {
    const { data } = await authApi.register({ username, password })
    user.value = data.user
    token.value = data.session.token
    localStorage.setItem('token', data.session.token)
  }

  async function login(username: string, password: string) {
    const { data } = await authApi.login({ username, password })
    token.value = data.session.token
    localStorage.setItem('token', data.session.token)
  }

  async function logout() {
    try {
      await authApi.logout()
    } finally {
      user.value = null
      token.value = null
      localStorage.removeItem('token')
    }
  }

  return { user, token, isLoggedIn, register, login, logout }
})
