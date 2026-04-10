<template>
  <div class="register-container">
    <n-card class="register-card" title="注册 Self-Media">
      <n-form ref="formRef" :model="form" :rules="rules" size="large">
        <n-form-item label="用户名" path="username">
          <n-input v-model:value="form.username" placeholder="3-32 位字母、数字或下划线" />
        </n-form-item>

        <n-form-item label="密码" path="password">
          <n-input
            v-model:value="form.password"
            type="password"
            show-password-on="click"
            placeholder="8-64 位，含大小写和数字"
          />
        </n-form-item>

        <n-form-item label="确认密码" path="confirmPassword">
          <n-input
            v-model:value="form.confirmPassword"
            type="password"
            show-password-on="click"
            placeholder="再次输入密码"
          />
        </n-form-item>

        <n-form-item label="邮箱" path="email">
          <n-input v-model:value="form.email" placeholder="用于接收通知" />
        </n-form-item>

        <n-form-item path="minimaxApiKey" :status="apiKeyError ? 'error' : undefined" :feedback="apiKeyError || undefined">
          <template #label>
            <span>MiniMax API Key <n-text depth="3">(注册时自动验证)</n-text></span>
          </template>
          <n-input
            v-model:value="form.minimaxApiKey"
            placeholder="MiniMax Token 套餐的 API Key"
            :status="apiKeyError ? 'error' : undefined"
            @input="apiKeyError = ''"
          />
        </n-form-item>

        <n-form-item label="手机号码（可选）" path="phone">
          <n-input v-model:value="form.phone" placeholder="11 位手机号码" />
        </n-form-item>

        <n-space vertical :size="16">
          <n-button
            type="primary"
            block
            :loading="loading"
            :disabled="!isFormValid"
            @click="handleRegister"
          >
            注册
          </n-button>
          <n-button block @click="handleBackToLogin">返回登录</n-button>
        </n-space>
      </n-form>
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { NCard, NForm, NFormItem, NInput, NButton, NSpace, NText, useMessage, FormInst, FormRules } from 'naive-ui'
import { authApi } from '@/api/auth'

const router = useRouter()
const message = useMessage()
const formRef = ref<FormInst | null>(null)
const loading = ref(false)
const apiKeyError = ref('')

const form = reactive({
  username: '',
  password: '',
  confirmPassword: '',
  email: '',
  minimaxApiKey: '',
  phone: '',
})

const validatePassword = () => {
  if (form.password !== form.confirmPassword) {
    return '两次输入的密码不一致'
  }
  return null
}

const rules: FormRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    {
      validator: (_rule, value: string) => {
        if (value.length < 3 || value.length > 32) {
          return new Error('用户名长度需在 3-32 之间')
        }
        if (!/^[a-zA-Z0-9_]+$/.test(value)) {
          return new Error('用户名仅允许字母、数字、下划线')
        }
        return true
      },
      trigger: 'blur',
    },
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    {
      validator: (_rule, value: string) => {
        if (value.length < 8 || value.length > 64) {
          return new Error('密码长度需在 8-64 之间')
        }
        if (!/[A-Z]/.test(value)) {
          return new Error('密码必须包含大写字母')
        }
        if (!/[a-z]/.test(value)) {
          return new Error('密码必须包含小写字母')
        }
        if (!/[0-9]/.test(value)) {
          return new Error('密码必须包含数字')
        }
        return true
      },
      trigger: 'blur',
    },
  ],
  confirmPassword: [
    { required: true, message: '请确认密码', trigger: 'blur' },
    {
      validator: () => {
        const error = validatePassword()
        if (error) return new Error(error)
        return true
      },
      trigger: 'blur',
    },
  ],
  email: [
    { required: true, message: '请输入邮箱', trigger: 'blur' },
    {
      validator: (_rule, value: string) => {
        const emailRegex = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/
        if (!emailRegex.test(value)) {
          return new Error('邮箱格式不正确')
        }
        return true
      },
      trigger: 'blur',
    },
  ],
  minimaxApiKey: [
    { required: true, message: '请输入 MiniMax API Key', trigger: 'blur' },
    {
      validator: (_rule, value: string) => {
        if (!value.startsWith('sk-')) {
          return new Error('API Key 格式不正确，应以 sk- 开头')
        }
        return true
      },
      trigger: 'blur',
    },
  ],
  phone: [
    {
      validator: (_rule, value: string) => {
        if (value && value.length !== 11) {
          return new Error('手机号码必须为 11 位')
        }
        if (value && !/^\d+$/.test(value)) {
          return new Error('手机号码只能包含数字')
        }
        return true
      },
      trigger: 'blur',
    },
  ],
}

const isFormValid = computed(() => {
  return (
    form.username.length >= 3 &&
    form.password.length >= 8 &&
    form.password === form.confirmPassword &&
    /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/.test(form.email) &&
    form.minimaxApiKey.startsWith('sk-') &&
    (!form.phone || (form.phone.length === 11 && /^\d+$/.test(form.phone)))
  )
})

async function handleRegister() {
  try {
    await formRef.value?.validate()
  } catch {
    return
  }

  if (form.password !== form.confirmPassword) {
    message.error('两次输入的密码不一致')
    return
  }

  apiKeyError.value = ''
  loading.value = true
  try {
    await authApi.register({
      username: form.username,
      password: form.password,
      confirmPassword: form.confirmPassword,
      email: form.email,
      minimaxApiKey: form.minimaxApiKey,
      phone: form.phone || undefined,
    })
    message.success('注册成功，请登录')
    router.push({ name: 'Login' })
  } catch (e: any) {
    const errorMessage = e.message || e.error || '注册失败'

    if (errorMessage.includes('MiniMax API Key')) {
      apiKeyError.value = 'MiniMax API Key 无效，请检查后重试'
    } else {
      message.error(errorMessage)
    }
  } finally {
    loading.value = false
  }
}

function handleBackToLogin() {
  router.push({ name: 'Login' })
}
</script>

<style scoped>
.register-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.register-card {
  width: 100%;
  max-width: 420px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
}
</style>
