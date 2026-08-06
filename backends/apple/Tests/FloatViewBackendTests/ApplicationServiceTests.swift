@testable import FloatViewBackend
import Foundation
import Testing

@Test
func readyCreatesDurableSession() throws {
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: FakeBluetooth(),
    clock: FixedClock(),
    logStore: store,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "Fixture", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))

  #expect(service.connectionState == .ready)
  #expect(store.active() != nil)
}

@Test
func onlyPersistedSamplesPublishLiveState() throws {
  let store = MemoryStore()
  let events = CollectingEvents()
  let service = ApplicationService(
    bluetooth: FakeBluetooth(),
    clock: FixedClock(),
    logStore: store,
    events: events
  )
  let id = DeviceID("fixture")
  try service.receive(.deviceFound(id, name: "Fixture", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  events.values.removeAll()

  try service.ingest(
    TelemetrySample(
      sequence: 1, observedAt: .init(timeIntervalSince1970: 0),
      elapsedMs: 0, rideState: .idle
    )
  )
  try service.ingest(
    TelemetrySample(
      sequence: 2, observedAt: .init(timeIntervalSince1970: 0.1),
      elapsedMs: 100, rideState: .idle
    )
  )

  #expect(store.appended.map(\.sequence) == [1])
  #expect(events.values.compactMap { $0 as? TelemetrySample }.map(\.sequence) == [1])
  #expect(service.activeConnectionSnapshot()?.latestSample?.sequence == 1)
}

@Test
func locationCaptureStartsForAuthorizedRidingAndStopsOnIdle() throws {
  let store = MemoryStore()
  let products = MemoryProductStore()
  products.settingsRecord = SettingsRecord(locationRecordingEnabled: true)
  let provider = FakeLocationProvider(
    state: LocationProviderState(authorization: .authorized, backgroundCapture: .available)
  )
  let clock = MutableClock()
  let service = readyLocationService(store: store, products: products, provider: provider, clock: clock)

  try service.ingest(TelemetrySample(
    sequence: 1,
    observedAt: .init(timeIntervalSince1970: 0),
    elapsedMs: 0,
    rideState: .riding
  ))
  #expect(provider.startCount == 1)
  clock.date = .init(timeIntervalSince1970: 1.5)
  provider.emit(.init(observedAt: clock.date, latitude: 1, longitude: 2, accuracyM: 5))
  #expect(store.locationObservations.count == 1)
  #expect(store.locationObservations[0].elapsedMs == 1_500)

  try service.ingest(TelemetrySample(sequence: 2, observedAt: clock.date, elapsedMs: 2_000, rideState: .idle))
  #expect(provider.stopCount == 1)
  provider.emit(.init(observedAt: clock.date, latitude: 3, longitude: 4, accuracyM: nil))
  #expect(store.locationObservations.count == 1)
}

@Test
func deniedLocationPermissionDoesNotStartCapture() throws {
  let store = MemoryStore()
  let products = MemoryProductStore()
  products.settingsRecord = SettingsRecord(locationRecordingEnabled: true)
  let provider = FakeLocationProvider(
    state: LocationProviderState(authorization: .denied, backgroundCapture: .unavailable)
  )
  let service = readyLocationService(store: store, products: products, provider: provider, clock: MutableClock())

  try service.ingest(TelemetrySample(
    sequence: 1,
    observedAt: .init(timeIntervalSince1970: 0),
    elapsedMs: 0,
    rideState: .riding
  ))
  #expect(provider.startCount == 0)
}

@Test
func locationProviderStatusTransitionStopsCaptureAndDisconnectClosesTheSession() throws {
  let store = MemoryStore()
  let products = MemoryProductStore()
  products.settingsRecord = SettingsRecord(locationRecordingEnabled: true)
  let provider = FakeLocationProvider(
    state: LocationProviderState(authorization: .authorized, backgroundCapture: .available)
  )
  let service = readyLocationService(store: store, products: products, provider: provider, clock: MutableClock())
  try service.ingest(TelemetrySample(
    sequence: 1,
    observedAt: .init(timeIntervalSince1970: 0),
    elapsedMs: 0,
    rideState: .riding
  ))
  #expect(provider.startCount == 1)
  provider.currentState = LocationProviderState(authorization: .denied, backgroundCapture: .unavailable)
  try service.ingest(TelemetrySample(
    sequence: 2,
    observedAt: .init(timeIntervalSince1970: 1),
    elapsedMs: 1_000,
    rideState: .riding
  ))
  #expect(provider.stopCount == 1)
  try service.disconnect()
  #expect(store.closeReasons == ["explicit_disconnect"])
}

@Test
func boardProfileUpdatesRemainLocalAndPublishPersistedRevision() throws {
  let bluetooth = FakeBluetooth()
  let products = MemoryProductStore()
  let events = CollectingEvents()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    productStore: products,
    events: events
  )

  let initial = try service.boardProfile(savedDeviceID: "saved-board")
  let updated = try service.updateBoardProfile(
    savedDeviceID: "saved-board",
    expectedRevision: initial.revision,
    patchJSON: #"{"seriesCount":18}"#
  )

  #expect(updated.revision == 1)
  #expect(products.profile == updated)
  #expect(events.values.count == 1)
  #expect(events.values.first as? BoardProfileRecord == updated)
  #expect(bluetooth.connected.isEmpty)
  #expect(bluetooth.writes.isEmpty)
}

@Test
func savedDeviceNameUpdatePublishesResolvedRevision() throws {
  let products = MemoryProductStore()
  let events = CollectingEvents()
  let service = ApplicationService(
    bluetooth: FakeBluetooth(),
    clock: FixedClock(),
    logStore: MemoryStore(),
    productStore: products,
    events: events
  )

  let renamed = try service.updateSavedDevice(
    savedDeviceID: "saved-board",
    expectedRevision: 0,
    customDisplayName: "Trail board"
  )
  #expect(renamed.displayName == "Trail board")
  #expect(renamed.revision == 1)
  #expect(events.values.first as? SavedDeviceRecord == renamed)

  let reset = try service.updateSavedDevice(
    savedDeviceID: "saved-board",
    expectedRevision: 1,
    customDisplayName: nil
  )
  #expect(reset.displayName == "Advertised")
}

@Test
func discoveryOwnsScanAndDeduplicatesDevices() throws {
  let bluetooth = FakeBluetooth()
  let clock = MutableClock()
  let events = CollectingEvents()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: clock,
    logStore: MemoryStore(),
    events: events
  )

  let scanID = try service.startDiscovery()
  #expect(scanID == "scan-1")
  #expect(try service.startDiscovery() == scanID)
  #expect(bluetooth.startCount == 1)

  try service.receive(.deviceFound(DeviceID("board"), name: nil, rssi: -70))
  clock.date = .init(timeIntervalSince1970: 1)
  try service.receive(.deviceFound(DeviceID("board"), name: "VESC", rssi: -55))

  #expect(service.knownDevices == [
    KnownDevice(
      deviceID: DeviceID("board"),
      name: "VESC",
      rssi: -55,
      lastSeenAt: .init(timeIntervalSince1970: 1)
    ),
  ])

  #expect(throws: ApplicationError.invalidScan) {
    try service.stopDiscovery(scanID: "scan-other")
  }
  try service.stopDiscovery(scanID: scanID)
  #expect(service.activeScanID == nil)
  #expect(bluetooth.stopCount == 1)
  #expect(events.values.compactMap { ($0 as? DiscoverySnapshot)?.activeScanID } == ["scan-1"])
  #expect((events.values.last as? DiscoverySnapshot)?.activeScanID == nil)
}

