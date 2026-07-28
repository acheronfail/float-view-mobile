package fail.acheron.floatview.core

import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ApplicationServiceTest {
    @Test
    fun `ready creates log before service exposes stable state`() {
        val store = FakeStore()
        val service = ApplicationService(FakeBluetooth(), FakeClock(), store) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Fixture", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))

        assertEquals(ConnectionState.READY, service.connectionState)
        assertNotNull(store.active())
    }

    @Test
    fun `discovery owns scan and deduplicates devices`() {
        val bluetooth = FakeBluetooth()
        val clock = FakeClock()
        val service = ApplicationService(bluetooth, clock, FakeStore()) {}

        val scanId = service.startDiscovery()
        assertEquals("scan-1", scanId)
        assertEquals(scanId, service.startDiscovery())
        assertEquals(1, bluetooth.startCount)

        service.onBluetoothEvent(
            BluetoothEvent.DeviceFound(DeviceId("board"), null, -70),
        )
        clock.instant = Instant.ofEpochSecond(1)
        val updated = DiscoveredDevice(DeviceId("board"), "VESC", -55, clock.now())
        service.onBluetoothEvent(BluetoothEvent.DeviceFound(updated.id, updated.advertisedName, updated.rssi))

        assertEquals(listOf(updated), service.knownDevices)
        service.stopDiscovery(scanId)
        assertEquals(null, service.activeScanId)
        assertEquals(1, bluetooth.stopCount)
    }

    @Test
    fun `logging failure fails and disconnects connection`() {
        val bluetooth = FakeBluetooth()
        val service = ApplicationService(bluetooth, FakeClock(), FailingStore()) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Fixture", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))

        assertEquals(ConnectionState.FAILED, service.connectionState)
        assertTrue(bluetooth.disconnected.contains(id))
    }

    @Test
    fun `ready requests firmware and notification updates snapshot`() {
        val bluetooth = FakeBluetooth()
        val protocol = FakeProtocol()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            FakeStore(),
            protocol = protocol,
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Fixture", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(9)))

        assertTrue(bluetooth.writes.any { it.contentEquals(byteArrayOf(1)) })
        assertTrue(bluetooth.writes.any { it.contentEquals(byteArrayOf(36, 101, 0)) })
        assertEquals(FirmwareRecord(6, 6, "THOR300", "refloat"), service.firmwareSnapshot())
    }

    @Test
    fun `custom transport discovers primary controller then refloat package in order`() {
        val bluetooth = FakeBluetooth()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            FakeStore(),
            protocol = CanDiscoveryProtocol(),
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(2)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(3)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(4)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(5)))

        assertEquals(
            listOf(
                listOf(1),
                listOf(62),
                listOf(1, 7),
                listOf(1, 42),
                listOf(36, 101, 0, 42),
                listOf(36, 101, 32, 42),
            ),
            bluetooth.writes.map { bytes -> bytes.map { it.toInt() and 0xff } },
        )
        assertEquals(42, service.firmwareSnapshot()?.canId)
        assertEquals("THOR300", service.firmwareSnapshot()?.hardwareName)
        assertEquals("1.2", service.refloatPackageVersionSnapshot())
    }

    @Test
    fun `refloat telemetry starts idle probe and switches to riding rate`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            protocol = TelemetryProtocol(),
            scheduler = scheduler,
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(2)))
        assertEquals(listOf(1_000L), scheduler.intervals)
        val firstLayoutRequestCount =
            bluetooth.writes.count { it.contentEquals(byteArrayOf(32, 42)) }
        scheduler.fire(0)
        assertEquals(
            firstLayoutRequestCount + 1,
            bluetooth.writes.count { it.contentEquals(byteArrayOf(32, 42)) },
        )
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(3)))
        assertEquals(listOf(1_000L, 500L), scheduler.intervals)
        assertTrue(bluetooth.writes.any { it.contentEquals(byteArrayOf(31, 42)) })

        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(4)))

        assertEquals(listOf(1_000L, 500L, 100L), scheduler.intervals)
        assertEquals(1, store.appended.size)
        assertEquals(RideState.RIDING, store.appended.single().rideState)
        assertEquals(64.0, store.appended.single().packVoltageV)
        assertEquals(
            mapOf(
                "motorCurrentA" to 8.0,
                "batteryCurrentA" to -4.0,
                "erpm" to 1024.0,
                "speedKph" to 36.0,
            ),
            store.appended.single().extension,
        )
    }

    @Test
    fun `missing refloat layout retries finitely and fails the logging session`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            protocol = TelemetryProtocol(),
            scheduler = scheduler,
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(2)))
        repeat(5) { scheduler.fire(0) }

        assertEquals(ConnectionState.FAILED, service.connectionState)
        assertEquals(listOf(id), bluetooth.disconnected)
        assertEquals(null, store.active())
        assertEquals(
            5,
            bluetooth.writes.count { it.contentEquals(byteArrayOf(32, 42)) },
        )
    }

    private class FakeBluetooth : BluetoothPort {
        var startCount = 0
        var stopCount = 0
        val disconnected = mutableListOf<DeviceId>()
        val writes = mutableListOf<ByteArray>()
        override fun currentState() = BluetoothState(
            BluetoothRadioState.POWERED_ON,
            BluetoothPermissionState.GRANTED,
            BluetoothPermissionState.GRANTED,
        )
        override fun requestPermission() = Unit
        override fun startDiscovery() { startCount++ }
        override fun stopDiscovery() { stopCount++ }
        override fun connect(deviceId: DeviceId) = Unit
        override fun write(deviceId: DeviceId, bytes: ByteArray) { writes += bytes }
        override fun disconnect(deviceId: DeviceId) { disconnected += deviceId }
    }

    private class FakeProtocol : VescProtocolPort {
        override fun reset() = Unit
        override fun firmwareRequest(canId: Int?) = byteArrayOf(1)
        override fun canPingRequest() = byteArrayOf(62)
        override fun refloatInfoRequest(canId: Int?) = byteArrayOf(36, 101, 0)
        override fun refloatTelemetryIdsRequest(canId: Int) = byteArrayOf(36, 101, 32)
        override fun refloatTelemetryRequest(canId: Int) = byteArrayOf(36, 101, 31)
        override fun receive(bytes: ByteArray) = listOf(
            VescProtocolMessage.Firmware(FirmwareRecord(6, 6, "THOR300", "refloat")),
        )
    }

    private class CanDiscoveryProtocol : VescProtocolPort {
        override fun reset() = Unit
        override fun firmwareRequest(canId: Int?) =
            if (canId == null) byteArrayOf(1) else byteArrayOf(1, canId.toByte())
        override fun canPingRequest() = byteArrayOf(62)
        override fun refloatInfoRequest(canId: Int?) =
            byteArrayOf(36, 101, 0, requireNotNull(canId).toByte())
        override fun refloatTelemetryIdsRequest(canId: Int) =
            byteArrayOf(36, 101, 32, canId.toByte())
        override fun refloatTelemetryRequest(canId: Int) =
            byteArrayOf(36, 101, 31, canId.toByte())
        override fun receive(bytes: ByteArray): List<VescProtocolMessage> =
            when (bytes.single().toInt()) {
                1 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(6, 5, "VESC Express T", null, hardwareType = 2),
                    ),
                )
                2 -> listOf(VescProtocolMessage.CanDevices(listOf(7, 42)))
                3 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(6, 5, "VESC Express T", null, hardwareType = 2, canId = 7),
                    ),
                )
                4 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(6, 6, "THOR300", null, hardwareType = 0, canId = 42),
                    ),
                )
                5 -> listOf(VescProtocolMessage.RefloatInfo("1.2"))
                else -> emptyList()
            }
    }

    private class TelemetryProtocol : VescProtocolPort {
        override fun reset() = Unit
        override fun firmwareRequest(canId: Int?) = byteArrayOf(1)
        override fun canPingRequest() = byteArrayOf(62)
        override fun refloatInfoRequest(canId: Int?) = byteArrayOf(0, requireNotNull(canId).toByte())
        override fun refloatTelemetryIdsRequest(canId: Int) = byteArrayOf(32, canId.toByte())
        override fun refloatTelemetryRequest(canId: Int) = byteArrayOf(31, canId.toByte())
        override fun receive(bytes: ByteArray): List<VescProtocolMessage> =
            when (bytes.single().toInt()) {
                1 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(6, 6, "THOR300", null, 0, 42),
                    ),
                )
                2 -> listOf(VescProtocolMessage.RefloatInfo("1.2.0"))
                3 -> listOf(
                    VescProtocolMessage.RefloatTelemetryLayout(
                        listOf("motor.batt_voltage"),
                        emptyList(),
                    ),
                )
                4 -> listOf(
                    VescProtocolMessage.RefloatTelemetry(
                        RefloatTelemetryRecord(
                            RideState.RIDING, 3, 3, 64.0, 0.5, 30.0, 25.0,
                            8.0, -4.0, 1024, 36.0,
                        ),
                    ),
                )
                else -> emptyList()
            }
    }

    private class FakeScheduler : SchedulerPort {
        val intervals = mutableListOf<Long>()
        private val actions = mutableListOf<() -> Unit>()
        override fun scheduleRepeating(intervalMs: Long, action: () -> Unit): CancellationPort {
            intervals += intervalMs
            actions += action
            return CancellationPort {}
        }
        fun fire(index: Int) = actions[index]()
    }

    private class FakeClock : ClockPort {
        var instant: Instant = Instant.EPOCH
        override fun monotonicMillis() = 0L
        override fun now(): Instant = instant
    }

    private class FakeStore : LogStorePort {
        private var current: SessionRecord? = null
        val appended = mutableListOf<TelemetrySample>()
        override fun create(deviceId: DeviceId, at: Instant): SessionRecord =
            SessionRecord(SessionId("session"), JobId("job"), deviceId, at, 0, true)
                .also { current = it }
        override fun append(jobId: JobId, samples: List<TelemetrySample>) {
            appended += samples
        }
        override fun close(jobId: JobId, at: Instant, reason: String): SessionRecord =
            requireNotNull(current).copy(active = false).also { current = null }
        override fun active() = current
    }

    private class FailingStore : LogStorePort {
        override fun create(deviceId: DeviceId, at: Instant): SessionRecord =
            error("storage failed")
        override fun append(jobId: JobId, samples: List<TelemetrySample>) = Unit
        override fun close(jobId: JobId, at: Instant, reason: String): SessionRecord =
            error("storage failed")
        override fun active(): SessionRecord? = null
    }
}
