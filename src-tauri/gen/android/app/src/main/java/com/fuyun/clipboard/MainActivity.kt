package com.fuyun.clipboard

import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    ensureLocalNetworkPermission()
    applyTopInsetsPadding()
    handleBackKey()
  }

  /**
   * 返回键：扫码中先关扫码，否则回桌面而不是 finish。
   *
   * barcode-scanner 的扫码画面是 addView 到 WebView 父容器上的原生 View
   * （PreviewView + GraphicOverlay），自身既没有关闭按钮也不处理返回键。
   * 不能用 JS 侧的状态标志判断——页面重载后标志会归零，但原生 View 还在，
   * 于是按返回就会误判成"没在扫码"，直接把 App 退到后台。
   */
  private fun handleBackKey() {
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        if (closeScanIfActive()) return
        moveTaskToBack(true)
      }
    })
  }

  /** 父容器平时只有 WebView 一个子 View，扫码时插件会再加两个，据此判断 */
  private fun closeScanIfActive(): Boolean {
    val webView = findWebView(window.decorView) ?: return false
    val parent = webView.parent as? ViewGroup ?: return false
    if (parent.childCount <= 1) return false

    webView.evaluateJavascript(
      "window.__fuyunCancelScan && window.__fuyunCancelScan()",
      null,
    )
    // 兜底：JS 侧异常时直接摘掉扫码视图，避免卡在相机画面
    webView.postDelayed({
      val p = webView.parent as? ViewGroup
      if (p != null && p.childCount > 1) {
        while (p.childCount > 1) p.removeViewAt(p.childCount - 1)
      }
    }, 600)
    return true
  }

  private fun findWebView(v: View): WebView? {
    if (v is WebView) return v
    if (v is ViewGroup) {
      for (i in 0 until v.childCount) {
        findWebView(v.getChildAt(i))?.let { return it }
      }
    }
    return null
  }

  /**
   * WebView 全屏铺满（enableEdgeToEdge + viewport-fit=cover），顶栏会画进状态栏
   * 被时间/电量图标遮挡。targetSdk 35+ 上 edge-to-edge 被系统强制、
   * setDecorFitsSystemWindows 已失效，只能给 content 根视图加状态栏 + 刘海高度的 padding。
   */
  private fun applyTopInsetsPadding() {
    val root = findViewById<ViewGroup>(android.R.id.content) ?: return
    ViewCompat.setOnApplyWindowInsetsListener(root) { v, insets ->
      val bars = insets.getInsets(
        WindowInsetsCompat.Type.statusBars() or WindowInsetsCompat.Type.displayCutout()
      )
      v.setPadding(v.paddingLeft, bars.top, v.paddingRight, v.paddingBottom)
      insets
    }
  }

  /**
   * Android 17（API 37）起，targetSdk >= 37 的应用默认被禁止访问本地网络：
   * 内网地址的 TCP 连接会被网络栈丢弃，最终表现为连接超时，而公网请求正常。
   *
   * 同步功能需要访问局域网内的电脑，因此必须持有 ACCESS_LOCAL_NETWORK 运行时权限。
   * （保护级别 dangerous，属于 NEARBY_DEVICES 权限组）
   */
  private fun ensureLocalNetworkPermission() {
    if (Build.VERSION.SDK_INT < LOCAL_NETWORK_MIN_SDK) return
    val granted = ContextCompat.checkSelfPermission(this, LOCAL_NETWORK_PERMISSION) ==
      PackageManager.PERMISSION_GRANTED
    if (granted) return
    requestPermissions(arrayOf(LOCAL_NETWORK_PERMISSION), LOCAL_NETWORK_REQUEST_CODE)
  }

  companion object {
    private const val LOCAL_NETWORK_MIN_SDK = 37
    private const val LOCAL_NETWORK_REQUEST_CODE = 4211
    private const val LOCAL_NETWORK_PERMISSION = "android.permission.ACCESS_LOCAL_NETWORK"
  }
}