@Test
func discoveryUpdatesAreCoalescedAndDeviceOrderIsStable() throws {
  let bluetooth = FakeBluetooth()
  let clock = MutableClock()
  let scheduler = FakeScheduler()
  let events = CollectingEvents()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: clock,
    logStore: MemoryStore(),
    scheduler: scheduler,
    events: events
  )

  _ = try service.startDiscovery()
  events.values.removeAll()

  try service.receive(.deviceFound(DeviceID("board-b"), name: "Second", rssi: -70))
  clock.date = .init(timeIntervalSince1970: 1)
  try service.receive(.deviceFound(DeviceID("board-a"), name: "First", rssi: -60))
  clock.date = .init(timeIntervalSince1970: 2)
  try service.receive(.deviceFound(DeviceID("board-b"), name: "Second", rssi: -50))

  #expect(scheduler.intervals == [120_000, 250, 120_000, 120_000])
  #expect(events.values.isEmpty)
  #expect(service.knownDevices.map(\.deviceID) == [DeviceID("board-a"), DeviceID("board-b")])

  scheduler.fire(1)

  let snapshots = events.values.compactMap { $0 as? DiscoverySnapshot }
  #expect(snapshots.count == 1)
  #expect(snapshots[0].knownDevices.map(\.deviceID) == [
    DeviceID("board-a"), DeviceID("board-b"),
  ])
  #expect(snapshots[0].knownDevices[1].rssi == -50)
}

@Test
func discoveredDevicesExpireAfterTwoMinutesOfInactivity() throws {
  let scheduler = FakeScheduler()
  let events = CollectingEvents()
  let service = ApplicationService(
    bluetooth: FakeBluetooth(),
    clock: FixedClock(),
    logStore: MemoryStore(),
    scheduler: scheduler,
    events: events
  )

  _ = try service.startDiscovery()
  try service.receive(.deviceFound(DeviceID("board"), name: "Fixture", rssi: -40))
  #expect(scheduler.intervals.first == 120_000)

  scheduler.fire(0)

  #expect(service.knownDevices.isEmpty)
  #expect((events.values.last as? DiscoverySnapshot)?.knownDevices.isEmpty == true)
}

@Test
func readyConnectionClearsDiscoveriesButKeepsActiveName() throws {
  let service = ApplicationService(
    bluetooth: FakeBluetooth(),
    clock: FixedClock(),
    logStore: MemoryStore(),
    events: IgnoreEvents()
  )
  let id = DeviceID("board")

  _ = try service.startDiscovery()
  try service.receive(.deviceFound(id, name: "Fixture", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))

  #expect(service.knownDevices.isEmpty)
  #expect(service.activeConnectionSnapshot()?.name == "Fixture")
}

@Test
func discoveryRequiresAvailableAuthorizedBluetooth() {
  let bluetooth = FakeBluetooth(
    state: .init(
      radio: .poweredOff,
      scanPermission: .granted,
      connectPermission: .granted
    )
  )
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    events: IgnoreEvents()
  )

  #expect(throws: ApplicationError.bluetoothUnavailable) {
    try service.startDiscovery()
  }
}

@Test
func deniedDiscoveryPermissionTakesPrecedenceOverUnknownRadioState() {
  let bluetooth = FakeBluetooth(
    state: .init(
      radio: .unknown,
      scanPermission: .denied,
      connectPermission: .denied
    )
  )
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    events: IgnoreEvents()
  )

  #expect(throws: ApplicationError.permissionRequired) {
    try service.startDiscovery()
  }
}

@Test
func loggingFailureFailsAndDisconnectsConnection() throws {
  let bluetooth = FakeBluetooth()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: FailingStore(),
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "Fixture", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))

  #expect(service.connectionState == .failed)
  #expect(bluetooth.disconnected == [id])
}

@Test
func readyRequestsFirmwareAndNotificationUpdatesSnapshot() throws {
  let bluetooth = FakeBluetooth()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    protocolCodec: FakeProtocol(),
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "Fixture", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([9])))

  #expect(bluetooth.writes.contains(Data([1])))
  #expect(bluetooth.writes.contains(Data([36, 101, 0])))
  #expect(
    service.firmwareSnapshot() ==
      FirmwareRecord(major: 6, minor: 6, hardwareName: "THOR300", firmwareName: "refloat")
  )
}

@Test
func initialFirmwareTimeoutRetriesOnceWithoutReconnecting() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let store = MemoryStore()
  let events = CollectingEvents()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    productStore: MemoryProductStore(),
    protocolCodec: TelemetryProtocol(),
    scheduler: scheduler,
    events: events
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "Fixture", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  #expect(bluetooth.writes == [Data([1])])

  scheduler.fireOneShots(afterMilliseconds: 2_000)
  #expect(bluetooth.connected == [id])
  #expect(bluetooth.writes == [Data([1]), Data([1])])

  for byte in UInt8(1) ... 3 {
    try service.receive(.notification(id, Data([byte])))
  }
  #expect(service.connectionState == .ready)
  #expect(store.createCount == 1)
  let diagnostics = events.values.compactMap { $0 as? String }
  #expect(diagnostics.contains {
    $0 == "connection_identification stage=firmware route=direct attempt=1 outcome=timed_out"
  })
  #expect(diagnostics.contains {
    $0 == "connection_identification stage=firmware route=direct attempt=2 outcome=response"
  })
}

