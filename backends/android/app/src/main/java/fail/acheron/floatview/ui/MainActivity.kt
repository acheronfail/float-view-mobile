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
import android.webkit.WebResourceResponse
import android.webkit.RenderProcessGoneDetail
import fail.acheron.floatview.application.StartupFailure
import fail.acheron.floatview.application.ThemePreference
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
            val app = application as FloatViewApplication
            if (app.startupFailure != null) return
            val runtime = app.runtimeOrNull ?: return
            if (!runtime.permissionBroker.requestRideNotifications(runtime.service.liveActivity != null)) handler.postDelayed(this, 1000)
        }
    }
    private val delayedTeardown = Runnable { destroyFrontend() }
    private var backPending = false
    private val backCallback = if (Build.VERSION.SDK_INT >= 33) OnBackInvokedCallback { navigateBack() } else null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        frontendContainer.addView(
            loadingPresentation,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT,
            ),
        )
        setContentView(frontendContainer)
        themeController.apply(ThemePreference.SYSTEM)
        attachRuntime()
        createFrontend()
        if (Build.VERSION.SDK_INT >= 33) {
            onBackInvokedDispatcher.registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_DEFAULT,
                requireNotNull(backCallback),
            )
        }
    }

    private fun attachRuntime() {
        if (themeSubscription != null) return
        val runtime = (application as FloatViewApplication).runtimeOrNull ?: return
        runtime.permissionBroker.attach(this)
        runtime.exportPresentationBroker.attach(this)
        themeSubscription = runtime.service.observeTheme { preference ->
            runOnUiThread { themeController.apply(preference) }
        }
    }

    private fun showFailure(failure: StartupFailure) {
        webView?.visibility = View.INVISIBLE
        loadingPresentation.visibility = View.VISIBLE
        loadingPresentation.showFailure(failure) {
            destroyFrontend()
            val app = application as FloatViewApplication
            app.retryStartup()
            attachRuntime()
            app.runtimeOrNull?.service?.setForeground(true)
            createFrontend()
            handler.removeCallbacks(notificationPermissionCheck)
            if (app.startupFailure == null) handler.post(notificationPermissionCheck)
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
        (application as FloatViewApplication).runtimeOrNull?.service?.setForeground(true)
        handler.removeCallbacks(delayedTeardown)
        createFrontend()
    }

    override fun onResume() {
        super.onResume()
        handler.removeCallbacks(notificationPermissionCheck)
        handler.post(notificationPermissionCheck)
        val preference =
            (application as FloatViewApplication).runtimeOrNull?.service?.theme ?: ThemePreference.SYSTEM
        themeController.apply(preference)
    }

    override fun onPause() {
        handler.removeCallbacks(notificationPermissionCheck)
        super.onPause()
    }

    override fun onStop() {
        // Configuration replacement keeps the same foreground visit and service owner.
        if (!isChangingConfigurations) {
            (application as FloatViewApplication).runtimeOrNull?.service?.setForeground(false)
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
        val app = application as FloatViewApplication
        app.startupFailure?.let { showFailure(it); return }
        val origin = app.backend.origin
        loadingPresentation.showLoading()
        loadingPresentation.visibility = View.VISIBLE
        try {
            val created = WebView(this)
            webView = created
            created.also {
                it.visibility = View.INVISIBLE
                // The native container owns the canvas while the document is loading.
                it.setBackgroundColor(android.graphics.Color.TRANSPARENT)
                it.settings.javaScriptEnabled = true
                it.settings.domStorageEnabled = true
                it.settings.setSupportZoom(false)
                it.settings.builtInZoomControls = false
                it.settings.displayZoomControls = false
                it.webViewClient = object : WebViewClient() {
                    private var failed = false
                    private fun fail(view: WebView) {
                        if (view !== webView) return
                        failed = true
                        showFailure(StartupFailure.INTERFACE)
                    }
                    override fun onPageCommitVisible(view: WebView, url: String) {
                        if (view !== webView || failed) return
                        view.visibility = View.VISIBLE
                        loadingPresentation.visibility = View.GONE
                    }

                    override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
                        if (request.isForMainFrame) fail(view)
                    }

                    override fun onReceivedHttpError(view: WebView, request: WebResourceRequest, response: WebResourceResponse) {
                        if (request.isForMainFrame) fail(view)
                    }

                    override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
                        if (view === webView) {
                            destroyFrontend(rendererGone = true)
                            showFailure(StartupFailure.INTERFACE)
                        }
                        return true
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
        } catch (_: Exception) {
            destroyFrontend()
            showFailure(StartupFailure.INTERFACE)
        }
    }

    private fun destroyFrontend(rendererGone: Boolean = false) {
        val current = webView ?: return
        webView = null
        backPending = false
        frontendContainer.removeView(current)
        if (!rendererGone) {
            current.stopLoading()
            current.loadUrl("about:blank")
        }
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
        val runtime = (application as FloatViewApplication).runtimeOrNull
        runtime?.permissionBroker?.detach(this)
        runtime?.exportPresentationBroker?.detach(this)
        sessionFiles.close()
        themeSubscription?.close()
        themeSubscription = null
        destroyFrontend()
        super.onDestroy()
    }

    @Deprecated("Android WebView file chooser uses the legacy activity result callback")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: android.content.Intent?) {
        (application as FloatViewApplication).runtimeOrNull?.exportPresentationBroker?.onActivityResult(requestCode, resultCode, data)
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
            val runtime = (application as FloatViewApplication).runtimeOrNull ?: return
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
