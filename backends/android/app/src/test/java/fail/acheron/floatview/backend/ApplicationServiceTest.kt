package fail.acheron.floatview.backend

import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ApplicationServiceTest {
    @Test
    fun `board profile updates remain local and publish the persisted revision`() {
        val bluetooth = FakeBluetooth()
        val products = FakeProductStore()
        val events = mutableListOf<Any>()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            FakeStore(),
            productStore = products,
        ) { events += it }

        val initial = service.boardProfile("saved-board")
        val updated = service.updateBoardProfile(
            "saved-board",
            initial.revision,
            """{"seriesCount":18}""",
        )

        assertEquals(1, updated.revision)
        assertEquals(updated, products.profile)
        assertEquals(updated, events.single())
        assertTrue(bluetooth.connected.isEmpty())
        assertTrue(bluetooth.writes.isEmpty())
    }

    @Test
    fun `saved device name update publishes resolved revision`() {
        val products = FakeProductStore()
        val events = mutableListOf<Any>()
        val service = ApplicationService(
            FakeBluetooth(),
            FakeClock(),
            FakeStore(),
            productStore = products,
        ) { events += it }

        val renamed = service.updateSavedDevice("saved-board", 0, "Trail board")
        assertEquals("Trail board", renamed.displayName)
        assertEquals(1, renamed.revision)
        assertEquals(renamed, events.single())
        val reset = service.updateSavedDevice("saved-board", 1, null)
        assertEquals("Advertised", reset.displayName)
    }

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
    fun `only persisted samples publish live state`() {
        val store = FakeStore()
        val events = mutableListOf<Any>()
        val service = ApplicationService(FakeBluetooth(), FakeClock(), store) { events += it }
        val id = DeviceId("fixture")
        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Fixture", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        events.clear()

        fun sample(sequence: Long, elapsedMs: Long) = TelemetrySample(
            sequence,
            Instant.EPOCH,
            elapsedMs,
            RideState.IDLE,
            packVoltageV = 60.0,
            dutyCycle = 0.0,
            controllerTempC = 20.0,
            motorTempC = 20.0,
            footpadState = 0,
        )
        service.ingest(sample(1, 0))
        service.ingest(sample(2, 100))

        assertEquals(listOf(1L), store.appended.map(TelemetrySample::sequence))
        assertEquals(listOf(1L), events.filterIsInstance<TelemetrySample>().map { it.sequence })
        assertEquals(1L, service.activeConnectionSnapshot()?.latestSample?.sequence)
    }

    @Test
    fun `discovery owns scan and deduplicates devices`() {
        val bluetooth = FakeBluetooth()
        val clock = FakeClock()
        val events = mutableListOf<Any>()
        val service = ApplicationService(bluetooth, clock, FakeStore()) { events += it }

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
        assertEquals(
            listOf("scan-1", null),
            events.filterIsInstance<DiscoverySnapshot>().map { it.activeScanId },
        )
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
        assertEquals(3, store.appended.single().boardState)
        assertEquals(64.0, store.appended.single().packVoltageV)
        assertEquals(store.appended.single(), service.activeConnectionSnapshot()?.latestSample)
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
    fun `refloat telemetry keeps riding probe rate through post ride tail`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val clock = FakeClock()
        val service = ApplicationService(
            bluetooth,
            clock,
            FakeStore(),
            protocol = TelemetryProtocol(),
            scheduler = scheduler,
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(2)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(3)))
        assertEquals(listOf(1_000L, 500L), scheduler.intervals)

        clock.monotonicMs = 1_000
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(4)))
        assertEquals(listOf(1_000L, 500L, 100L), scheduler.intervals)

        clock.monotonicMs = 2_000
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(5)))
        clock.monotonicMs = 16_999
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(5)))
        assertEquals(listOf(1_000L, 500L, 100L), scheduler.intervals)

        clock.monotonicMs = 17_001
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(5)))
        assertEquals(listOf(1_000L, 500L, 100L, 500L), scheduler.intervals)
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

    @Test
    fun `unexpected disconnect uses bounded backoff and preserves the session on recovery`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            scheduler = scheduler,
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        val active = store.active()
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.DISCONNECTED))

        assertEquals(ConnectionState.RECONNECTING, service.connectionState)
        assertEquals(listOf(1_000L), scheduler.intervals)
        scheduler.fire(0)
        assertEquals(listOf(id, id), bluetooth.connected)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        assertEquals(ConnectionState.READY, service.connectionState)
        assertEquals(active, store.active())

        service.disconnect()
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.DISCONNECTED))
        assertEquals(ConnectionState.DISCONNECTED, service.connectionState)
        assertEquals(listOf(1_000L), scheduler.intervals)
    }

    @Test
    fun `reconnect window expiry closes the durable session`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val clock = FakeClock()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            clock,
            store,
            scheduler = scheduler,
            samplingSettings = SamplingSettings(reconnectDurationMs = 2_500),
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.DISCONNECTED))
        clock.monotonicMs = 1_000
        scheduler.fire(0)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.FAILED))
        assertEquals(listOf(1_000L, 1_500L), scheduler.intervals)
        clock.monotonicMs = 2_500
        scheduler.fire(1)

        assertEquals(ConnectionState.DISCONNECTED, service.connectionState)
        assertEquals(null, service.activeDeviceId)
        assertEquals(null, store.active())
        assertEquals(listOf(id), bluetooth.disconnected)
    }

    @Test
    fun `radio off waits inside the reconnect window until powered on`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            FakeStore(),
            scheduler = scheduler,
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(
            BluetoothEvent.StateChanged(
                BluetoothState(
                    BluetoothRadioState.POWERED_OFF,
                    BluetoothPermissionState.GRANTED,
                    BluetoothPermissionState.GRANTED,
                ),
            ),
        )
        scheduler.fire(0)
        assertEquals(listOf(id), bluetooth.connected)
        assertEquals(listOf(1_000L, 1_000L), scheduler.intervals)

        service.onBluetoothEvent(
            BluetoothEvent.StateChanged(
                BluetoothState(
                    BluetoothRadioState.POWERED_ON,
                    BluetoothPermissionState.GRANTED,
                    BluetoothPermissionState.GRANTED,
                ),
            ),
        )
        scheduler.fire(1)
        assertEquals(listOf(id, id), bluetooth.connected)
        assertEquals(ConnectionState.RECONNECTING, service.connectionState)
    }

    @Test
    fun `denied connect permission terminates the active session without retrying`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            scheduler = scheduler,
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(
            BluetoothEvent.StateChanged(
                BluetoothState(
                    BluetoothRadioState.UNKNOWN,
                    BluetoothPermissionState.DENIED,
                    BluetoothPermissionState.DENIED,
                ),
            ),
        )

        assertEquals(ConnectionState.DISCONNECTED, service.connectionState)
        assertEquals(null, service.activeDeviceId)
        assertEquals(null, store.active())
        assertEquals(listOf("bluetooth_permission_denied"), store.closeReasons)
        assertEquals(listOf(id), bluetooth.disconnected)
        assertTrue(scheduler.intervals.isEmpty())
    }

    private class FakeBluetooth : BluetoothPort {
        var startCount = 0
        var stopCount = 0
        val connected = mutableListOf<DeviceId>()
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
        override fun connect(deviceId: DeviceId) { connected += deviceId }
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
                5 -> listOf(
                    VescProtocolMessage.RefloatTelemetry(
                        RefloatTelemetryRecord(
                            RideState.IDLE, 2, 0, 64.0, 0.0, 30.0, 25.0,
                            0.0, 0.0, 0, 0.0,
                        ),
                    ),
                )
                else -> emptyList()
            }
    }

    private class FakeScheduler : SchedulerPort {
        val intervals = mutableListOf<Long>()
        private val actions = mutableListOf<() -> Unit>()
        override fun schedule(delayMs: Long, action: () -> Unit): CancellationPort {
            intervals += delayMs
            actions += action
            return CancellationPort {}
        }
        override fun scheduleRepeating(intervalMs: Long, action: () -> Unit): CancellationPort {
            intervals += intervalMs
            actions += action
            return CancellationPort {}
        }
        fun fire(index: Int) = actions[index]()
    }

    private class FakeClock : ClockPort {
        var instant: Instant = Instant.EPOCH
        var monotonicMs = 0L
        override fun monotonicMillis() = monotonicMs
        override fun now(): Instant = instant
    }

    private class FakeStore : LogStorePort {
        private var current: SessionRecord? = null
        val appended = mutableListOf<TelemetrySample>()
        val closeReasons = mutableListOf<String>()
        override fun create(deviceId: DeviceId, advertisedName: String?, at: Instant): SessionRecord =
            SessionRecord(SessionId("session"), JobId("job"), deviceId, at, 0, true)
                .also { current = it }
        override fun append(jobId: JobId, samples: List<TelemetrySample>) {
            appended += samples
        }
        override fun close(jobId: JobId, at: Instant, reason: String): SessionRecord {
            closeReasons += reason
            return requireNotNull(current).copy(active = false).also { current = null }
        }
        override fun active() = current
    }

    private class FakeProductStore : ProductStorePort {
        var saved = SavedDeviceRecord(
            "saved-board",
            "history-board",
            0,
            "Advertised",
            advertisedName = "Advertised",
            bluetoothIdentifier = "identifier",
            profileComplete = false,
        )
        var profile = BoardProfileRecord(
            "saved-board",
            0,
            "{}",
            listOf("seriesCount", "parallelCount", "motor"),
        )

        override fun savedDevices() = listOf(saved)

        override fun updateSavedDevice(
            savedDeviceId: String,
            expectedRevision: Int,
            customDisplayName: String?,
            at: Instant,
        ): SavedDeviceRecord {
            if (savedDeviceId != saved.savedDeviceId) throw ProductStoreException.SavedDeviceNotFound
            if (expectedRevision != saved.revision) {
                throw ProductStoreException.SavedDeviceRevisionConflict(saved.revision)
            }
            val normalized = customDisplayName?.trim()?.takeIf(String::isNotEmpty)
            return saved.copy(
                revision = saved.revision + 1,
                displayName = normalized ?: saved.advertisedName ?: saved.bluetoothIdentifier,
                customDisplayName = normalized,
            ).also { saved = it }
        }

        override fun boardProfile(savedDeviceId: String): BoardProfileRecord {
            if (savedDeviceId != profile.savedDeviceId) {
                throw ProductStoreException.SavedDeviceNotFound
            }
            return profile
        }

        override fun updateBoardProfile(
            savedDeviceId: String,
            expectedRevision: Int,
            patchJson: String,
            at: Instant,
        ): BoardProfileRecord {
            if (expectedRevision != profile.revision) {
                throw ProductStoreException.ProfileRevisionConflict(profile.revision)
            }
            return profile.copy(
                revision = profile.revision + 1,
                valuesJson = patchJson,
                missingFields = listOf("parallelCount", "motor"),
            ).also { profile = it }
        }
    }

    private class FailingStore : LogStorePort {
        override fun create(deviceId: DeviceId, advertisedName: String?, at: Instant): SessionRecord =
            error("storage failed")
        override fun append(jobId: JobId, samples: List<TelemetrySample>) = Unit
        override fun close(jobId: JobId, at: Instant, reason: String): SessionRecord =
            error("storage failed")
        override fun active(): SessionRecord? = null
    }
}
