<template>
  <div class="app">
    <!-- 配对视图 -->
    <div v-if="!paired" class="page">
      <div class="brand">
        <div class="brand-icon">📋</div>
        <h1>浮云剪贴板</h1>
        <p class="brand-sub">与电脑互相同步剪贴板</p>
      </div>

      <div class="card">
        <label class="field">
          <span class="field-label">电脑地址</span>
          <input
            v-model="form.host"
            class="input"
            type="text"
            inputmode="url"
            placeholder="192.168.1.100:45817"
          />
        </label>
        <p class="hint">
          在电脑端「设置 → 手机同步」中查看地址；手机与电脑需在同一 Wi-Fi
        </p>
      </div>

      <div class="card">
        <label class="field">
          <span class="field-label">配对码</span>
          <input
            v-model="form.code"
            class="input code-input"
            type="text"
            inputmode="numeric"
            maxlength="6"
            placeholder="6 位数字"
          />
        </label>
        <p class="hint">配对码在电脑端显示，5 分钟内有效，仅可配对一次</p>
      </div>

      <button class="btn-primary" :disabled="pairing" @click="onPair">
        {{ pairing ? '配对中…' : '配对' }}
      </button>
      <p v-if="error" class="error">{{ error }}</p>
    </div>

    <!-- 主页视图 -->
    <div v-else class="page">
      <div class="statusbar">
        <div class="status-left">
          <span :class="['dot', loopRunning ? 'dot-on' : 'dot-off']" />
          <span v-if="loopRunning">已连接 {{ pcName }}</span>
          <span v-else>连接中断，重试中…</span>
        </div>
        <button class="btn-text" @click="onUnpair">断开</button>
      </div>

      <div class="card">
        <h2 class="card-title">发送到电脑</h2>
        <textarea
          v-model="draft"
          class="textarea"
          rows="4"
          placeholder="输入文本，发送后即可在电脑上粘贴"
        />
        <div class="btn-row">
          <button class="btn-primary flex" :disabled="sending" @click="onSendText">
            {{ sending ? '发送中…' : '发送文本' }}
          </button>
          <button class="btn-tonal flex" :disabled="sending" @click="onPickImage">
            发送图片
          </button>
        </div>
        <button class="btn-text btn-clipboard" @click="onSendClipboard">
          发送当前剪贴板文本
        </button>
        <input
          ref="fileInput"
          type="file"
          accept="image/*"
          class="hidden-input"
          @change="onFileChosen"
        />
      </div>

      <div class="card">
        <h2 class="card-title">收到的内容</h2>
        <p v-if="items.length === 0" class="hint">
          电脑复制的内容会出现在这里，并自动写入剪贴板
        </p>
        <div v-for="item in items" :key="item.id" class="item" @click="onCopyItem(item)">
          <div v-if="item.kind === 'text'" class="item-text">{{ item.text }}</div>
          <img v-else class="item-image" :src="item.dataUrl" alt="收到的图片" />
          <div class="item-meta">
            {{ item.kind === 'text' ? '点击复制' : '点击复制图片' }}
          </div>
        </div>
      </div>
    </div>

    <div v-if="toast" class="toast">{{ toast }}</div>
  </div>
</template>

<script setup>
import {onMounted, onUnmounted, reactive, ref} from 'vue'
import {listen} from '@tauri-apps/api/event'
import {api} from './api'

const paired = ref(false)
const pairing = ref(false)
const sending = ref(false)
const error = ref('')
const toast = ref('')
const pcName = ref('')
const loopRunning = ref(false)
const draft = ref('')
const items = ref([])
const fileInput = ref(null)
const form = reactive({host: '', code: ''})
let unlistenItem = null
let toastTimer = null

function showToast(msg) {
  toast.value = msg
  clearTimeout(toastTimer)
  toastTimer = setTimeout(() => (toast.value = ''), 2000)
}

