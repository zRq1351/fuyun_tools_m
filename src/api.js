// 与 Rust 命令层的一一对应封装
import {invoke} from '@tauri-apps/api/core'

export const api = {
    /** 探测电脑连接并获取状态 */
    probeStatus: (host, port) =>
        invoke('sync_probe_status', {host, port}),

    /** 自动发现局域网内的电脑（mDNS 优先，子网扫描兜底） */
    discover: (timeoutMs) => invoke('sync_discover', {timeoutMs}),

    /** 配对：配对码换令牌 */
    pair: (host, port, code) =>
        invoke('sync_pair', {host, port, code}),

    /** 发送文本到电脑剪贴板 */
    pushText: (host, port, token, text) =>
        invoke('sync_push_text', {host, port, token, text}),

    /** 发送图片（base64，不含 data: 前缀）到电脑剪贴板 */
    pushImage: (host, port, token, imageBase64) =>
        invoke('sync_push_image', {host, port, token, imageBase64}),

    /** 启动同步循环（前台轮询，收到推送自动写剪贴板） */
    startSyncLoop: (host, port, token) =>
        invoke('sync_start_loop', {host, port, token}),

    /** 停止同步循环 */
    stopSyncLoop: () => invoke('sync_stop_loop'),

    /** 读取本机剪贴板文本（用户点「发送当前剪贴板」时用） */
    readClipboard: () => invoke('clipboard_read_text'),

    /** 把文本写入本机剪贴板 */
    writeClipboard: (text) => invoke('clipboard_write_text', {text}),

    /** 按历史条目 id 重新下载图片并写入本机剪贴板 */
    writeImageToClipboard: (host, port, token, id) =>
        invoke('sync_write_image_clipboard', {host, port, token, id}),

    /** 远程开启电脑端投屏（电脑需已开启同步服务） */
    mirrorStart: (host, port, token) =>
        invoke('sync_mirror_start', {host, port, token}),

    /** 查询电脑端投屏会话状态 */
    mirrorStatus: (host, port, token) =>
        invoke('sync_mirror_status', {host, port, token}),
}
