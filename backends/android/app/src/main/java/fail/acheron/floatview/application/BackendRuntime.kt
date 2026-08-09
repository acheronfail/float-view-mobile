package fail.acheron.floatview.application

import android.content.pm.ApplicationInfo
import android.os.Build
import android.util.Log
import fail.acheron.floatview.bluetooth.AndroidBluetoothAdapter
import fail.acheron.floatview.bluetooth.BluetoothPermissionBroker
import fail.acheron.floatview.bluetooth.SoftwareBluetoothAdapter
import fail.acheron.floatview.files.ExportPresentationBroker
import fail.acheron.floatview.lifecycle.FloatViewApplication
import fail.acheron.floatview.lifecycle.LoggingForegroundService
import fail.acheron.floatview.location.LocationPermissionBroker
import fail.acheron.floatview.protocol.VescProtocolService
import fail.acheron.floatview.protocol.vesc.SoftwareVescPeripheral
import fail.acheron.floatview.location.AndroidLocationProvider
import fail.acheron.floatview.location.LocationPermissionRequester
import fail.acheron.floatview.storage.SQLiteLogStore
import java.time.Instant
import java.util.concurrent.atomic.AtomicInteger

class BackendRuntime(application: FloatViewApplication) {
    private val applicationContext = application.applicationContext
    private val debuggable =
        application.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
    val permissionBroker = BluetoothPermissionBroker(application)
    val locationPermissionBroker = LocationPermissionBroker(application)
    val exportPresentationBroker = ExportPresentationBroker(application)
    private val relay = BluetoothEventRelay()
    val events = EventHub()
    private val logStore = SQLiteLogStore(application).also { store ->
        val recovered = store.recoverInterrupted()
        if (debuggable) Log.d("FloatViewState", "recovery interruptedSessions=$recovered")
    }
    val bluetooth: BluetoothPort =
        if (isEmulator()) {
            val peripheral = SoftwareVescPeripheral { path ->
                application.assets.open("refloat-configuration/$path").use { it.readBytes() }
            }
            SoftwareBluetoothAdapter(relay::emit, peripheral::response)
        } else {
            AndroidBluetoothAdapter(
                application,
                permissionBroker,
                relay::emit,
            ) { diagnostic ->
                if (debuggable) Log.d("FloatViewState", "adapter $diagnostic")
            }
        }
    private val locationProvider = AndroidLocationProvider(
        applicationContext,
        LocationPermissionRequester(locationPermissionBroker::request),
    )
    val applicationService = ApplicationService(
        bluetooth,
        SystemClock,
        logStore,
        productStore = logStore,
        sessionStore = logStore,
        protocol = VescProtocolService(),
        scheduler = SystemScheduler(),
        locationProvider = locationProvider,
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
                is String -> Log.d("FloatViewBackend", event)
            }
        }
        if (event is SessionRecord) {
            if (event.active) {
                LoggingForegroundService.start(applicationContext)
            } else {
                LoggingForegroundService.stop(applicationContext)
            }
        }
        if (event is LocationProviderState) {
            LoggingForegroundService.refreshForegroundType(applicationContext)
        }
    }

    init {
        val remainingDebugNotifications = AtomicInteger(64)
        relay.handler = { event ->
            if (
                debuggable &&
                event is BluetoothEvent.Notification &&
                remainingDebugNotifications.getAndDecrement() > 0
            ) {
                Log.d(
                    "FloatViewProtocol",
                    "notification_bytes=${event.bytes.size}",
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
