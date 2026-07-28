import Foundation
import Testing
@testable import FloatViewCore

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
func discoveryOwnsScanAndDeduplicatesDevices() throws {
  let bluetooth = FakeBluetooth()
  let clock = MutableClock()
  let service = ApplicationService(
    bluetooth: bluetooth,
    clock: clock,
    logStore: MemoryStore(),
    events: IgnoreEvents()
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
  for byte in UInt8(1)...5 {
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
  #expect(service.refloatPackageVersionSnapshot() == "1.2")
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
  #expect(scheduler.intervals == [1_000])
  let firstLayoutRequestCount = bluetooth.writes.filter { $0 == Data([32, 42]) }.count
  scheduler.fire(0)
  #expect(
    bluetooth.writes.filter { $0 == Data([32, 42]) }.count
      == firstLayoutRequestCount + 1
  )
  try service.receive(.notification(id, Data([3])))
  #expect(scheduler.intervals == [1_000, 500])
  #expect(bluetooth.writes.contains(Data([31, 42])))

  try service.receive(.notification(id, Data([4])))

  #expect(scheduler.intervals == [1_000, 500, 100])
  #expect(store.appended.count == 1)
  #expect(store.appended.first?.rideState == .riding)
  #expect(store.appended.first?.packVoltageV == 64)
  #expect(store.appended.first?.extensionFields == [
    "motorCurrentA": 8,
    "batteryCurrentA": -4,
    "erpm": 1024,
    "speedKph": 36,
  ])
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
  for _ in 0..<5 { scheduler.fire(0) }

  #expect(service.connectionState == .failed)
  #expect(bluetooth.disconnected == [id])
  #expect(store.active() == nil)
  #expect(bluetooth.writes.filter { $0 == Data([32, 42]) }.count == 5)
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

  func currentState() -> BluetoothState { state }
  func requestPermission() {}
  func startDiscovery() { startCount += 1 }
  func stopDiscovery() { stopCount += 1 }
  func connect(_ deviceID: DeviceID) { connected.append(deviceID) }
  func write(_ bytes: Data, to _: DeviceID) { writes.append(bytes) }
  func disconnect(_ deviceID: DeviceID) { disconnected.append(deviceID) }
}

private final class FakeProtocol: VescProtocolPort, @unchecked Sendable {
  func reset() {}
  func firmwareRequest(canID _: UInt8?) throws -> Data { Data([1]) }
  func canPingRequest() throws -> Data { Data([62]) }
  func refloatInfoRequest(canID _: UInt8?) throws -> Data { Data([36, 101, 0]) }
  func refloatTelemetryIDsRequest(canID _: UInt8) throws -> Data { Data([36, 101, 32]) }
  func refloatTelemetryRequest(canID _: UInt8) throws -> Data { Data([36, 101, 31]) }
  func receive(_: Data) throws -> [VescProtocolMessage] {
    [.firmware(.init(major: 6, minor: 6, hardwareName: "THOR300", firmwareName: "refloat"))]
  }
}

