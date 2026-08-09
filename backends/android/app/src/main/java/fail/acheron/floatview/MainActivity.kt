package fail.acheron.floatview

import android.annotation.SuppressLint
import android.app.Activity
import android.content.ComponentCallbacks2
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.webkit.ConsoleMessage
import android.webkit.JavascriptInterface
import android.webkit.ValueCallback
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import fail.acheron.floatview.host.FloatViewAppearance

class MainActivity : Activity() {
    private var webView: WebView? = null
    private var fileChooserCallback: ValueCallback<Array<Uri>>? = null
    private var rustOrigin: String? = null
    private var activityStarted = false
    private var frontendLoadAttempts = 0
    private val frontendContainer by lazy { FrameLayout(this) }
    private val handler = Handler(Looper.getMainLooper())
    private val delayedTeardown = Runnable { destroyFrontend() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(frontendContainer)
        FloatViewAppearance.applyNativeChrome(this, frontendContainer)
        (application as FloatViewApplication).rustRuntime.attach(
            this,
            onReady = { endpoint, token ->
                val launchUrl = Uri.parse(endpoint).buildUpon()
                    .appendQueryParameter("token", token)
                    .build()
                    .toString()
                if (rustOrigin != null && rustOrigin != launchUrl) destroyFrontend()
                rustOrigin = launchUrl
                createFrontend()
            },
        )
        createFrontend()
    }

    override fun onStart() {
        super.onStart()
        activityStarted = true
        if (BuildConfig.DEBUG) Log.d("FloatViewCore", "stage=activity_lifecycle code=start")
        handler.removeCallbacks(delayedTeardown)
        (application as FloatViewApplication).rustRuntime.updateLifecycle(foreground = true)
        createFrontend()
    }

    override fun onStop() {
        activityStarted = false
        if (BuildConfig.DEBUG) Log.d("FloatViewCore", "stage=activity_lifecycle code=stop")
        handler.removeCallbacks(delayedTeardown)
        handler.postDelayed(delayedTeardown, FRONTEND_BACKGROUND_GRACE_MS)
        (application as FloatViewApplication).rustRuntime.updateLifecycle(foreground = false)
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
        if (!activityStarted || webView != null || isFinishing || isDestroyed) return
        val origin = rustOrigin ?: return
        frontendLoadAttempts = 1
        webView = WebView(this).also { view ->
            view.settings.javaScriptEnabled = true
            view.settings.domStorageEnabled = true
            view.settings.setSupportZoom(false)
            view.settings.builtInZoomControls = false
            view.settings.displayZoomControls = false
            view.webChromeClient = object : WebChromeClient() {
                override fun onShowFileChooser(
                    webView: WebView?,
                    filePathCallback: ValueCallback<Array<Uri>>?,
                    fileChooserParams: FileChooserParams?,
                ): Boolean {
                    this@MainActivity.fileChooserCallback?.onReceiveValue(null)
                    this@MainActivity.fileChooserCallback = filePathCallback
                    val intent = fileChooserParams?.createIntent()
                    if (intent == null) {
                        this@MainActivity.fileChooserCallback = null
                        filePathCallback?.onReceiveValue(null)
                        return false
                    }
                    return try {
                        startActivityForResult(intent, FILE_CHOOSER_REQUEST_CODE)
                        true
                    } catch (_: Exception) {
                        this@MainActivity.fileChooserCallback = null
                        filePathCallback?.onReceiveValue(null)
                        false
                    }
                }

                override fun onConsoleMessage(message: ConsoleMessage): Boolean {
                    if (applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE != 0) {
                        Log.d("FloatViewWebView", "${message.messageLevel()}: ${message.message()}")
                    }
                    return true
                }
            }
            view.webViewClient = object : WebViewClient() {
                override fun onReceivedError(
                    webView: WebView,
                    request: WebResourceRequest,
                    error: WebResourceError,
                ) {
                    super.onReceivedError(webView, request, error)
                    if (
                        request.isForMainFrame &&
                        error.errorCode == ERROR_CONNECT &&
                        frontendLoadAttempts < MAX_FRONTEND_LOAD_ATTEMPTS
                    ) {
                        val attempt = ++frontendLoadAttempts
                        handler.postDelayed(
                            {
                                if (activityStarted && this@MainActivity.webView === webView) {
                                    webView.loadUrl(origin)
                                }
                            },
                            FRONTEND_RETRY_DELAY_MS * attempt,
                        )
                    }
                }
            }
            if (applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE != 0) {
                view.addJavascriptInterface(FrontendPerformanceBridge(), "FloatViewPerformance")
            }
            view.loadUrl(origin)
            frontendContainer.addView(
                view,
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
        frontendLoadAttempts = 0
        frontendContainer.removeView(current)
        current.stopLoading()
        current.loadUrl("about:blank")
        current.removeAllViews()
        current.destroy()
    }

    internal fun frontendIsAttached(): Boolean = webView != null

    override fun onDestroy() {
        handler.removeCallbacks(delayedTeardown)
        (application as FloatViewApplication).rustRuntime.detach(this)
        fileChooserCallback?.onReceiveValue(null)
        fileChooserCallback = null
        destroyFrontend()
        super.onDestroy()
    }

    @Deprecated("Android WebView file chooser uses the legacy activity result callback")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: android.content.Intent?) {
        val runtime = (application as FloatViewApplication).rustRuntime
        if (runtime.onActivityResult(requestCode, resultCode, data)) {
            super.onActivityResult(requestCode, resultCode, data)
            return
        }
        if (requestCode == FILE_CHOOSER_REQUEST_CODE) {
            fileChooserCallback?.onReceiveValue(
                WebChromeClient.FileChooserParams.parseResult(resultCode, data),
            )
            fileChooserCallback = null
        }
        super.onActivityResult(requestCode, resultCode, data)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        val runtime = (application as FloatViewApplication).rustRuntime
        when (requestCode) {
            BluetoothPermissionBroker.BLUETOOTH_PERMISSION_REQUEST ->
                runtime.onBluetoothPermissionResult(permissions)
            LocationPermissionBroker.LOCATION_PERMISSION_REQUEST ->
                runtime.onLocationPermissionResult(permissions)
        }
    }

    companion object {
        internal const val FRONTEND_BACKGROUND_GRACE_MS = 30_000L
        private const val FRONTEND_RETRY_DELAY_MS = 250L
        private const val MAX_FRONTEND_LOAD_ATTEMPTS = 8
        private const val FILE_CHOOSER_REQUEST_CODE = 0xF10A
    }

    private class FrontendPerformanceBridge {
        @JavascriptInterface
        fun report(message: String) {
            Log.d("FloatViewFrontend", message)
        }
    }
}
