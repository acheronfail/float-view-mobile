@testable import FloatViewBackend
import FloatViewCoreBridge
import Foundation
import XCTest

final class RustCorePlatformAdapterTests: XCTestCase {
  func testBluetoothMappingUsesOpaqueTokensOrderedGenerationsAndCancellation() throws {
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
    let discovery = try XCTUnwrap(runtime.events.last.map(FloatViewPlatformEventEnvelope.decode))
    guard case let .bluetoothDevice(token, name, rssi) = discovery.event else {
      return XCTFail("Expected Bluetooth device event")
    }
    XCTAssertEqual(discovery.runtimeGeneration, 77)
    XCTAssertEqual(discovery.adapterOrder, 1)
    XCTAssertEqual(token.count, 16)
    XCTAssertFalse(String(decoding: token, as: UTF8.self).contains(deviceID.value))
    XCTAssertEqual(name, "Test Board")
    XCTAssertEqual(rssi, -42)

    let connect = try hostCommand(
      operation: 1,
      command: .bluetoothConnect(deviceToken: token)
    )
    adapter.dispatch(connect)
    XCTAssertEqual(bluetooth.connected, [deviceID])
    XCTAssertTrue(runtime.completions.isEmpty)

    adapter.receiveBluetooth(.connectionChanged(deviceID, .ready))
    let connection = try XCTUnwrap(runtime.events.last.map(FloatViewPlatformEventEnvelope.decode))
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
    let discovery = try XCTUnwrap(runtime.events.last.map(FloatViewPlatformEventEnvelope.decode))
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
    let connectionEvents = try runtime.events
      .map(FloatViewPlatformEventEnvelope.decode)
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
      try FloatViewPlatformEventEnvelope.decode(runtime.events[0]).event,
      .locationAuthorization(.background)
    )

    location.handler?(LocationProviderObservation(
      observedAt: Date(), latitude: 1.25, longitude: -2.5, accuracyM: 4
    ))
    XCTAssertEqual(
      try FloatViewPlatformEventEnvelope.decode(runtime.events[1]).event,
      .location(latitude: 1.25, longitude: -2.5, accuracyM: 4)
    )
    XCTAssertEqual(
      try FloatViewPlatformEventEnvelope.decode(runtime.events[1]).adapterOrder,
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
    let resources = root.appending(path: "resources", directoryHint: .isDirectory)
    let storage = root.appending(path: "storage", directoryHint: .isDirectory)
    try FileManager.default.createDirectory(at: resources, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: storage, withIntermediateDirectories: true)
    let frontend = Data(repeating: 0x5A, count: 475_465)
    try frontend.write(to: resources.appending(path: "index.html"))
    let source = resources.appending(path: "import.floatview")
    try Data([7, 8, 9]).write(to: source)
    let export = storage.appending(path: "export.floatview")
    try Data([4, 5, 6]).write(to: export)
    let capture = HostPresentationCapture()
    let host = ApplePlatformHostServices(
      resourceRoots: [.frontend: resources, .sql: resources, .softwareFixture: resources],
      storageRoots: [.durable: storage, .cache: storage, .temporary: storage],
      backgroundLogging: capture.background,
      documentPresenter: { destination, maximum, completion in
        do {
          try ApplePlatformHostServices.copySecurityScopedDocument(
            source: source,
            destination: destination,
            maximumBytes: maximum
          )
          completion(.success(()))
        } catch let error as ApplePlatformHostError {
          completion(.failure(error))
        } catch {
          completion(.failure(.ioFailure))
        }
      },
      notificationPresenter: { true },
      themePresenter: capture.theme,
      servicePresenter: capture.service,
      exportPresenter: capture.export,
      diagnosticHandler: capture.diagnostic
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
      operation: 10,
      command: .readResource(
        namespace: .frontend,
        logicalName: "index.html",
        maximumBytes: 8 * 1024 * 1024
      )
    ))
    XCTAssertEqual(runtime.completions.last?.payload, frontend)
    XCTAssertThrowsError(try host.readResource(
      namespace: .frontend,
      logicalName: "../secret",
      maximumBytes: 100
    ))

