package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
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
            footpadState = FootpadState.NONE,
        )
        service.ingest(sample(1, 0))
        service.ingest(sample(2, 100))

        assertEquals(listOf(1L), store.appended.map(TelemetrySample::sequence))
        assertEquals(listOf(1L), events.filterIsInstance<TelemetrySample>().map { it.sequence })
        assertEquals(1L, service.activeConnectionSnapshot()?.latestSample?.sequence)
    }

    @Test
    fun `location capture starts only for an authorized riding session and stops on idle`() {
        val store = FakeStore()
        val products = FakeProductStore().apply {
            settingsRecord = SettingsRecord(locationRecordingEnabled = true)
        }
        val provider = FakeLocationProvider(
            LocationProviderState(
                LocationProviderAuthorization.AUTHORIZED,
                LocationProviderBackgroundCapture.AVAILABLE,
            ),
        )
        val clock = FakeClock()
        val service = readyService(store, products, provider, clock)

        service.ingest(TelemetrySample(1, Instant.EPOCH, 0, RideState.RIDING))
        assertEquals(1, provider.startCount)
        clock.monotonicMs = 1_500
        provider.emit(LocationProviderObservation(Instant.EPOCH.plusSeconds(1), 1.0, 2.0, 5.0))
        assertEquals(1, store.locationObservations.size)
        assertEquals(1_500L, store.locationObservations.single().elapsedMs)

        service.ingest(TelemetrySample(2, Instant.EPOCH.plusSeconds(2), 2_000, RideState.IDLE))
        assertEquals(1, provider.stopCount)
        provider.emit(LocationProviderObservation(Instant.EPOCH.plusSeconds(2), 3.0, 4.0, null))
        assertEquals(1, store.locationObservations.size)
    }

    @Test
    fun `denied location permission prevents capture and foreground-only remains usable`() {
        val store = FakeStore()
        val products = FakeProductStore().apply {
            settingsRecord = SettingsRecord(locationRecordingEnabled = true)
        }
        val denied = FakeLocationProvider(
            LocationProviderState(
                LocationProviderAuthorization.DENIED,
                LocationProviderBackgroundCapture.UNAVAILABLE,
            ),
        )
        val service = readyService(store, products, denied, FakeClock())
        service.ingest(TelemetrySample(1, Instant.EPOCH, 0, RideState.RIDING))
        assertEquals(0, denied.startCount)

        val foregroundOnly = FakeLocationProvider(
            LocationProviderState(
                LocationProviderAuthorization.FOREGROUND_ONLY,
                LocationProviderBackgroundCapture.FOREGROUND_ONLY,
            ),
        )
        val foregroundService = readyService(store = FakeStore(), products = products, provider = foregroundOnly, clock = FakeClock())
        foregroundService.ingest(TelemetrySample(1, Instant.EPOCH, 0, RideState.RIDING))
        assertEquals(1, foregroundOnly.startCount)
    }

    @Test
    fun `location capture stops on disconnect and restarts with a recovered service`() {
        val store = FakeStore()
        val products = FakeProductStore().apply {
            settingsRecord = SettingsRecord(locationRecordingEnabled = true)
        }
        val clock = FakeClock()
        val firstProvider = FakeLocationProvider(
            LocationProviderState(
                LocationProviderAuthorization.AUTHORIZED,
                LocationProviderBackgroundCapture.AVAILABLE,
            ),
        )
        val first = readyService(store, products, firstProvider, clock)
        first.ingest(TelemetrySample(1, Instant.EPOCH, 0, RideState.RIDING))
        first.disconnect()
        assertEquals(1, firstProvider.stopCount)
        assertEquals(listOf("explicit_disconnect"), store.closeReasons)

        // A restarted service can reuse an active durable session and reacquire
        // the provider when the board becomes ready again.
        val recoveredStore = FakeStore()
        recoveredStore.seedActive(DeviceId("fixture"))
        val recoveredProvider = FakeLocationProvider(
            LocationProviderState(
                LocationProviderAuthorization.AUTHORIZED,
                LocationProviderBackgroundCapture.AVAILABLE,
            ),
        )
        val recovered = readyService(recoveredStore, products, recoveredProvider, clock)
        recovered.ingest(TelemetrySample(1, Instant.EPOCH, 0, RideState.RIDING))
        assertEquals(1, recoveredProvider.startCount)
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
        clock.instant = Instant.ofEpochSecond(2)
        val earlierId = DiscoveredDevice(DeviceId("another-board"), "Another VESC", -45, clock.now())
        service.onBluetoothEvent(
            BluetoothEvent.DeviceFound(
                earlierId.id,
                earlierId.advertisedName,
                earlierId.rssi,
            ),
        )
        assertEquals(listOf(earlierId, updated), service.knownDevices)
        service.stopDiscovery(scanId)
        assertEquals(null, service.activeScanId)
        assertEquals(1, bluetooth.stopCount)
        assertEquals(
            listOf("scan-1", null),
            events.filterIsInstance<DiscoverySnapshot>().map { it.activeScanId },
        )
    }

    @Test
    fun `discovered devices expire after two minutes of inactivity`() {
        val scheduler = FakeScheduler()
        val events = mutableListOf<Any>()
        val service = ApplicationService(
            FakeBluetooth(),
            FakeClock(),
            FakeStore(),
            scheduler = scheduler,
        ) { events += it }

        service.startDiscovery()
        service.onBluetoothEvent(
            BluetoothEvent.DeviceFound(DeviceId("board"), "Fixture", -40),
        )
        assertEquals(120_000L, scheduler.intervals.first())

        scheduler.fire(0)

        assertTrue(service.knownDevices.isEmpty())
        assertTrue(events.filterIsInstance<DiscoverySnapshot>().last().knownDevices.isEmpty())
    }

    @Test
    fun `ready connection clears discoveries but keeps active name`() {
        val service = ApplicationService(FakeBluetooth(), FakeClock(), FakeStore()) {}
        val id = DeviceId("board")

        service.startDiscovery()
        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Fixture", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))

        assertTrue(service.knownDevices.isEmpty())
        assertEquals("Fixture", service.activeConnectionSnapshot()?.advertisedName)
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
    fun `initial firmware timeout retries once without reconnecting`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val store = FakeStore()
        val events = mutableListOf<Any>()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            productStore = FakeProductStore(),
            protocol = TelemetryProtocol(),
            scheduler = scheduler,
        ) { events += it }
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Fixture", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        assertEquals(listOf(listOf(1)), bluetooth.writes.map { bytes -> bytes.map { it.toInt() } })

        scheduler.fireOneShots(2_000)
        assertEquals(listOf(id), bluetooth.connected)
        assertEquals(
            listOf(listOf(1), listOf(1)),
            bluetooth.writes.map { bytes -> bytes.map { it.toInt() } },
        )

        (1..3).forEach {
            service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
        }
        assertEquals(ConnectionState.READY, service.connectionState)
        assertEquals(1, store.createCount)
        val diagnostics = events.filterIsInstance<String>()
        assertTrue(
            diagnostics.contains(
                "connection_identification stage=firmware route=direct attempt=1 outcome=timed_out",
            ),
        )
        assertTrue(
            diagnostics.contains(
                "connection_identification stage=firmware route=direct attempt=2 outcome=response",
            ),
        )
    }

    @Test
    fun `initial firmware recovery is bounded and generation safe`() {
        val exhaustedBluetooth = FakeBluetooth()
        val exhaustedScheduler = FakeScheduler()
        val exhaustedStore = FakeStore()
        val exhausted = ApplicationService(
            exhaustedBluetooth,
            FakeClock(),
            exhaustedStore,
            protocol = TelemetryProtocol(),
            scheduler = exhaustedScheduler,
        ) {}
        val id = DeviceId("fixture")
        exhausted.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Fixture", -40))
        exhausted.connect(id)
        exhausted.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        exhaustedScheduler.fireOneShots(2_000)
        exhaustedScheduler.fireOneShots(8_000)
        exhaustedScheduler.fireOneShots(10_000)
        assertEquals(
            listOf(listOf(1), listOf(1)),
            exhaustedBluetooth.writes.map { bytes -> bytes.map { it.toInt() } },
        )
        assertEquals(ConnectionState.FAILED, exhausted.connectionState)
        assertEquals(
            ConnectionIssueCode.IDENTIFICATION_TIMEOUT,
            exhausted.activeConnectionSnapshot()?.connectionIssue?.code,
        )
        assertNull(exhaustedStore.active())
        (1..3).forEach {
            exhausted.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
        }
        assertEquals(ConnectionState.FAILED, exhausted.connectionState)
        assertEquals(
            listOf(listOf(1), listOf(1)),
            exhaustedBluetooth.writes.map { bytes -> bytes.map { it.toInt() } },
        )

        val staleBluetooth = FakeBluetooth()
        val staleScheduler = FakeScheduler()
        val stale = ApplicationService(
            staleBluetooth,
            FakeClock(),
            FakeStore(),
            protocol = TelemetryProtocol(),
            scheduler = staleScheduler,
        ) {}
        stale.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Fixture", -40))
        stale.connect(id)
        stale.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        stale.disconnect()
        stale.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.DISCONNECTED))
        staleScheduler.fireOneShots(2_000)
        assertEquals(listOf(listOf(1)), staleBluetooth.writes.map { bytes -> bytes.map { it.toInt() } })
        assertEquals(ConnectionState.DISCONNECTED, stale.connectionState)
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
        assertEquals("1.2.3", service.refloatPackageVersionSnapshot())
    }

    @Test
    fun `ambiguous controllers stay visible until one supported candidate is selected`() {
        val bluetooth = FakeBluetooth()
        val products = FakeProductStore()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            productStore = products,
            protocol = AmbiguousProtocol(),
            scheduler = FakeScheduler(),
        ) {}
        val id = DeviceId("fixture")
        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        (1..4).forEach {
            service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
        }

        val selecting = requireNotNull(service.activeConnectionSnapshot())
        assertEquals(ConnectionState.SELECTING_CONTROLLER, selecting.state)
        assertEquals(null, selecting.activeSession)
        assertEquals(listOf(7, 42), selecting.controllerCandidates.map { it.canId })
        assertEquals(listOf(false, true), selecting.controllerCandidates.map { it.vescSupported })
        assertThrows(IllegalStateException::class.java) {
            service.selectController(id, 7)
        }
        service.selectController(id, 42)
        assertTrue(products.profile.valuesJson.contains("\"preferredCanId\":42"))
        assertEquals(1, products.profile.revision)
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(5)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(6)))
        assertEquals(ConnectionState.READY, service.connectionState)
        assertEquals(ControllerRoute.Can(42), service.activeConnectionSnapshot()?.controllerRoute)
        assertNotNull(store.active())
    }

    @Test
    fun `configured CAN skips ping and learned UUID follows a moved controller`() {
        val configuredBluetooth = FakeBluetooth()
        val configuredProducts = FakeProductStore().apply {
            profile = profile.copy(valuesJson = """{"preferredCanId":42}""", missingFields = emptyList())
        }
        val configured = ApplicationService(
            configuredBluetooth,
            FakeClock(),
            FakeStore(),
            productStore = configuredProducts,
            protocol = AmbiguousProtocol(),
            scheduler = FakeScheduler(),
        ) {}
        val id = DeviceId("fixture")
        configured.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Gateway", -40))
        configured.connect(id)
        configured.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        configured.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        assertEquals(listOf(0, 42), configuredBluetooth.writes.last().map { it.toInt() and 0xff })
        assertFalse(configuredBluetooth.writes.any { it.contentEquals(byteArrayOf(62)) })
        (4..6).forEach {
            configured.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
        }
        assertEquals(ConnectionState.READY, configured.connectionState)
        assertEquals(
            ControllerRoute.Can(42),
            configured.activeConnectionSnapshot()?.controllerRoute,
        )

        val learnedBluetooth = FakeBluetooth()
        val learnedProducts = FakeProductStore().apply {
            routingHint = ControllerRoutingHint(
                "saved-board",
                7,
                ByteArray(12) { (it + 1).toByte() },
            )
        }
        val learned = ApplicationService(
            learnedBluetooth,
            FakeClock(),
            FakeStore(),
            productStore = learnedProducts,
            protocol = AmbiguousProtocol(),
            scheduler = FakeScheduler(),
        ) {}
        learned.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Gateway", -40))
        learned.connect(id)
        learned.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        listOf(1, 3, 2, 3, 4, 5, 6).forEach {
            learned.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
        }
        assertEquals(ConnectionState.READY, learned.connectionState)
        assertEquals(ControllerRoute.Can(42), learned.activeConnectionSnapshot()?.controllerRoute)
        assertEquals(42, learnedProducts.routingHint?.canId)
    }

    @Test
    fun `silent CAN nodes are skipped within the overall discovery deadline`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            productStore = FakeProductStore(),
            protocol = AmbiguousProtocol(),
            scheduler = scheduler,
        ) {}
        val id = DeviceId("fixture")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "ExpressT", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(2)))
        assertEquals(listOf(0, 7), bluetooth.writes.last().map { it.toInt() and 0xff })

        scheduler.fireOneShots(1_000)
        assertEquals(listOf(0, 42), bluetooth.writes.last().map { it.toInt() and 0xff })
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(4)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(5)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(6)))

        assertEquals(ConnectionState.READY, service.connectionState)
        assertEquals(ControllerRoute.Can(42), service.activeConnectionSnapshot()?.controllerRoute)
        assertNotNull(store.active())
    }

    @Test
    fun `valid transport is saved before direct session becomes ready`() {
        val bluetooth = FakeBluetooth()
        val products = FakeProductStore()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            productStore = products,
            protocol = TelemetryProtocol(),
            scheduler = FakeScheduler(),
        ) {}
        val id = DeviceId("direct")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Direct THOR", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        assertEquals(ConnectionState.IDENTIFYING, service.connectionState)
        assertEquals(null, service.activeConnectionSnapshot()?.savedDevice)
        assertEquals(null, store.active())

        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        val identified = requireNotNull(service.activeConnectionSnapshot())
        assertEquals(id.value, identified.savedDevice?.bluetoothIdentifier)
        assertEquals(ControllerRoute.Direct, identified.controllerRoute)
        assertEquals(null, store.active())

        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(2)))
        assertEquals(null, store.active())
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(3)))
        assertEquals(ConnectionState.READY, service.connectionState)
        assertEquals(1, store.createCount)
        assertNotNull(store.active())
    }

    @Test
    fun `private hint fast path skips ping when controller UUID matches`() {
        val bluetooth = FakeBluetooth()
        val products = FakeProductStore().apply {
            routingHint = ControllerRoutingHint(
                "saved-board",
                42,
                ByteArray(12) { (it + 1).toByte() },
            )
        }
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            FakeStore(),
            productStore = products,
            protocol = AmbiguousProtocol(),
            scheduler = FakeScheduler(),
        ) {}
        val id = DeviceId("gateway")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Generic gateway", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        assertEquals(listOf(0, 42), bluetooth.writes.last().map { it.toInt() and 0xff })
        assertFalse(bluetooth.writes.any { it.contentEquals(byteArrayOf(62)) })
        (4..6).forEach {
            service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
        }
        assertEquals(ConnectionState.READY, service.connectionState)
        assertEquals(ControllerRoute.Can(42), service.activeConnectionSnapshot()?.controllerRoute)
    }

    @Test
    fun `wrong preferred controller retry and empty discovery remain recoverable`() {
        val bluetooth = FakeBluetooth()
        val products = FakeProductStore().apply {
            profile = profile.copy(
                valuesJson = """{"preferredCanId":42}""",
                missingFields = emptyList(),
            )
        }
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            FakeStore(),
            productStore = products,
            protocol = DiscoveryBoundaryProtocol(),
            scheduler = FakeScheduler(),
        ) {}
        val id = DeviceId("gateway")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Generic gateway", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        assertEquals(listOf(0, 42), bluetooth.writes.last().map { it.toInt() and 0xff })
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(3)))
        assertEquals(ConnectionState.FAILED, service.connectionState)
        assertEquals(
            ConnectionIssueCode.PREFERRED_CONTROLLER_UNAVAILABLE,
            service.activeConnectionSnapshot()?.connectionIssue?.code,
        )

        products.profile = products.profile.copy(revision = 1, valuesJson = "{}")
        service.connect(id)
        assertEquals(ConnectionState.CONNECTING, service.connectionState)
        assertEquals(null, service.activeConnectionSnapshot()?.connectionIssue)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        assertTrue(bluetooth.writes.last().contentEquals(byteArrayOf(62)))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(2)))
        assertEquals(ConnectionState.FAILED, service.connectionState)
        assertEquals(
            ConnectionIssueCode.NO_CONTROLLER_FOUND,
            service.activeConnectionSnapshot()?.connectionIssue?.code,
        )

        service.disconnect()
        service.onBluetoothEvent(
            BluetoothEvent.ConnectionChanged(id, ConnectionState.DISCONNECTED),
        )
        assertEquals(null, service.activeConnectionSnapshot())
    }

    @Test
    fun `discovery bounds fail without publishing truncated candidates`() {
        val id = DeviceId("gateway")
        val excessiveIds = ApplicationService(
            FakeBluetooth(),
            FakeClock(),
            FakeStore(),
            productStore = FakeProductStore(),
            protocol = DiscoveryBoundaryProtocol((0..64).toList()),
            scheduler = FakeScheduler(),
        ) {}
        excessiveIds.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Gateway", -40))
        excessiveIds.connect(id)
        excessiveIds.onBluetoothEvent(
            BluetoothEvent.ConnectionChanged(id, ConnectionState.READY),
        )
        excessiveIds.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        excessiveIds.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(2)))
        assertEquals(ConnectionState.FAILED, excessiveIds.connectionState)
        assertEquals(
            ConnectionIssueCode.TOO_MANY_CAN_DEVICES,
            excessiveIds.activeConnectionSnapshot()?.connectionIssue?.code,
        )
        assertTrue(excessiveIds.activeConnectionSnapshot()?.controllerCandidates?.isEmpty() == true)

        val excessiveControllers = ApplicationService(
            FakeBluetooth(),
            FakeClock(),
            FakeStore(),
            productStore = FakeProductStore(),
            protocol = DiscoveryBoundaryProtocol((0..16).toList()),
            scheduler = FakeScheduler(),
        ) {}
        excessiveControllers.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Gateway", -40))
        excessiveControllers.connect(id)
        excessiveControllers.onBluetoothEvent(
            BluetoothEvent.ConnectionChanged(id, ConnectionState.READY),
        )
        excessiveControllers.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        excessiveControllers.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(2)))
        (10..26).forEach {
            excessiveControllers.onBluetoothEvent(
                BluetoothEvent.Notification(id, byteArrayOf(it.toByte())),
            )
        }
        assertEquals(ConnectionState.FAILED, excessiveControllers.connectionState)
        assertEquals(
            ConnectionIssueCode.TOO_MANY_CONTROLLERS,
            excessiveControllers.activeConnectionSnapshot()?.connectionIssue?.code,
        )
        assertTrue(
            excessiveControllers.activeConnectionSnapshot()
                ?.controllerCandidates?.isEmpty() == true,
        )
    }

    @Test
    fun `reconnect revalidates direct identity and resumes one session`() {
        val bluetooth = FakeBluetooth()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            productStore = FakeProductStore(),
            protocol = TelemetryProtocol(),
            scheduler = FakeScheduler(),
        ) {}
        val id = DeviceId("direct")
        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Direct THOR", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        (1..3).forEach {
            service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
        }
        assertEquals(1, store.createCount)

        service.onBluetoothEvent(
            BluetoothEvent.ConnectionChanged(id, ConnectionState.DISCONNECTED),
        )
        assertEquals(ConnectionState.RECONNECTING, service.connectionState)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        (1..3).forEach {
            service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
        }
        assertEquals(ConnectionState.READY, service.connectionState)
        assertEquals(ControllerRoute.Direct, service.activeConnectionSnapshot()?.controllerRoute)
        assertEquals(1, store.createCount)
        assertNotNull(store.active())
    }

    @Test
    fun `auxiliary CAN scan allows for the full firmware scan window`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            FakeStore(),
            protocol = TelemetryProtocol(bmsEnabled = true),
            scheduler = scheduler,
        ) {}
        val id = DeviceId("direct-with-vbms")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Thor301", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        (1..3).forEach {
            service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
        }
        val longTimeoutsBeforeScan = scheduler.intervals.count { it == 10_000L }
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(4)))

        assertTrue(bluetooth.writes.any { it.contentEquals(byteArrayOf(62)) })
        assertEquals(
            longTimeoutsBeforeScan + 1,
            scheduler.intervals.count { it == 10_000L },
        )
    }

    @Test
    fun `exact direct VBMS becomes BMS only without controller or session work`() {
        val bluetooth = FakeBluetooth()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            FakeClock(),
            store,
            productStore = FakeProductStore(),
            protocol = StandaloneBmsProtocol(),
            scheduler = FakeScheduler(),
        ) {}
        val id = DeviceId("numeric-vbms")

        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "42508004", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))

        val active = requireNotNull(service.activeConnectionSnapshot())
        assertEquals(ConnectionState.READY, service.connectionState)
        assertEquals(DeviceKind.BMS, active.deviceKind)
        assertEquals(null, active.controllerRoute)
        assertEquals(null, active.activeSession)
        assertEquals(BmsConnection.Direct, active.bms.connection)
        assertEquals(0, store.createCount)
        assertEquals(listOf(byteArrayOf(1), byteArrayOf(96)).map(ByteArray::toList), bluetooth.writes.map(ByteArray::toList))

        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.DISCONNECTED))
        assertEquals(ConnectionState.RECONNECTING, service.connectionState)
        assertEquals(null, service.activeConnectionSnapshot()?.deviceKind)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(1)))
        assertEquals(DeviceKind.BMS, service.activeConnectionSnapshot()?.deviceKind)
        assertEquals(0, store.createCount)
        assertEquals(0, bluetooth.writes.count { it.contentEquals(byteArrayOf(62)) })
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
        assertEquals(listOf(1_000L), scheduler.repeatingIntervals)
        assertEquals(null, store.active())
        val firstLayoutRequestCount =
            bluetooth.writes.count { it.contentEquals(byteArrayOf(32, 255.toByte())) }
        scheduler.fireOneShots(900)
        scheduler.fireRepeating(0)
        assertEquals(
            firstLayoutRequestCount + 1,
            bluetooth.writes.count { it.contentEquals(byteArrayOf(32, 255.toByte())) },
        )
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(3)))
        assertEquals(listOf(1_000L, 500L), scheduler.repeatingIntervals)
        assertNotNull(store.active())
        assertTrue(bluetooth.writes.any { it.contentEquals(byteArrayOf(31, 255.toByte())) })

        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(4)))

        assertEquals(listOf(1_000L, 500L, 100L), scheduler.repeatingIntervals)
        assertEquals(1, store.appended.size)
        assertEquals(RideState.RIDING, store.appended.single().rideState)
        assertEquals(PackageState.RUNNING, store.appended.single().packageState)
        assertEquals(64.0, store.appended.single().packVoltageV)
        assertEquals(store.appended.single(), service.activeConnectionSnapshot()?.latestSample)
        assertEquals(8.0, store.appended.single().motorCurrentA)
        assertEquals(-4.0, store.appended.single().batteryCurrentA)
        assertEquals(1024.0, store.appended.single().erpm)
        assertEquals(36.0, store.appended.single().speedKph)
        assertTrue(store.appended.single().extension.isEmpty())
    }

    @Test
    fun `controller shutdown expires or classifies delayed BLE loss as intentional`() {
        data class Ready(
            val service: ApplicationService,
            val bluetooth: FakeBluetooth,
            val scheduler: FakeScheduler,
            val id: DeviceId,
        )

        fun makeReady(): Ready {
            val bluetooth = FakeBluetooth()
            val scheduler = FakeScheduler()
            val service = ApplicationService(
                bluetooth, FakeClock(), FakeStore(), protocol = TelemetryProtocol(), scheduler = scheduler,
            ) {}
            val id = DeviceId("shutdown-fixture")
            service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "THOR300", -40))
            service.connect(id)
            service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
            listOf(1, 2, 3, 5).forEach {
                service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(it.toByte())))
            }
            assertEquals(true, service.activeConnectionSnapshot()?.controllerActions?.shutdownAvailable)
            return Ready(service, bluetooth, scheduler, id)
        }

        val noOp = makeReady()
        noOp.service.shutdownController()
        assertTrue(noOp.bluetooth.writes.any { it.contentEquals(byteArrayOf(156.toByte(), 0, 0)) })
        assertEquals(ControllerPendingAction.SHUTDOWN, noOp.service.activeConnectionSnapshot()?.controllerActions?.pendingAction)
        noOp.scheduler.fireOneShots(30_000)
        assertNull(noOp.service.activeConnectionSnapshot()?.controllerActions?.pendingAction)
        assertEquals(true, noOp.service.activeConnectionSnapshot()?.controllerActions?.shutdownAvailable)

        val lost = makeReady()
        lost.service.shutdownController()
        lost.service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(lost.id, ConnectionState.FAILED))
        assertNull(lost.service.activeConnectionSnapshot())
        assertEquals(ConnectionState.DISCONNECTED, lost.service.connectionState)
        assertEquals(listOf(lost.id), lost.bluetooth.connected)
    }

    @Test
    fun `refloat telemetry keeps riding probe rate through post ride tail`() {
        val bluetooth = FakeBluetooth()
        val scheduler = FakeScheduler()
        val clock = FakeClock()
        val store = FakeStore()
        val service = ApplicationService(
            bluetooth,
            clock,
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
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(3)))
        assertEquals(listOf(1_000L, 500L), scheduler.repeatingIntervals)

        clock.monotonicMs = 1_000
        clock.instant = Instant.ofEpochSecond(1)
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(4)))
        assertEquals(listOf(1_000L, 500L, 100L), scheduler.repeatingIntervals)
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(6)))
        scheduler.fireRepeating(2)

        clock.monotonicMs = 2_000
        clock.instant = Instant.ofEpochSecond(2)
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(5)))
        assertEquals(Instant.ofEpochSecond(1), store.appended.last().slowObservedAt)
        assertEquals(1.5, store.appended.last().fieldWeakeningCurrentA)
        assertEquals(42.0, store.appended.last().distanceM)
        clock.monotonicMs = 6_999
        clock.instant = Instant.ofEpochMilli(6_999)
        scheduler.fireRepeating(2)
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(6)))
        scheduler.fireRepeating(2)
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(5)))
        assertEquals(7.0, store.appended.last().distanceM)
        assertEquals(Instant.ofEpochMilli(6_999), store.appended.last().slowObservedAt)
        assertEquals(listOf(1_000L, 500L, 100L), scheduler.repeatingIntervals)

        clock.monotonicMs = 7_001
        scheduler.fireRepeating(2)
        service.onBluetoothEvent(BluetoothEvent.Notification(id, byteArrayOf(5)))
        assertEquals(listOf(1_000L, 500L, 100L, 500L), scheduler.repeatingIntervals)
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
        assertEquals(null, store.active())
        repeat(5) {
            scheduler.fireOneShots(900)
            scheduler.fireRepeating(0)
        }

        assertEquals(ConnectionState.FAILED, service.connectionState)
        assertEquals(emptyList<DeviceId>(), bluetooth.disconnected)
        assertEquals(null, store.active())
        assertEquals(
            ConnectionIssueCode.IDENTIFICATION_TIMEOUT,
            service.activeConnectionSnapshot()?.connectionIssue?.code,
        )
        assertEquals(
            5,
            bluetooth.writes.count { it.contentEquals(byteArrayOf(32, 255.toByte())) },
        )

        service.connect(id)
        assertEquals(ConnectionState.CONNECTING, service.connectionState)
        assertEquals(null, service.activeConnectionSnapshot()?.connectionIssue)
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
        override fun firmwareRequest(route: ControllerRoute) = byteArrayOf(1)
        override fun canPingRequest() = byteArrayOf(62)
        override fun refloatInfoRequest(route: ControllerRoute) = byteArrayOf(36, 101, 0)
        override fun refloatTelemetryIdsRequest(route: ControllerRoute) = byteArrayOf(36, 101, 32)
        override fun refloatTelemetryRequest(route: ControllerRoute) = byteArrayOf(36, 101, 31)
        override fun receive(bytes: ByteArray) = listOf(
            VescProtocolMessage.Firmware(FirmwareRecord(6, 6, "THOR300", "refloat")),
        )
    }

    private class CanDiscoveryProtocol : VescProtocolPort {
        override fun reset() = Unit
        override fun firmwareRequest(route: ControllerRoute) =
            route.canIdOrNull?.let { byteArrayOf(1, it.toByte()) } ?: byteArrayOf(1)
        override fun canPingRequest() = byteArrayOf(62)
        override fun refloatInfoRequest(route: ControllerRoute) =
            byteArrayOf(36, 101, 0, requireNotNull(route.canIdOrNull).toByte())
        override fun refloatTelemetryIdsRequest(route: ControllerRoute) =
            byteArrayOf(36, 101, 32, (route.canIdOrNull ?: 255).toByte())
        override fun refloatTelemetryRequest(route: ControllerRoute) =
            byteArrayOf(36, 101, 31, (route.canIdOrNull ?: 255).toByte())
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
                5 -> listOf(VescProtocolMessage.RefloatInfo("1.2.3"))
                else -> emptyList()
            }
    }

    private class TelemetryProtocol(
        private val bmsEnabled: Boolean = false,
    ) : VescProtocolPort {
        private var compactCount = 0
        override val supportsBmsTelemetry = bmsEnabled
        override fun reset() = Unit
        override fun firmwareRequest(route: ControllerRoute) = byteArrayOf(1)
        override fun canPingRequest() = byteArrayOf(62)
        override fun bmsValuesRequest(connection: BmsConnection) = when (connection) {
            BmsConnection.Direct -> byteArrayOf(96)
            is BmsConnection.Can -> byteArrayOf(34, connection.canId.toByte(), 96)
        }
        override fun refloatInfoRequest(route: ControllerRoute) =
            byteArrayOf(0, (route.canIdOrNull ?: 255).toByte())
        override fun refloatTelemetryIdsRequest(route: ControllerRoute) =
            byteArrayOf(32, (route.canIdOrNull ?: 255).toByte())
        override fun refloatTelemetryRequest(route: ControllerRoute) =
            byteArrayOf(31, (route.canIdOrNull ?: 255).toByte())
        override fun controllerShutdownRequest(route: ControllerRoute) = byteArrayOf(156.toByte(), 0, 0)
        override fun receive(bytes: ByteArray): List<VescProtocolMessage> =
            when (bytes.single().toInt()) {
                1 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(6, 6, "THOR300", null, 0, null),
                    ),
                )
                2 -> listOf(VescProtocolMessage.RefloatInfo("1.2.3"))
                3 -> listOf(
                    VescProtocolMessage.RefloatTelemetryLayout(
                        RefloatCompatibility.requiredFields,
                        RefloatCompatibility.requiredRuntimeFields,
                    ),
                )
                4 -> listOf(
                    VescProtocolMessage.RefloatTelemetry(
                        RefloatTelemetryRecord(
                            rideState = RideState.RIDING,
                            packageState = PackageState.RUNNING,
                            footpadState = FootpadState.BOTH,
                            packVoltageV = 64.0,
                            dutyCycle = 0.5,
                            controllerTempC = 30.0,
                            motorTempC = 25.0,
                            motorCurrentA = 8.0,
                            batteryCurrentA = -4.0,
                            erpm = 1024.0,
                            speedKph = 36.0,
                        ),
                    ),
                )
                5 -> listOf(
                    VescProtocolMessage.RefloatTelemetry(
                        RefloatTelemetryRecord(
                            rideState = RideState.IDLE,
                            packageState = PackageState.READY,
                            footpadState = FootpadState.NONE,
                            packVoltageV = 64.0,
                            dutyCycle = 0.0,
                            controllerTempC = 30.0,
                            motorTempC = 25.0,
                            motorCurrentA = 0.0,
                            batteryCurrentA = 0.0,
                            erpm = 0.0,
                            speedKph = 0.0,
                        ),
                    ),
                )
                6 -> {
                    compactCount += 1
                    listOf(
                        VescProtocolMessage.RefloatTelemetry(
                            RefloatTelemetryRecord(
                                rideState = RideState.IDLE,
                                kind = RefloatTelemetryKind.COMPACT,
                                fieldWeakeningCurrentA = 1.5,
                                distanceM = if (compactCount == 1) 42.0 else 7.0,
                            ),
                        ),
                    )
                }
                else -> emptyList()
            }
    }

    private class StandaloneBmsProtocol : VescProtocolPort {
        override val supportsBmsTelemetry = true
        override fun reset() = Unit
        override fun firmwareRequest(route: ControllerRoute) = byteArrayOf(1)
        override fun canPingRequest() = byteArrayOf(62)
        override fun bmsValuesRequest(connection: BmsConnection) = byteArrayOf(96)
        override fun refloatInfoRequest(route: ControllerRoute) = byteArrayOf(36, 101, 0)
        override fun refloatTelemetryIdsRequest(route: ControllerRoute) = byteArrayOf(36, 101, 32)
        override fun refloatTelemetryRequest(route: ControllerRoute) = byteArrayOf(36, 101, 31)
        override fun receive(bytes: ByteArray): List<VescProtocolMessage> =
            if (bytes.contentEquals(byteArrayOf(1))) {
                listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(6, 6, "VBMS32", null, hardwareType = 2),
                    ),
                )
            } else {
                emptyList()
            }
    }

    private class AmbiguousProtocol : VescProtocolPort {
        override fun reset() = Unit
        override fun firmwareRequest(route: ControllerRoute) =
            route.canIdOrNull?.let { byteArrayOf(0, it.toByte()) } ?: byteArrayOf(0)
        override fun canPingRequest() = byteArrayOf(62)
        override fun refloatInfoRequest(route: ControllerRoute) =
            byteArrayOf(2, (route.canIdOrNull ?: 255).toByte())
        override fun refloatTelemetryIdsRequest(route: ControllerRoute) =
            byteArrayOf(32, (route.canIdOrNull ?: 255).toByte())
        override fun refloatTelemetryRequest(route: ControllerRoute) =
            byteArrayOf(31, (route.canIdOrNull ?: 255).toByte())
        override fun receive(bytes: ByteArray): List<VescProtocolMessage> =
            when (bytes.single().toInt()) {
                1 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(6, 5, "Gateway", null, hardwareType = 2),
                    ),
                )
                2 -> listOf(VescProtocolMessage.CanDevices(listOf(42, 7)))
                3 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(5, 3, "Old VESC", null, 0, 7),
                    ),
                )
                4 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(
                            6,
                            6,
                            "THOR300",
                            "Refloat",
                            0,
                            42,
                            ByteArray(12) { (it + 1).toByte() },
                        ),
                    ),
                )
                5 -> listOf(VescProtocolMessage.RefloatInfo("1.2.3"))
                6 -> listOf(
                    VescProtocolMessage.RefloatTelemetryLayout(
                        RefloatCompatibility.requiredFields,
                        RefloatCompatibility.requiredRuntimeFields,
                    ),
                )
                else -> emptyList()
            }
    }

    private class DiscoveryBoundaryProtocol(
        private val canIds: List<Int> = emptyList(),
    ) : VescProtocolPort {
        override fun reset() = Unit
        override fun firmwareRequest(route: ControllerRoute) =
            route.canIdOrNull?.let { byteArrayOf(0, it.toByte()) } ?: byteArrayOf(0)
        override fun canPingRequest() = byteArrayOf(62)
        override fun refloatInfoRequest(route: ControllerRoute) =
            byteArrayOf(2, (route.canIdOrNull ?: 255).toByte())
        override fun refloatTelemetryIdsRequest(route: ControllerRoute) =
            byteArrayOf(32, (route.canIdOrNull ?: 255).toByte())
        override fun refloatTelemetryRequest(route: ControllerRoute) =
            byteArrayOf(31, (route.canIdOrNull ?: 255).toByte())
        override fun receive(bytes: ByteArray): List<VescProtocolMessage> =
            when (val marker = bytes.single().toInt() and 0xff) {
                1 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(
                            6,
                            5,
                            "Generic gateway",
                            null,
                            hardwareType = 2,
                        ),
                    ),
                )
                2 -> listOf(VescProtocolMessage.CanDevices(canIds))
                3 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(
                            6,
                            5,
                            "BMS",
                            null,
                            hardwareType = 2,
                            canId = 42,
                        ),
                    ),
                )
                in 10..26 -> listOf(
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(
                            6,
                            6,
                            "THOR300",
                            "Refloat",
                            hardwareType = 0,
                            canId = marker - 10,
                        ),
                    ),
                )
                else -> emptyList()
            }
    }

    private class FakeScheduler : SchedulerPort {
        val intervals = mutableListOf<Long>()
        private val actions = mutableListOf<() -> Unit>()
        val repeatingIntervals = mutableListOf<Long>()
        private val repeatingActions = mutableListOf<() -> Unit>()
        private val oneShotIntervals = mutableListOf<Long>()
        private val oneShotActions = mutableListOf<() -> Unit>()
        override fun schedule(delayMs: Long, action: () -> Unit): CancellationPort {
            var cancelled = false
            val guarded = { if (!cancelled) action() }
            intervals += delayMs
            actions += guarded
            oneShotIntervals += delayMs
            oneShotActions += guarded
            return CancellationPort { cancelled = true }
        }
        override fun scheduleRepeating(intervalMs: Long, action: () -> Unit): CancellationPort {
            var cancelled = false
            val guarded = { if (!cancelled) action() }
            intervals += intervalMs
            repeatingIntervals += intervalMs
            actions += guarded
            repeatingActions += guarded
            return CancellationPort { cancelled = true }
        }
        fun fire(index: Int) = actions[index]()
        fun fireRepeating(index: Int) = repeatingActions[index]()
        fun fireOneShots(delayMs: Long) = oneShotIntervals.indices
            .filter { oneShotIntervals[it] == delayMs }
            .forEach { oneShotActions[it]() }
    }

    private class FakeClock : ClockPort {
        var instant: Instant = Instant.EPOCH
        var monotonicMs = 0L
        override fun monotonicMillis() = monotonicMs
        override fun now(): Instant = instant
    }

    private class FakeStore : LogStorePort {
        private var current: SessionRecord? = null
        var createCount = 0
        val appended = mutableListOf<TelemetrySample>()
        val closeReasons = mutableListOf<String>()
        val locationObservations = mutableListOf<LocationObservation>()
        override fun create(deviceId: DeviceId, advertisedName: String?, at: Instant): SessionRecord =
            SessionRecord(SessionId("session"), JobId("job"), deviceId, at, 0, true)
                .also {
                    createCount++
                    current = it
                }
        override fun append(jobId: JobId, samples: List<TelemetrySample>) {
            appended += samples
        }
        override fun appendLocations(jobId: JobId, observations: List<LocationObservation>) {
            locationObservations += observations
        }
        fun seedActive(deviceId: DeviceId) {
            current = SessionRecord(SessionId("recovered-session"), JobId("recovered-job"), deviceId, Instant.EPOCH, 0, true)
        }
        override fun close(jobId: JobId, at: Instant, reason: String): SessionRecord {
            closeReasons += reason
            return requireNotNull(current).copy(active = false).also { current = null }
        }
        override fun active() = current
    }

    private class FakeProductStore : ProductStorePort {
        var routingHint: ControllerRoutingHint? = null
        var settingsRecord = SettingsRecord()
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

        override fun settings() = settingsRecord

        override fun ensureSavedDevice(
            deviceId: DeviceId,
            advertisedName: String?,
            at: Instant,
        ): SavedDeviceRecord = saved.copy(
            advertisedName = advertisedName,
            bluetoothIdentifier = deviceId.value,
            displayName = advertisedName ?: deviceId.value,
        ).also { saved = it }

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
                displayName = normalized ?: saved.advertisedName ?: saved.bluetoothIdentifier ?: "Board",
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

        override fun selectController(
            savedDeviceId: String,
            canId: Int,
            controllerUuid: ByteArray?,
            at: Instant,
        ): BoardProfileRecord {
            return profile.copy(
                revision = profile.revision + 1,
                valuesJson = """{"preferredCanId":$canId}""",
            ).also { profile = it }
        }

        override fun controllerRoutingHint(savedDeviceId: String): ControllerRoutingHint? {
            if (savedDeviceId != saved.savedDeviceId) {
                throw ProductStoreException.SavedDeviceNotFound
            }
            return routingHint
        }

        override fun storeControllerRoutingHint(hint: ControllerRoutingHint, at: Instant) {
            routingHint = hint
        }

        override fun clearControllerRoutingHint(savedDeviceId: String) {
            if (savedDeviceId != saved.savedDeviceId) {
                throw ProductStoreException.SavedDeviceNotFound
            }
            routingHint = null
        }
    }

    private class FakeLocationProvider(
        private var currentState: LocationProviderState,
    ) : LocationProviderPort {
        var startCount = 0
        var stopCount = 0
        private var handler: ((LocationProviderObservation) -> Unit)? = null

        override fun state() = currentState
        override fun requestPermission() = Unit
        override fun openSettings() = Unit
        override fun startUpdates(handler: (LocationProviderObservation) -> Unit) {
            startCount += 1
            this.handler = handler
        }
        override fun stopUpdates() {
            stopCount += 1
            handler = null
        }
        fun emit(observation: LocationProviderObservation) {
            handler?.invoke(observation)
        }
    }

    private fun readyService(
        store: FakeStore,
        products: FakeProductStore,
        provider: LocationProviderPort,
        clock: FakeClock,
    ): ApplicationService {
        val service = ApplicationService(
            FakeBluetooth(),
            clock,
            store,
            productStore = products,
            locationProvider = provider,
        ) {}
        val id = DeviceId("fixture")
        service.onBluetoothEvent(BluetoothEvent.DeviceFound(id, "Fixture", -40))
        service.connect(id)
        service.onBluetoothEvent(BluetoothEvent.ConnectionChanged(id, ConnectionState.READY))
        return service
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

private val ControllerRoute.canIdOrNull: Int?
    get() = (this as? ControllerRoute.Can)?.canId
