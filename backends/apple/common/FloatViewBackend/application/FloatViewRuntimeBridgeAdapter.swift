#if canImport(FloatViewCoreBridge)
  import FloatViewCoreBridge
#endif
import Foundation

/// Maps the existing Apple Bluetooth and location ports onto the private Rust
/// platform vocabulary. It owns no protocol, persistence, or application state.
public final class FloatViewRuntimeBridgeAdapter: @unchecked Sendable {
  public typealias DiagnosticHandler = @Sendable (_ stage: String, _ code: String) -> Void
  public typealias RuntimeReadyHandler = @Sendable (_ endpoint: URL, _ launchToken: String) -> Void

  private let runtime: any FloatViewPlatformRuntime
  private let bluetooth: any BluetoothPort
  private let location: any LocationProviderPort
  private let platformHost: any FloatViewRuntimeHostProtocol
  private let diagnosticHandler: DiagnosticHandler
  private let runtimeReadyHandler: RuntimeReadyHandler
  private let lock = NSLock()
  private var adapterOrder: UInt64 = 0
  private var connectionGeneration: UInt64 = 0
  private var activeDeviceID: DeviceID?
  private var locationActive = false
  private var stopped = false

  public init(
    runtime: any FloatViewPlatformRuntime,
    bluetooth: any BluetoothPort,
    location: any LocationProviderPort,
    platformHost: any FloatViewRuntimeHostProtocol,
    diagnosticHandler: @escaping DiagnosticHandler = { _, _ in },
    runtimeReadyHandler: @escaping RuntimeReadyHandler = { _, _ in }
  ) {
    self.runtime = runtime
    self.bluetooth = bluetooth
    self.location = location
    self.platformHost = platformHost
    self.diagnosticHandler = diagnosticHandler
    self.runtimeReadyHandler = runtimeReadyHandler
    location.setStateHandler { [weak self] state in
      self?.submitLocationAuthorization(state.authorization)
    }
  }

