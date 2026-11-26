# 浮云剪贴板 · 移动端（Tauri）

与电脑端「浮云剪贴板」互相同步剪贴板内容的 Android 客户端。
**Rust 核心（Tauri v2）+ Vue 前端**，与 PC 端同步协议一致。

## 功能

| 方向 | 方式 |
|------|------|
| 电脑 → 手机 | App 内自动轮询接收（3 秒），文本/图片自动写入剪贴板并出现在列表中 |
| 手机 → 电脑 | App 内输入文本 / 选图 / 发当前剪贴板，点击即写入电脑剪贴板 |

> Android 10+ 系统限制后台读剪贴板，故采用「App 内主动发送」模型，无需任何敏感权限。

## 开发环境要求

- Node.js 18+
- Rust（stable）
- Android Studio + Android SDK（构建 APK 必需；NDK 与 cmdline-tools 由 `tauri android init` 引导安装）
- JDK 17

## 开发

```bash
npm install
npm run tauri dev        # 桌面窗口调试（同一套前端+Rust，方便快速迭代）
```

## 构建 APK

```bash
# 首次：生成 gen/android 脚手架（需 ANDROID_HOME 指向 SDK）
npm run tauri android init

# 构建
npm run tauri android build --debug     # 调试包
npm run tauri android build             # 发布包
```

产物位于 `src-tauri/gen/android/app/build/outputs/apk/`。

## 通过 GitHub Actions 发布 APK

`.github/workflows/android-release.yml` 负责云端打包，无需本地装 Android Studio。

**触发方式**

| 方式 | 结果 |
|------|------|
| 推送 `v*` 标签（如 `git tag v1.0.1 && git push origin v1.0.1`） | 构建签名 APK → 自动创建 GitHub Release 并上传 |
| Actions 页面手动 Run workflow（可填版本号） | 只构建并上传 Artifact（Actions 运行页可下载，保留 30 天） |

**产物命名**：`fuyun-clipboard-<版本>-universal.apk`（全 ABI 合集）、`-arm64-v8a.apk` 等。

**所需 Secrets**（Settings → Secrets and variables → Actions）

| Secret | 说明 |
|--------|------|
| `ANDROID_KEYSTORE_BASE64` | `.keystore` 文件的 base64：`base64 -w0 fuyun-clipboard.keystore` |
| `ANDROID_KEYSTORE_PASSWORD` | keystore 口令 |
| `ANDROID_KEY_ALIAS` | 密钥别名（当前为 `fuyun`） |
| `ANDROID_KEY_PASSWORD` | 密钥口令 |

工作流会用 Secrets 还原 keystore 并生成 `src-tauri/gen/android/keystore.properties`（该文件不进版本库）。
缺少 `ANDROID_KEYSTORE_BASE64` 时会直接失败——未签名的 release APK 无法安装，不做兜底。

**版本号**：tag 触发的 `v1.0.1` 会写入 `src-tauri/tauri.conf.json` 的 `version`，
`versionName` 与 `versionCode` 由 Tauri CLI 据此生成（`1.0.1` → `1000001`）。

**CI 环境**：Ubuntu + JDK 21 + Android SDK platform 37 + NDK r28（自动探测回退 r27）+ Rust 四个 Android target；
Gradle 走 `gen/android/gradlew`（9.6.1），Rust 构建有缓存。


## 目录结构

```
fuyun_tools_m/
├── index.html             前端入口
├── src/
│   ├── App.vue            配对页 + 主页（发送/接收）
│   ├── api.js             与 Rust 命令层的一一对应封装
│   └── style.css          移动端样式
└── src-tauri/
    ├── src/lib.rs         命令层 + 前台轮询循环 + 剪贴板读写
    ├── src/sync_proto.rs  同步协议类型（与桌面端保持一致，改协议需双边同步）
    └── src/sync_client.rs reqwest 同步客户端
```

## 与电脑端配对

1. 电脑端：设置 → 手机同步 → 启动服务，记下配对码与「手动连接地址」（如 `http://192.168.1.100:45817`）
2. 手机：打开 App → 输入电脑地址 → 输入 6 位配对码 → 配对
   （v1 采用手动输入地址；自动发现后续版本再做）
3. 配对信息保存在手机本地，重启免重配

## 协议

HTTP + JSON，Bearer 令牌认证。端点定义与桌面端一致，详见
`fuyun_tools/docs/mobile-sync.md`。
