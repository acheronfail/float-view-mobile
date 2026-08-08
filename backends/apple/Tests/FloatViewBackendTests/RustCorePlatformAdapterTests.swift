@testable import FloatViewBackend
import FloatViewCoreBridge
import Foundation
import XCTest

final class RustCorePlatformAdapterTests: XCTestCase {
  func testBluetoothMappingUsesPrivateAssociationsOrderedGenerationsAndCancellation() throws {
    let runtime = FakePlatformRuntime(generation: 77)
    let bluetooth = FakeBluetoothPort()
    let location = FakeLocationPort()
    let diagnostics = DiagnosticCapture()
    let adapter = RustCorePlatformAdapter(
      runtime: runtime,
      bluetooth: bluetooth,
      location: location,
      platformHost: FakeApplePlatformHost(),
      diagnosticHandler: diagnostics.record
    )
    let deviceID = DeviceID("platform-identifier-must-not-cross")

    adapter.receiveBluetooth(.deviceFound(deviceID, name: "Test Board", rssi: -42))
    let discovery = try XCTUnwrap(runtime.events.last)
    guard case let .bluetoothDevice(token, name, rssi) = discovery.event else {
      return XCTFail("Expected Bluetooth device event")
    }
    XCTAssertEqual(discovery.runtimeGeneration, 77)
    XCTAssertEqual(discovery.adapterOrder, 1)
    XCTAssertEqual(token, Data(deviceID.value.utf8))
    XCTAssertEqual(name, "Test Board")
    XCTAssertEqual(rssi, -42)

    let connect = try hostCommand(
      operation: 1,
      command: .bluetoothConnect(deviceToken: token)
    )
    adapter.dispatch(connect)
    XCTAssertEqual(bluetooth.connected, [deviceID])
    XCTAssertEqual(runtime.completions.map(\.resultKind), [0])

    adapter.receiveBluetooth(.connectionChanged(deviceID, .ready))
    let connection = try XCTUnwrap(runtime.events.last)
    XCTAssertEqual(connection.connectionGeneration, 1)
    XCTAssertEqual(connection.adapterOrder, 2)
    XCTAssertEqual(connection.event, .bluetoothConnection(connectionGeneration: 1, state: .ready))
    XCTAssertEqual(runtime.completions.map(\.resultKind), [0])

    adapter.receiveBluetooth(.notification(deviceID, Data([1, 2, 3])))
    XCTAssertEqual(runtime.notifications, [
      .init(connectionGeneration: 1, adapterOrder: 3, bytes: Data([1, 2, 3])),
    ])

    try adapter.dispatch(hostCommand(
      operation: 2,
      command: .bluetoothWrite(connectionGeneration: 0, bytes: Data([9]))
    ))
    XCTAssertTrue(bluetooth.writes.isEmpty)
    XCTAssertEqual(runtime.completions.map(\.resultKind), [0, 1])

    adapter.receiveBluetooth(.scanFailed("Address AA:BB/secret value"))
    let diagnostic = try XCTUnwrap(diagnostics.values.last)
    XCTAssertLessThanOrEqual(diagnostic.stage.utf8.count, floatViewMaximumDiagnosticBytes)
    XCTAssertLessThanOrEqual(diagnostic.code.utf8.count, floatViewMaximumDiagnosticBytes)
    XCTAssertFalse(diagnostic.code.contains(":"))
    XCTAssertFalse(diagnostic.code.contains("/"))
  }