onMounted(async () => {
  // 恢复本地保存的配对信息
  const saved = localStorage.getItem('fuyun-sync')
  if (saved) {
    try {
      const cfg = JSON.parse(saved)
      form.host = cfg.host || ''
      pcName.value = cfg.pcName || ''
      if (cfg.token) {
        paired.value = true
        await startLoop()
      }
    } catch {}
  }
  // 监听 Rust 推送的新内容
  unlistenItem = await listen('sync-item', (event) => {
    const payload = event.payload
    items.value.unshift(payload)
    if (items.value.length > 50) items.value.pop()
  })
})

onUnmounted(() => {
  if (unlistenItem) unlistenItem()
})

async function onPair() {
  error.value = ''
  const [host, portStr] = form.host.split(':')
  const port = Number(portStr)
  if (!host || !port || port < 1 || port > 65535) {
    error.value = '地址格式应为 IP:端口，如 192.168.1.100:45817'
    return
  }
  if (form.code.length !== 6) {
    error.value = '请输入 6 位配对码'
    return
  }
  pairing.value = true
  try {
    const result = await api.pair(host, port, form.code)
    paired.value = true
    pcName.value = result.pcName || '电脑'
    localStorage.setItem(
      'fuyun-sync',
      JSON.stringify({host, port, token: result.token, pcName: pcName.value}),
    )
    await startLoop()
  } catch (e) {
    error.value = String(e)
  } finally {
    pairing.value = false
  }
}

async function startLoop() {
  const saved = JSON.parse(localStorage.getItem('fuyun-sync') || '{}')
  if (!saved.token) return
  loopRunning.value = true
  try {
    await api.startSyncLoop(saved.host, saved.port, saved.token)
  } catch {
    loopRunning.value = false
  }
}

function onUnpair() {
  api.stopSyncLoop()
  localStorage.removeItem('fuyun-sync')
  paired.value = false
  loopRunning.value = false
  items.value = []
  form.code = ''
}

async function onSendText() {
  const text = draft.value.trim()
  if (!text) {
    showToast('请输入内容')
    return
  }
  sending.value = true
  try {
    const saved = JSON.parse(localStorage.getItem('fuyun-sync'))
    await api.pushText(saved.host, saved.port, saved.token, text)
    draft.value = ''
    showToast('已发送到电脑剪贴板')
  } catch (e) {
    showToast(String(e))
  } finally {
    sending.value = false
  }
}

async function onSendClipboard() {
  sending.value = true
  try {
    const text = await api.readClipboard()
    if (!text) {
      showToast('剪贴板为空')
      return
    }
    const saved = JSON.parse(localStorage.getItem('fuyun-sync'))
    await api.pushText(saved.host, saved.port, saved.token, text)
    showToast('已发送到电脑剪贴板')
  } catch (e) {
    showToast(String(e))
  } finally {
    sending.value = false
  }
}

function onPickImage() {
  fileInput.value?.click()
}

async function onFileChosen(event) {
  const file = event.target.files?.[0]
  event.target.value = ''
  if (!file) return
  sending.value = true
  try {
    const base64 = await fileToBase64(file)
    const saved = JSON.parse(localStorage.getItem('fuyun-sync'))
    await api.pushImage(saved.host, saved.port, saved.token, base64)
    showToast('图片已发送到电脑')
  } catch (e) {
    showToast(String(e))
  } finally {
    sending.value = false
  }
}

function fileToBase64(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const result = String(reader.result)
      resolve(result.slice(result.indexOf(',') + 1))
    }
    reader.onerror = reject
    reader.readAsDataURL(file)
  })
}

async function onCopyItem(item) {
  try {
    if (item.kind === 'text') {
      await api.writeClipboard(item.text)
    } else {
      // 图片已在收到时自动写入剪贴板；此处重新写入当前项
      await api.writeImageToClipboard(item.id)
    }
    showToast('已复制')
  } catch (e) {
    showToast(String(e))
  }
}
</script>
