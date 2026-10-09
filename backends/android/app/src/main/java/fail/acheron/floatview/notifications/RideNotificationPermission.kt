package fail.acheron.floatview.notifications

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.content.edit

/** Ask once, in the foreground after connecting; denial never gates riding. */
class RideNotificationPermission(private val activity: Activity) {
    private val preferences = activity.getSharedPreferences("ride-notification-permission", Context.MODE_PRIVATE)

    fun requestIfNeeded(connected: Boolean): Boolean {
        if (Build.VERSION.SDK_INT < 33 ||
            activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED ||
            preferences.getBoolean("requested", false)) return true
        if (!connected || !activity.hasWindowFocus() || activity.isFinishing || activity.isDestroyed) return false
        preferences.edit { putBoolean("requested", true) }
        activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), REQUEST_CODE)
        return true
    }

    companion object { const val REQUEST_CODE = 7002 }
}
