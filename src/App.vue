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
          <!-- 选中后把「已选择」换成扫码按钮，扫电脑上显示的二维码自动填配对码 -->
          <button
            v-if="form.host === `${c.host}:${c.port}`"
            class="btn-scan"
            aria-label="扫码配对"
            @click.stop="onScanQr"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                 stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 8V5a2 2 0 0 1 2-2h3" />
              <path d="M16 3h3a2 2 0 0 1 2 2v3" />
              <path d="M21 16v3a2 2 0 0 1-2 2h-3" />
              <path d="M8 21H5a2 2 0 0 1-2-2v-3" />
              <line x1="3" y1="12" x2="21" y2="12" />
            </svg>
          </button>
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
        <div class="btn-row mt-10">
          <!-- 选中电脑时扫码入口在电脑条目上；手动输入地址时这里补一个 -->
          <button v-if="!form.host" class="btn-tonal flex" @click="onScanQr">扫码配对</button>
          <button class="btn-tonal flex" @click="onPastePairInfo">粘贴信息</button>
        </div>
        <button
          v-if="remembered.length"
          class="btn-text btn-clipboard"
          @click="showDevicePicker = true"
        >
          已配对的电脑（{{ remembered.length }}）
        </button>
        <p class="hint">电脑端「手机同步」页会显示二维码，扫一下即可填入地址和配对码</p>
      </div>

      <button class="btn-primary" :disabled="pairing" @click="onPair">
        {{ pairing ? '配对中…' : '配对' }}
      </button>
      <p v-if="error" class="error">{{ error }}</p>
    </div>

    <!-- 主页视图 -->
    <div v-else class="page">
      <div class="statusbar" :class="`status-${statusKind}`">
        <span class="dot" :class="`dot-${statusKind}`" />
        <div class="status-text">
          <div class="status-main">{{ statusText }}</div>
          <div v-if="lastSyncAgo" class="status-sub">{{ lastSyncAgo }}</div>
        </div>
        <button class="btn-text" @click="showDevicePicker = true">切换</button>
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
        <div class="btn-row mt-8">
          <button class="btn-tonal flex" :disabled="sending" @click="onSendClipboard">
            发送剪贴板文本
          </button>
          <button class="btn-tonal flex" :disabled="sending" @click="onSendClipboardImage">
            发送剪贴板图片
          </button>
        </div>
        <input
          ref="fileInput"
          type="file"
          accept="image/*"
          class="hidden-input"
          @change="onFileChosen"
        />
      </div>

      <div class="card card-compact">
        <div class="mirror-row" @click="mirrorExpanded = !mirrorExpanded">
          <div class="mirror-main">
            <span class="card-title inline">投屏到电脑</span>
            <span v-if="mirrorRunning" class="badge badge-on">运行中</span>
          </div>
          <div class="mirror-actions">
            <button class="btn-small" :disabled="mirroring" @click.stop="onMirrorStart">
              {{ mirroring ? '开启中…' : mirrorRunning ? '重新开启' : '开启' }}
            </button>
            <span class="chevron" :class="{ open: mirrorExpanded }">›</span>
          </div>
        </div>
        <div v-if="mirrorExpanded" class="mirror-detail">
          <p class="hint">
            通过同步连接远程开启电脑端的手机投屏，电脑会自动弹出投屏窗口。
          </p>
          <p v-if="mirrorStatusText" class="hint">{{ mirrorStatusText }}</p>
        </div>
      </div>

      <div class="card">
        <div class="card-title-row">
          <h2 class="card-title">收到的内容</h2>
          <button v-if="items.length" class="btn-text danger" @click="onClearItems">清空</button>
        </div>
        <p v-if="items.length === 0" class="hint">
          电脑复制的内容会出现在这里，并自动写入剪贴板
        </p>
        <div v-for="item in items" :key="item.id" class="item">
          <button class="item-del" @click.stop="removeItem(item)" aria-label="删除">×</button>
          <div v-if="item.kind === 'text'" class="item-text" @click="onCopyItem(item)">
            {{ item.text }}
          </div>
          <img
            v-else
            class="item-image"
            :src="item.dataUrl"
            alt="收到的图片"
            @click="openPreview(item)"
          />
          <div class="item-meta">
            {{ item.kind === 'text' ? '点击复制文本' : '点击查看大图' }}
          </div>
        </div>
      </div>
    </div>

    <!-- 电脑切换弹层 -->
    <div v-if="showDevicePicker" class="sheet-mask" @click.self="showDevicePicker = false">
      <div class="sheet">
        <div class="sheet-title">已配对的电脑</div>
        <p v-if="!remembered.length" class="hint">还没有配对过电脑</p>
        <div
          v-for="d in remembered"
          :key="`${d.host}:${d.port}`"
          class="sheet-item"
          :class="{ current: isCurrent(d) }"
          @click="switchDevice(d)"
        >
          <div>
            <div class="pc-name">{{ d.pcName || d.host }}</div>
            <div class="pc-addr">{{ d.host }}:{{ d.port }}</div>
          </div>
          <span v-if="isCurrent(d)" class="pc-check">当前</span>
        </div>
        <div class="sheet-actions">
          <button class="btn-tonal flex" @click="startAddPc">配对新的</button>
          <button v-if="paired" class="btn-tonal flex danger-text" @click="onUnpair">断开</button>
          <button class="btn-tonal flex" @click="showDevicePicker = false">关闭</button>
        </div>
      </div>
    </div>

    <!-- 图片预览 -->
    <div v-if="previewItem" class="preview-mask" @click.self="closePreview">
      <img class="preview-img" :src="previewItem.dataUrl" alt="预览" />
      <div class="preview-bar">
        <button class="btn-preview" @click="copyPreviewImage">复制图片</button>
        <button class="btn-preview ghost" @click="closePreview">关闭</button>
      </div>
    </div>

    <div v-if="toast" class="toast">{{ toast }}</div>
  </div>
