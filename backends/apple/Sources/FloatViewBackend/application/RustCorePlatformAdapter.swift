#if canImport(FloatViewCoreBridge)
  import FloatViewCoreBridge
#endif
import Foundation

/// Maps the existing Apple Bluetooth and location ports onto the private Rust
/// platform vocabulary. It owns no protocol, persistence, or application state.
public final class RustCorePlatformAdapter: @unchecked Sendable {
  public typealias DiagnosticHandler = @Sendable (_ stage: String, _ code: String) -> Void
  public typealias RuntimeReadyHandler = @Sendable (_ endpoint: URL, _ launchToken: String) -> Void

  private let runtime: any FloatViewPlatformRuntime
  private let bluetooth: any BluetoothPort
  private let location: any LocationProviderPort
  private let platformHost: any ApplePlatformHost
  private let diagnosticHandler: DiagnosticHandler
  private let runtimeReadyHandler: RuntimeReadyHandler
  private let lock = NSLock()
  private var adapterOrder: UInt64 = 0
  private var connectionGeneration: UInt64 = 0
  private var activeDeviceID: DeviceID?
  private var tokenByDeviceID: [DeviceID: Data] = [:]
  private var deviceIDByToken: [Data: DeviceID] = [:]
  private var pendingConnect: FloatViewHostCommand?
  private var locationActive = false
  private var pendingPresentations: [UInt64: FloatViewHostCommand] = [:]
  private var stopped = false