@Test
func initialFirmwareRecoveryIsBoundedAndGenerationSafe() throws {
  let exhaustedBluetooth = FakeBluetooth()
  let exhaustedScheduler = FakeScheduler()
  let exhaustedStore = MemoryStore()
  let exhausted = ApplicationService(
    bluetooth: exhaustedBluetooth,
    clock: FixedClock(),
    logStore: exhaustedStore,
    protocolCodec: TelemetryProtocol(),
    scheduler: exhaustedScheduler,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")
  try exhausted.receive(.deviceFound(id, name: "Fixture", rssi: -40))
  try exhausted.connect(id)
  try exhausted.receive(.connectionChanged(id, .ready))
  exhaustedScheduler.fireOneShots(afterMilliseconds: 2_000)
  exhaustedScheduler.fireOneShots(afterMilliseconds: 8_000)
  exhaustedScheduler.fireOneShots(afterMilliseconds: 10_000)
  #expect(exhaustedBluetooth.writes == [Data([1]), Data([1])])
  #expect(exhausted.connectionState == .failed)
  #expect(exhausted.activeConnectionSnapshot()?.connectionIssue?.code == .identificationTimeout)
  #expect(exhaustedStore.active() == nil)
  for byte in UInt8(1) ... 3 {
    try exhausted.receive(.notification(id, Data([byte])))
  }
  #expect(exhausted.connectionState == .failed)
  #expect(exhaustedBluetooth.writes == [Data([1]), Data([1])])

  let staleBluetooth = FakeBluetooth()
  let staleScheduler = FakeScheduler()
  let stale = ApplicationService(
    bluetooth: staleBluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    protocolCodec: TelemetryProtocol(),
    scheduler: staleScheduler,
    events: IgnoreEvents()
  )
  try stale.receive(.deviceFound(id, name: "Fixture", rssi: -40))
  try stale.connect(id)
  try stale.receive(.connectionChanged(id, .ready))
  try stale.disconnect()
  try stale.receive(.connectionChanged(id, .disconnected))
  staleScheduler.fireOneShots(afterMilliseconds: 2_000)
  #expect(staleBluetooth.writes == [Data([1])])
  #expect(stale.connectionState == .disconnected)
}

@Test
func controllerShutdownExpiresOrClassifiesDelayedBLELossAsIntentional() throws {
  func makeReady() throws -> (ApplicationService, FakeBluetooth, FakeScheduler, DeviceID) {
    let bluetooth = FakeBluetooth()
    let scheduler = FakeScheduler()
    let service = ApplicationService(
      bluetooth: bluetooth, clock: FixedClock(), logStore: MemoryStore(),
      protocolCodec: TelemetryProtocol(), scheduler: scheduler, events: IgnoreEvents()
    )
    let id = DeviceID("shutdown-fixture")
    try service.receive(.deviceFound(id, name: "THOR300", rssi: -40))
    try service.connect(id)
    try service.receive(.connectionChanged(id, .ready))
    for marker in [UInt8(1), 2, 3, 5] {
      try service.receive(.notification(id, Data([marker])))
    }
    #expect(service.activeConnectionSnapshot()?.controllerActions.shutdownAvailable == true)
    return (service, bluetooth, scheduler, id)
  }

  let (noOp, noOpBluetooth, noOpScheduler, _) = try makeReady()
  try noOp.shutdownController()
  #expect(noOpBluetooth.writes.contains(Data([156, 0, 0])))
  #expect(noOp.activeConnectionSnapshot()?.controllerActions.pendingAction == .shutdown)
  #expect(noOpScheduler.oneShotIntervals.contains(30_000))
  noOpScheduler.fireOneShots(afterMilliseconds: 30_000)
  #expect(noOp.activeConnectionSnapshot()?.controllerActions.pendingAction == nil)
  #expect(noOp.activeConnectionSnapshot()?.controllerActions.shutdownAvailable == true)

  let (lost, lostBluetooth, _, id) = try makeReady()
  try lost.shutdownController()
  try lost.receive(.connectionChanged(id, .failed))
  #expect(lost.activeConnectionSnapshot() == nil)
  #expect(lost.connectionState == .disconnected)
  #expect(lostBluetooth.connected == [id])
}

@Test
func customTransportDiscoversPrimaryThenRefloatPackageInOrder() throws {
  let bluetooth = FakeBluetooth()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    protocolCodec: CanDiscoveryProtocol(),
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  for byte in UInt8(1) ... 5 {
    try service.receive(.notification(id, Data([byte])))
  }

  #expect(
    bluetooth.writes == [
      Data([1]),
      Data([62]),
      Data([1, 7]),
      Data([1, 42]),
      Data([36, 101, 0, 42]),
      Data([36, 101, 32, 42]),
    ]
  )
  #expect(service.firmwareSnapshot()?.canID == 42)
  #expect(service.firmwareSnapshot()?.hardwareName == "THOR300")
  #expect(service.refloatPackageVersionSnapshot() == "1.2.3")
}

@Test
func ambiguousControllersStayVisibleUntilOneSupportedCandidateIsSelected() throws {
  let bluetooth = FakeBluetooth()
  let products = MemoryProductStore()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    productStore: products,
    protocolCodec: AmbiguousProtocol(),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")
  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  for byte in UInt8(1) ... 4 {
    try service.receive(.notification(id, Data([byte])))
  }

  let selecting = try #require(service.activeConnectionSnapshot())
  #expect(selecting.state == .selectingController)
  #expect(selecting.activeSession == nil)
  #expect(selecting.controllerCandidates.map(\.canID) == [7, 42])
  #expect(selecting.controllerCandidates.map(\.vescSupported) == [false, true])
  #expect(throws: ApplicationError.invalidControllerSelection) {
    try service.selectController(deviceID: id, canID: 7)
  }
  try service.selectController(deviceID: id, canID: 42)
  #expect(products.profile.valuesJSON.contains(#""preferredCanId":42"#))
  #expect(products.profile.revision == 1)
  try service.receive(.notification(id, Data([5])))
  try service.receive(.notification(id, Data([6])))
  #expect(service.connectionState == .ready)
  #expect(service.activeConnectionSnapshot()?.controllerRoute == .can(42))
  #expect(store.active() != nil)
}

@Test
func configuredCanConnectsWithoutPingAndLearnedUUIDFollowsAMovedController() throws {
  let configuredBluetooth = FakeBluetooth()
  let configuredProducts = MemoryProductStore()
  configuredProducts.profile = BoardProfileRecord(
    savedDeviceID: "saved-board",
    revision: 0,
    valuesJSON: #"{"preferredCanId":42}"#,
    missingFields: []
  )
  let configured = ApplicationService(
    bluetooth: configuredBluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    productStore: configuredProducts,
    protocolCodec: AmbiguousProtocol(),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")
  try configured.receive(.deviceFound(id, name: "Gateway", rssi: -40))
  try configured.connect(id)
  try configured.receive(.connectionChanged(id, .ready))
  try configured.receive(.notification(id, Data([1])))
  #expect(configuredBluetooth.writes.last == Data([0, 42]))
  #expect(!configuredBluetooth.writes.contains(Data([62])))
  for byte in UInt8(4) ... 6 {
    try configured.receive(.notification(id, Data([byte])))
  }
  #expect(configured.connectionState == .ready)
  #expect(configured.activeConnectionSnapshot()?.controllerRoute == .can(42))

  let learnedBluetooth = FakeBluetooth()
  let learnedProducts = MemoryProductStore()
  learnedProducts.routingHint = ControllerRoutingHint(
    savedDeviceID: "saved-board",
    canID: 7,
    controllerUUID: Data((1 ... 12).map(UInt8.init))
  )
  let learned = ApplicationService(
    bluetooth: learnedBluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    productStore: learnedProducts,
    protocolCodec: AmbiguousProtocol(),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  try learned.receive(.deviceFound(id, name: "Gateway", rssi: -40))
  try learned.connect(id)
  try learned.receive(.connectionChanged(id, .ready))
  for byte in [UInt8(1), 3, 2, 3, 4, 5, 6] {
    try learned.receive(.notification(id, Data([byte])))
  }
  #expect(learned.connectionState == .ready)
  #expect(learned.activeConnectionSnapshot()?.controllerRoute == .can(42))
  #expect(learnedProducts.routingHint?.canID == 42)
}

@Test
func silentCanNodesAreSkippedWithinTheOverallDiscoveryDeadline() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    productStore: MemoryProductStore(),
    protocolCodec: AmbiguousProtocol(),
    scheduler: scheduler,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([1])))
  try service.receive(.notification(id, Data([2])))
  #expect(bluetooth.writes.last == Data([0, 7]))

  scheduler.fireOneShots(afterMilliseconds: 1_000)
  #expect(bluetooth.writes.last == Data([0, 42]))
  try service.receive(.notification(id, Data([4])))
  try service.receive(.notification(id, Data([5])))
  try service.receive(.notification(id, Data([6])))

  #expect(service.connectionState == .ready)
  #expect(service.activeConnectionSnapshot()?.controllerRoute == .can(42))
  #expect(store.active() != nil)
}