</template>

<script setup>
import {computed, onMounted, onUnmounted, reactive, ref} from 'vue'
import {listen} from '@tauri-apps/api/event'
import {invoke} from '@tauri-apps/api/core'
import {Format, requestPermissions, scan} from '@tauri-apps/plugin-barcode-scanner'
import {api} from './api'

const KEY_DEVICES = 'fuyun-devices-v2'
const KEY_CURRENT = 'fuyun-current-v2'
const KEY_HISTORY = 'fuyun-history-v1'
const KEY_LEGACY = 'fuyun-sync'
const MAX_ITEMS = 50
const MAX_IMAGE_ITEMS = 8
const LONG_EDGE = 1600

const paired = ref(false)
const pairing = ref(false)
const sending = ref(false)
const discovering = ref(false)
const error = ref('')
const toast = ref('')
const manualMode = ref(false)
const computers = ref([])
const form = reactive({host: '', code: ''})
const devices = ref([])
const current = ref(null)
const online = ref(false)
const lastSyncAt = ref(0)
const draft = ref('')
const items = ref([])
const previewItem = ref(null)
const showDevicePicker = ref(false)
const mirrorExpanded = ref(false)
const mirroring = ref(false)
const mirrorRunning = ref(false)
const mirrorStatusText = ref('')
const fileInput = ref(null)

let unlistenItem = null
let unlistenStatus = null
let toastTimer = null
let tickTimer = null
const nowTick = ref(Date.now())

const remembered = computed(() => devices.value)
const pcName = computed(() => current.value?.pcName || '电脑')

const statusKind = computed(() => {
  if (!paired.value) return 'off'
  if (online.value) return 'on'
  return 'retry'
})

const statusText = computed(() => {
  if (!paired.value) return '未连接'
  if (online.value) return `已连接 ${pcName.value}`
  return '连接中断，重试中…'
})