  /// Entry point for the runtime's dedicated off-main command-pump handler.
  public func dispatch(_ command: FloatViewHostCommand) {
    guard !lock.withLock({ stopped }) else {
      complete(command, resultKind: 1)
      return
    }
    let envelope: FloatViewPlatformCommandEnvelope
    do {
      envelope = try FloatViewPlatformCommandEnvelope.decode(command)
    } catch {
      diagnose(stage: "bridge_command", code: "invalid_payload")
      complete(command, resultKind: 4)
      return
    }

    switch envelope.command {
    case .bluetoothStartScan:
      bluetooth.startDiscovery()
      complete(command)
    case .bluetoothStopScan:
      bluetooth.stopDiscovery()
      complete(command)
    case let .bluetoothConnect(expectedGeneration, deviceToken):
      guard let value = String(data: deviceToken, encoding: .utf8), !value.isEmpty else {
        diagnose(stage: "bluetooth_connect", code: "unknown_token")
        complete(command, resultKind: 3)
        return
      }
      let deviceID = DeviceID(value)
      lock.withLock {
        activeDeviceID = deviceID
        connectionGeneration = expectedGeneration
      }
      trace("connect generation=\(currentConnectionGeneration)")
      bluetooth.connect(deviceID)
      complete(command)
    case let .bluetoothDisconnect(expectedGeneration):
      let context = lock.withLock { (connectionGeneration, activeDeviceID) }
      guard expectedGeneration == context.0, let deviceID = context.1 else {
        complete(command, resultKind: 1)
        return
      }
      bluetooth.disconnect(deviceID)
      complete(command)
    case let .bluetoothWrite(expectedGeneration, bytes):
      let context = lock.withLock { (connectionGeneration, activeDeviceID) }
      guard expectedGeneration == context.0, let deviceID = context.1 else {
        let code = if expectedGeneration < context.0 {
          "stale_generation_older"
        } else if expectedGeneration > context.0 {
          "stale_generation_newer"
        } else {
          "stale_generation_inactive"
        }
        diagnose(stage: "bluetooth_write", code: code)
        complete(command, resultKind: 1)
        return
      }
      guard !bytes.isEmpty, bytes.count <= FloatViewCoreRuntime.maximumPayloadBytes else {
        complete(command, resultKind: 4)
        return
      }
      trace("write generation=\(expectedGeneration)")
      bluetooth.write(Data(bytes), to: deviceID)
      complete(command)
    case let .locationStart(background):
      let state = location.state()
      submitLocationAuthorization(state.authorization)
      guard state.authorization == .authorized || state.authorization == .foregroundOnly else {
        complete(command, resultKind: state.authorization == .denied ? 2 : 3)
        return
      }
      guard !background || state.backgroundCapture == .available else {
        complete(command, resultKind: 2)
        return
      }
      lock.withLock { locationActive = true }
      location.startUpdates { [weak self] observation in
        self?.receiveLocation(observation)
      }
      complete(command)
    case .locationStop:
      lock.withLock { locationActive = false }
      location.stopUpdates()
      complete(command)
    case let .setBackgroundLogging(enabled):
      complete(command, resultKind: platformHost.setBackgroundLogging(enabled) ? 0 : 3)
    case let .presentPermission(permission):
      let presented = switch permission {
      case .bluetooth:
        { bluetooth.requestPermission(); return true }()
      case .location:
        { location.requestPermission(); return true }()
      case .backgroundLocation:
        { location.openSettings(); return true }()
      case .notifications:
        platformHost.presentNotificationsPermission()
      }
      complete(command, resultKind: presented ? 0 : 3)
    case .presentSettings:
      location.openSettings()
      complete(command)
    case let .runtimeReady(endpoint, launchToken):
      guard let url = URL(string: endpoint),
            url.host == "127.0.0.1" || url.host == "localhost",
            !launchToken.isEmpty
      else {
        complete(command, resultKind: 4)
        return
      }
      runtimeReadyHandler(url, launchToken)
      complete(command)
    case let .presentImport(stagingPath, allowedContentTypes, maximumBytes):
      platformHost.presentImport(
        stagingPath: stagingPath,
        allowedContentTypes: allowedContentTypes,
        maximumBytes: maximumBytes
      ) { [weak self] result in
        self?.completeDocument(command, result)
      }
    case let .presentExport(stagingPath, displayName, contentType):
      platformHost.presentExport(
        stagingPath: stagingPath,
        displayName: displayName,
        contentType: contentType
      ) { [weak self] result in
        self?.completeDocument(command, result)
      }
    }
  }

  /// Receives copied events from `CoreBluetoothAdapter`'s existing callback.
  public func receiveBluetooth(_ event: BluetoothEvent) {
    guard !lock.withLock({ stopped }) else { return }
    switch event {
    case let .stateChanged(state):
      submit(
        .bluetoothState(
          radio: Self.radio(state.radio),
          authorization: Self.authorization(state.connectPermission)
        )
      )
    case let .deviceFound(deviceID, name, rssi):
      submit(
        .bluetoothDevice(
          deviceToken: Data(deviceID.value.utf8),
          name: Self.boundedName(name),
          rssi: Int16(clamping: rssi ?? 0)
        )
      )
    case let .scanFailed(reason):
      diagnose(stage: "bluetooth_scan", code: Self.safeCode(reason))
    case let .connectionChanged(deviceID, state):
      guard lock.withLock({ activeDeviceID == deviceID }) else { return }
      let mapped = Self.connection(state)
      trace("connection generation=\(currentConnectionGeneration) state=\(mapped.rawValue)")
      submit(.bluetoothConnection(connectionGeneration: currentConnectionGeneration, state: mapped))
      if mapped == .failed || mapped == .disconnected {
        lock.withLock {
          if mapped == .failed || mapped == .disconnected { activeDeviceID = nil }
        }
      }
    case let .notification(deviceID, bytes):
      let context = nextContext(for: deviceID)
      guard let context, !bytes.isEmpty, bytes.count <= floatViewMaximumBluetoothBytes else {
        if bytes.count > floatViewMaximumBluetoothBytes {
          diagnose(stage: "bluetooth_notify", code: "payload_too_large")
        }
        return
      }
      do {
        try runtime.submitBluetoothBytes(
          connectionGeneration: context.connection,
          adapterOrder: context.order,
          bytes: Data(bytes)
        )
      } catch {
        diagnose(stage: "bluetooth_notify", code: "submit_failed")
      }
    }
  }

