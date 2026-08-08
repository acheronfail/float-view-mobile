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
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class RustAndroidPlatformAdapterTest {
    @Test
    fun dispatchesLocationLifecycleAndValidatedRuntimeReadiness() {
        val runtime = HostFakeRuntime()
        val bluetooth = RustBluetoothPlatformAdapter(runtime, HostFakeBluetooth())
        val location = HostFakeLocation()
        val host = HostFakePlatformHost()
        val adapter = RustAndroidPlatformAdapter(runtime, bluetooth, location, host)

        adapter.dispatch(hostCommand(1, 6) { boolean(false) })
        assertEquals(0, runtime.completions.single { it.first == 1L }.second)
        assertTrue(location.started)
        location.emit(LocationProviderObservation(Instant.EPOCH, -33.86, 151.21, 4.0))
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
    override fun state() = LocationProviderState(
        LocationProviderAuthorization.AUTHORIZED,
        LocationProviderBackgroundCapture.AVAILABLE,
    )
    override fun requestPermission() = Unit
    override fun openSettings() = Unit
    override fun startUpdates(handler: (LocationProviderObservation) -> Unit) {
        started = true
        this.handler = handler
    }
    override fun stopUpdates() { started = false }
    fun emit(value: LocationProviderObservation) = requireNotNull(handler).invoke(value)
}

private class HostFakePlatformHost : AndroidPlatformHost {
    var endpoint: String? = null
    var token: String? = null
    override fun setBackgroundLogging(enabled: Boolean) = true
    override fun diagnostic(stage: String, code: String) = Unit
    override fun presentNotificationsPermission() = true
    override fun runtimeReady(endpoint: String, launchToken: String): Boolean {
        this.endpoint = endpoint
        token = launchToken
        return true
    }
}
