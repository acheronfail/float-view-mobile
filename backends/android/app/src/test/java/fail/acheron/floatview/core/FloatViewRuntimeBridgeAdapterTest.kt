package fail.acheron.floatview.core

import fail.acheron.floatview.backend.BluetoothPermissionState
import fail.acheron.floatview.backend.BluetoothPort
import fail.acheron.floatview.backend.BluetoothRadioState
import fail.acheron.floatview.backend.BluetoothState
import fail.acheron.floatview.backend.DeviceId
import fail.acheron.floatview.backend.LocationProviderAuthorization
import fail.acheron.floatview.backend.LocationProviderBackgroundCapture
import fail.acheron.floatview.backend.LocationProviderObservation
import fail.acheron.floatview.backend.LocationProviderPort
import fail.acheron.floatview.backend.LocationProviderState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class FloatViewRuntimeBridgeAdapterTest {
    @Test
    fun dispatchesLocationLifecycleAndValidatedRuntimeReadiness() {
        val runtime = HostFakeRuntime()
        val bluetooth = RustBluetoothPlatformAdapter(runtime, HostFakeBluetooth())
        val location = HostFakeLocation()
        val host = HostFakeRuntimeHost()
        val adapter = FloatViewRuntimeBridgeAdapter(runtime, bluetooth, location, host)

        assertTrue(runtime.events.first().event is PlatformEvent.LocationAuthorizationChanged)
        listOf(
            LocationProviderAuthorization.NOT_DETERMINED to LocationAuthorization.UNKNOWN,
            LocationProviderAuthorization.FOREGROUND_ONLY to LocationAuthorization.FOREGROUND,
            LocationProviderAuthorization.AUTHORIZED to LocationAuthorization.BACKGROUND,
            LocationProviderAuthorization.DENIED to LocationAuthorization.DENIED,
            LocationProviderAuthorization.RESTRICTED to LocationAuthorization.DENIED,
            LocationProviderAuthorization.UNAVAILABLE to LocationAuthorization.UNKNOWN,
        ).forEach { (provider, expected) ->
            location.emitState(provider)
            assertEquals(
                PlatformEvent.LocationAuthorizationChanged(expected),
                runtime.events.last().event,
            )
        }
        location.emitState(LocationProviderAuthorization.AUTHORIZED)

        adapter.dispatch(hostCommand(1, 6) { boolean(false) })
        assertEquals(0, runtime.completions.single { it.first == 1L }.second)
        assertTrue(location.started)
        location.emit(LocationProviderObservation(-33.86, 151.21, 4.0))
        assertTrue(runtime.events.last().event is PlatformEvent.Location)

        adapter.updateLifecycle(foreground = true, backgroundEligible = false)
        assertEquals(
            PlatformEvent.Lifecycle(true, false),
            runtime.events.last().event,
        )

        adapter.dispatch(hostCommand(3, 11) {
            bytes("http://127.0.0.1:32123".encodeToByteArray(), 256)
            bytes("launch-token".encodeToByteArray(), 128)
        })
        assertEquals("http://127.0.0.1:32123", host.endpoint)
        assertEquals("launch-token", host.token)
        assertEquals(0, runtime.completions.single { it.first == 3L }.second)

        adapter.dispatch(hostCommand(4, 11) {
            bytes("https://example.com".encodeToByteArray(), 256)
            bytes("secret".encodeToByteArray(), 128)
        })
        assertEquals(3, runtime.completions.single { it.first == 4L }.second)

        adapter.dispatch(hostCommand(5, 12) {
            bytes("/data/user/0/fail.acheron.floatview/files/float-view/staging/import.tmp".encodeToByteArray(), 4096)
            u8(1)
            bytes("application/octet-stream".encodeToByteArray(), 128)
            i64(128L * 1024 * 1024)
        })
        adapter.dispatch(hostCommand(6, 13) {
            bytes("/data/user/0/fail.acheron.floatview/files/float-view/exports/ride.csv".encodeToByteArray(), 4096)
            bytes("ride.csv".encodeToByteArray(), 255)
            bytes("text/csv".encodeToByteArray(), 128)
        })
        assertEquals(0, runtime.completions.single { it.first == 5L }.second)
        assertEquals(0, runtime.completions.single { it.first == 6L }.second)
        assertEquals(128L * 1024 * 1024, host.maximumImportBytes)
        assertEquals("ride.csv", host.exportDisplayName)
    }

    private fun hostCommand(
        operation: Long,
        tag: Int,
        payload: BridgeWriter.() -> Unit = {},
    ): NativeHostCommand {
        val writer = BridgeWriter().apply(payload)
        return NativeHostCommand(
            tag,
            17,
            operation,
            writer.finish(),
            0,
        )
    }
}