  public func refreshLocationAuthorization() {
    submitLocationAuthorization(location.state().authorization)
  }

  public func updateLifecycle(foreground: Bool, backgroundEligible: Bool) {
    submit(.lifecycle(foreground: foreground, backgroundEligible: backgroundEligible))
  }

  /// Cancels host work before the Rust runtime is stopped and its waiter wakes.
  public func shutdown() {
    let state = lock.withLock {
      () -> DeviceID? in
      guard !stopped else { return nil }
      stopped = true
      locationActive = false
      let state = activeDeviceID
      activeDeviceID = nil
      return state
    }
    bluetooth.stopDiscovery()
    if let deviceID = state { bluetooth.disconnect(deviceID) }
    location.stopUpdates()
  }

  private var currentConnectionGeneration: UInt64 {
    lock.withLock { connectionGeneration }
  }

  private func receiveLocation(_ observation: LocationProviderObservation) {
    guard lock.withLock({ locationActive && !stopped }) else { return }
    submit(
      .location(
        latitude: observation.latitude,
        longitude: observation.longitude,
        accuracyM: observation.accuracyM
      )
    )
  }

  private func submitLocationAuthorization(_ value: LocationProviderAuthorization) {
    let mapped: FloatViewLocationAuthorization = switch value {
    case .notDetermined, .unavailable: .unknown
    case .denied, .restricted: .denied
    case .foregroundOnly: .foreground
    case .authorized: .background
    }
    submit(.locationAuthorization(mapped))
  }

  private func submit(_ event: FloatViewPlatformEvent) {
    let context = nextContext()
    do {
      try runtime.submitPlatformEvent(
        event,
        connectionGeneration: context.connection,
        adapterOrder: context.order
      )
    } catch {
      diagnose(stage: "platform_event", code: "submit_failed")
    }
  }

  private func nextContext(for deviceID: DeviceID? = nil) -> (connection: UInt64, order: UInt64)? {
    lock.withLock {
      guard !stopped, deviceID == nil || deviceID == activeDeviceID else { return nil }
      adapterOrder &+= 1
      return (connectionGeneration, adapterOrder)
    }
  }

  private func nextContext() -> (connection: UInt64, order: UInt64) {
    nextContext(for: nil) ?? (currentConnectionGeneration, 0)
  }

  private func complete(
    _ command: FloatViewHostCommand,
    resultKind: UInt32 = 0,
    payload: Data = Data()
  ) {
    do {
      try runtime.complete(command, resultKind: resultKind, payload: payload)
    } catch {
      diagnose(stage: "bridge_completion", code: "rejected")
    }
  }

  private func completeDocument(
    _ command: FloatViewHostCommand,
    _ result: Result<Void, FloatViewDocumentError>
  ) {
    let resultKind: UInt32 = switch result {
    case .success: 0
    case .failure(.cancelled): 1
    case .failure(.unavailable): 3
    case .failure(.invalidPath), .failure(.payloadTooLarge), .failure(.ioFailure): 4
    }
    complete(command, resultKind: resultKind)
  }

  private func diagnose(stage: String, code: String) {
    diagnosticHandler(Self.safeCode(stage), Self.safeCode(code))
  }

  private func trace(_ message: String) {
    guard ProcessInfo.processInfo.environment["FLOAT_VIEW_CONFORMANCE_TRACE"] == "1" else { return }
    FileHandle.standardError.write(Data("[FloatViewApple] \(message)\n".utf8))
  }

  private static func radio(_ value: BluetoothRadioState) -> FloatViewBluetoothRadio {
    switch value {
    case .poweredOn: .poweredOn
    case .poweredOff: .poweredOff
    case .unknown, .unsupported, .resetting: .unknown
    }
  }

  private static func authorization(
    _ value: BluetoothPermissionState
  ) -> FloatViewBluetoothAuthorization {
    switch value {
    case .granted, .notRequired: .granted
    case .denied, .restricted: .denied
    case .unknown, .notDetermined: .unknown
    }
  }

