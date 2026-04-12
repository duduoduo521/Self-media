<template>
  <n-space vertical :size="16">
    <n-card title="API Key 配置">
      <n-form ref="apiKeyFormRef" :model="apiKeyForm" label-placement="left" label-width="100">
        <n-form-item label="提供商">
          <n-select v-model:value="apiKeyForm.provider" :options="[{ label: 'MiniMax', value: 'minimax' }]" style="width: 200px" />
        </n-form-item>
        <n-form-item label="API Key">
          <n-input v-model:value="apiKeyForm.key" type="password" show-password-on="click" placeholder="输入 MiniMax API Key" />
        </n-form-item>
        <n-button type="primary" :loading="savingKey" @click="handleSaveApiKey">保存</n-button>
      </n-form>
    </n-card>

    <n-card title="模型配置">
      <n-form :model="modelConfig" label-placement="left" label-width="100">
        <n-form-item label="文本模型">
          <n-select v-model:value="modelConfig.text_model" :options="MODEL_OPTIONS.text" style="width: 300px" />
        </n-form-item>
        <n-form-item label="图片模型">
          <n-select v-model:value="modelConfig.image_model" :options="MODEL_OPTIONS.image" style="width: 300px" />
        </n-form-item>
        <n-form-item label="视频模型">
          <n-select v-model:value="modelConfig.video_model" :options="MODEL_OPTIONS.video" style="width: 300px" />
        </n-form-item>
        <n-form-item label="配音模型">
          <n-select v-model:value="modelConfig.speech_model" :options="MODEL_OPTIONS.speech" style="width: 300px" />
        </n-form-item>
        <n-form-item label="音乐模型">
          <n-select v-model:value="modelConfig.music_model" :options="MODEL_OPTIONS.music" style="width: 300px" />
        </n-form-item>
        <n-button type="primary" :loading="savingModel" @click="handleSaveModel">保存</n-button>
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
import type { FormInst } from 'naive-ui'
import { NCard, NForm, NFormItem, NInput, NSelect, NSpace, NButton, NSwitch, NDynamicTags, NRadio, NRadioGroup, useMessage } from 'naive-ui'
import { authApi } from '@/api/auth'
import { configApi, MODEL_OPTIONS, type ModelConfig, type UserPreferences } from '@/api/config'
import { useUserStore } from '@/stores/user'

const message = useMessage()
const userStore = useUserStore()

const apiKeyFormRef = ref<FormInst | null>(null)
const savingKey = ref(false)
const savingPrefs = ref(false)
const savingModel = ref(false)
const changingPassword = ref(false)

const apiKeyForm = reactive({
  provider: 'minimax',
  key: '',
})

const modelConfig = reactive<ModelConfig>({
  text_model: 'MiniMax-M2.7',
  image_model: 'image-01',
  video_model: 'video-01',
  speech_model: 'speech-02-hd',
  music_model: 'music-01',
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
    await configApi.setApiKey({
      provider: apiKeyForm.provider,
      key: apiKeyForm.key,
      region: 'cn',
    })
    message.success('API Key 保存成功')
    apiKeyForm.key = ''
  } catch (e: any) {
    message.error(e.message || '保存失败')
  } finally {
    savingKey.value = false
  }
}

async function handleSaveModel() {
  savingModel.value = true
  try {
    await configApi.setModelConfig(modelConfig)
    message.success('模型配置保存成功')
  } catch (e: any) {
    message.error(e.message || '保存失败')
  } finally {
    savingModel.value = false
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

  try {
    const { data: keyData } = await configApi.getApiKey()
    if (keyData.key) {
      apiKeyForm.key = keyData.key
    }
  } catch {}

  try {
    const { data: modelData } = await configApi.getModelConfig()
    Object.assign(modelConfig, modelData)
  } catch {}
})
</script>