private class HostFakeRuntime : CorePlatformRuntime {
    data class Event(val connectionGeneration: Long, val adapterOrder: Long, val event: PlatformEvent)

    override val runtimeGeneration = 17L
    val events = mutableListOf<Event>()
    val completions = mutableListOf<Triple<Long, Int, ByteArray>>()
    override fun submitPlatformEvent(
        connectionGeneration: Long,
        adapterOrder: Long,
        event: PlatformEvent,
    ) {
        events += Event(connectionGeneration, adapterOrder, event)
    }
    override fun submitBluetoothBytes(connectionGeneration: Long, adapterOrder: Long, payload: ByteArray) = Unit
    override fun complete(command: NativeHostCommand, resultKind: Int, payload: ByteArray) {
        completions += Triple(command.operationId, resultKind, payload.copyOf())
    }
}

private class HostFakeBluetooth : BluetoothPort {
    override fun currentState() = BluetoothState(
        BluetoothRadioState.POWERED_ON,
        BluetoothPermissionState.GRANTED,
        BluetoothPermissionState.GRANTED,
    )
    override fun requestPermission() = Unit
    override fun startDiscovery() = Unit
    override fun stopDiscovery() = Unit
    override fun connect(deviceId: DeviceId) = Unit
    override fun write(deviceId: DeviceId, bytes: ByteArray) = Unit
    override fun disconnect(deviceId: DeviceId) = Unit
}

private class HostFakeLocation : LocationProviderPort {
    var started = false
    private var handler: ((LocationProviderObservation) -> Unit)? = null
    private var stateHandler: ((LocationProviderState) -> Unit)? = null
    private var authorization = LocationProviderAuthorization.AUTHORIZED
    override fun state() = LocationProviderState(
        authorization,
        LocationProviderBackgroundCapture.AVAILABLE,
    )
    override fun setStateHandler(handler: (LocationProviderState) -> Unit) {
        stateHandler = handler
        handler(state())
    }
    override fun requestPermission() = Unit
    override fun openSettings() = Unit
    override fun startUpdates(handler: (LocationProviderObservation) -> Unit) {
        started = true
        this.handler = handler
    }
    override fun stopUpdates() { started = false }
    fun emit(value: LocationProviderObservation) = requireNotNull(handler).invoke(value)
    fun emitState(value: LocationProviderAuthorization) {
        authorization = value
        requireNotNull(stateHandler).invoke(state())
    }
}

private class HostFakeRuntimeHost : FloatViewRuntimeHostProtocol {
    var endpoint: String? = null
    var token: String? = null
    var maximumImportBytes: Long? = null
    var exportDisplayName: String? = null
    override fun setBackgroundLogging(enabled: Boolean) = true
    override fun diagnostic(stage: String, code: String) = Unit
    override fun presentNotificationsPermission() = true
    override fun runtimeReady(endpoint: String, launchToken: String): Boolean {
        this.endpoint = endpoint
        token = launchToken
        return true
    }
    override fun presentImport(
        stagingPath: String,
        allowedContentTypes: List<String>,
        maximumBytes: Long,
        completion: (DocumentResult) -> Unit,
    ) {
        maximumImportBytes = maximumBytes
        completion(DocumentResult.SUCCESS)
    }
    override fun presentExport(
        stagingPath: String,
        displayName: String,
        contentType: String,
        completion: (DocumentResult) -> Unit,
    ) {
        exportDisplayName = displayName
        completion(DocumentResult.SUCCESS)
    }
}