const lastSyncAgo = computed(() => {
  if (!lastSyncAt.value) return ''
  const sec = Math.max(0, Math.round((nowTick.value - lastSyncAt.value) / 1000))
  if (sec < 5) return '刚刚同步过'
  if (sec < 60) return `最后同步 ${sec} 秒前`
  if (sec < 3600) return `最后同步 ${Math.floor(sec / 60)} 分钟前`
  return `最后同步 ${new Date(lastSyncAt.value).toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
  })}`
})

function showToast(msg) {
  toast.value = msg
  clearTimeout(toastTimer)
  toastTimer = setTimeout(() => (toast.value = ''), 2200)
}

/**
 * 关闭扫码界面。
 * 插件的扫码画面是直接挂在 WebView 父容器上的原生 View，没有关闭按钮，
 * 只能通过 cancel 命令销毁。
 */
function cancelScan() {
  invoke('plugin:barcode-scanner|cancel').catch(() => {})
}

/**
 * 供原生返回键调用：扫码画面没有关闭按钮，只能通过插件的 cancel 命令销毁。
 * 原生侧用视图层级判断是否在扫码，这里只负责执行关闭。
 */
window.__fuyunCancelScan = cancelScan

/** Tauri 抛的错误可能是字符串、Error 或普通对象，统一取可读文本 */
function errorText(e) {
  if (!e) return ''
  if (typeof e === 'string') return e
  if (typeof e === 'object') return e.message || e.error || JSON.stringify(e)
  return String(e)
}

/** 把底层错误翻译成用户能照着做的提示 */
function humanizeError(e) {
  const s = errorText(e)
  if (/401|403|令牌|token|unauthorized|配对已失效/i.test(s)) {
    return '配对已失效，请在电脑端重新生成配对码后再配一次'
  }
  if (/refused|拒绝|actively refused/i.test(s)) {
    return '电脑端同步服务没开，或端口填错了'
  }
  if (/timed out|timeout|超时|elapsed|deadline/i.test(s)) {
    return '连不上电脑：确认手机和电脑在同一个 Wi-Fi，电脑没有休眠'
  }
  if (/No route|unreachable|不可达/i.test(s)) {
    return '网络不通：手机和电脑可能不在同一个 Wi-Fi'
  }
  if (/404/.test(s)) {
    return '电脑端版本不匹配，没有这个接口'
  }
  if (/\b5\d\d\b/.test(s)) {
    return '电脑端出错了，请查看电脑上的提示'
  }
  if (/dns|resolve|解析/i.test(s)) {
    return '地址格式不对，应为 IP:端口，如 192.168.1.100:45817'
  }
  return s || '操作失败，请重试'
}

// ---------- 本地存储 ----------

function loadJson(key, fallback) {
  try {
    const raw = localStorage.getItem(key)
    return raw ? JSON.parse(raw) : fallback
  } catch {
    return fallback
  }
}

function saveDevices() {
  localStorage.setItem(KEY_DEVICES, JSON.stringify(devices.value))
}

function saveHistory() {
  // 只持久化文本：图片是 base64，写 localStorage 会超配额
  const texts = items.value.filter((i) => i.kind === 'text').slice(0, 30)
  localStorage.setItem(KEY_HISTORY, JSON.stringify(texts))
}

function rememberDevice(cfg) {
  const idx = devices.value.findIndex((d) => d.host === cfg.host && d.port === cfg.port)
  const entry = {...cfg, lastUsed: Date.now()}
  if (idx >= 0) {
    devices.value.splice(idx, 1, {...devices.value[idx], ...entry})
  } else {
    devices.value.unshift(entry)
  }
  devices.value = devices.value.slice(0, 8)
  saveDevices()
}

function forgetDevice(d) {
  devices.value = devices.value.filter((x) => !(x.host === d.host && x.port === d.port))
  saveDevices()
}

function isCurrent(d) {
  return current.value && d.host === current.value.host && d.port === current.value.port
}

// ---------- 生命周期 ----------

