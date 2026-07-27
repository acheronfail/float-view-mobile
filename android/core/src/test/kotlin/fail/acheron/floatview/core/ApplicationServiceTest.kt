package fail.acheron.floatview.core

import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test

class ApplicationServiceTest {
    @Test
    fun `ready creates log before service exposes stable state`() {
        val store = FakeStore()
        val service = ApplicationService(FakeBluetooth(), FakeClock(), store) {}
        val id = DeviceId("fixture")

        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))

        assertEquals(ConnectionState.READY, service.connectionState)
        assertNotNull(store.active())
    }

    private class FakeBluetooth : BluetoothPort {
        override fun startDiscovery() = Unit
        override fun stopDiscovery() = Unit
        override fun connect(deviceId: DeviceId) = Unit
        override fun write(deviceId: DeviceId, bytes: ByteArray) = Unit
        override fun disconnect(deviceId: DeviceId) = Unit
    }

    private class FakeClock : ClockPort {
        override fun monotonicMillis() = 0L
        override fun now(): Instant = Instant.EPOCH
    }

    private class FakeStore : LogStorePort {
        private var current: SessionRecord? = null
        override fun create(deviceId: DeviceId, at: Instant): SessionRecord =
            SessionRecord(SessionId("session"), JobId("job"), deviceId, at, 0, true)
                .also { current = it }
        override fun append(jobId: JobId, samples: List<TelemetrySample>) = Unit
        override fun close(jobId: JobId, at: Instant, reason: String): SessionRecord =
            requireNotNull(current).copy(active = false).also { current = null }
        override fun active() = current
    }
}