@Test
func validTransportIsSavedBeforeDirectSessionBecomesReady() throws {
  let bluetooth = FakeBluetooth()
  let products = MemoryProductStore()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    productStore: products,
    protocolCodec: TelemetryProtocol(),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  let id = DeviceID("direct")

  try service.receive(.deviceFound(id, name: "Direct THOR", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  #expect(service.connectionState == .identifying)
  #expect(service.activeConnectionSnapshot()?.savedDevice == nil)
  #expect(store.active() == nil)

  try service.receive(.notification(id, Data([1])))
  let identified = try #require(service.activeConnectionSnapshot())
  #expect(identified.savedDevice?.bluetoothIdentifier == id.value)
  #expect(identified.controllerRoute == .direct)
  #expect(store.active() == nil)

  try service.receive(.notification(id, Data([2])))
  #expect(store.active() == nil)
  try service.receive(.notification(id, Data([3])))
  #expect(service.connectionState == .ready)
  #expect(store.createCount == 1)
  #expect(store.active() != nil)
}

@Test
func privateHintFastPathSkipsPingWhenControllerUUIDMatches() throws {
  let bluetooth = FakeBluetooth()
  let products = MemoryProductStore()
  products.routingHint = ControllerRoutingHint(
    savedDeviceID: "saved-board",
    canID: 42,
    controllerUUID: Data((1 ... 12).map(UInt8.init))
  )
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    productStore: products,
    protocolCodec: AmbiguousProtocol(),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  let id = DeviceID("gateway")

  try service.receive(.deviceFound(id, name: "Generic gateway", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([1])))
  #expect(bluetooth.writes.last == Data([0, 42]))
  #expect(!bluetooth.writes.contains(Data([62])))
  for byte in UInt8(4) ... 6 {
    try service.receive(.notification(id, Data([byte])))
  }
  #expect(service.connectionState == .ready)
  #expect(service.activeConnectionSnapshot()?.controllerRoute == .can(42))
}

@Test
func wrongPreferredControllerRetryAndEmptyDiscoveryRemainRecoverable() throws {
  let bluetooth = FakeBluetooth()
  let products = MemoryProductStore()
  products.profile = BoardProfileRecord(
    savedDeviceID: "saved-board",
    revision: 0,
    valuesJSON: #"{"preferredCanId":42}"#,
    missingFields: []
  )
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    productStore: products,
    protocolCodec: DiscoveryBoundaryProtocol(),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  let id = DeviceID("gateway")

  try service.receive(.deviceFound(id, name: "Generic gateway", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([1])))
  #expect(bluetooth.writes.last == Data([0, 42]))
  try service.receive(.notification(id, Data([3])))
  #expect(service.connectionState == .failed)
  #expect(
    service.activeConnectionSnapshot()?.connectionIssue?.code
      == .preferredControllerUnavailable
  )

  products.profile = BoardProfileRecord(
    savedDeviceID: "saved-board",
    revision: 1,
    valuesJSON: "{}",
    missingFields: []
  )
  try service.connect(id)
  #expect(service.connectionState == .connecting)
  #expect(service.activeConnectionSnapshot()?.connectionIssue == nil)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([1])))
  #expect(bluetooth.writes.last == Data([62]))
  try service.receive(.notification(id, Data([2])))
  #expect(service.connectionState == .failed)
  #expect(service.activeConnectionSnapshot()?.connectionIssue?.code == .noControllerFound)

  try service.disconnect()
  try service.receive(.connectionChanged(id, .disconnected))
  #expect(service.activeConnectionSnapshot() == nil)
}

@Test
func discoveryBoundsFailWithoutPublishingTruncatedCandidates() throws {
  let id = DeviceID("gateway")

  let excessiveIDs = ApplicationService(
    bluetooth: FakeBluetooth(),
    clock: FixedClock(),
    logStore: MemoryStore(),
    productStore: MemoryProductStore(),
    protocolCodec: DiscoveryBoundaryProtocol(canIDs: Array(0 ... 64)),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  try excessiveIDs.receive(.deviceFound(id, name: "Gateway", rssi: -40))
  try excessiveIDs.connect(id)
  try excessiveIDs.receive(.connectionChanged(id, .ready))
  try excessiveIDs.receive(.notification(id, Data([1])))
  try excessiveIDs.receive(.notification(id, Data([2])))
  #expect(excessiveIDs.connectionState == .failed)
  #expect(
    excessiveIDs.activeConnectionSnapshot()?.connectionIssue?.code == .tooManyCanDevices
  )
  #expect(excessiveIDs.activeConnectionSnapshot()?.controllerCandidates.isEmpty == true)

  let excessiveControllers = ApplicationService(
    bluetooth: FakeBluetooth(),
    clock: FixedClock(),
    logStore: MemoryStore(),
    productStore: MemoryProductStore(),
    protocolCodec: DiscoveryBoundaryProtocol(canIDs: Array(0 ... 16)),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  try excessiveControllers.receive(.deviceFound(id, name: "Gateway", rssi: -40))
  try excessiveControllers.connect(id)
  try excessiveControllers.receive(.connectionChanged(id, .ready))
  try excessiveControllers.receive(.notification(id, Data([1])))
  try excessiveControllers.receive(.notification(id, Data([2])))
  for marker in UInt8(10) ... 26 {
    try excessiveControllers.receive(.notification(id, Data([marker])))
  }
  #expect(excessiveControllers.connectionState == .failed)
  #expect(
    excessiveControllers.activeConnectionSnapshot()?.connectionIssue?.code
      == .tooManyControllers
  )
  #expect(excessiveControllers.activeConnectionSnapshot()?.controllerCandidates.isEmpty == true)
}

@Test
func reconnectRevalidatesDirectIdentityAndResumesOneSession() throws {
  let bluetooth = FakeBluetooth()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    productStore: MemoryProductStore(),
    protocolCodec: TelemetryProtocol(),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  let id = DeviceID("direct")
  try service.receive(.deviceFound(id, name: "Direct THOR", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  for marker in UInt8(1) ... 3 {
    try service.receive(.notification(id, Data([marker])))
  }
  #expect(store.createCount == 1)

  try service.receive(.connectionChanged(id, .disconnected))
  #expect(service.connectionState == .reconnecting)
  try service.receive(.connectionChanged(id, .ready))
  for marker in UInt8(1) ... 3 {
    try service.receive(.notification(id, Data([marker])))
  }
  #expect(service.connectionState == .ready)
  #expect(service.activeConnectionSnapshot()?.controllerRoute == .direct)
  #expect(store.createCount == 1)
  #expect(store.active() != nil)
}

@Test
func auxiliaryCanScanAllowsForTheFullFirmwareScanWindow() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    protocolCodec: TelemetryProtocol(bmsEnabled: true),
    scheduler: scheduler,
    events: IgnoreEvents()
  )
  let id = DeviceID("direct-with-vbms")

  try service.receive(.deviceFound(id, name: "Thor301", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  for marker in UInt8(1) ... 3 {
    try service.receive(.notification(id, Data([marker])))
  }
  try service.receive(.notification(id, Data([4])))

  #expect(bluetooth.writes.contains(Data([62])))
  #expect(scheduler.oneShotIntervals.contains(10_000))
}

@Test
func exactDirectVBMSBecomesBMSOnlyWithoutControllerOrSessionWork() throws {
  let bluetooth = FakeBluetooth()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    productStore: MemoryProductStore(),
    protocolCodec: StandaloneBMSProtocol(),
    scheduler: FakeScheduler(),
    events: IgnoreEvents()
  )
  let id = DeviceID("numeric-vbms")

  try service.receive(.deviceFound(id, name: "42508004", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([1])))

  let active = try #require(service.activeConnectionSnapshot())
  #expect(service.connectionState == .ready)
  #expect(active.deviceKind == .bms)
  #expect(active.controllerRoute == nil)
  #expect(active.activeSession == nil)
  #expect(active.bms.connection == .direct)
  #expect(store.createCount == 0)
  #expect(bluetooth.writes == [Data([1]), Data([96])])

  try service.receive(.connectionChanged(id, .disconnected))
  #expect(service.connectionState == .reconnecting)
  #expect(service.activeConnectionSnapshot()?.deviceKind == nil)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([1])))
  #expect(service.activeConnectionSnapshot()?.deviceKind == .bms)
  #expect(store.createCount == 0)
  #expect(bluetooth.writes.filter { $0 == Data([62]) }.isEmpty)
}

@Test
func refloatTelemetryStartsIdleProbeAndSwitchesToRidingRate() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    protocolCodec: TelemetryProtocol(),
    scheduler: scheduler,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([1])))
  try service.receive(.notification(id, Data([2])))
  #expect(scheduler.repeatingIntervals == [1_000])
  #expect(store.active() == nil)
  let firstLayoutRequestCount = bluetooth.writes.filter { $0 == Data([32, 255]) }.count
  scheduler.fireOneShots(afterMilliseconds: 900)
  scheduler.fireRepeating(0)
  #expect(
    bluetooth.writes.filter { $0 == Data([32, 255]) }.count
      == firstLayoutRequestCount + 1
  )
  try service.receive(.notification(id, Data([3])))
  #expect(scheduler.repeatingIntervals == [1_000, 500])
  #expect(store.active() != nil)
  #expect(bluetooth.writes.contains(Data([31, 255])))

  try service.receive(.notification(id, Data([4])))

  #expect(scheduler.repeatingIntervals == [1_000, 500, 100])
  #expect(store.appended.count == 1)
  #expect(store.appended.first?.rideState == .riding)
  #expect(store.appended.first?.packageState == .running)
  #expect(store.appended.first?.packVoltageV == 64)
  #expect(service.activeConnectionSnapshot()?.latestSample == store.appended.first)
  #expect(store.appended.first?.motorCurrentA == 8)
  #expect(store.appended.first?.batteryCurrentA == -4)
  #expect(store.appended.first?.erpm == 1024)
  #expect(store.appended.first?.speedKph == 36)
  #expect(store.appended.first?.extensionFields.isEmpty == true)
}

