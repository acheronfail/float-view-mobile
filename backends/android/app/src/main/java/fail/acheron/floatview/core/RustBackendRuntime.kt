package fail.acheron.floatview.core

import android.app.Activity
import android.content.pm.ApplicationInfo
import android.util.Log
import fail.acheron.floatview.BluetoothPermissionBroker
import fail.acheron.floatview.ANDROID_BACKEND_PORT
import fail.acheron.floatview.FloatViewApplication
import fail.acheron.floatview.LocationPermissionBroker
import fail.acheron.floatview.LoggingForegroundService
import fail.acheron.floatview.bluetooth.AndroidBluetoothAdapter
import fail.acheron.floatview.bluetooth.SoftwareBluetoothAdapter
import fail.acheron.floatview.bluetooth.SoftwareVescPeripheral
import fail.acheron.floatview.isEmulator
import fail.acheron.floatview.location.AndroidLocationProvider
import fail.acheron.floatview.location.LocationPermissionRequester
import java.io.Closeable
import java.util.concurrent.atomic.AtomicReference

/** Exactly one Rust backend/runtime graph owned by the Android Application process. */
internal class RustBackendRuntime(private val application: FloatViewApplication) : Closeable {
    val permissionBroker = BluetoothPermissionBroker(application)
    val locationPermissionBroker = LocationPermissionBroker()
    val presentationBroker = AndroidPresentationBroker(application)
    private val commandRelay = AtomicReference<(NativeHostCommand) -> Unit>({})
    private val bluetoothRelay = AtomicReference<(fail.acheron.floatview.backend.BluetoothEvent) -> Unit>({})
    private val core = ProcessCoreBridge(ANDROID_BACKEND_PORT) { command -> commandRelay.get().invoke(command) }
    private val bluetoothPort =
        if (isEmulator()) {
            val peripheral = SoftwareVescPeripheral { path ->
                application.assets.open("refloat-configuration/$path").use { it.readBytes() }
            }
            SoftwareBluetoothAdapter(
                { event -> bluetoothRelay.get().invoke(event) },
                peripheral::response,
            )
        } else {
            AndroidBluetoothAdapter(
                application,
                permissionBroker,
                { event -> bluetoothRelay.get().invoke(event) },
            ) { code -> diagnostic("bluetooth_adapter", code) }
        }
    private val bluetooth = RustBluetoothPlatformAdapter(core, bluetoothPort, ::diagnostic)
    private val location = AndroidLocationProvider(
        application,
        LocationPermissionRequester(locationPermissionBroker::request),
    )
    private val host = AndroidPlatformHostServices(
        application,
        backgroundLogging = { enabled ->
            if (enabled) LoggingForegroundService.start(application)
            else LoggingForegroundService.stop(application)
            true
        },
        documentPresenter = presentationBroker::selectDocument,
        notificationPresenter = presentationBroker::presentNotificationsPermission,
        themePresenter = presentationBroker::presentTheme,
        servicePresenter = presentationBroker::presentServiceState,
        exportPresenter = presentationBroker::presentExport,
        readyPresenter = presentationBroker::runtimeReady,
        presentationCanceller = presentationBroker::cancelPending,
        diagnosticHandler = ::diagnostic,
    )
    private val platform = RustAndroidPlatformAdapter(core, bluetooth, location, host)

    init {
        commandRelay.set(platform::dispatch)
        bluetoothRelay.set(bluetooth::receive)
        bluetooth.receive(
            fail.acheron.floatview.backend.BluetoothEvent.StateChanged(bluetoothPort.currentState()),
        )
    }

    fun attach(
        activity: Activity,
        onReady: (String, String) -> Unit,
        onTheme: (PlatformTheme) -> Unit,
    ) {
        permissionBroker.attach(activity)
        locationPermissionBroker.attach(activity)
        presentationBroker.attach(activity, onReady, onTheme)
        core.replacePresentation(System.identityHashCode(activity).toLong(), platform::dispatch)
    }

    fun detach(activity: Activity) {
        permissionBroker.detach(activity)
        locationPermissionBroker.detach(activity)
        presentationBroker.detach(activity)
    }

    fun updateLifecycle(foreground: Boolean) {
        platform.updateLifecycle(
            foreground,
            backgroundEligible = foreground || LoggingForegroundService.running.get(),
        )
    }

    fun updateLoggingService(active: Boolean) = platform.updateLoggingService(active)

    fun readinessForTest(): Triple<Long, String, String>? = presentationBroker.readinessForTest()

    fun onActivityResult(requestCode: Int, resultCode: Int, data: android.content.Intent?): Boolean =
        presentationBroker.onActivityResult(requestCode, resultCode, data)

    fun onBluetoothPermissionResult(permissions: Array<out String>) {
        permissionBroker.recordResult(permissions)
        bluetooth.receive(fail.acheron.floatview.backend.BluetoothEvent.StateChanged(bluetoothPort.currentState()))
    }

    fun onLocationPermissionResult(permissions: Array<out String>) {
        locationPermissionBroker.recordResult(permissions)
        platform.refreshLocationAuthorization()
    }

    override fun close() {
        platform.shutdown()
        core.close()
    }

    private fun diagnostic(stage: String, code: String) {
        if (application.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0) {
            Log.d("FloatViewCore", "stage=${safe(stage)} code=${safe(code)}")
        }
    }

    private fun safe(value: String): String =
        value.lowercase().map { character ->
            if (character.isLetterOrDigit() || character in "_-." ) character else '_'
        }.take(64).joinToString("")
}
