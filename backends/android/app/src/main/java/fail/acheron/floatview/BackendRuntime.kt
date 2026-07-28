package fail.acheron.floatview

import android.app.Activity
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.util.Log
import android.os.Build
import androidx.core.content.FileProvider
import fail.acheron.floatview.bluetooth.AndroidBluetoothAdapter
import fail.acheron.floatview.bluetooth.BluetoothPermissionRequester
import fail.acheron.floatview.bluetooth.SoftwareBluetoothAdapter
import fail.acheron.floatview.backend.ApplicationService
import fail.acheron.floatview.backend.BluetoothEvent
import fail.acheron.floatview.backend.BluetoothState
import fail.acheron.floatview.backend.BluetoothPort
import fail.acheron.floatview.backend.ClockPort
import fail.acheron.floatview.backend.ConnectionState
import fail.acheron.floatview.backend.EventHub
import fail.acheron.floatview.backend.SamplingSettings
import fail.acheron.floatview.backend.SessionExportRecord
import fail.acheron.floatview.backend.SessionRecord
import fail.acheron.floatview.backend.SystemScheduler
import fail.acheron.floatview.storage.SQLiteLogStore
import fail.acheron.floatview.protocol.VescProtocolService
import java.lang.ref.WeakReference
import java.io.File
import java.time.Instant
import java.util.concurrent.atomic.AtomicInteger

class BackendRuntime(application: FloatViewApplication) {
    private val applicationContext = application.applicationContext
    private val debuggable =
        application.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
    val permissionBroker = BluetoothPermissionBroker(application)
    val exportPresentationBroker = ExportPresentationBroker(application)
    private val relay = BluetoothEventRelay()
    val events = EventHub()
    private val logStore = SQLiteLogStore(application).also { store ->
        val recovered = store.recoverInterrupted()
        if (debuggable) Log.d("FloatViewState", "recovery interruptedSessions=$recovered")
    }
    val bluetooth: BluetoothPort =
        if (isEmulator()) {
            SoftwareBluetoothAdapter(relay::emit)
        } else {
            AndroidBluetoothAdapter(
                application,
                permissionBroker,
                relay::emit,
            ) { diagnostic ->
                if (debuggable) Log.d("FloatViewState", "adapter $diagnostic")
            }
        }
    val applicationService = ApplicationService(
        bluetooth,
        SystemClock,
        logStore,
        productStore = logStore,
        sessionStore = logStore,
        protocol = VescProtocolService(),
        scheduler = SystemScheduler(),
        samplingSettings = logStore.settings().let { settings ->
            SamplingSettings(
                idlePersistIntervalMs = settings.idlePersistIntervalMs,
                ridingPersistIntervalMs = settings.ridingPersistIntervalMs,
                postRideTailMs = settings.postRideTailMs,
                reconnectDurationMs = settings.reconnectDurationMs,
                idleProbeIntervalMs = settings.idleProbeIntervalMs,
            )
        },
    ) { event -> events.publish(event) }

    private val eventSubscription = events.subscribe { event ->
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
        if (event is SessionRecord) {
            if (event.active) {
                LoggingForegroundService.start(applicationContext)
            } else {
                LoggingForegroundService.stop(applicationContext)
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

internal fun isEmulator(): Boolean =
    Build.HARDWARE == "ranchu" ||
        Build.HARDWARE == "goldfish" ||
        Build.PRODUCT.startsWith("sdk_") ||
        Build.FINGERPRINT.contains("generic") ||
        Build.FINGERPRINT.contains("emulator")

class ExportPresentationBroker(private val application: FloatViewApplication) {
    private var activity = WeakReference<Activity>(null)

    @Synchronized
    fun attach(value: Activity) {
        activity = WeakReference(value)
    }

    @Synchronized
    fun detach(value: Activity) {
        if (activity.get() === value) activity.clear()
    }

    @Synchronized
    fun present(value: SessionExportRecord): Boolean {
        val current = activity.get() ?: return false
        val file = File(value.filePath)
        if (!file.isFile) return false
        val uri = FileProvider.getUriForFile(
            application,
            "${application.packageName}.exports",
            file,
        )
        val intent = Intent(Intent.ACTION_SEND)
            .setType(if (value.fileName.endsWith(".csv")) "text/csv" else "application/octet-stream")
            .putExtra(Intent.EXTRA_STREAM, uri)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        current.runOnUiThread {
            current.startActivity(Intent.createChooser(intent, "Save or share session"))
        }
        return true
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
