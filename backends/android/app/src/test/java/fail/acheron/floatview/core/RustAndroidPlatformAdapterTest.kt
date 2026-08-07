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
import java.io.File
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class RustAndroidPlatformAdapterTest {
    @Test
    fun dispatchesLocationLifecycleClockAndValidatedRuntimeReadiness() {
        val runtime = HostFakeRuntime()
        val bluetooth = RustBluetoothPlatformAdapter(runtime, HostFakeBluetooth())
        val location = HostFakeLocation()
        val host = HostFakePlatformHost()
        val adapter = RustAndroidPlatformAdapter(runtime, bluetooth, location, host)

        adapter.dispatch(hostCommand(1, 6) { boolean(false) })
        assertEquals(0, runtime.completions.single { it.first == 1L }.second)
        assertTrue(location.started)
        location.emit(LocationProviderObservation(Instant.EPOCH, -33.86, 151.21, 4.0))
        assertTrue(PlatformEventEnvelope.decode(runtime.events.last()).event is PlatformEvent.Location)

        adapter.updateLifecycle(foreground = true, backgroundEligible = false)
        assertEquals(
            PlatformEvent.Lifecycle(true, false),
            PlatformEventEnvelope.decode(runtime.events.last()).event,
        )

        adapter.dispatch(hostCommand(2, 13))
        assertEquals(8, runtime.completions.single { it.first == 2L }.third.size)

        adapter.dispatch(hostCommand(3, 22) {
            bytes("http://127.0.0.1:32123".encodeToByteArray(), 256)
            bytes("launch-token".encodeToByteArray(), 128)
        })
        assertEquals("http://127.0.0.1:32123", host.endpoint)
        assertEquals("launch-token", host.token)
        assertEquals(0, runtime.completions.single { it.first == 3L }.second)

        adapter.dispatch(hostCommand(4, 22) {
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
            PLATFORM_COMMAND_KIND,
            17,
            operation,
            PlatformCommandEnvelope(17, operation, PlatformCommand.Host(tag, writer.finish())).encode(),
            0,
        )
    }
}

private class HostFakeRuntime : CorePlatformRuntime {
    override val runtimeGeneration = 17L
    val events = mutableListOf<ByteArray>()
    val completions = mutableListOf<Triple<Long, Int, ByteArray>>()
    override fun submitPlatformEvent(payload: ByteArray) { events += payload.copyOf() }
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
    override fun readResource(namespace: ResourceNamespace, logicalName: String, maximumBytes: Long) = byteArrayOf(1)
    override fun stageDocument(destination: File, maximumBytes: Long, completion: (AndroidPlatformHostError?) -> Unit) = completion(null)
    override fun storageRoot(kind: StorageRootKind) = File("/tmp/core")
    override fun availableSpace(root: File) = 1024L
    override fun monotonicMilliseconds() = 123L
    override fun wallMilliseconds() = 456L
    override fun secureRandom(count: Int) = ByteArray(count) { 7 }
    override fun diagnostic(stage: String, code: String) = Unit
    override fun presentNotificationsPermission() = true
    override fun presentTheme(theme: PlatformTheme) = true
    override fun presentServiceState(active: Boolean) = true
    override fun presentExport(stagedFile: File, completion: (AndroidPlatformHostError?) -> Unit) = completion(null)
    override fun runtimeReady(endpoint: String, launchToken: String): Boolean {
        this.endpoint = endpoint
        token = launchToken
        return true
    }
    override fun cancelPendingPresentation() = Unit
}
