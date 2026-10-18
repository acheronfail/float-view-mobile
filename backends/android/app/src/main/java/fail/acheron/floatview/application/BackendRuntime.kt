package fail.acheron.floatview.application

import android.content.pm.ApplicationInfo
import android.util.Log
import fail.acheron.floatview.bluetooth.hostBluetooth
import fail.acheron.floatview.permissions.PermissionBroker
import fail.acheron.floatview.files.ExportPresentationBroker
import fail.acheron.floatview.lifecycle.FloatViewApplication
import fail.acheron.floatview.lifecycle.LoggingForegroundService
import fail.acheron.floatview.location.AndroidLocationProvider

class BackendRuntime(application: FloatViewApplication) {
    private val applicationContext = application.applicationContext
    private val debuggable =
        application.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
    val permissionBroker = PermissionBroker(application)
    val exportPresentationBroker = ExportPresentationBroker(application)
    private val relay = BluetoothEventRelay()
    val bluetooth: BluetoothPort = hostBluetooth(application, permissionBroker, relay::emit) { diagnostic ->
        if (debuggable) Log.d("FloatViewState", "adapter $diagnostic")
    }
    private val locationProvider = AndroidLocationProvider(
        applicationContext,
        permissionBroker::wasResolved,
    )
    val service: SharedService = run {
        var prepared: SharedService? = null
        try {
            SharedService(
                applicationContext,
                java.io.File(application.filesDir, "float-view/logs"), bluetooth, locationProvider, permissionBroker,
                exportPresentationBroker::present,
                foregroundRequiredChanged = { active ->
                    if (active) LoggingForegroundService.start(applicationContext)
                    else LoggingForegroundService.stop(applicationContext)
                },
                locationChanged = { LoggingForegroundService.refreshForegroundType(applicationContext) },
            ).also {
                prepared = it
                relay.handler = it::receive
                it.start()
            }
        } catch (error: Exception) {
            relay.handler = null
            try { prepared?.close() } finally { (bluetooth as? AutoCloseable)?.close() }
            throw error
        }
    }
}

private class BluetoothEventRelay {
    @Volatile
    var handler: ((BluetoothEvent) -> Unit)? = null
    fun emit(event: BluetoothEvent) {
        handler?.invoke(event)
    }
}
