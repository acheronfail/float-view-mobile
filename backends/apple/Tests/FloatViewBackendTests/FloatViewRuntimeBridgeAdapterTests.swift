@testable import FloatViewBackend
import FloatViewCoreBridge
import Foundation
import XCTest

final class FloatViewRuntimeBridgeAdapterTests: XCTestCase {
  func testBluetoothRelayWaitsForReadinessAndReplaysCurrentStateExactlyOnce() {
    let relay = FloatViewBluetoothEventRelay()
    let events = BluetoothEventCapture()
    relay.setHandler(events.record)
    relay.emit(.stateChanged(.init(
      radio: .unknown,
      scanPermission: .notDetermined,
      connectPermission: .notDetermined
    )))
    XCTAssertTrue(events.states.isEmpty)

    let ready = BluetoothState(
      radio: .poweredOn,
      scanPermission: .granted,
      connectPermission: .granted
    )
    relay.activate(initialState: ready)
    relay.activate(initialState: ready)
    XCTAssertEqual(events.states, [ready])

    let poweredOff = BluetoothState(
      radio: .poweredOff,
      scanPermission: .granted,
      connectPermission: .granted
    )
    relay.emit(.stateChanged(poweredOff))
    XCTAssertEqual(events.states, [ready, poweredOff])
  }

  func testLocationStateSinkSubmitsAuthorizationWithoutCoordinateOrPoll() {
    let runtime = FakePlatformRuntime(generation: 3)
    let location = FakeLocationPort()
    let adapter = FloatViewRuntimeBridgeAdapter(
      runtime: runtime,
      bluetooth: FakeBluetoothPort(),
      location: location,
      platformHost: FakeRuntimeHost()
    )

    location.emitState(.init(authorization: .authorized, backgroundCapture: .available))
    XCTAssertEqual(runtime.events.last?.event, .locationAuthorization(.background))
    withExtendedLifetime(adapter) {}
  }

