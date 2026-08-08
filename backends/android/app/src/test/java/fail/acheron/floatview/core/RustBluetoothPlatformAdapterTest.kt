package fail.acheron.floatview.core

import fail.acheron.floatview.backend.BluetoothEvent
import fail.acheron.floatview.backend.BluetoothPermissionState
import fail.acheron.floatview.backend.BluetoothPort
import fail.acheron.floatview.backend.BluetoothRadioState
import fail.acheron.floatview.backend.BluetoothState
import fail.acheron.floatview.backend.ConnectionState
import fail.acheron.floatview.backend.DeviceId
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.CountDownLatch

class RustBluetoothPlatformAdapterTest {
    @Test
    fun concurrentCallbacksEnterTheRuntimeInOneEstablishedOrder() {
        val runtime = FakeRuntime()
        val adapter = RustBluetoothPlatformAdapter(runtime, FakeBluetooth())
        val start = CountDownLatch(1)
        val threads = listOf(
            Thread {
                start.await()
                adapter.receive(
                    BluetoothEvent.StateChanged(
                        BluetoothState(
                            BluetoothRadioState.POWERED_ON,
                            BluetoothPermissionState.GRANTED,
                            BluetoothPermissionState.GRANTED,
                        ),
                    ),
                )
            },
            Thread {
                start.await()
                adapter.receive(BluetoothEvent.DeviceFound(DeviceId("private"), "Board", -1))
            },
        )
        threads.forEach(Thread::start)
        start.countDown()
        threads.forEach(Thread::join)
        assertEquals(
            listOf(1L, 2L),
            runtime.events.map { it.adapterOrder }.sorted(),
        )
    }

    @Test
    fun mapsPrivateAssociationsOrderedGenerationsNotificationsAndStaleCancellation() {
        val runtime = FakeRuntime()
        val bluetooth = FakeBluetooth()
        val diagnostics = mutableListOf<Pair<String, String>>()
        val adapter = RustBluetoothPlatformAdapter(runtime, bluetooth) { stage, code ->
            diagnostics += stage to code
        }
        val device = DeviceId("private-platform-id")

        adapter.receive(
            BluetoothEvent.StateChanged(
                BluetoothState(
                    BluetoothRadioState.POWERED_ON,
                    BluetoothPermissionState.GRANTED,
                    BluetoothPermissionState.GRANTED,
                ),
            ),
        )
        adapter.receive(BluetoothEvent.DeviceFound(device, "Test Board", -42))

        val state = runtime.events[0]
        val discovery = runtime.events[1]
        assertEquals(1, state.adapterOrder)
        assertEquals(2, discovery.adapterOrder)
        val token = (discovery.event as PlatformEvent.BluetoothDevice).token
        assertArrayEquals(device.value.encodeToByteArray(), token)

        val connect = command(7, PlatformCommand.BluetoothConnect(token))
        assertTrue(adapter.dispatch(connect))
        assertEquals(listOf(device), bluetooth.connected)
        adapter.receive(BluetoothEvent.ConnectionChanged(device, ConnectionState.READY))
        assertEquals(0, runtime.completions.single { it.first == 7L }.second)

        adapter.receive(BluetoothEvent.Notification(device, byteArrayOf(1, 2, 3)))
        assertEquals(1, runtime.bluetooth.single().first)
        assertEquals(4, runtime.bluetooth.single().second)
        assertArrayEquals(byteArrayOf(1, 2, 3), runtime.bluetooth.single().third)

        val staleWrite = command(8, PlatformCommand.BluetoothWrite(0, byteArrayOf(9)))
        assertTrue(adapter.dispatch(staleWrite))
        assertEquals(1, runtime.completions.single { it.first == 8L }.second)
        assertTrue(bluetooth.writes.isEmpty())

        adapter.receive(BluetoothEvent.ScanFailed("Address AA:BB/secret"))
        assertEquals("bluetooth_scan", diagnostics.single().first)
        assertEquals("address_aa_bb_secret", diagnostics.single().second)
    }

    private fun command(operation: Long, command: PlatformCommand): NativeHostCommand =
        NativeHostCommand(
            kind = command.kind,
            runtimeGeneration = 11,
            operationId = operation,
            payload = PlatformCommandEnvelope(11, operation, command).encode(),
            presentationGeneration = 0,
        )
}

private class FakeRuntime : CorePlatformRuntime {
    data class Event(
        val connectionGeneration: Long,
        val adapterOrder: Long,
        val event: PlatformEvent,
    )

    override val runtimeGeneration = 11L
    val events = mutableListOf<Event>()
    val bluetooth = mutableListOf<Triple<Long, Long, ByteArray>>()
    val completions = mutableListOf<Triple<Long, Int, ByteArray>>()

    override fun submitPlatformEvent(
        connectionGeneration: Long,
        adapterOrder: Long,
        event: PlatformEvent,
    ) {
        events += Event(connectionGeneration, adapterOrder, event)
    }

    override fun submitBluetoothBytes(
        connectionGeneration: Long,
        adapterOrder: Long,
        payload: ByteArray,
    ) {
        bluetooth += Triple(connectionGeneration, adapterOrder, payload.copyOf())
    }

    override fun complete(command: NativeHostCommand, resultKind: Int, payload: ByteArray) {
        completions += Triple(command.operationId, resultKind, payload.copyOf())
    }
}

private class FakeBluetooth : BluetoothPort {
    val connected = mutableListOf<DeviceId>()
    val writes = mutableListOf<Pair<DeviceId, ByteArray>>()

    override fun currentState() = BluetoothState(
        BluetoothRadioState.POWERED_ON,
        BluetoothPermissionState.GRANTED,
        BluetoothPermissionState.GRANTED,
    )

    override fun requestPermission() = Unit
    override fun startDiscovery() = Unit
    override fun stopDiscovery() = Unit
    override fun connect(deviceId: DeviceId) {
        connected += deviceId
    }
    override fun write(deviceId: DeviceId, bytes: ByteArray) {
        writes += deviceId to bytes.copyOf()
    }
    override fun disconnect(deviceId: DeviceId) = Unit
}