  func testBluetoothGenerationAdvancesAcrossRepeatedConnections() throws {
    let runtime = FakePlatformRuntime(generation: 77)
    let bluetooth = FakeBluetoothPort()
    let adapter = RustCorePlatformAdapter(
      runtime: runtime,
      bluetooth: bluetooth,
      location: FakeLocationPort(),
      platformHost: FakeApplePlatformHost()
    )
    let deviceID = DeviceID("repeat-connection")

    adapter.receiveBluetooth(.deviceFound(deviceID, name: "Test Board", rssi: -42))
    let discovery = try XCTUnwrap(runtime.events.last)
    guard case let .bluetoothDevice(token, _, _) = discovery.event else {
      return XCTFail("Expected Bluetooth device event")
    }

    try adapter.dispatch(hostCommand(
      operation: 1,
      command: .bluetoothConnect(deviceToken: token)
    ))
    adapter.receiveBluetooth(.connectionChanged(deviceID, .ready))
    adapter.receiveBluetooth(.connectionChanged(deviceID, .disconnected))
    try adapter.dispatch(hostCommand(
      operation: 2,
      command: .bluetoothConnect(deviceToken: token)
    ))
    adapter.receiveBluetooth(.connectionChanged(deviceID, .ready))
    try adapter.dispatch(hostCommand(
      operation: 3,
      command: .bluetoothWrite(connectionGeneration: 2, bytes: Data([9]))
    ))

    XCTAssertEqual(bluetooth.connected, [deviceID, deviceID])
    XCTAssertEqual(bluetooth.writes.count, 1)
    XCTAssertEqual(bluetooth.writes.first?.0, Data([9]))
    XCTAssertEqual(bluetooth.writes.first?.1, deviceID)
    let connectionEvents = runtime.events
      .filter {
        if case .bluetoothConnection = $0.event { return true }
        return false
      }
    XCTAssertEqual(connectionEvents.map(\.connectionGeneration), [1, 1, 2])
  }

  func testLocationMappingStopsLateCallbacksAndCommandsRoundTrip() throws {
    let runtime = FakePlatformRuntime(generation: 9)
    let bluetooth = FakeBluetoothPort()
    let location = FakeLocationPort()
    location.currentState = LocationProviderState(
      authorization: .authorized,
      backgroundCapture: .available
    )
    let adapter = RustCorePlatformAdapter(
      runtime: runtime,
      bluetooth: bluetooth,
      location: location,
      platformHost: FakeApplePlatformHost()
    )

    let start = try hostCommand(
      generation: 9,
      operation: 4,
      command: .locationStart(background: true)
    )
    XCTAssertEqual(
      try FloatViewPlatformCommandEnvelope.decode(start).command,
      .locationStart(background: true)
    )
    adapter.dispatch(start)
    XCTAssertEqual(runtime.completions.map(\.resultKind), [0])
    XCTAssertNotNil(location.handler)
    XCTAssertEqual(
      runtime.events[0].event,
      .locationAuthorization(.background)
    )

    location.handler?(LocationProviderObservation(
      observedAt: Date(), latitude: 1.25, longitude: -2.5, accuracyM: 4
    ))
    XCTAssertEqual(
      runtime.events[1].event,
      .location(latitude: 1.25, longitude: -2.5, accuracyM: 4)
    )
    XCTAssertEqual(
      runtime.events[1].adapterOrder,
      2
    )

    let retainedHandler = location.handler
    adapter.shutdown()
    retainedHandler?(LocationProviderObservation(
      observedAt: Date(), latitude: 3, longitude: 4, accuracyM: nil
    ))
    XCTAssertEqual(runtime.events.count, 2)
    XCTAssertEqual(location.stopCount, 1)
  }

