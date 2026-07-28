package fail.acheron.floatview

import android.app.Activity
import android.content.pm.ApplicationInfo
import android.util.Log
import fail.acheron.floatview.bluetooth.AndroidBluetoothAdapter
import fail.acheron.floatview.bluetooth.BluetoothPermissionRequester
import fail.acheron.floatview.core.ApplicationService
import fail.acheron.floatview.core.BluetoothEvent
import fail.acheron.floatview.core.ClockPort
import fail.acheron.floatview.core.SystemScheduler
import fail.acheron.floatview.logging.SQLiteLogStore
import fail.acheron.floatview.protocol.VescProtocolService
import java.lang.ref.WeakReference
import java.time.Instant
import java.util.concurrent.atomic.AtomicInteger

class BackendRuntime(application: FloatViewApplication) {
    val permissionBroker = BluetoothPermissionBroker()
    private val relay = BluetoothEventRelay()
    val bluetooth = AndroidBluetoothAdapter(application, permissionBroker, relay::emit)
    val applicationService = ApplicationService(
        bluetooth,
        SystemClock,
        SQLiteLogStore(application),
        protocol = VescProtocolService(),
        scheduler = SystemScheduler(),
    ) { event ->
        // Stable diagnostic only; never log device identifiers or telemetry.
        if (event is Throwable) {
            Log.e("FloatViewBackend", "Native backend operation failed", event)
        }
    }

    init {
        val debuggable =
            application.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
        val remainingDebugCaptures = AtomicInteger(64)
        relay.handler = { event ->
            if (
                debuggable &&
                event is BluetoothEvent.Notification &&
                remainingDebugCaptures.getAndDecrement() > 0
            ) {
                Log.d(
                    "FloatViewProtocolCapture",
                    event.bytes.joinToString(separator = "") { "%02x".format(it) },
                )
            }
            applicationService.onBluetoothEvent(event)
        }
    }
}

class BluetoothPermissionBroker : BluetoothPermissionRequester {
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
            current.requestPermissions(permissions, BLUETOOTH_PERMISSION_REQUEST)
        }
    }

    companion object {
        const val BLUETOOTH_PERMISSION_REQUEST = 7001
    }
}

private class BluetoothEventRelay {
    @Volatile
    var handler: ((BluetoothEvent) -> Unit)? = null
    fun emit(event: BluetoothEvent) {
        handler?.invoke(event)
    }
}

private object SystemClock : ClockPort {
    override fun monotonicMillis(): Long = android.os.SystemClock.elapsedRealtime()
    override fun now(): Instant = Instant.now()
}
