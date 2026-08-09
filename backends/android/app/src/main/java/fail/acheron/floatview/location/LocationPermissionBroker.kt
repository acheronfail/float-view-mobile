package fail.acheron.floatview.location

import android.app.Activity
import fail.acheron.floatview.lifecycle.FloatViewApplication
import java.lang.ref.WeakReference

class LocationPermissionBroker(application: FloatViewApplication) : LocationPermissionRequester {
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
            current.requestPermissions(permissions, LOCATION_PERMISSION_REQUEST)
        }
    }

    @Synchronized
    fun recordResult(permissions: Array<out String>) {
        permissions.forEach { awaitingResult -= it }
    }

    companion object {
        const val LOCATION_PERMISSION_REQUEST = 7002
    }
}
