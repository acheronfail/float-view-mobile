package fail.acheron.floatview

import android.app.Activity
import android.content.pm.ApplicationInfo
import android.util.Log
import fail.acheron.floatview.bluetooth.AndroidBluetoothAdapter
import fail.acheron.floatview.bluetooth.BluetoothPermissionRequester
import fail.acheron.floatview.core.ApplicationService
import fail.acheron.floatview.core.BluetoothEvent
import fail.acheron.floatview.core.BluetoothState
import fail.acheron.floatview.core.ClockPort
import fail.acheron.floatview.core.ConnectionState
import fail.acheron.floatview.core.SessionRecord
import fail.acheron.floatview.core.SystemScheduler
import fail.acheron.floatview.logging.SQLiteLogStore
import fail.acheron.floatview.protocol.VescProtocolService
import java.lang.ref.WeakReference
import java.time.Instant
import java.util.concurrent.atomic.AtomicInteger

class BackendRuntime(application: FloatViewApplication) {
    private val debuggable =
        application.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
    val permissionBroker = BluetoothPermissionBroker(application)
    private val relay = BluetoothEventRelay()
    private val logStore = SQLiteLogStore(application).also { store ->
        val recovered = store.recoverInterrupted()
        if (debuggable) Log.d("FloatViewState", "recovery interruptedSessions=$recovered")
    }
    val bluetooth = AndroidBluetoothAdapter(
        application,
        permissionBroker,
        relay::emit,
    ) { diagnostic ->
        if (debuggable) Log.d("FloatViewState", "adapter $diagnostic")
    }
    val applicationService = ApplicationService(
        bluetooth,
        SystemClock,
        logStore,
        productStore = logStore,
        sessionStore = logStore,
        protocol = VescProtocolService(),
        scheduler = SystemScheduler(),
    ) { event ->
        // Stable diagnostic only; never log device identifiers or telemetry.
        if (event is Throwable) {
            Log.e("FloatViewBackend", "Native backend operation failed", event)
        }
        if (debuggable) {
            when (event) {
                is BluetoothState -> Log.d(
                    "FloatViewState",
                    "bluetooth radio=${event.radio.name.lowercase()} " +
                        "scanPermission=${event.scanPermission.name.lowercase()} " +
                        "connectPermission=${event.connectPermission.name.lowercase()}",
                )
                is ConnectionState -> Log.d(
                    "FloatViewState",
                    "connection state=${event.name.lowercase()}",
                )
                is SessionRecord -> Log.d(
                    "FloatViewState",
                    "session active=${event.active}",
                )
            }
        }
    }

    init {
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
