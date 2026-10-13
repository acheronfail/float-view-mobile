package fail.acheron.floatview.theme

import android.app.Activity
import android.content.res.Configuration
import android.graphics.Color
import android.widget.FrameLayout
import androidx.core.view.WindowInsetsControllerCompat
import fail.acheron.floatview.application.ThemePreference
import fail.acheron.floatview.ui.LoadingPresentation

class AndroidThemeController(
    private val activity: Activity,
    private val frontendContainer: FrameLayout,
) {
    @Suppress("DEPRECATION")
    fun apply(preference: ThemePreference) {
        val dark =
            when (preference) {
                ThemePreference.DARK -> true
                ThemePreference.LIGHT -> false
                ThemePreference.SYSTEM ->
                    activity.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
                        Configuration.UI_MODE_NIGHT_YES
            }
        val background = Color.parseColor(if (dark) "#020617" else "#F8FAFC")
        activity.window.statusBarColor = background
        activity.window.navigationBarColor = background
        activity.window.navigationBarDividerColor = background
        activity.window.isNavigationBarContrastEnforced = false
        activity.window.setBackgroundDrawable(android.graphics.drawable.ColorDrawable(background))
        frontendContainer.setBackgroundColor(background)
        for (index in 0 until frontendContainer.childCount) {
            (frontendContainer.getChildAt(index) as? LoadingPresentation)?.applyTheme(dark)
        }
        WindowInsetsControllerCompat(activity.window, activity.window.decorView).apply {
            isAppearanceLightStatusBars = !dark
            isAppearanceLightNavigationBars = !dark
        }
    }
}