    let staged = storage.appending(path: "staging/import.floatview")
    try adapter.dispatch(hostCommand(
      operation: 11,
      command: .stageDocument(destination: staged.path, maximumBytes: 1024)
    ))
    XCTAssertEqual(try Data(contentsOf: staged), Data([7, 8, 9]))
    try adapter.dispatch(hostCommand(
      operation: 12,
      command: .resolveStorageRoot(kind: .durable)
    ))
    XCTAssertEqual(
      try String(data: XCTUnwrap(runtime.completions.last?.payload), encoding: .utf8),
      storage.path
    )
    try adapter.dispatch(hostCommand(operation: 13, command: .availableSpace(root: storage.path)))
    XCTAssertEqual(runtime.completions.last?.payload.count, 8)
    try adapter.dispatch(hostCommand(operation: 14, command: .monotonicClock))
    XCTAssertEqual(runtime.completions.last?.payload.count, 8)
    try adapter.dispatch(hostCommand(operation: 15, command: .wallClock))
    XCTAssertEqual(runtime.completions.last?.payload.count, 8)
    try adapter.dispatch(hostCommand(operation: 16, command: .secureRandom(count: 32)))
    XCTAssertEqual(runtime.completions.last?.payload.count, 32)
    try adapter.dispatch(hostCommand(
      operation: 17,
      command: .setBackgroundLogging(enabled: true)
    ))
    try adapter.dispatch(hostCommand(
      operation: 18,
      command: .diagnostic(stage: "storage", code: "open_failed")
    ))
    try adapter.dispatch(hostCommand(operation: 19, command: .presentTheme(theme: .dark)))
    try adapter.dispatch(hostCommand(
      operation: 20,
      command: .presentServiceState(active: true)
    ))
    try adapter.dispatch(hostCommand(
      operation: 21,
      command: .presentExport(stagedPath: export.path)
    ))
    try adapter.dispatch(hostCommand(
      operation: 22,
      command: .runtimeReady(
        endpoint: "http://127.0.0.1:41234",
        launchToken: "opaque-launch-token"
      )
    ))
    XCTAssertEqual(capture.backgroundValues, [true])
    XCTAssertEqual(capture.themes, [.dark])
    XCTAssertEqual(capture.serviceValues, [true])
    XCTAssertEqual(capture.exportURLs, [export])
    XCTAssertEqual(capture.readyEndpoints, try [XCTUnwrap(URL(string: "http://127.0.0.1:41234"))])
    XCTAssertEqual(capture.readyTokens, ["opaque-launch-token"])
    XCTAssertEqual(capture.diagnostics.count, 1)

    adapter.updateLifecycle(foreground: false, backgroundEligible: true)
    adapter.updateLoggingService(active: true)
    adapter.updateTheme(.dark)
    let events = try runtime.events.suffix(3).map(FloatViewPlatformEventEnvelope.decode)
    XCTAssertEqual(events.map(\.event), [
      .lifecycle(foreground: false, backgroundEligible: true),
      .loggingService(active: true),
      .theme(.dark),
    ])
    XCTAssertEqual(events.map(\.adapterOrder), [1, 2, 3])
  }

  func testShutdownCancelsPendingPresentationAndIgnoresLateResult() throws {
    let runtime = FakePlatformRuntime(generation: 77)
    let host = FakeApplePlatformHost(holdPresentations: true)
    let adapter = RustCorePlatformAdapter(
      runtime: runtime,
      bluetooth: FakeBluetoothPort(),
      location: FakeLocationPort(),
      platformHost: host
    )
    try adapter.dispatch(hostCommand(
      operation: 40,
      command: .stageDocument(destination: "/tmp/float-view-stage", maximumBytes: 1024)
    ))
    XCTAssertTrue(runtime.completions.isEmpty)
    XCTAssertNotNil(host.pendingCompletion)

    adapter.shutdown()
    XCTAssertEqual(runtime.completions.map(\.resultKind), [1])
    XCTAssertEqual(host.cancelCount, 1)
    host.pendingCompletion?(.success(()))
    XCTAssertEqual(runtime.completions.map(\.resultKind), [1])
  }

  private func hostCommand(
    generation: UInt64 = 77,
    operation: UInt64,
    command: FloatViewPlatformCommand
  ) throws -> FloatViewHostCommand {
    let payload = try FloatViewPlatformCommandEnvelope(
      runtimeGeneration: generation,
      operationID: operation,
      command: command
    ).encode()
    return FloatViewHostCommand(
      kind: floatViewPlatformCommandKind,
      runtimeGeneration: generation,
      operationID: operation,
      payload: payload
    )
  }
}

