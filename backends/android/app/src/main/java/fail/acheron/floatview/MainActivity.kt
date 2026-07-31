package fail.acheron.floatview

import android.annotation.SuppressLint
import android.app.Activity
import android.content.ComponentCallbacks2
import android.content.res.Configuration
import android.graphics.Color
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import android.widget.FrameLayout
import androidx.core.view.WindowInsetsControllerCompat
import fail.acheron.floatview.backend.SettingsRecord
import fail.acheron.floatview.backend.ThemePreference

class MainActivity : Activity() {
    private var webView: WebView? = null
    private var themeSubscription: AutoCloseable? = null
    private val frontendContainer by lazy { FrameLayout(this) }
    private val handler = Handler(Looper.getMainLooper())
    private val delayedTeardown = Runnable { destroyFrontend() }

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
        themeSubscription?.close()
        themeSubscription = null
        destroyFrontend()
        super.onDestroy()
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
    }
}
