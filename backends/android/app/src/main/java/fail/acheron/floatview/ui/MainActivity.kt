package fail.acheron.floatview.ui

import android.annotation.SuppressLint
import android.app.Activity
import android.content.ComponentCallbacks2
import android.net.Uri
import android.os.Bundle
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.webkit.JavascriptInterface
import android.webkit.ConsoleMessage
import android.webkit.ValueCallback
import android.webkit.WebView
import android.webkit.WebChromeClient
import android.webkit.WebViewClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceError
import android.view.View
import android.widget.FrameLayout
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import fail.acheron.floatview.application.BluetoothEvent
import fail.acheron.floatview.permissions.PermissionBroker
import fail.acheron.floatview.files.SessionFileImportController
import fail.acheron.floatview.lifecycle.FloatViewApplication
import fail.acheron.floatview.theme.AndroidThemeController

class MainActivity : Activity() {
    private var webView: WebView? = null
    private var themeSubscription: AutoCloseable? = null
    private val frontendContainer by lazy { FrameLayout(this) }
    private val loadingPresentation by lazy { LoadingPresentation(this) }
    private val themeController by lazy { AndroidThemeController(this, frontendContainer) }
    private val sessionFiles by lazy {
        SessionFileImportController(
            this,
            application as FloatViewApplication,
            webView = { webView },
        )
    }
    private val handler = Handler(Looper.getMainLooper())
    private val notificationPermissionCheck = object : Runnable {
        override fun run() {
            val runtime = (application as FloatViewApplication).runtime
            if (!runtime.permissionBroker.requestRideNotifications(runtime.service.liveActivity != null)) handler.postDelayed(this, 1000)
        }
    }
    private val delayedTeardown = Runnable { destroyFrontend() }
    private var backPending = false
    private val backCallback = if (Build.VERSION.SDK_INT >= 33) OnBackInvokedCallback { navigateBack() } else null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val app = application as FloatViewApplication
        app.runtime.permissionBroker.attach(this)
        app.runtime.exportPresentationBroker.attach(this)
        frontendContainer.addView(
            loadingPresentation,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT,
            ),
        )
        themeSubscription = app.runtime.service.observeTheme { preference ->
            runOnUiThread { themeController.apply(preference) }
        }
        setContentView(frontendContainer)
        createFrontend()
        if (Build.VERSION.SDK_INT >= 33) {
            onBackInvokedDispatcher.registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_DEFAULT,
                requireNotNull(backCallback),
            )
        }
    }

    @Deprecated("Used by Android versions before API 33")
    override fun onBackPressed() = navigateBack()

    private fun navigateBack() {
        if (backPending) return
        val current = webView
        if (current == null) {
            moveTaskToBack(true)
            return
        }
        backPending = true
        current.evaluateJavascript(
            "window.dispatchEvent(new Event('floatview:back', { cancelable: true }))",
        ) { result ->
            backPending = false
            if (webView === current && result != "false") moveTaskToBack(true)
        }
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
        // Configuration replacement keeps the same foreground visit and service owner.
        if (!isChangingConfigurations) {
            (application as FloatViewApplication).runtime.service.setForeground(false)
        }
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
        loadingPresentation.showLoading()
        loadingPresentation.visibility = View.VISIBLE
        webView = WebView(this).also {
            it.visibility = View.INVISIBLE
            // The native container owns the canvas while the document is loading.
            it.setBackgroundColor(android.graphics.Color.TRANSPARENT)
            it.settings.javaScriptEnabled = true
            it.settings.domStorageEnabled = true
            it.settings.setSupportZoom(false)
            it.settings.builtInZoomControls = false
            it.settings.displayZoomControls = false
            it.webViewClient = object : WebViewClient() {
                override fun onPageCommitVisible(view: WebView, url: String) {
                    if (view !== webView) return
                    view.visibility = View.VISIBLE
                    loadingPresentation.visibility = View.GONE
                }

                override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
                    if (view === webView && request.isForMainFrame) loadingPresentation.showFailure()
                }
            }
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
        backPending = false
        frontendContainer.removeView(current)
        current.stopLoading()
        current.loadUrl("about:blank")
        current.removeAllViews()
        current.destroy()
        loadingPresentation.showLoading()
        loadingPresentation.visibility = View.VISIBLE
    }

    internal fun frontendIsAttached(): Boolean = webView != null

    override fun onDestroy() {
        if (Build.VERSION.SDK_INT >= 33) {
            onBackInvokedDispatcher.unregisterOnBackInvokedCallback(requireNotNull(backCallback))
        }
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
