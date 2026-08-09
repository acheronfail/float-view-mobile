package fail.acheron.floatview.bluetooth

import android.app.Activity
import fail.acheron.floatview.lifecycle.FloatViewApplication
import java.lang.ref.WeakReference

class BluetoothPermissionBroker(application: FloatViewApplication) : BluetoothPermissionRequester {
    private val preferences =
        application.getSharedPreferences("bluetooth-permission-history", 0)
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
