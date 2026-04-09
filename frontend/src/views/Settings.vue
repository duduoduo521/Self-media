<template>
  <n-space vertical :size="16">
    <n-card title="API Key 配置">
      <n-form ref="apiKeyForm" :model="apiKeyForm" label-placement="left" label-width="100">
        <n-form-item label="提供商">
          <n-select v-model:value="apiKeyForm.provider" :options="[{ label: 'MiniMax', value: 'minimax' }]" style="width: 200px" />
        </n-form-item>
        <n-form-item label="API Key">
          <n-input v-model:value="apiKeyForm.key" type="password" show-password-on="click" placeholder="输入 MiniMax API Key" />
        </n-form-item>
        <n-form-item label="区域">
          <n-radio-group v-model:value="apiKeyForm.region">
            <n-radio value="cn">国内 (CN)</n-radio>
            <n-radio value="global">国际 (Global)</n-radio>
          </n-radio-group>
        </n-form-item>
        <n-button type="primary" :loading="savingKey" @click="handleSaveApiKey">保存</n-button>
      </n-form>
    </n-card>

    <n-card title="偏好设置">
      <n-form :model="prefs" label-placement="left" label-width="100">
        <n-form-item label="默认模式">
          <n-radio-group v-model:value="prefs.default_mode">
            <n-radio value="text">图文</n-radio>
            <n-radio value="video">视频</n-radio>
          </n-radio-group>
        </n-form-item>
        <n-form-item label="自动发布">
          <n-switch v-model:value="prefs.auto_publish" />
        </n-form-item>
        <n-form-item label="默认标签">
          <n-dynamic-tags v-model:value="prefs.default_tags" />
        </n-form-item>
        <n-button type="primary" :loading="savingPrefs" @click="handleSavePrefs">保存</n-button>
      </n-form>
    </n-card>

    <n-card title="修改密码">
      <n-form :model="passwordForm" label-placement="left" label-width="100">
        <n-form-item label="旧密码">
          <n-input v-model:value="passwordForm.old_password" type="password" />
        </n-form-item>
        <n-form-item label="新密码">
          <n-input v-model:value="passwordForm.new_password" type="password" show-password-on="click" />
        </n-form-item>
        <n-button type="primary" :loading="changingPassword" @click="handleChangePassword">修改</n-button>
      </n-form>
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { NCard, NForm, NFormItem, NInput, NSelect, NRadioGroup, NRadio, NButton, NSwitch, NDynamicTags, useMessage } from 'naive-ui'
import { authApi } from '@/api/auth'
import { configApi, type UserPreferences } from '@/api/config'
import { useUserStore } from '@/stores/user'

const message = useMessage()
const userStore = useUserStore()

const savingKey = ref(false)
const savingPrefs = ref(false)
const changingPassword = ref(false)

const apiKeyForm = reactive({
  provider: 'minimax',
  key: '',
  region: 'cn',
})

const prefs = reactive<UserPreferences>({
  default_mode: 'text',
  default_tags: [],
  auto_publish: false,
})

const passwordForm = reactive({
  old_password: '',
  new_password: '',
})

async function handleSaveApiKey() {
  savingKey.value = true
  try {
    await configApi.setApiKey(apiKeyForm)
    message.success('API Key 保存成功')
    apiKeyForm.key = ''
  } catch (e: any) {
    message.error(e.message || '保存失败')
  } finally {
    savingKey.value = false
  }
}

async function handleSavePrefs() {
  savingPrefs.value = true
  try {
    await configApi.setPreferences(prefs)
    message.success('偏好设置保存成功')
  } catch (e: any) {
    message.error(e.message || '保存失败')
  } finally {
    savingPrefs.value = false
  }
}

async function handleChangePassword() {
  changingPassword.value = true
  try {
    await authApi.changePassword(passwordForm)
    message.success('密码修改成功，请重新登录')
    await userStore.logout()
  } catch (e: any) {
    message.error(e.message || '修改失败')
  } finally {
    changingPassword.value = false
  }
}

onMounted(async () => {
  try {
    const { data } = await configApi.getPreferences()
    Object.assign(prefs, data.preferences)
  } catch {}
})
</script>