  func testRemainingAppleCommandsAreBoundedAndLifecycleEventsStayOrdered() throws {
    let root = FileManager.default.temporaryDirectory.appending(
      path: "float-view-host-tests-\(UUID().uuidString)", directoryHint: .isDirectory
    )
    defer { try? FileManager.default.removeItem(at: root) }
    let capture = HostPresentationCapture()
    let host = ApplePlatformHostServices(
      durableRoot: root,
      backgroundLogging: capture.background,
      notificationPresenter: { true }
    )
    let runtime = FakePlatformRuntime(generation: 77)
    let adapter = RustCorePlatformAdapter(
      runtime: runtime,
      bluetooth: FakeBluetoothPort(),
      location: FakeLocationPort(),
      platformHost: host,
      runtimeReadyHandler: capture.ready
    )

    try adapter.dispatch(hostCommand(
      operation: 17,
      command: .setBackgroundLogging(enabled: true)
    ))
    try adapter.dispatch(hostCommand(
      operation: 22,
      command: .runtimeReady(
        endpoint: "http://127.0.0.1:41234",
        launchToken: "opaque-launch-token"
      )
    ))
    XCTAssertEqual(capture.backgroundValues, [true])
    XCTAssertEqual(capture.readyEndpoints, try [XCTUnwrap(URL(string: "http://127.0.0.1:41234"))])
    XCTAssertEqual(capture.readyTokens, ["opaque-launch-token"])

    adapter.updateLifecycle(foreground: false, backgroundEligible: true)
    adapter.updateLoggingService(active: true)
    adapter.updateTheme(.dark)
    let events = runtime.events.suffix(3)
    XCTAssertEqual(events.map(\.event), [
      .lifecycle(foreground: false, backgroundEligible: true),
      .loggingService(active: true),
      .theme(.dark),
    ])
    XCTAssertEqual(events.map(\.adapterOrder), [1, 2, 3])
  }

  private func hostCommand(
    generation: UInt64 = 77,
    operation: UInt64,
    command: FloatViewPlatformCommand
  ) throws -> FloatViewHostCommand {
    let payload = try TestCommandEncoder.encode(command)
    return FloatViewHostCommand(
      kind: command.bridgeKind,
      runtimeGeneration: generation,
      operationID: operation,
      payload: payload
    )
  }
}

private enum TestCommandEncoder {
  static func encode(_ command: FloatViewPlatformCommand) throws -> Data {
    var writer = Writer()
    switch command {
    case .bluetoothStartScan, .bluetoothStopScan, .locationStop, .presentSettings:
      break
    case let .bluetoothConnect(deviceToken):
      try writer.bytes(deviceToken, maximum: 256)
    case let .bluetoothDisconnect(connectionGeneration):
      writer.u64(connectionGeneration)
    case let .bluetoothWrite(connectionGeneration, bytes):
      writer.u64(connectionGeneration)
      try writer.bytes(bytes, maximum: FloatViewCoreRuntime.maximumPayloadBytes)
    case let .locationStart(background), let .setBackgroundLogging(background):
      writer.data.append(background ? 1 : 0)
    case let .presentPermission(permission):
      writer.data.append(permission.rawValue)
    case let .runtimeReady(endpoint, launchToken):
      try writer.bytes(Data(endpoint.utf8), maximum: 256)
      try writer.bytes(Data(launchToken.utf8), maximum: 128)
    }
    return writer.data
  }

  private struct Writer {
    var data = Data()

    mutating func u64(_ value: UInt64) {
      var encoded = value.bigEndian
      withUnsafeBytes(of: &encoded) { data.append(contentsOf: $0) }
    }

    mutating func bytes(_ value: Data, maximum: Int) throws {
      guard !value.isEmpty, value.count <= maximum, let count = UInt32(exactly: value.count) else {
        throw FloatViewBridgeCodecError.payloadTooLarge
      }
      var encoded = count.bigEndian
      withUnsafeBytes(of: &encoded) { data.append(contentsOf: $0) }
      data.append(value)
    }
  }
}

private final class FakePlatformRuntime: FloatViewPlatformRuntime, @unchecked Sendable {
  struct Event: Equatable {
    let runtimeGeneration: UInt64
    let connectionGeneration: UInt64
    let adapterOrder: UInt64
    let event: FloatViewPlatformEvent
  }

  struct Completion: Equatable {
    let operation: UInt64
    let resultKind: UInt32
    let payload: Data
  }

  struct Notification: Equatable {
    let connectionGeneration: UInt64
    let adapterOrder: UInt64
    let bytes: Data
  }

  let generation: UInt64
  private let lock = NSLock()
  private var storedEvents: [Event] = []
  private var storedNotifications: [Notification] = []
  private var storedCompletions: [Completion] = []

