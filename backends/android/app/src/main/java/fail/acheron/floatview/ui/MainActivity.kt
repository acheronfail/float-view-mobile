package fail.acheron.floatview.ui

import android.annotation.SuppressLint
import android.app.Activity
import android.content.ComponentCallbacks2
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.webkit.JavascriptInterface
import android.webkit.ConsoleMessage
import android.webkit.ValueCallback
import android.webkit.WebView
import android.webkit.WebChromeClient
import android.widget.FrameLayout
import fail.acheron.floatview.application.BluetoothEvent
import fail.acheron.floatview.permissions.PermissionBroker
import fail.acheron.floatview.files.SessionFileImportController
import fail.acheron.floatview.lifecycle.FloatViewApplication
import fail.acheron.floatview.theme.AndroidThemeController

class MainActivity : Activity() {
    private var webView: WebView? = null
    private var themeSubscription: AutoCloseable? = null
    private val frontendContainer by lazy { FrameLayout(this) }
    private val themeController by lazy { AndroidThemeController(this, frontendContainer) }
    private val sessionFiles by lazy {
        SessionFileImportController(
            this,
            application as FloatViewApplication,
            webView = { webView },
        )
    }
    private val handler = Handler(Looper.getMainLooper())
    private val notificationPermission by lazy { fail.acheron.floatview.notifications.RideNotificationPermission(this) }
    private val notificationPermissionCheck = object : Runnable {
        override fun run() {
            val connected = (application as FloatViewApplication).runtime.service.liveActivity != null
            if (!notificationPermission.requestIfNeeded(connected)) handler.postDelayed(this, 1000)
        }
    }
    private val delayedTeardown = Runnable { destroyFrontend() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val app = application as FloatViewApplication
        app.runtime.permissionBroker.attach(this)
        app.runtime.exportPresentationBroker.attach(this)
        themeSubscription = app.runtime.service.observeTheme { preference ->
            runOnUiThread { themeController.apply(preference) }
        }
        setContentView(frontendContainer)
        createFrontend()
    }

    override fun onStart() {
        super.onStart()
        (application as FloatViewApplication).runtime.service.setForeground(true)
        handler.removeCallbacks(delayedTeardown)
        createFrontend()
    }

    override fun onResume() {
        super.onResume()
        handler.removeCallbacks(notificationPermissionCheck)
        handler.post(notificationPermissionCheck)
        val preference =
            (application as FloatViewApplication).runtime.service.theme
        themeController.apply(preference)
    }

    override fun onPause() {
        handler.removeCallbacks(notificationPermissionCheck)
        super.onPause()
    }

    override fun onStop() {
        (application as FloatViewApplication).runtime.service.setForeground(false)
        handler.removeCallbacks(delayedTeardown)
        handler.postDelayed(delayedTeardown, FRONTEND_BACKGROUND_GRACE_MS)
        super.onStop()
    }

    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)
        if (
            level >= ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN ||
            level == ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL
        ) {
            handler.removeCallbacks(delayedTeardown)
            destroyFrontend()
        }
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun createFrontend() {
        if (webView != null || isFinishing || isDestroyed) return
        val origin = (application as FloatViewApplication).backend.origin
        webView = WebView(this).also {
            // The native container owns the canvas while the document is loading.
            it.setBackgroundColor(android.graphics.Color.TRANSPARENT)
            it.settings.javaScriptEnabled = true
            it.settings.domStorageEnabled = true
            it.settings.setSupportZoom(false)
            it.settings.builtInZoomControls = false
            it.settings.displayZoomControls = false
            it.webChromeClient = object : WebChromeClient() {
                override fun onShowFileChooser(
                    view: WebView?,
                    filePathCallback: ValueCallback<Array<Uri>>?,
                    fileChooserParams: FileChooserParams?,
                ): Boolean {
                    return sessionFiles.showFileChooser(filePathCallback, fileChooserParams)
                }

                override fun onConsoleMessage(message: ConsoleMessage): Boolean {
                    if (applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE != 0) {
                        Log.d("FloatViewWebView", "${message.messageLevel()}: ${message.message()}")
                    }
                    return true
                }
            }
            if (applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE != 0) {
                it.addJavascriptInterface(FrontendPerformanceBridge(), "FloatViewPerformance")
            }
            it.loadUrl(origin)
            frontendContainer.addView(
                it,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT,
                ),
            )
        }
    }

    private fun destroyFrontend() {
        val current = webView ?: return
        webView = null
        frontendContainer.removeView(current)
        current.stopLoading()
        current.loadUrl("about:blank")
        current.removeAllViews()
        current.destroy()
    }

    internal fun frontendIsAttached(): Boolean = webView != null

    override fun onDestroy() {
        handler.removeCallbacks(delayedTeardown)
        handler.removeCallbacks(notificationPermissionCheck)
        val runtime = (application as FloatViewApplication).runtime
        runtime.permissionBroker.detach(this)
        runtime.exportPresentationBroker.detach(this)
        sessionFiles.close()
        themeSubscription?.close()
        themeSubscription = null
        destroyFrontend()
        super.onDestroy()
    }

    @Deprecated("Android WebView file chooser uses the legacy activity result callback")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: android.content.Intent?) {
        (application as FloatViewApplication).runtime.exportPresentationBroker.onActivityResult(requestCode, resultCode, data)
        sessionFiles.handleActivityResult(requestCode, resultCode, data)
        super.onActivityResult(requestCode, resultCode, data)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == PermissionBroker.PERMISSION_REQUEST) {
            val runtime = (application as FloatViewApplication).runtime
            runtime.permissionBroker.recordResult(permissions)
            runtime.service.receive(
                BluetoothEvent.StateChanged(
                    runtime.bluetooth.currentState(),
                ),
            )
            runtime.service.refreshCapabilities()
        }
    }

    companion object {
        internal const val FRONTEND_BACKGROUND_GRACE_MS = 30_000L
    }

    private class FrontendPerformanceBridge {
        @JavascriptInterface
        fun report(message: String) {
            Log.d("FloatViewFrontend", message)
        }
    }
}