  public init(
    runtime: any FloatViewPlatformRuntime,
    bluetooth: any BluetoothPort,
    location: any LocationProviderPort,
    platformHost: any ApplePlatformHost,
    diagnosticHandler: @escaping DiagnosticHandler = { _, _ in },
    runtimeReadyHandler: @escaping RuntimeReadyHandler = { _, _ in }
  ) {
    self.runtime = runtime
    self.bluetooth = bluetooth
    self.location = location
    self.platformHost = platformHost
    self.diagnosticHandler = diagnosticHandler
    self.runtimeReadyHandler = runtimeReadyHandler
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
    case let .bluetoothConnect(deviceToken):
      guard let deviceID = lock.withLock({ deviceIDByToken[Data(deviceToken)] }) else {
        diagnose(stage: "bluetooth_connect", code: "unknown_token")
        complete(command, resultKind: 3)
        return
      }
      let replaced = lock.withLock { () -> FloatViewHostCommand? in
        let replaced = pendingConnect
        pendingConnect = command
        activeDeviceID = deviceID
        connectionGeneration &+= 1
        return replaced
      }
      trace("connect generation=\(currentConnectionGeneration)")
      if let replaced { complete(replaced, resultKind: 1) }
      bluetooth.connect(deviceID)
    case let .bluetoothDisconnect(expectedGeneration):
      let context = lock.withLock { (connectionGeneration, activeDeviceID, pendingConnect) }
      guard expectedGeneration == context.0, let deviceID = context.1 else {
        complete(command, resultKind: 1)
        return
      }
      if let pending = context.2 {
        lock.withLock { pendingConnect = nil }
        complete(pending, resultKind: 1)
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
    case let .readResource(namespace, logicalName, maximumBytes):
      do {
        let data = try platformHost.readResource(
          namespace: namespace,
          logicalName: logicalName,
          maximumBytes: maximumBytes
        )
        complete(command, payload: data)
      } catch {
        complete(command, resultKind: Self.resultKind(error))
      }
    case let .stageDocument(destination, maximumBytes):
      guard let destinationURL = Self.fileURL(destination), beginPresentation(command) else {
        complete(command, resultKind: 4)
        return
      }
      platformHost.stageDocument(destination: destinationURL, maximumBytes: maximumBytes) {
        [weak self] result in self?.finishPresentation(command, result: result)
      }
    case let .resolveStorageRoot(kind):
      do {
        let root = try platformHost.storageRoot(kind)
        complete(command, payload: Data(root.path.utf8))
      } catch {
        complete(command, resultKind: Self.resultKind(error))
      }
    case let .availableSpace(root):
      guard let rootURL = Self.fileURL(root) else {
        complete(command, resultKind: 4)
        return
      }
      do {
        try complete(command, payload: Self.u64(platformHost.availableSpace(at: rootURL)))
      } catch {
        complete(command, resultKind: Self.resultKind(error))
      }
    case .monotonicClock:
      complete(command, payload: Self.u64(platformHost.monotonicMilliseconds()))
    case .wallClock:
      complete(command, payload: Self.i64(platformHost.wallMilliseconds()))
    case let .secureRandom(count):
      do {
        try complete(command, payload: platformHost.secureRandom(count: count))
      } catch {
        complete(command, resultKind: Self.resultKind(error))
      }
    case let .diagnostic(stage, code):
      platformHost.diagnostic(stage: Self.safeCode(stage), code: Self.safeCode(code))
      complete(command)
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
    case let .presentTheme(theme):
      complete(command, resultKind: platformHost.presentTheme(theme) ? 0 : 3)
    case let .presentServiceState(active):
      complete(command, resultKind: platformHost.presentServiceState(active: active) ? 0 : 3)
    case let .presentExport(stagedPath):
      guard let stagedURL = Self.fileURL(stagedPath), beginPresentation(command) else {
        complete(command, resultKind: 4)
        return
      }
      platformHost.presentExport(stagedURL: stagedURL) { [weak self] result in
        self?.finishPresentation(command, result: result)
      }
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
      let token = opaqueToken(for: deviceID)
      submit(
        .bluetoothDevice(
          deviceToken: token,
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
      if mapped == .ready || mapped == .failed || mapped == .disconnected {
        let pending = lock.withLock { () -> FloatViewHostCommand? in
          let pending = pendingConnect
          pendingConnect = nil
          if mapped == .failed || mapped == .disconnected { activeDeviceID = nil }
          return pending
        }
        if let pending {
          complete(pending, resultKind: mapped == .ready ? 0 : (mapped == .failed ? 4 : 1))
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

  public func updateLoggingService(active: Bool) {
    submit(.loggingService(active: active))
  }

  public func updateTheme(_ theme: FloatViewThemePreference) {
    submit(.theme(theme))
  }

  /// Cancels host work before the Rust runtime is stopped and its waiter wakes.
  public func shutdown() {
    let state = lock.withLock {
      () -> (DeviceID?, FloatViewHostCommand?, [FloatViewHostCommand]) in
      guard !stopped else { return (nil, nil, []) }
      stopped = true
      locationActive = false
      let state = (activeDeviceID, pendingConnect, Array(pendingPresentations.values))
      activeDeviceID = nil
      pendingConnect = nil
      pendingPresentations.removeAll(keepingCapacity: false)
      return state
    }
    bluetooth.stopDiscovery()
    if let deviceID = state.0 { bluetooth.disconnect(deviceID) }
    location.stopUpdates()
    platformHost.cancelPendingPresentation()
    if let pending = state.1 { complete(pending, resultKind: 1) }
    for pending in state.2 {
      complete(pending, resultKind: 1)
    }
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
      let payload = try FloatViewPlatformEventEnvelope(
        runtimeGeneration: runtime.generation,
        connectionGeneration: context.connection,
        adapterOrder: context.order,
        event: event
      ).encode()
      try runtime.submitPlatformEvent(payload)
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

  private func opaqueToken(for deviceID: DeviceID) -> Data {
    lock.withLock {
      if let token = tokenByDeviceID[deviceID] { return token }
      var uuid = UUID().uuid
      let token = withUnsafeBytes(of: &uuid) { Data($0) }
      tokenByDeviceID[deviceID] = token
      deviceIDByToken[token] = deviceID
      return token
    }
  }

  private func beginPresentation(_ command: FloatViewHostCommand) -> Bool {
    lock.withLock {
      guard !stopped, pendingPresentations.count < 4,
            pendingPresentations[command.operationID] == nil
      else { return false }
      pendingPresentations[command.operationID] = command
      return true
    }
  }

  private func finishPresentation(
    _ command: FloatViewHostCommand,
    result: Result<Void, ApplePlatformHostError>
  ) {
    let accepted = lock.withLock {
      pendingPresentations.removeValue(forKey: command.operationID) != nil && !stopped
    }
    guard accepted else { return }
    switch result {
    case .success: complete(command)
    case let .failure(error): complete(command, resultKind: Self.resultKind(error))
    }
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
    default: .connecting
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

  private static func resultKind(_ error: Error) -> UInt32 {
    switch error as? ApplePlatformHostError {
    case .payloadTooLarge, .invalidPath, .ioFailure: 4
    case .unavailable: 3
    case nil: 4
    }
  }

  private static func fileURL(_ path: String) -> URL? {
    guard !path.isEmpty, path.utf8.count <= 1024, path.hasPrefix("/") else { return nil }
    return URL(fileURLWithPath: path).standardizedFileURL
  }

  private static func u64(_ value: UInt64) -> Data {
    var value = value.bigEndian
    return withUnsafeBytes(of: &value) { Data($0) }
  }

  private static func i64(_ value: Int64) -> Data {
    u64(UInt64(bitPattern: value))
  }
}