@Test
func refloatTelemetryKeepsRidingProbeRateThroughPostRideTail() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let clock = MutableClock()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: clock,
    logStore: store,
    protocolCodec: TelemetryProtocol(),
    scheduler: scheduler,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([1])))
  try service.receive(.notification(id, Data([2])))
  try service.receive(.notification(id, Data([3])))
  #expect(scheduler.repeatingIntervals == [1_000, 500])

  clock.date = .init(timeIntervalSince1970: 1)
  try service.receive(.notification(id, Data([4])))
  #expect(scheduler.repeatingIntervals == [1_000, 500, 100])
  try service.receive(.notification(id, Data([6])))
  scheduler.fireRepeating(2)

  clock.date = .init(timeIntervalSince1970: 2)
  try service.receive(.notification(id, Data([5])))
  #expect(store.appended.last?.slowObservedAt == Date(timeIntervalSince1970: 1))
  #expect(store.appended.last?.fieldWeakeningCurrentA == 1.5)
  #expect(store.appended.last?.distanceM == 42)
  clock.date = .init(timeIntervalSince1970: 6.999)
  scheduler.fireRepeating(2)
  try service.receive(.notification(id, Data([6])))
  scheduler.fireRepeating(2)
  try service.receive(.notification(id, Data([5])))
  #expect(store.appended.last?.distanceM == 7)
  #expect(store.appended.last?.slowObservedAt == Date(timeIntervalSince1970: 6.999))
  #expect(scheduler.repeatingIntervals == [1_000, 500, 100])

  clock.date = .init(timeIntervalSince1970: 7.001)
  scheduler.fireRepeating(2)
  try service.receive(.notification(id, Data([5])))
  #expect(scheduler.repeatingIntervals == [1_000, 500, 100, 500])
}

@Test
func missingRefloatLayoutRetriesFinitelyAndFailsLoggingSession() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    protocolCodec: TelemetryProtocol(),
    scheduler: scheduler,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.notification(id, Data([1])))
  try service.receive(.notification(id, Data([2])))
  #expect(store.active() == nil)
  for _ in 0 ..< 5 {
    scheduler.fireOneShots(afterMilliseconds: 900)
    scheduler.fireRepeating(0)
  }

  #expect(service.connectionState == .failed)
  #expect(bluetooth.disconnected.isEmpty)
  #expect(store.active() == nil)
  #expect(service.activeConnectionSnapshot()?.connectionIssue?.code == .identificationTimeout)
  #expect(bluetooth.writes.filter { $0 == Data([32, 255]) }.count == 5)

  try service.connect(id)
  #expect(service.connectionState == .connecting)
  #expect(service.activeConnectionSnapshot()?.connectionIssue == nil)
}

@Test
func unexpectedDisconnectUsesBoundedBackoffAndPreservesSessionOnRecovery() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    scheduler: scheduler,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  let active = store.active()
  try service.receive(.connectionChanged(id, .disconnected))

  #expect(service.connectionState == .reconnecting)
  #expect(scheduler.intervals == [1_000])
  scheduler.fire(0)
  #expect(bluetooth.connected == [id, id])
  try service.receive(.connectionChanged(id, .ready))
  #expect(service.connectionState == .ready)
  #expect(store.active() == active)

  try service.disconnect()
  try service.receive(.connectionChanged(id, .disconnected))
  #expect(service.connectionState == .disconnected)
  #expect(scheduler.intervals == [1_000])
}

@Test
func reconnectWindowExpiryClosesDurableSession() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let clock = MutableClock()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: clock,
    logStore: store,
    scheduler: scheduler,
    events: IgnoreEvents(),
    samplingSettings: .init(reconnectDurationMs: 2_500)
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(.connectionChanged(id, .disconnected))
  clock.date = Date(timeIntervalSince1970: 1)
  scheduler.fire(0)
  try service.receive(.connectionChanged(id, .failed))
  #expect(scheduler.intervals == [1_000, 1_500])
  clock.date = Date(timeIntervalSince1970: 2.5)
  scheduler.fire(1)

  #expect(service.connectionState == .disconnected)
  #expect(service.activeDeviceID == nil)
  #expect(store.active() == nil)
  #expect(bluetooth.disconnected == [id])
}

@Test
func radioOffWaitsInsideReconnectWindowUntilPoweredOn() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: MemoryStore(),
    scheduler: scheduler,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(
    .stateChanged(
      .init(radio: .poweredOff, scanPermission: .granted, connectPermission: .granted)
    )
  )
  scheduler.fire(0)
  #expect(bluetooth.connected == [id])
  #expect(scheduler.intervals == [1_000, 1_000])

  try service.receive(
    .stateChanged(
      .init(radio: .poweredOn, scanPermission: .granted, connectPermission: .granted)
    )
  )
  scheduler.fire(1)
  #expect(bluetooth.connected == [id, id])
  #expect(service.connectionState == .reconnecting)
}

