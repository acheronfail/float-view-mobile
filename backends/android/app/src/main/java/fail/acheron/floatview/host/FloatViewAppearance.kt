package fail.acheron.floatview.host

import android.app.Activity
import android.content.res.Configuration
import android.graphics.Color
import android.widget.FrameLayout
import androidx.core.view.WindowInsetsControllerCompat

// Capability status: implemented — verified on emulator and physical Android hardware.
// Concrete source: frontend prefers-color-scheme; this keeps native system bars aligned.
internal object FloatViewAppearance {
    const val source = "frontend-prefers-color-scheme"

    fun isDarkMode(uiMode: Int): Boolean =
        uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES

    fun backgroundColorHex(dark: Boolean): String = if (dark) "#020617" else "#F8FAFC"

    fun applyNativeChrome(activity: Activity, container: FrameLayout) {
        val dark = isDarkMode(activity.resources.configuration.uiMode)
        val background = Color.parseColor(backgroundColorHex(dark))
        activity.window.statusBarColor = background
        activity.window.navigationBarColor = background
        activity.window.navigationBarDividerColor = background
        activity.window.isNavigationBarContrastEnforced = false
        container.setBackgroundColor(background)
        WindowInsetsControllerCompat(activity.window, activity.window.decorView).apply {
            isAppearanceLightStatusBars = !dark
            isAppearanceLightNavigationBars = !dark
        }
    }
}
