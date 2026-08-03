package fail.acheron.floatview

import android.annotation.SuppressLint
import android.app.Activity
import android.content.ComponentCallbacks2
import android.content.res.Configuration
import android.graphics.Color
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
import androidx.core.view.WindowInsetsControllerCompat
import fail.acheron.floatview.backend.SettingsRecord
import fail.acheron.floatview.backend.ThemePreference
import java.io.ByteArrayOutputStream
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

class MainActivity : Activity() {
    private var webView: WebView? = null
    private var fileChooserCallback: ValueCallback<Array<Uri>>? = null
    private var themeSubscription: AutoCloseable? = null
    private val frontendContainer by lazy { FrameLayout(this) }
    private val handler = Handler(Looper.getMainLooper())
    private val delayedTeardown = Runnable { destroyFrontend() }
    private val importScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val app = application as FloatViewApplication
        app.runtime.permissionBroker.attach(this)
        app.runtime.exportPresentationBroker.attach(this)
        applySystemBarTheme(app.runtime.applicationService.settings().themePreference)
        themeSubscription = app.runtime.events.subscribe { event ->
            if (event is SettingsRecord) {
                runOnUiThread { applySystemBarTheme(event.themePreference) }
            }
        }
        setContentView(frontendContainer)
        createFrontend()
    }

    override fun onStart() {
        super.onStart()
        handler.removeCallbacks(delayedTeardown)
        createFrontend()
    }

    override fun onResume() {
        super.onResume()
        val preference =
            (application as FloatViewApplication).runtime.applicationService.settings().themePreference
        applySystemBarTheme(preference)
    }

    override fun onStop() {
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
                    Log.d("FloatViewFileChooser", "show")
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
                        Log.d("FloatViewFileChooser", "launch_failed")
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

    @Suppress("DEPRECATION")
    private fun applySystemBarTheme(preference: ThemePreference) {
        val dark =
            when (preference) {
                ThemePreference.DARK -> true
                ThemePreference.LIGHT -> false
                ThemePreference.SYSTEM ->
                    resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
                        Configuration.UI_MODE_NIGHT_YES
            }
        val background = Color.parseColor(if (dark) "#020617" else "#F8FAFC")
        window.statusBarColor = background
        window.navigationBarColor = background
        window.navigationBarDividerColor = background
        window.isNavigationBarContrastEnforced = false
        frontendContainer.setBackgroundColor(background)
        WindowInsetsControllerCompat(window, window.decorView).apply {
            isAppearanceLightStatusBars = !dark
            isAppearanceLightNavigationBars = !dark
        }
    }

    override fun onDestroy() {
        handler.removeCallbacks(delayedTeardown)
        val runtime = (application as FloatViewApplication).runtime
        runtime.permissionBroker.detach(this)
        runtime.exportPresentationBroker.detach(this)
        importScope.cancel()
        themeSubscription?.close()
        themeSubscription = null
        destroyFrontend()
        super.onDestroy()
    }

    @Deprecated("Android WebView file chooser uses the legacy activity result callback")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: android.content.Intent?) {
        if (requestCode == FILE_CHOOSER_REQUEST_CODE) {
            Log.d(
                "FloatViewFileChooser",
                "result code=$resultCode hasData=${data?.data != null} clipCount=${data?.clipData?.itemCount ?: 0} callback=${fileChooserCallback != null}",
            )
            val result = WebChromeClient.FileChooserParams.parseResult(resultCode, data)
            // Galaxy S9 WebView 148 returns the URI to the host but does not
            // populate the DOM FileList. Import the URI natively, then notify
            // the frontend so the same archive importer is used on-device.
            fileChooserCallback?.onReceiveValue(null)
            fileChooserCallback = null
            result?.singleOrNull()?.let(::importSelectedSession)
        }
        super.onActivityResult(requestCode, resultCode, data)
    }

    private fun importSelectedSession(uri: Uri) {
        val app = application as FloatViewApplication
        dispatchFrontendImport("floatview:session-import-started", JSONObject())
        importScope.launch {
            try {
                val sessionId = withContext(Dispatchers.IO) {
                    app.runtime.applicationService.importSessionArchive(readImportUri(uri))
                }
                app.backend.publishSessionCatalogChanged()
                Log.d("FloatViewImport", "complete")
                dispatchFrontendImport(
                    "floatview:session-imported",
                    JSONObject().put("sessionId", sessionId),
                )
            } catch (error: Exception) {
                Log.e("FloatViewImport", "session import failed: ${error::class.simpleName}")
                dispatchFrontendImport(
                    "floatview:session-import-failed",
                    JSONObject().put("message", "Session import failed"),
                )
            }
        }
    }

    private fun readImportUri(uri: Uri): ByteArray {
        val input = contentResolver.openInputStream(uri) ?: error("Selected file cannot be opened")
        input.use { stream ->
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(64 * 1024)
            var total = 0
            while (true) {
                val count = stream.read(buffer)
                if (count < 0) break
                total += count
                check(total <= MAX_SESSION_IMPORT_BYTES) { "Selected file is too large" }
                output.write(buffer, 0, count)
            }
            return output.toByteArray()
        }
    }

    private fun dispatchFrontendImport(event: String, detail: JSONObject) {
        val script =
            "window.dispatchEvent(new CustomEvent(${JSONObject.quote(event)}, { detail: $detail }));"
        webView?.post { webView?.evaluateJavascript(script, null) }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == BluetoothPermissionBroker.BLUETOOTH_PERMISSION_REQUEST) {
            val runtime = (application as FloatViewApplication).runtime
            runtime.permissionBroker.recordResult(permissions)
            runtime.applicationService.onBluetoothEvent(
                fail.acheron.floatview.backend.BluetoothEvent.StateChanged(
                    runtime.bluetooth.currentState(),
                ),
            )
        }
    }

    companion object {
        internal const val FRONTEND_BACKGROUND_GRACE_MS = 30_000L
        private const val FILE_CHOOSER_REQUEST_CODE = 0xF10A
    }

    private class FrontendPerformanceBridge {
        @JavascriptInterface
        fun report(message: String) {
            Log.d("FloatViewFrontend", message)
        }
    }
}