private final class FakePlatformRuntime: FloatViewPlatformRuntime, @unchecked Sendable {
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
  private var storedEvents: [Data] = []
  private var storedNotifications: [Notification] = []
  private var storedCompletions: [Completion] = []

  init(generation: UInt64) {
    self.generation = generation
  }

  var events: [Data] {
    lock.withLock { storedEvents }
  }

  var notifications: [Notification] {
    lock.withLock { storedNotifications }
  }

  var completions: [Completion] {
    lock.withLock { storedCompletions }
  }

  func submitPlatformEvent(_ payload: Data) {
    lock.withLock { storedEvents.append(Data(payload)) }
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
  private var storedThemes: [FloatViewThemePreference] = []
  private var storedService: [Bool] = []
  private var storedExports: [URL] = []
  private var storedDiagnostics: [(String, String)] = []
  private var storedReady: [(URL, String)] = []

  var backgroundValues: [Bool] {
    lock.withLock { storedBackground }
  }

  var themes: [FloatViewThemePreference] {
    lock.withLock { storedThemes }
  }

  var serviceValues: [Bool] {
    lock.withLock { storedService }
  }

  var exportURLs: [URL] {
    lock.withLock { storedExports }
  }

  var diagnostics: [(String, String)] {
    lock.withLock { storedDiagnostics }
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

  func theme(_ value: FloatViewThemePreference) -> Bool {
    lock.withLock { storedThemes.append(value) }
    return true
  }

  func service(_ value: Bool) -> Bool {
    lock.withLock { storedService.append(value) }
    return true
  }

  func export(
    _ url: URL,
    _ completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  ) {
    lock.withLock { storedExports.append(url) }
    completion(.success(()))
  }

  func diagnostic(_ stage: String, _ code: String) {
    lock.withLock { storedDiagnostics.append((stage, code)) }
  }

  func ready(_ endpoint: URL, _ token: String) {
    lock.withLock { storedReady.append((endpoint, token)) }
  }
}

private final class FakeApplePlatformHost: ApplePlatformHost, @unchecked Sendable {
  private let holdPresentations: Bool
  private let lock = NSLock()
  private var storedPending: (@Sendable (Result<Void, ApplePlatformHostError>) -> Void)?
  private var storedCancelCount = 0

  init(holdPresentations: Bool = false) {
    self.holdPresentations = holdPresentations
  }

  var pendingCompletion: (@Sendable (Result<Void, ApplePlatformHostError>) -> Void)? {
    lock.withLock { storedPending }
  }

  var cancelCount: Int {
    lock.withLock { storedCancelCount }
  }

  func setBackgroundLogging(_: Bool) -> Bool {
    true
  }

  func readResource(
    namespace _: FloatViewResourceNamespace,
    logicalName _: String,
    maximumBytes _: UInt64
  ) throws -> Data {
    Data([1])
  }

  func stageDocument(
    destination _: URL,
    maximumBytes _: UInt64,
    completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  ) {
    if holdPresentations { lock.withLock { storedPending = completion } }
    else { completion(.success(())) }
  }

  func storageRoot(_: FloatViewStorageRootKind) throws -> URL {
    FileManager.default.temporaryDirectory
  }

  func availableSpace(at _: URL) throws -> UInt64 {
    1
  }

  func monotonicMilliseconds() -> UInt64 {
    2
  }

  func wallMilliseconds() -> Int64 {
    3
  }

  func secureRandom(count: UInt16) throws -> Data {
    Data(repeating: 4, count: Int(count))
  }

  func diagnostic(stage _: String, code _: String) {}
  func presentNotificationsPermission() -> Bool {
    true
  }

  func presentTheme(_: FloatViewThemePreference) -> Bool {
    true
  }

  func presentServiceState(active _: Bool) -> Bool {
    true
  }

  func presentExport(
    stagedURL _: URL,
    completion: @escaping @Sendable (Result<Void, ApplePlatformHostError>) -> Void
  ) {
    if holdPresentations { lock.withLock { storedPending = completion } }
    else { completion(.success(())) }
  }

  func cancelPendingPresentation() {
    lock.withLock { storedCancelCount += 1 }
  }
}