  private static func connection(_ value: ConnectionState) -> FloatViewBluetoothConnectionState {
    switch value {
    case .ready: .ready
    case .failed: .failed
    case .disconnected: .disconnected
    }
  }

  private static func boundedName(_ value: String?) -> String? {
    guard let value else { return nil }
    var bytes = Data(value.utf8.prefix(128))
    while !bytes.isEmpty, String(data: bytes, encoding: .utf8) == nil {
      bytes.removeLast()
    }
    return String(data: bytes, encoding: .utf8)
  }

  private static func safeCode(_ value: String) -> String {
    let safe = value.lowercased().unicodeScalars.map { scalar -> Character in
      let allowed = ("a" ... "z").contains(Character(String(scalar)))
        || ("0" ... "9").contains(Character(String(scalar)))
        || scalar == "_" || scalar == "-" || scalar == "."
      return allowed ? Character(String(scalar)) : "_"
    }
    return String(safe.prefix(floatViewMaximumDiagnosticBytes))
  }
}

/// Shared iOS/macOS owner for one Rust runtime, one command pump, and the
/// platform capability adapters that must outlive disposable presentation.
public final class FloatViewAppleRuntime: @unchecked Sendable {
  public typealias BluetoothFactory = @Sendable (
    _ eventHandler: @escaping @Sendable (BluetoothEvent) -> Void
  ) -> any BluetoothPort

  private let runtime: FloatViewCoreRuntime
  private let adapter: FloatViewRuntimeBridgeAdapter
  private let relay: FloatViewBluetoothEventRelay
  private let retainedPresentation: AnyObject?
  private let backgroundEligibility: @Sendable () -> Bool

  public var generation: UInt64 {
    runtime.generation
  }

  public init(
    requestedPort: UInt16,
    softwareTransport: Bool,
    storageRoot: URL,
    frontendIndex: Data,
    bluetoothFactory: BluetoothFactory,
    location: any LocationProviderPort,
    platformHost: any FloatViewRuntimeHostProtocol,
    retainedPresentation: AnyObject? = nil,
    backgroundEligibility: @escaping @Sendable () -> Bool,
    diagnosticHandler: @escaping FloatViewRuntimeBridgeAdapter.DiagnosticHandler = { _, _ in },
    runtimeReadyHandler: @escaping FloatViewRuntimeBridgeAdapter.RuntimeReadyHandler
  ) throws {
    let runtime = try FloatViewCoreRuntime()
    let relay = FloatViewBluetoothEventRelay()
    let bluetooth = bluetoothFactory(relay.emit)
    let adapter = FloatViewRuntimeBridgeAdapter(
      runtime: runtime,
      bluetooth: bluetooth,
      location: location,
      platformHost: platformHost,
      diagnosticHandler: diagnosticHandler,
      runtimeReadyHandler: runtimeReadyHandler
    )
    self.runtime = runtime
    self.adapter = adapter
    self.relay = relay
    self.retainedPresentation = retainedPresentation
    self.backgroundEligibility = backgroundEligibility
    try runtime.startConfigured(
      requestedPort: requestedPort,
      softwareTransport: softwareTransport,
      storageRoot: storageRoot,
      frontendIndex: frontendIndex,
      handler: adapter.dispatch
    )
    relay.setHandler(adapter.receiveBluetooth)
  }

  public func updateLifecycle(foreground: Bool) {
    adapter.updateLifecycle(
      foreground: foreground,
      backgroundEligible: backgroundEligibility()
    )
  }

  public func shutdown() {
    adapter.shutdown()
    try? runtime.stop()
  }
}

private final class FloatViewBluetoothEventRelay: @unchecked Sendable {
  private let lock = NSLock()
  private var handler: (@Sendable (BluetoothEvent) -> Void)?

  func setHandler(_ value: @escaping @Sendable (BluetoothEvent) -> Void) {
    lock.withLock { handler = value }
  }

  func emit(_ event: BluetoothEvent) {
    lock.withLock { handler }?(event)
  }
}