@Test
func deniedConnectPermissionTerminatesTheActiveSessionWithoutRetrying() throws {
  let bluetooth = FakeBluetooth()
  let scheduler = FakeScheduler()
  let store = MemoryStore()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: FixedClock(),
    logStore: store,
    scheduler: scheduler,
    events: IgnoreEvents()
  )
  let id = DeviceID("fixture")

  try service.receive(.deviceFound(id, name: "ExpressT", rssi: -40))
  try service.connect(id)
  try service.receive(.connectionChanged(id, .ready))
  try service.receive(
    .stateChanged(
      .init(radio: .unknown, scanPermission: .denied, connectPermission: .denied)
    )
  )

  #expect(service.connectionState == .disconnected)
  #expect(service.activeDeviceID == nil)
  #expect(store.active() == nil)
  #expect(store.closeReasons == ["bluetooth_permission_denied"])
  #expect(bluetooth.disconnected == [id])
  #expect(scheduler.intervals.isEmpty)
}

private final class FakeBluetooth: BluetoothPort, @unchecked Sendable {
  var state: BluetoothState
  var startCount = 0
  var stopCount = 0
  var connected: [DeviceID] = []
  var disconnected: [DeviceID] = []
  var writes: [Data] = []

  init(
    state: BluetoothState = .init(
      radio: .poweredOn,
      scanPermission: .granted,
      connectPermission: .granted
    )
  ) {
    self.state = state
  }

  func currentState() -> BluetoothState {
    state
  }

  func requestPermission() {}
  func startDiscovery() {
    startCount += 1
  }

  func stopDiscovery() {
    stopCount += 1
  }

  func connect(_ deviceID: DeviceID) {
    connected.append(deviceID)
  }

  func write(_ bytes: Data, to _: DeviceID) {
    writes.append(bytes)
  }

  func disconnect(_ deviceID: DeviceID) {
    disconnected.append(deviceID)
  }
}

private final class FakeProtocol: VescProtocolPort, @unchecked Sendable {
  func reset() {}
  func firmwareRequest(route _: ControllerRoute) throws -> Data {
    Data([1])
  }

  func canPingRequest() throws -> Data {
    Data([62])
  }

  func refloatInfoRequest(route _: ControllerRoute) throws -> Data {
    Data([36, 101, 0])
  }

  func refloatTelemetryIDsRequest(route _: ControllerRoute) throws -> Data {
    Data([36, 101, 32])
  }

  func refloatTelemetryRequest(route _: ControllerRoute) throws -> Data {
    Data([36, 101, 31])
  }

  func receive(_: Data) throws -> [VescProtocolMessage] {
    [.firmware(.init(major: 6, minor: 6, hardwareName: "THOR300", firmwareName: "refloat"))]
  }
}

private final class CanDiscoveryProtocol: VescProtocolPort, @unchecked Sendable {
  func reset() {}
  func firmwareRequest(route: ControllerRoute) throws -> Data {
    route.canID.map { Data([1, $0]) } ?? Data([1])
  }

  func canPingRequest() throws -> Data {
    Data([62])
  }

  func refloatInfoRequest(route: ControllerRoute) throws -> Data {
    try Data([36, 101, 0, #require(route.canID)])
  }

  func refloatTelemetryIDsRequest(route: ControllerRoute) throws -> Data {
    Data([36, 101, 32, route.canID ?? 255])
  }

  func refloatTelemetryRequest(route: ControllerRoute) throws -> Data {
    Data([36, 101, 31, route.canID ?? 255])
  }

  func receive(_ bytes: Data) throws -> [VescProtocolMessage] {
    switch try #require(bytes.first) {
    case 1:
      return [
        .firmware(
          .init(
            major: 6,
            minor: 5,
            hardwareName: "VESC Express T",
            firmwareName: nil,
            hardwareType: 2
          )
        ),
      ]
    case 2:
      return [.canDevices([7, 42])]
    case 3:
      return [
        .firmware(
          .init(
            major: 6,
            minor: 5,
            hardwareName: "VESC Express T",
            firmwareName: nil,
            hardwareType: 2,
            canID: 7
          )
        ),
      ]
    case 4:
      return [
        .firmware(
          .init(
            major: 6,
            minor: 6,
            hardwareName: "THOR300",
            firmwareName: nil,
            hardwareType: 0,
            canID: 42
          )
        ),
      ]
    case 5:
      return [.refloatInfo(version: "1.2.3")]
    default:
      return []
    }
  }
}

private final class TelemetryProtocol: VescProtocolPort, @unchecked Sendable {
  private var compactCount = 0
  private let bmsEnabled: Bool

  init(bmsEnabled: Bool = false) {
    self.bmsEnabled = bmsEnabled
  }

  var supportsBmsTelemetry: Bool {
    bmsEnabled
  }

  func reset() {}
  func firmwareRequest(route _: ControllerRoute) throws -> Data {
    Data([1])
  }

  func canPingRequest() throws -> Data {
    Data([62])
  }

  func bmsValuesRequest(connection: BmsConnection) throws -> Data {
    switch connection {
    case .direct: Data([96])
    case let .can(canID): Data([34, canID, 96])
    }
  }

  func refloatInfoRequest(route: ControllerRoute) throws -> Data {
    Data([0, route.canID ?? 255])
  }

  func refloatTelemetryIDsRequest(route: ControllerRoute) throws -> Data {
    Data([32, route.canID ?? 255])
  }

  func refloatTelemetryRequest(route: ControllerRoute) throws -> Data {
    Data([31, route.canID ?? 255])
  }

  func controllerShutdownRequest(route _: ControllerRoute) throws -> Data {
    Data([156, 0, 0])
  }

  func receive(_ bytes: Data) throws -> [VescProtocolMessage] {
    switch try #require(bytes.first) {
    case 1:
      return [
        .firmware(
          .init(
            major: 6, minor: 6, hardwareName: "THOR300", firmwareName: nil,
            hardwareType: 0, canID: nil
          )
        ),
      ]
    case 6:
      compactCount += 1
      return [
        .refloatTelemetry(
          .init(
            kind: .compact, rideState: .idle,
            fieldWeakeningCurrentA: 1.5, distanceM: compactCount == 1 ? 42 : 7
          )
        ),
      ]
    case 2:
      return [.refloatInfo(version: "1.2.3")]
    case 3:
      return [.refloatTelemetryLayout(
        fields: RefloatCompatibility.requiredFields,
        runtimeFields: RefloatCompatibility.requiredRuntimeFields
      )]
    case 4:
      return [
        .refloatTelemetry(
          .init(
            rideState: .riding, packageState: .running, footpadState: .both,
            packVoltageV: 64, dutyCycle: 0.5, controllerTempC: 30,
            motorTempC: 25, motorCurrentA: 8, batteryCurrentA: -4,
            erpm: 1024, speedKph: 36
          )
        ),
      ]
    case 5:
      return [
        .refloatTelemetry(
          .init(
            rideState: .idle, packageState: .ready, footpadState: FootpadState.none,
            packVoltageV: 64, dutyCycle: 0, controllerTempC: 30,
            motorTempC: 25, motorCurrentA: 0, batteryCurrentA: 0,
            erpm: 0, speedKph: 0
          )
        ),
      ]
    default:
      return []
    }
  }
}

