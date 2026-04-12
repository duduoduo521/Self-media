<template>
  <n-space vertical :size="16">
    <n-card title="草稿箱">
      <template #header-extra>
        <n-button size="small" @click="fetchDrafts">刷新</n-button>
      </template>
      <n-data-table :columns="columns" :data="drafts" :bordered="false" :loading="loading" />
      <n-empty v-if="!loading && (!drafts || drafts.length === 0)" description="暂无草稿" style="margin-top: 40px" />
    </n-card>

    <n-modal v-model:show="showDetail" preset="card" title="草稿详情" style="width: 700px">
      <n-descriptions v-if="currentDraft" label-placement="top" :column="1">
        <n-descriptions-item label="主题">{{ currentDraft.topic }}</n-descriptions-item>
        <n-descriptions-item label="模式">{{ currentDraft.mode }}</n-descriptions-item>
        <n-descriptions-item label="状态">
          <n-tag :type="statusTagType[currentDraft.status]">{{ statusText[currentDraft.status] }}</n-tag>
        </n-descriptions-item>
        <n-descriptions-item label="平台">{{ currentDraft.platforms }}</n-descriptions-item>
        <n-descriptions-item label="原始内容">
          <n-input type="textarea" :value="currentDraft.original_content || ''" :rows="6" readonly />
        </n-descriptions-item>
        <n-descriptions-item label="适配内容" v-if="currentDraft.adapted_contents">
          <pre style="max-height: 200px; overflow: auto; font-size: 12px; background: #f5f5f5; padding: 8px;">{{ formatContents(currentDraft.adapted_contents) }}</pre>
        </n-descriptions-item>
        <n-descriptions-item label="配图" v-if="currentDraft.generated_images">
          <n-image-group>
            <n-space>
              <n-image v-for="(img, idx) in parseImages(currentDraft.generated_images)" :key="idx" :src="img" width="120" object-fit="cover" />
            </n-space>
          </n-image-group>
        </n-descriptions-item>
        <n-descriptions-item label="创建时间">{{ currentDraft.created_at }}</n-descriptions-item>
      </n-descriptions>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showDetail = false">关闭</n-button>
          <n-button type="primary" @click="handlePublish(currentDraft.id)">发布</n-button>
        </n-space>
      </template>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { NCard, NButton, NSpace, NDataTable, NTag, NEmpty, NModal, NDescriptions, NDescriptionsItem, NInput, NImage, NImageGroup, useMessage, useDialog } from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import { draftApi, type Draft } from '@/api/draft'

const message = useMessage()
const dialog = useDialog()
const drafts = ref<Draft[]>([])
const loading = ref(false)
const showDetail = ref(false)
const currentDraft = ref<Draft | null>(null)

function parseImages(imagesStr: string): string[] {
  try {
    return JSON.parse(imagesStr)
  } catch {
    return []
  }
}

function formatContents(contents: string): string {
  try {
    return JSON.stringify(JSON.parse(contents), null, 2)
  } catch {
    return contents
  }
}

const statusTagType: Record<string, any> = {
  draft: 'default',
  published: 'success',
  partially_published: 'warning',
}

const statusText: Record<string, string> = {
  draft: '草稿',
  published: '已发布',
  partially_published: '部分发布',
}

const columns: DataTableColumns<Draft> = [
  { title: '主题', key: 'topic', ellipsis: { tooltip: true } },
  { title: '模式', key: 'mode', width: 100 },
  {
    title: '状态', key: 'status', width: 100,
    render: (row) => h(NTag, { type: statusTagType[row.status] || 'default', size: 'small' }, { default: () => statusText[row.status] || row.status }),
  },
  { title: '平台', key: 'platforms', ellipsis: { tooltip: true } },
  { title: '创建时间', key: 'created_at', width: 180 },
  {
    title: '操作', key: 'actions', width: 200,
    render: (row) => {
      const actions = [
        h(NButton, { size: 'tiny', type: 'primary', onClick: () => handleView(row) }, { default: () => '查看' }),
        h(NButton, { size: 'tiny', type: 'warning', style: 'margin-left: 8px', onClick: () => handlePublish(row.id) }, { default: () => '发布' }),
        h(NButton, { size: 'tiny', type: 'error', style: 'margin-left: 8px', onClick: () => handleDelete(row.id) }, { default: () => '删除' }),
      ]
      return h(NSpace, { size: 0 }, { default: () => actions })
    },
  },
]

function handleView(draft: Draft) {
  currentDraft.value = draft
  showDetail.value = true
}

async function fetchDrafts() {
  loading.value = true
  try {
    const data = await draftApi.list()
    drafts.value = data.data || []
  } catch (e: any) {
    message.error(e.message || '获取草稿失败')
    drafts.value = []
  } finally {
    loading.value = false
  }
}

async function handlePublish(id: string) {
  try {
    const result = await draftApi.publish(id)
    if (result.data.warning) {
      dialog.warning({
        title: '提示',
        content: result.data.warning,
      })
    } else {
      message.success('发布成功')
    }
    await fetchDrafts()
  } catch (e: any) {
    message.error(e.message || '发布失败')
  }
}

async function handleDelete(id: string) {
  dialog.warning({
    title: '确认删除',
    content: '确定要删除这个草稿吗？',
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await draftApi.delete(id)
        message.success('删除成功')
        await fetchDrafts()
      } catch (e: any) {
        message.error(e.message || '删除失败')
      }
    },
  })
}

onMounted(() => {
  fetchDrafts()
})
</script>