private final class CanDiscoveryProtocol: VescProtocolPort, @unchecked Sendable {
  func reset() {}
  func firmwareRequest(canID: UInt8?) throws -> Data {
    canID.map { Data([1, $0]) } ?? Data([1])
  }
  func canPingRequest() throws -> Data { Data([62]) }
  func refloatInfoRequest(canID: UInt8?) throws -> Data {
    Data([36, 101, 0, try #require(canID)])
  }
  func refloatTelemetryIDsRequest(canID: UInt8) throws -> Data {
    Data([36, 101, 32, canID])
  }
  func refloatTelemetryRequest(canID: UInt8) throws -> Data {
    Data([36, 101, 31, canID])
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
      return [.refloatInfo(version: "1.2")]
    default:
      return []
    }
  }
}

private final class TelemetryProtocol: VescProtocolPort, @unchecked Sendable {
  func reset() {}
  func firmwareRequest(canID _: UInt8?) throws -> Data { Data([1]) }
  func canPingRequest() throws -> Data { Data([62]) }
  func refloatInfoRequest(canID: UInt8?) throws -> Data {
    Data([0, try #require(canID)])
  }
  func refloatTelemetryIDsRequest(canID: UInt8) throws -> Data { Data([32, canID]) }
  func refloatTelemetryRequest(canID: UInt8) throws -> Data { Data([31, canID]) }
  func receive(_ bytes: Data) throws -> [VescProtocolMessage] {
    switch try #require(bytes.first) {
    case 1:
      return [
        .firmware(
          .init(
            major: 6, minor: 6, hardwareName: "THOR300", firmwareName: nil,
            hardwareType: 0, canID: 42
          )
        ),
      ]
    case 2:
      return [.refloatInfo(version: "1.2.0")]
    case 3:
      return [.refloatTelemetryLayout(fields: ["motor.batt_voltage"], runtimeFields: [])]
    case 4:
      return [
        .refloatTelemetry(
          .init(
            rideState: .riding, boardState: 3, footpadState: 3,
            packVoltageV: 64, dutyCycle: 0.5, controllerTempC: 30,
            motorTempC: 25, motorCurrentA: 8, batteryCurrentA: -4,
            erpm: 1024, speedKph: 36
          )
        ),
      ]
    default:
      return []
    }
  }
}

private final class FakeScheduler: SchedulerPort, @unchecked Sendable {
  var intervals: [Int64] = []
  var actions: [@Sendable () -> Void] = []
  func schedule(
    afterMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort {
    intervals.append(afterMilliseconds)
    actions.append(action)
    return FakeCancellation()
  }

  func scheduleRepeating(
    everyMilliseconds: Int64,
    action: @escaping @Sendable () -> Void
  ) -> any CancellationPort {
    intervals.append(everyMilliseconds)
    actions.append(action)
    return FakeCancellation()
  }

  func fire(_ index: Int) {
    actions[index]()
  }
}

private final class FakeCancellation: CancellationPort, @unchecked Sendable {
  func cancel() {}
}

private struct FixedClock: ClockPort {
  func monotonicMilliseconds() -> Int64 { 0 }
  func now() -> Date { .init(timeIntervalSince1970: 0) }
}

private final class MutableClock: ClockPort, @unchecked Sendable {
  var date = Date(timeIntervalSince1970: 0)
  func monotonicMilliseconds() -> Int64 { Int64(date.timeIntervalSince1970 * 1_000) }
  func now() -> Date { date }
}

private final class MemoryStore: LogStorePort, @unchecked Sendable {
  private var record: SessionRecord?
  var appended: [TelemetrySample] = []
  var closeReasons: [String] = []
  func create(deviceID: DeviceID, at: Date) throws -> SessionRecord {
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
  func close(jobID _: JobID, at _: Date, reason: String) throws -> SessionRecord {
    closeReasons.append(reason)
    let result = try #require(record).withActive(false)
    record = nil
    return result
  }
  func active() -> SessionRecord? { record }
}

private final class FailingStore: LogStorePort, @unchecked Sendable {
  func create(deviceID _: DeviceID, at _: Date) throws -> SessionRecord {
    throw TestError.storage
  }
  func append(jobID _: JobID, samples _: [TelemetrySample]) throws {}
  func close(jobID _: JobID, at _: Date, reason _: String) throws -> SessionRecord {
    throw TestError.storage
  }
  func active() -> SessionRecord? { nil }
}

private final class MemoryProductStore: ProductStorePort, @unchecked Sendable {
  var profile = BoardProfileRecord(
    savedDeviceID: "saved-board",
    revision: 0,
    valuesJSON: "{}",
    missingFields: ["seriesCount", "parallelCount", "motor"]
  )

  func savedDevices() throws -> [SavedDeviceRecord] {
    [
      SavedDeviceRecord(
        savedDeviceID: "saved-board",
        deviceHistoryID: "history-board",
        displayName: "Board",
        profileComplete: false
      ),
    ]
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
}

private enum TestError: Error {
  case storage
}

private struct IgnoreEvents: EventSink {
  func publish(_: any Sendable) {}
}

private final class CollectingEvents: EventSink, @unchecked Sendable {
  var values: [any Sendable] = []
  func publish(_ event: any Sendable) { values.append(event) }
}

extension SessionRecord {
  fileprivate func withActive(_ active: Bool) -> SessionRecord {
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