private final class StandaloneBMSProtocol: VescProtocolPort, @unchecked Sendable {
  var supportsBmsTelemetry: Bool {
    true
  }

  func reset() {}
  func firmwareRequest(route _: ControllerRoute) throws -> Data {
    Data([1])
  }

  func canPingRequest() throws -> Data {
    Data([62])
  }

  func bmsValuesRequest(connection _: BmsConnection) throws -> Data {
    Data([96])
  }

  func refloatInfoRequest(route _: ControllerRoute) throws -> Data {
    Data([36, 101, 0])
  }

  func refloatTelemetryIDsRequest(route _: ControllerRoute) throws -> Data {
    Data([36, 101, 32])
  }

  func refloatTelemetryRequest(route _: ControllerRoute) throws -> Data {
    Data([36, 101, 31])
  }

  func receive(_ bytes: Data) throws -> [VescProtocolMessage] {
    guard bytes == Data([1]) else { return [] }
    return [.firmware(.init(
      major: 6, minor: 6, hardwareName: "VBMS32", firmwareName: nil,
      hardwareType: 2, canID: nil
    ))]
  }
}

private final class AmbiguousProtocol: VescProtocolPort, @unchecked Sendable {
  func reset() {}
  func firmwareRequest(route: ControllerRoute) throws -> Data {
    route.canID.map { Data([0, $0]) } ?? Data([0])
  }

  func canPingRequest() throws -> Data {
    Data([62])
  }

  func refloatInfoRequest(route: ControllerRoute) throws -> Data {
    Data([2, route.canID ?? 255])
  }

  func refloatTelemetryIDsRequest(route: ControllerRoute) throws -> Data {
    Data([32, route.canID ?? 255])
  }

  func refloatTelemetryRequest(route: ControllerRoute) throws -> Data {
    Data([31, route.canID ?? 255])
  }

  func receive(_ bytes: Data) throws -> [VescProtocolMessage] {
    switch try #require(bytes.first) {
    case 1:
      return [.firmware(.init(
        major: 6, minor: 5, hardwareName: "Gateway", firmwareName: nil, hardwareType: 2
      ))]
    case 2:
      return [.canDevices([42, 7])]
    case 3:
      return [.firmware(.init(
        major: 5, minor: 3, hardwareName: "Old VESC", firmwareName: nil,
        hardwareType: 0, canID: 7
      ))]
    case 4:
      return [.firmware(.init(
        major: 6, minor: 6, hardwareName: "THOR300", firmwareName: "Refloat",
        hardwareType: 0, canID: 42,
        controllerUUID: Data((1 ... 12).map(UInt8.init))
      ))]
    case 5:
      return [.refloatInfo(version: "1.2.3")]
    case 6:
      return [.refloatTelemetryLayout(
        fields: RefloatCompatibility.requiredFields,
        runtimeFields: RefloatCompatibility.requiredRuntimeFields
      )]
    default:
      return []
    }
  }
}

private final class DiscoveryBoundaryProtocol: VescProtocolPort, @unchecked Sendable {
  private let canIDs: [UInt8]

  init(canIDs: [UInt8] = []) {
    self.canIDs = canIDs
  }

  func reset() {}
  func firmwareRequest(route: ControllerRoute) throws -> Data {
    route.canID.map { Data([0, $0]) } ?? Data([0])
  }

  func canPingRequest() throws -> Data {
    Data([62])
  }

  func refloatInfoRequest(route: ControllerRoute) throws -> Data {
    Data([2, route.canID ?? 255])
  }

  func refloatTelemetryIDsRequest(route: ControllerRoute) throws -> Data {
    Data([32, route.canID ?? 255])
  }

  func refloatTelemetryRequest(route: ControllerRoute) throws -> Data {
    Data([31, route.canID ?? 255])
  }

  func receive(_ bytes: Data) throws -> [VescProtocolMessage] {
    switch try #require(bytes.first) {
    case 1:
      return [.firmware(.init(
        major: 6, minor: 5, hardwareName: "Generic gateway", firmwareName: nil,
        hardwareType: 2
      ))]
    case 2:
      return [.canDevices(canIDs)]
    case 3:
      return [.firmware(.init(
        major: 6, minor: 5, hardwareName: "BMS", firmwareName: nil,
        hardwareType: 2, canID: 42
      ))]
    case 10 ... 26:
      let canID = try #require(UInt8(exactly: Int(bytes[0]) - 10))
      return [.firmware(.init(
        major: 6, minor: 6, hardwareName: "THOR300", firmwareName: "Refloat",
        hardwareType: 0, canID: canID
      ))]
    default:
      return []
    }
  }
}