  func testBluetoothMappingUsesPrivateAssociationsOrderedGenerationsAndCancellation() throws {
    let runtime = FakePlatformRuntime(generation: 77)
    let bluetooth = FakeBluetoothPort()
    let location = FakeLocationPort()
    let diagnostics = DiagnosticCapture()
    let adapter = FloatViewRuntimeBridgeAdapter(
      runtime: runtime,
      bluetooth: bluetooth,
      location: location,
      platformHost: FakeRuntimeHost(),
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
      command: .bluetoothConnect(connectionGeneration: 1, deviceToken: token)
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
    let adapter = FloatViewRuntimeBridgeAdapter(
      runtime: runtime,
      bluetooth: bluetooth,
      location: FakeLocationPort(),
      platformHost: FakeRuntimeHost()
    )
    let deviceID = DeviceID("repeat-connection")

    adapter.receiveBluetooth(.deviceFound(deviceID, name: "Test Board", rssi: -42))
    let discovery = try XCTUnwrap(runtime.events.last)
    guard case let .bluetoothDevice(token, _, _) = discovery.event else {
      return XCTFail("Expected Bluetooth device event")
    }

    try adapter.dispatch(hostCommand(
      operation: 1,
      command: .bluetoothConnect(connectionGeneration: 1, deviceToken: token)
    ))
    adapter.receiveBluetooth(.connectionChanged(deviceID, .ready))
    adapter.receiveBluetooth(.connectionChanged(deviceID, .disconnected))
    try adapter.dispatch(hostCommand(
      operation: 2,
      command: .bluetoothConnect(connectionGeneration: 2, deviceToken: token)
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
    let adapter = FloatViewRuntimeBridgeAdapter(
      runtime: runtime,
      bluetooth: bluetooth,
      location: location,
      platformHost: FakeRuntimeHost()
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
      latitude: 1.25, longitude: -2.5, accuracyM: 4
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
      latitude: 3, longitude: 4, accuracyM: nil
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
    let importPath = root.appending(path: "staging/import.floatview.tmp")
    let exportPath = root.appending(path: "exports/ride.floatview")
    try FileManager.default.createDirectory(
      at: exportPath.deletingLastPathComponent(), withIntermediateDirectories: true
    )
    try Data([1]).write(to: exportPath)
    let host = FloatViewRuntimeHost(
      durableRoot: root,
      backgroundLogging: capture.background,
      notificationPresenter: { true },
      documentPresenter: { destination, contentTypes, maximumBytes, completion in
        XCTAssertEqual(destination.standardizedFileURL, importPath.standardizedFileURL)
        XCTAssertEqual(contentTypes, ["application/octet-stream"])
        XCTAssertEqual(maximumBytes, 128 * 1024 * 1024)
        completion(.success(()))
      },
      exportPresenter: { stagedURL, completion in
        XCTAssertEqual(stagedURL.standardizedFileURL, exportPath.standardizedFileURL)
        completion(.success(()))
      }
    )
    let runtime = FakePlatformRuntime(generation: 77)
    let adapter = FloatViewRuntimeBridgeAdapter(
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
      operation: 23,
      command: .presentImport(
        stagingPath: importPath.path,
        allowedContentTypes: ["application/octet-stream"],
        maximumBytes: 128 * 1024 * 1024
      )
    ))
    try adapter.dispatch(hostCommand(
      operation: 24,
      command: .presentExport(
        stagingPath: exportPath.path,
        displayName: "ride.floatview",
        contentType: "application/octet-stream"
      )
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
    XCTAssertEqual(runtime.completions.suffix(2).map(\.resultKind), [0, 0])

    adapter.updateLifecycle(foreground: false, backgroundEligible: true)
    let events = runtime.events.suffix(1)
    XCTAssertEqual(events.map(\.event), [
      .lifecycle(foreground: false, backgroundEligible: true),
    ])
    XCTAssertEqual(events.map(\.adapterOrder), [1])
  }

  func testDocumentPresentationRejectsSymlinkEscapeBeforeCallingThePresenter() throws {
    let base = FileManager.default.temporaryDirectory.appending(
      path: "float-view-document-containment-\(UUID().uuidString)", directoryHint: .isDirectory
    )
    defer { try? FileManager.default.removeItem(at: base) }
    let root = base.appending(path: "root", directoryHint: .isDirectory)
    let outside = base.appending(path: "outside", directoryHint: .isDirectory)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
    try FileManager.default.createSymbolicLink(
      at: root.appending(path: "staging"), withDestinationURL: outside
    )
    let host = FloatViewRuntimeHost(
      durableRoot: root,
      documentPresenter: { _, _, _, _ in XCTFail("escaped path reached presenter") }
    )
    let completed = expectation(description: "rejected")
    host.presentImport(
      stagingPath: root.appending(path: "staging/import.tmp").path,
      allowedContentTypes: ["application/octet-stream"],
      maximumBytes: 1024
    ) { result in
      guard case .failure(.invalidPath) = result else {
        XCTFail("expected invalid path")
        completed.fulfill()
        return
      }
      completed.fulfill()
    }
    wait(for: [completed], timeout: 1)
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
    case let .bluetoothConnect(connectionGeneration, deviceToken):
      writer.u64(connectionGeneration)
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
    case let .presentImport(stagingPath, allowedContentTypes, maximumBytes):
      try writer.bytes(Data(stagingPath.utf8), maximum: 4096)
      writer.data.append(UInt8(allowedContentTypes.count))
      for contentType in allowedContentTypes {
        try writer.bytes(Data(contentType.utf8), maximum: 128)
      }
      writer.u64(maximumBytes)
    case let .presentExport(stagingPath, displayName, contentType):
      try writer.bytes(Data(stagingPath.utf8), maximum: 4096)
      try writer.bytes(Data(displayName.utf8), maximum: 255)
      try writer.bytes(Data(contentType.utf8), maximum: 128)
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
  var stateHandler: (@Sendable (LocationProviderState) -> Void)?
  var stopCount = 0

  func state() -> LocationProviderState {
    currentState
  }

  func requestPermission() {}
  func setStateHandler(_ handler: @escaping @Sendable (LocationProviderState) -> Void) {
    stateHandler = handler
  }

  func openSettings() {}
  func startUpdates(_ handler: @escaping @Sendable (LocationProviderObservation) -> Void) {
    self.handler = handler
  }

  func stopUpdates() {
    stopCount += 1
    handler = nil
  }

  func emitState(_ state: LocationProviderState) {
    currentState = state
    stateHandler?(state)
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

private final class BluetoothEventCapture: @unchecked Sendable {
  private let lock = NSLock()
  private var storedStates: [BluetoothState] = []

  var states: [BluetoothState] {
    lock.withLock { storedStates }
  }

  func record(_ event: BluetoothEvent) {
    guard case let .stateChanged(state) = event else { return }
    lock.withLock { storedStates.append(state) }
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

private final class FakeRuntimeHost: FloatViewRuntimeHostProtocol, @unchecked Sendable {
  func setBackgroundLogging(_: Bool) -> Bool {
    true
  }

  func presentNotificationsPermission() -> Bool {
    true
  }
}