onMounted(async () => {
  devices.value = loadJson(KEY_DEVICES, [])
  // 迁移旧版本的单一配对记录
  const legacy = loadJson(KEY_LEGACY, null)
  if (legacy && legacy.token && !devices.value.length) {
    const cfg = {host: legacy.host, port: legacy.port, token: legacy.token, pcName: legacy.pcName}
    devices.value = [cfg]
    saveDevices()
    localStorage.removeItem(KEY_LEGACY)
  }

  items.value = loadJson(KEY_HISTORY, [])

  const cur = loadJson(KEY_CURRENT, null) || devices.value[0]
  if (cur && cur.token) {
    current.value = cur
    paired.value = true
    online.value = false
    await startLoop()
    refreshMirrorStatus()
  } else {
    onDiscover()
  }

  unlistenItem = await listen('sync-item', (event) => {
    const payload = event.payload
    lastSyncAt.value = Date.now()
    online.value = true
    items.value.unshift({...payload, ts: Date.now()})
    trimItems()
    saveHistory()
  })

  // 轮询成功/失败由 Rust 推送，避免"绿点常亮"的假在线
  unlistenStatus = await listen('sync-status', (event) => {
    const p = event.payload
    online.value = !!p.online
    if (p.online) lastSyncAt.value = Date.now()
  })

  tickTimer = setInterval(() => (nowTick.value = Date.now()), 1000)
})

onUnmounted(() => {
  if (unlistenItem) unlistenItem()
  if (unlistenStatus) unlistenStatus()
  clearInterval(tickTimer)
})

function trimItems() {
  // 图片条数单独限流：dataUrl 很占内存
  let images = 0
  items.value = items.value.filter((it) => {
    if (it.kind !== 'image') return true
    images += 1
    return images <= MAX_IMAGE_ITEMS
  })
  if (items.value.length > MAX_ITEMS) items.value.length = MAX_ITEMS
}

// ---------- 配对 ----------

async function onDiscover() {
  if (discovering.value) return
  discovering.value = true
  try {
    const r = await api.discover(2000)
    computers.value = r.computers || []
    if (computers.value.length === 1) selectPc(computers.value[0])
  } catch (e) {
    showToast(humanizeError(e))
  } finally {
    discovering.value = false
  }
}

function selectPc(c) {
  form.host = `${c.host}:${c.port}`
}

/** 从任意文本里抽 IP:端口 与 6 位配对码（兼容 fuyun://pair?host=..&code=.. 与纯文本） */
function applyPairInfo(text) {
  const addr = text.match(/(\d{1,3}(?:\.\d{1,3}){3})\s*:\s*(\d{2,5})/)
  const code = text.match(/(?:^|\D)(\d{6})(?:\D|$)/)
  if (!addr && !code) return false
  if (addr) {
    form.host = `${addr[1]}:${addr[2]}`
    manualMode.value = true
  }
  if (code) form.code = code[1]
  showToast(addr && code ? '已填入地址与配对码，点「配对」即可' : addr ? '已填入电脑地址' : '已填入配对码')
  return true
}

/** 扫码配对：电脑端「手机同步」页展示二维码 */
async function onScanQr() {
  let timer = null
  try {
    // 插件不会自动申请相机权限，必须显式请求，否则 scan() 直接抛错
    const perm = await requestPermissions()
    if (perm !== 'granted') {
      showToast('需要相机权限才能扫码，请在系统设置里允许')
      return
    }
    // 45 秒没扫到就自动关掉，避免一直停在相机画面（界面本身没有取消按钮）
    timer = setTimeout(() => {
      cancelScan()
      showToast('扫码超时，已退出')
    }, 45000)
    // windowed 模式在部分机型上 overlay 不显示（相机在跑但界面不可见），用原生全屏扫描界面
    const res = await scan({windowed: false, formats: [Format.QRCode]})
    // 用户取消时可能返回 null 而不是抛错
    if (!res || !res.content) return
    if (!applyPairInfo(res.content)) {
      showToast('这个二维码不是浮云的配对码')
    }
  } catch (e) {
    const msg = errorText(e)
    if (/cancel|取消/i.test(msg)) return
    if (/permission|camera|相机/i.test(msg)) {
      showToast('需要相机权限才能扫码，请在系统设置里允许')
      return
    }
    showToast(msg.slice(0, 80) || '扫码失败，请重试')
  } finally {
    if (timer) clearTimeout(timer)
  }
}

