package fail.acheron.floatview.host

import android.app.Activity
import android.content.res.Configuration
import android.graphics.Color
import android.widget.FrameLayout
import androidx.core.view.WindowInsetsControllerCompat

// Capability status: incomplete — physical live-change verification remains.
// Concrete source: frontend prefers-color-scheme; this keeps native system bars aligned.
internal object FloatViewAppearance {
    fun applyNativeChrome(activity: Activity, container: FrameLayout) {
        val dark = activity.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
            Configuration.UI_MODE_NIGHT_YES
        val background = Color.parseColor(if (dark) "#020617" else "#F8FAFC")
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
