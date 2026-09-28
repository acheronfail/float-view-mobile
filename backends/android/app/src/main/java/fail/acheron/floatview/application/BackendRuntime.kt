package fail.acheron.floatview.application

import android.content.pm.ApplicationInfo
import android.util.Log
import fail.acheron.floatview.bluetooth.hostBluetooth
import fail.acheron.floatview.bluetooth.BluetoothPermissionBroker
import fail.acheron.floatview.files.ExportPresentationBroker
import fail.acheron.floatview.lifecycle.FloatViewApplication
import fail.acheron.floatview.lifecycle.LoggingForegroundService
import fail.acheron.floatview.location.LocationPermissionBroker
import fail.acheron.floatview.location.AndroidLocationProvider
import fail.acheron.floatview.location.LocationPermissionRequester

class BackendRuntime(application: FloatViewApplication) {
    private val applicationContext = application.applicationContext
    private val debuggable =
        application.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
    val permissionBroker = BluetoothPermissionBroker(application)
    val locationPermissionBroker = LocationPermissionBroker(application)
    val exportPresentationBroker = ExportPresentationBroker(application)
    private val relay = BluetoothEventRelay()
    val bluetooth: BluetoothPort = hostBluetooth(application, permissionBroker, relay::emit) { diagnostic ->
        if (debuggable) Log.d("FloatViewState", "adapter $diagnostic")
    }
    private val locationProvider = AndroidLocationProvider(
        applicationContext,
        LocationPermissionRequester(locationPermissionBroker::request),
    )
    val service = SharedService(
        java.io.File(application.filesDir, "float-view/logs"), bluetooth, locationProvider,
        exportPresentationBroker::present,
        recordingChanged = { active ->
            if (active) LoggingForegroundService.start(applicationContext)
            else LoggingForegroundService.stop(applicationContext)
        },
        locationChanged = { LoggingForegroundService.refreshForegroundType(applicationContext) },
    )
    init {
        relay.handler = service::receive
        service.start()
    }

}

private class BluetoothEventRelay {
    @Volatile
    var handler: ((BluetoothEvent) -> Unit)? = null
    fun emit(event: BluetoothEvent) {
        handler?.invoke(event)
    }
}
