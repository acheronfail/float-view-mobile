package fail.acheron.floatview

import android.app.Activity
import android.os.Build
import fail.acheron.floatview.bluetooth.BluetoothPermissionRequester
import fail.acheron.floatview.location.LocationPermissionRequester
import java.lang.ref.WeakReference

internal const val ANDROID_BACKEND_PORT = 11012

internal fun isEmulator(): Boolean =
    Build.HARDWARE == "ranchu" ||
        Build.HARDWARE == "goldfish" ||
        Build.PRODUCT.startsWith("sdk_") ||
        Build.FINGERPRINT.contains("generic") ||
        Build.FINGERPRINT.contains("emulator")

class BluetoothPermissionBroker(application: FloatViewApplication) : BluetoothPermissionRequester {
    private val preferences = application.getSharedPreferences("bluetooth-permission-history", 0)
    private var activity = WeakReference<Activity>(null)
    private var pending: Array<String>? = null
    private val awaitingResult = mutableSetOf<String>()

    @Synchronized
    fun attach(value: Activity) {
        activity = WeakReference(value)
        pending?.also {
            pending = null
            request(it)
        }
    }

    @Synchronized
    fun detach(value: Activity) {
        if (activity.get() === value) activity.clear()
    }

    @Synchronized
    override fun request(permissions: Array<String>) {
        val current = activity.get()
        if (current == null) {
            pending = permissions
            return
        }
        awaitingResult += permissions
        current.runOnUiThread {
            current.requestPermissions(permissions, BLUETOOTH_PERMISSION_REQUEST)
        }
    }

    @Synchronized
    override fun wasResolved(permission: String): Boolean =
        permission !in awaitingResult && preferences.getBoolean(permission, false)

    @Synchronized
    fun recordResult(permissions: Array<out String>) {
        preferences.edit().also { editor ->
            permissions.forEach { permission ->
                awaitingResult -= permission
                editor.putBoolean(permission, true)
            }
        }.apply()
    }

    companion object {
        const val BLUETOOTH_PERMISSION_REQUEST = 7001
    }
}

class LocationPermissionBroker : LocationPermissionRequester {
    private var activity = WeakReference<Activity>(null)
    private var pending: Array<String>? = null

    @Synchronized
    fun attach(value: Activity) {
        activity = WeakReference(value)
        pending?.also {
            pending = null
            request(it)
        }
    }

    @Synchronized
    fun detach(value: Activity) {
        if (activity.get() === value) activity.clear()
    }

    @Synchronized
    override fun request(permissions: Array<String>) {
        val current = activity.get()
        if (current == null) {
            pending = permissions
            return
        }
        current.runOnUiThread {
            current.requestPermissions(permissions, LOCATION_PERMISSION_REQUEST)
        }
    }

    fun recordResult(@Suppress("UNUSED_PARAMETER") permissions: Array<out String>) = Unit

    companion object {
        const val LOCATION_PERMISSION_REQUEST = 7002
    }
}