  init(generation: UInt64) {
    self.generation = generation
  }

  var events: [Event] {
    lock.withLock { storedEvents }
  }

  var notifications: [Notification] {
    lock.withLock { storedNotifications }
  }

  var completions: [Completion] {
    lock.withLock { storedCompletions }
  }

  func submitPlatformEvent(
    _ event: FloatViewPlatformEvent,
    connectionGeneration: UInt64,
    adapterOrder: UInt64
  ) {
    let value = Event(
      runtimeGeneration: generation,
      connectionGeneration: connectionGeneration,
      adapterOrder: adapterOrder,
      event: event
    )
    lock.withLock { storedEvents.append(value) }
  }

  func submitBluetoothBytes(
    connectionGeneration: UInt64,
    adapterOrder: UInt64,
    bytes: Data
  ) {
    lock.withLock {
      storedNotifications.append(.init(
        connectionGeneration: connectionGeneration,
        adapterOrder: adapterOrder,
        bytes: Data(bytes)
      ))
    }
  }

  func complete(_ command: FloatViewHostCommand, resultKind: UInt32, payload: Data) {
    lock.withLock {
      storedCompletions.append(.init(
        operation: command.operationID,
        resultKind: resultKind,
        payload: Data(payload)
      ))
    }
  }
}

private final class FakeBluetoothPort: BluetoothPort, @unchecked Sendable {
  private let lock = NSLock()
  private(set) var connected: [DeviceID] = []
  private(set) var writes: [(Data, DeviceID)] = []

  func currentState() -> BluetoothState {
    BluetoothState(radio: .poweredOn, scanPermission: .granted, connectPermission: .granted)
  }

  func requestPermission() {}
  func startDiscovery() {}
  func stopDiscovery() {}
  func connect(_ deviceID: DeviceID) {
    lock.withLock { connected.append(deviceID) }
  }

  func write(_ bytes: Data, to deviceID: DeviceID) {
    lock.withLock { writes.append((Data(bytes), deviceID)) }
  }

  func disconnect(_: DeviceID) {}
}

private final class FakeLocationPort: LocationProviderPort, @unchecked Sendable {
  var currentState = LocationProviderState(
    authorization: .notDetermined,
    backgroundCapture: .unavailable
  )
  var handler: (@Sendable (LocationProviderObservation) -> Void)?
  var stopCount = 0

  func state() -> LocationProviderState {
    currentState
  }

  func requestPermission() {}
  func openSettings() {}
  func startUpdates(_ handler: @escaping @Sendable (LocationProviderObservation) -> Void) {
    self.handler = handler
  }

  func stopUpdates() {
    stopCount += 1
    handler = nil
  }
}

private final class DiagnosticCapture: @unchecked Sendable {
  struct Value {
    let stage: String
    let code: String
  }

  private let lock = NSLock()
  private var stored: [Value] = []
  var values: [Value] {
    lock.withLock { stored }
  }

  func record(stage: String, code: String) {
    lock.withLock { stored.append(.init(stage: stage, code: code)) }
  }
}

private final class HostPresentationCapture: @unchecked Sendable {
  private let lock = NSLock()
  private var storedBackground: [Bool] = []
  private var storedReady: [(URL, String)] = []

  var backgroundValues: [Bool] {
    lock.withLock { storedBackground }
  }

  var readyEndpoints: [URL] {
    lock.withLock { storedReady.map(\.0) }
  }

  var readyTokens: [String] {
    lock.withLock { storedReady.map(\.1) }
  }

  func background(_ value: Bool) -> Bool {
    lock.withLock { storedBackground.append(value) }
    return true
  }

  func ready(_ endpoint: URL, _ token: String) {
    lock.withLock { storedReady.append((endpoint, token)) }
  }
}

private final class FakeApplePlatformHost: ApplePlatformHost, @unchecked Sendable {
  func setBackgroundLogging(_: Bool) -> Bool {
    true
  }
  func presentNotificationsPermission() -> Bool {
    true
  }
}