async function onPastePairInfo() {
  try {
    const text = await api.readClipboard()
    if (!text) {
      showToast('剪贴板是空的')
      return
    }
    if (!applyPairInfo(text)) {
      showToast('剪贴板里没找到 IP:端口 或 6 位配对码')
    }
  } catch (e) {
    showToast(humanizeError(e))
  }
}

async function onPair() {
  error.value = ''
  const raw = form.host.trim()
  const [host, portStr] = raw.split(':')
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
    const cfg = {
      host,
      port,
      token: result.token,
      pcName: result.pcName || '电脑',
    }
    current.value = cfg
    rememberDevice(cfg)
    localStorage.setItem(KEY_CURRENT, JSON.stringify(cfg))
    paired.value = true
    form.code = ''
    await startLoop()
  } catch (e) {
    error.value = humanizeError(e)
  } finally {
    pairing.value = false
  }
}

async function startLoop() {
  if (!current.value?.token) return
  online.value = false
  try {
    await api.startSyncLoop(current.value.host, current.value.port, current.value.token)
  } catch (e) {
    showToast(humanizeError(e))
  }
}

function startAddPc() {
  showDevicePicker.value = false
  paired.value = false
  error.value = ''
  form.host = ''
  form.code = ''
  computers.value = []
  onDiscover()
}

async function switchDevice(d) {
  showDevicePicker.value = false
  if (isCurrent(d)) return
  await api.stopSyncLoop()
  current.value = d
  localStorage.setItem(KEY_CURRENT, JSON.stringify(d))
  paired.value = true
  form.host = `${d.host}:${d.port}`
  await startLoop()
  refreshMirrorStatus()
  showToast(`已切换到 ${d.pcName || d.host}`)
}

async function onUnpair() {
  showDevicePicker.value = false
  await api.stopSyncLoop()
  if (current.value) forgetDevice(current.value)
  localStorage.removeItem(KEY_CURRENT)
  paired.value = false
  current.value = null
  online.value = false
  items.value = []
  computers.value = []
  onDiscover()
}

// ---------- 发送 ----------

function requireCurrent() {
  if (!current.value?.token) throw new Error('还没配对电脑')
  return current.value
}

async function onSendText() {
  const text = draft.value.trim()
  if (!text) {
    showToast('请输入内容')
    return
  }
  sending.value = true
  try {
    const c = requireCurrent()
    await api.pushText(c.host, c.port, c.token, text)
    draft.value = ''
    showToast('已发送到电脑剪贴板')
  } catch (e) {
    showToast(humanizeError(e))
  } finally {
    sending.value = false
  }
}

async function onSendClipboard() {
  sending.value = true
  try {
    const text = await api.readClipboard()
    if (!text) {
      showToast('剪贴板里没有文本')
      return
    }
    const c = requireCurrent()
    await api.pushText(c.host, c.port, c.token, text)
    showToast('已发送到电脑剪贴板')
  } catch (e) {
    showToast(humanizeError(e))
  } finally {
    sending.value = false
  }
}

async function onSendClipboardImage() {
  sending.value = true
  try {
    const img = await api.readClipboardImage()
    if (!img?.imageBase64) {
      showToast('剪贴板里没有图片')
      return
    }
    const compressed = await compressBase64Image(img.imageBase64, img.width, img.height)
    const c = requireCurrent()
    await api.pushImage(c.host, c.port, c.token, compressed)
    showToast('已发送剪贴板图片')
  } catch (e) {
    showToast(humanizeError(e))
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
    const base64 = await fileToCompressedBase64(file)
    const c = requireCurrent()
    await api.pushImage(c.host, c.port, c.token, base64)
    showToast('图片已发送到电脑')
  } catch (e) {
    showToast(humanizeError(e))
  } finally {
    sending.value = false
  }
}