private final class FakeScheduler: SchedulerPort, @unchecked Sendable {
  var intervals: [Int64] = []
  var actions: [@Sendable () -> Void] = []
  var repeatingIntervals: [Int64] = []
  var repeatingActions: [@Sendable () -> Void] = []
  var oneShotIntervals: [Int64] = []
  var oneShotActions: [@Sendable () -> Void] = []
  func schedule(
    afterMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort {
    let cancellation = FakeCancellation()
    intervals.append(afterMilliseconds)
    let guarded: @Sendable () -> Void = { if !cancellation.isCancelled { action() } }
    actions.append(guarded)
    oneShotIntervals.append(afterMilliseconds)
    oneShotActions.append(guarded)
    return cancellation
  }

  func scheduleRepeating(
    everyMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort {
    let cancellation = FakeCancellation()
    intervals.append(everyMilliseconds)
    repeatingIntervals.append(everyMilliseconds)
    let guarded: @Sendable () -> Void = { if !cancellation.isCancelled { action() } }
    actions.append(guarded)
    repeatingActions.append(guarded)
    return cancellation
  }

  func fire(_ index: Int) {
    actions[index]()
  }

  func fireRepeating(_ index: Int) {
    repeatingActions[index]()
  }

  func fireOneShots(afterMilliseconds: Int64) {
    for (index, interval) in oneShotIntervals.enumerated() where interval == afterMilliseconds {
      oneShotActions[index]()
    }
  }
}

private final class FakeCancellation: CancellationPort, @unchecked Sendable {
  private let lock = NSLock()
  private var cancelled = false
  var isCancelled: Bool {
    lock.withLock { cancelled }
  }

  func cancel() {
    lock.withLock { cancelled = true }
  }
}

private struct FixedClock: ClockPort {
  func monotonicMilliseconds() -> Int64 {
    0
  }

  func now() -> Date {
    .init(timeIntervalSince1970: 0)
  }
}

private final class MutableClock: ClockPort, @unchecked Sendable {
  var date = Date(timeIntervalSince1970: 0)
  func monotonicMilliseconds() -> Int64 {
    Int64(date.timeIntervalSince1970 * 1_000)
  }

  func now() -> Date {
    date
  }
}

private final class MemoryStore: LogStorePort, @unchecked Sendable {
  private var record: SessionRecord?
  var createCount = 0
  var appended: [TelemetrySample] = []
  var closeReasons: [String] = []
  var locationObservations: [LocationObservation] = []
  func create(
    deviceID: DeviceID,
    advertisedName _: String?,
    at: Date
  ) throws -> SessionRecord {
    createCount += 1
    let result = SessionRecord(
      sessionID: SessionID("session"),
      jobID: JobID("job"),
      deviceID: deviceID,
      startedAt: at,
      committedSequence: 0,
      active: true
    )
    record = result
    return result
  }

  func append(jobID _: JobID, samples: [TelemetrySample]) throws {
    appended += samples
  }

  func appendLocations(jobID _: JobID, observations: [LocationObservation]) throws {
    locationObservations += observations
  }

  func close(jobID _: JobID, at _: Date, reason: String) throws -> SessionRecord {
    closeReasons.append(reason)
    let result = try #require(record).withActive(false)
    record = nil
    return result
  }

  func active() -> SessionRecord? {
    record
  }
}

private final class FailingStore: LogStorePort, @unchecked Sendable {
  func create(
    deviceID _: DeviceID,
    advertisedName _: String?,
    at _: Date
  ) throws -> SessionRecord {
    throw TestError.storage
  }

  func append(jobID _: JobID, samples _: [TelemetrySample]) throws {}
  func close(jobID _: JobID, at _: Date, reason _: String) throws -> SessionRecord {
    throw TestError.storage
  }

  func active() -> SessionRecord? {
    nil
  }
}

private final class MemoryProductStore: ProductStorePort, @unchecked Sendable {
  var routingHint: ControllerRoutingHint?
  var settingsRecord = SettingsRecord()
  var savedDevice = SavedDeviceRecord(
    savedDeviceID: "saved-board",
    deviceHistoryID: "history-board",
    advertisedName: "Advertised",
    bluetoothIdentifier: "identifier",
    profileComplete: false
  )
  var profile = BoardProfileRecord(
    savedDeviceID: "saved-board",
    revision: 0,
    valuesJSON: "{}",
    missingFields: ["seriesCount", "parallelCount", "motor"]
  )

  func savedDevices() throws -> [SavedDeviceRecord] {
    [savedDevice]
  }

  func settings() throws -> SettingsRecord {
    settingsRecord
  }

  func ensureSavedDevice(
    deviceID: DeviceID,
    advertisedName: String?,
    at _: Date
  ) throws -> SavedDeviceRecord {
    savedDevice = SavedDeviceRecord(
      savedDeviceID: savedDevice.savedDeviceID,
      deviceHistoryID: savedDevice.deviceHistoryID,
      revision: savedDevice.revision,
      customDisplayName: savedDevice.customDisplayName,
      advertisedName: advertisedName,
      bluetoothIdentifier: deviceID.value,
      profileComplete: false
    )
    return savedDevice
  }

  func updateSavedDevice(
    savedDeviceID: String,
    expectedRevision: Int,
    customDisplayName: String?
  ) throws -> SavedDeviceRecord {
    guard savedDeviceID == savedDevice.savedDeviceID else {
      throw ProductStoreError.savedDeviceNotFound
    }
    guard expectedRevision == savedDevice.revision else {
      throw ProductStoreError.savedDeviceRevisionConflict(currentRevision: savedDevice.revision)
    }
    savedDevice = SavedDeviceRecord(
      savedDeviceID: savedDevice.savedDeviceID,
      deviceHistoryID: savedDevice.deviceHistoryID,
      revision: savedDevice.revision + 1,
      customDisplayName: customDisplayName,
      advertisedName: savedDevice.advertisedName,
      bluetoothIdentifier: savedDevice.bluetoothIdentifier,
      profileComplete: false
    )
    return savedDevice
  }

  func boardProfile(savedDeviceID: String) throws -> BoardProfileRecord {
    guard savedDeviceID == profile.savedDeviceID else {
      throw ProductStoreError.savedDeviceNotFound
    }
    return profile
  }

  func updateBoardProfile(
    savedDeviceID: String,
    expectedRevision: Int,
    patchJSON: String,
    at _: Date
  ) throws -> BoardProfileRecord {
    guard savedDeviceID == profile.savedDeviceID else {
      throw ProductStoreError.savedDeviceNotFound
    }
    guard expectedRevision == profile.revision else {
      throw ProductStoreError.profileRevisionConflict(currentRevision: profile.revision)
    }
    profile = BoardProfileRecord(
      savedDeviceID: savedDeviceID,
      revision: profile.revision + 1,
      valuesJSON: patchJSON,
      missingFields: ["parallelCount", "motor"]
    )
    return profile
  }

  func selectController(
    savedDeviceID: String,
    canID: UInt8,
    controllerUUID _: Data?,
    at _: Date
  ) throws -> BoardProfileRecord {
    guard savedDeviceID == profile.savedDeviceID else {
      throw ProductStoreError.savedDeviceNotFound
    }
    var values = (try? JSONSerialization.jsonObject(with: Data(profile.valuesJSON.utf8)))
      as? [String: Any] ?? [:]
    values["preferredCanId"] = Int(canID)
    let data = try JSONSerialization.data(withJSONObject: values, options: [.sortedKeys])
    profile = BoardProfileRecord(
      savedDeviceID: savedDeviceID,
      revision: profile.revision + 1,
      valuesJSON: String(decoding: data, as: UTF8.self),
      missingFields: profile.missingFields
    )
    return profile
  }

  func controllerRoutingHint(savedDeviceID: String) throws -> ControllerRoutingHint? {
    guard savedDeviceID == savedDevice.savedDeviceID else {
      throw ProductStoreError.savedDeviceNotFound
    }
    return routingHint
  }

  func storeControllerRoutingHint(_ hint: ControllerRoutingHint, at _: Date) throws {
    routingHint = hint
  }

  func clearControllerRoutingHint(savedDeviceID: String) throws {
    guard savedDeviceID == savedDevice.savedDeviceID else {
      throw ProductStoreError.savedDeviceNotFound
    }
    routingHint = nil
  }
}

private final class FakeLocationProvider: LocationProviderPort, @unchecked Sendable {
  var currentState: LocationProviderState
  var startCount = 0
  var stopCount = 0
  private var handler: (@Sendable (LocationProviderObservation) -> Void)?

  init(state: LocationProviderState) {
    currentState = state
  }

  func state() -> LocationProviderState {
    currentState
  }

  func requestPermission() {}
  func openSettings() {}
  func startUpdates(_ handler: @escaping @Sendable (LocationProviderObservation) -> Void) {
    startCount += 1
    self.handler = handler
  }

  func stopUpdates() {
    stopCount += 1
    handler = nil
  }

  func emit(_ observation: LocationProviderObservation) {
    handler?(observation)
  }
}

private func readyLocationService(
  store: MemoryStore,
  products: MemoryProductStore,
  provider: FakeLocationProvider,
  clock: MutableClock
) -> ApplicationService {
  let service = ApplicationService(
    bluetooth: FakeBluetooth(),
    clock: clock,
    logStore: store,
    productStore: products,
    events: IgnoreEvents(),
    locationProvider: provider
  )
  let id = DeviceID("fixture")
  try! service.receive(.deviceFound(id, name: "Fixture", rssi: -40))
  try! service.connect(id)
  try! service.receive(.connectionChanged(id, .ready))
  return service
}

private enum TestError: Error {
  case storage
}

private struct IgnoreEvents: EventSink {
  func publish(_: any Sendable) {}
}

private final class CollectingEvents: EventSink, @unchecked Sendable {
  var values: [any Sendable] = []
  func publish(_ event: any Sendable) {
    values.append(event)
  }
}

private extension SessionRecord {
  func withActive(_ active: Bool) -> SessionRecord {
    SessionRecord(
      sessionID: sessionID,
      jobID: jobID,
      deviceID: deviceID,
      startedAt: startedAt,
      committedSequence: committedSequence,
      active: active
    )
  }
}
