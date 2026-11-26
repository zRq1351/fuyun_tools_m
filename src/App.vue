<template>
  <div class="app">
    <!-- 配对视图 -->
    <div v-if="!paired" class="page">

      <div class="card">
        <div class="card-title-row">
          <h2 class="card-title">发现的电脑</h2>
          <button class="btn-text" :disabled="discovering" @click="onDiscover">
            {{ discovering ? '搜索中…' : '重新搜索' }}
          </button>
        </div>
        <p v-if="discovering && !computers.length" class="hint">
          正在搜索同一 Wi-Fi 下的电脑…
        </p>
        <p v-else-if="!computers.length" class="hint">
          未发现电脑。请确认电脑端已开启「手机同步」，且手机与电脑连的是同一个 Wi-Fi。
        </p>
        <div
          v-for="c in computers"
          :key="`${c.host}:${c.port}`"
          class="pc-item"
          :class="{ selected: form.host === `${c.host}:${c.port}` }"
          @click="selectPc(c)"
        >
          <div class="pc-item-main">
            <div class="pc-name">{{ c.name }}</div>
            <div class="pc-addr">{{ c.host }}:{{ c.port }}</div>
          </div>
          <span v-if="form.host === `${c.host}:${c.port}`" class="pc-check">已选择</span>
        </div>

        <button class="btn-text btn-clipboard" @click="manualMode = !manualMode">
          {{ manualMode ? '收起手动输入' : '找不到电脑？手动输入地址' }}
        </button>
        <label v-if="manualMode" class="field">
          <span class="field-label">电脑地址</span>
          <input
            v-model="form.host"
            class="input"
            type="text"
            inputmode="url"
            placeholder="192.168.1.100:45817"
          />
        </label>
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
        <h2 class="card-title">投屏到电脑</h2>
        <p class="hint">
          通过同步连接远程开启电脑端的手机投屏，电脑会自动弹出投屏窗口
        </p>
        <button class="btn-primary" :disabled="mirroring" @click="onMirrorStart">
          {{ mirroring ? '开启中…' : '投屏到电脑' }}
        </button>
        <p v-if="mirrorStatusText" class="hint">{{ mirrorStatusText }}</p>
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
const computers = ref([])
const discovering = ref(false)
const manualMode = ref(false)
const mirroring = ref(false)
const mirrorStatusText = ref('')
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
        refreshMirrorStatus()
      }
    } catch {}
  }
  if (!paired.value) onDiscover()
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

// 自动发现同一局域网内的电脑（mDNS 优先，子网扫描兜底）
async function onDiscover() {
  if (discovering.value) return
  discovering.value = true
  try {
    const r = await api.discover(2000)
    computers.value = r.computers || []
    // 只发现一台时自动选中，用户只需输入配对码
    if (computers.value.length === 1) {
      selectPc(computers.value[0])
    }
  } catch (e) {
    showToast(String(e))
  } finally {
    discovering.value = false
  }
}

function selectPc(c) {
  form.host = `${c.host}:${c.port}`
  pcName.value = c.name
}

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
  computers.value = []
  onDiscover()
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

// 远程开启电脑端投屏：复用同步令牌走后端 HTTP，电脑自动弹出投屏窗口
async function onMirrorStart() {
  mirroring.value = true
  try {
    const saved = JSON.parse(localStorage.getItem('fuyun-sync'))
    const r = await api.mirrorStart(saved.host, saved.port, saved.token)
    if (r && r.ok) {
      mirrorStatusText.value = `投屏运行中${r.deviceName ? `（${r.deviceName}）` : ''}`
      showToast('投屏已开启，请看电脑屏幕')
    } else {
      mirrorStatusText.value = ''
      showToast((r && r.error) || '开启失败')
    }
  } catch (e) {
    showToast(String(e))
  } finally {
    mirroring.value = false
  }
}

async function refreshMirrorStatus() {
  try {
    const saved = JSON.parse(localStorage.getItem('fuyun-sync') || '{}')
    if (!saved.token) return
    const r = await api.mirrorStatus(saved.host, saved.port, saved.token)
    if (r && r.running) {
      mirrorStatusText.value = `投屏运行中${r.deviceName ? `（${r.deviceName}）` : ''}`
    } else {
      mirrorStatusText.value = ''
    }
  } catch { /* 状态查询失败不打扰用户 */ }
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