function readAsDataUrl(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result))
    reader.onerror = reject
    reader.readAsDataURL(file)
  })
}

function loadImage(src) {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.onload = () => resolve(img)
    img.onerror = reject
    img.src = src
  })
}

/** 超过 1.5MB 才压缩，小图保持原样避免掉画质 */
async function fileToCompressedBase64(file) {
  const dataUrl = await readAsDataUrl(file)
  if (file.size <= 1.5 * 1024 * 1024) {
    return dataUrl.slice(dataUrl.indexOf(',') + 1)
  }
  const img = await loadImage(dataUrl)
  return drawToJpegBase64(img, img.naturalWidth, img.naturalHeight)
}

async function compressBase64Image(base64, width, height) {
  const img = await loadImage(`data:image/png;base64,${base64}`)
  return drawToJpegBase64(img, width || img.naturalWidth, height || img.naturalHeight)
}

/** 长边限到 1600px，白底 + JPEG 0.85（避免透明区域变黑） */
function drawToJpegBase64(img, w, h) {
  const scale = Math.min(1, LONG_EDGE / Math.max(w, h))
  const cw = Math.max(1, Math.round(w * scale))
  const ch = Math.max(1, Math.round(h * scale))
  const canvas = document.createElement('canvas')
  canvas.width = cw
  canvas.height = ch
  const ctx = canvas.getContext('2d')
  ctx.fillStyle = '#ffffff'
  ctx.fillRect(0, 0, cw, ch)
  ctx.drawImage(img, 0, 0, cw, ch)
  const out = canvas.toDataURL('image/jpeg', 0.85)
  return out.slice(out.indexOf(',') + 1)
}

// ---------- 投屏 ----------

async function onMirrorStart() {
  mirroring.value = true
  try {
    const c = requireCurrent()
    const r = await api.mirrorStart(c.host, c.port, c.token)
    if (r && r.ok) {
      mirrorRunning.value = true
      mirrorStatusText.value = `投屏运行中${r.deviceName ? `（${r.deviceName}）` : ''}`
      showToast('投屏已开启，请看电脑屏幕')
    } else {
      mirrorRunning.value = false
      mirrorStatusText.value = ''
      showToast(humanizeError((r && r.error) || '开启失败'))
    }
  } catch (e) {
    showToast(humanizeError(e))
  } finally {
    mirroring.value = false
  }
}

async function refreshMirrorStatus() {
  try {
    const c = requireCurrent()
    const r = await api.mirrorStatus(c.host, c.port, c.token)
    mirrorRunning.value = !!(r && r.running)
    mirrorStatusText.value =
      r && r.running ? `投屏运行中${r.deviceName ? `（${r.deviceName}）` : ''}` : ''
  } catch {
    /* 状态查询失败不打扰用户 */
  }
}

// ---------- 收到的内容 ----------

async function onCopyItem(item) {
  try {
    if (item.kind === 'text') {
      await api.writeClipboard(item.text)
    } else {
      const c = requireCurrent()
      await api.writeImageToClipboard(c.host, c.port, c.token, item.id)
    }
    showToast('已复制到剪贴板')
  } catch (e) {
    showToast(humanizeError(e))
  }
}

function openPreview(item) {
  previewItem.value = item
}

function closePreview() {
  previewItem.value = null
}

async function copyPreviewImage() {
  if (!previewItem.value) return
  await onCopyItem(previewItem.value)
  closePreview()
}

function removeItem(item) {
  items.value = items.value.filter((i) => i.id !== item.id)
  saveHistory()
}

function onClearItems() {
  items.value = []
  localStorage.removeItem(KEY_HISTORY)
  showToast('已清空')
}
</script>
