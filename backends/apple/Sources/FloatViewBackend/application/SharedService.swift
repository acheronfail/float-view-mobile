import CFloatViewApplicationRuntime
import Foundation

public struct NativeFilePresentation: Sendable {
  public let fileURL: URL
  public let fileName: String
  public let finish: @Sendable () -> Void
}

/// Executes OS capabilities only. Rust owns all product state and policy.
public final class SharedService: @unchecked Sendable {
  let handle: UInt64
  private let bluetooth: any BluetoothPort
  private let location: any LocationProviderPort
  private let root: URL
  private let queue = DispatchQueue(label: "float-view.os-capabilities")
  private let queueKey = DispatchSpecificKey<Bool>()
  private let admission = DispatchSemaphore(value: 64)
  private let callbackLock = NSLock()
  private var callbackGeneration: UInt64 = 0
  private var callbackScan: UInt64 = 0
  private var overflow = false
  private var foreground: Bool?
  private var completedPresentations: Set<UInt64> = []
  private var lifetime: UInt64 = 0
  private var sequence: UInt64 = 0
  private var activeDevice: DeviceID?
  private var timers: [UInt64: (UInt64, Bool)] = [:]
  private var poll: DispatchSourceTimer?
  private var lastCapabilities = Date.distantPast
  private var presenter: (@Sendable (NativeFilePresentation) -> Bool)?
  private var themeHandler: (@Sendable (ThemePreference) -> Void)?
  private var stopped = false

  public init(root: URL, bluetooth: any BluetoothPort, location: any LocationProviderPort) throws {
    self.root = root
    self.bluetooth = bluetooth
    self.location = location
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    #if os(iOS)
      try FileManager.default.setAttributes(
        [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
        ofItemAtPath: root.path
      )
    #endif
    let path = Data(root.resolvingSymlinksInPath().path.utf8)
    var handle: UInt64 = 0
    let status = path.withUnsafeBytes {
      float_view_application_runtime_service_create($0.bindMemory(to: UInt8.self).baseAddress, $0.count, &handle)
    }
    guard status == 0, handle != 0 else { throw CocoaError(.fileReadUnknown) }
    self.handle = handle
    queue.setSpecific(key: queueKey, value: true)
  }

  public func start(presenter: (@Sendable (NativeFilePresentation) -> Bool)? = nil,
                    theme: (@Sendable (ThemePreference) -> Void)? = nil) throws
  {
    try queue.sync {
      self.presenter = presenter
      themeHandler = theme
      guard poll == nil else { return }
      try exchange(["command": "drain"])
      capabilities()
      let timer = DispatchSource.makeTimerSource(queue: queue)
      timer.schedule(deadline: .now(), repeating: .milliseconds(20), leeway: .milliseconds(5))
      timer.setEventHandler { [weak self] in self?.tick() }
      poll = timer
      timer.resume()
    }
  }

  public func setForeground(_ foreground: Bool) {
    callbackLock.withLock { self.foreground = foreground }
  }

  public func receive(_ event: BluetoothEvent) {
    let (generation, scan) = callbackLock.withLock { (callbackGeneration, callbackScan) }
    enqueue { service in
      switch event {
      case let .stateChanged(state): service.bluetoothFact(state)
      case let .deviceFound(id, name, rssi):
        service.fact(["fact": "discovered", "scan_id": scan, "device_id": id.value,
                      "name": name as Any? ?? NSNull(), "rssi": rssi as Any? ?? NSNull()])
      case .scanFailed: service.bluetoothFact(service.bluetooth.currentState())
      case let .connectionChanged(id, state):
        guard service.activeDevice == id else { return }
        let wire: String
        switch state {
        case .connecting: wire = "connecting"
        case .connected, .discoveringServices: wire = "discoveringServices"
        case .enablingNotifications: wire = "enablingNotifications"
        case .ready: wire = "ready"
        case .disconnected: wire = "disconnected"
        case .failed: wire = "failed"
        default: return
        }
        service.fact(["fact": "link", "generation": generation, "state": wire])
      case let .notification(id, bytes):
        guard service.activeDevice == id, bytes.count <= 8192 else { return }
        service.fact(["fact": "bytes", "generation": generation, "bytes": Array(bytes)])
      }
    }
  }

  private func enqueue(_ action: @escaping @Sendable (SharedService) -> Void) {
    guard admission.wait(timeout: .now()) == .success else {
      callbackLock.withLock { overflow = true }
      return
    }
    queue.async { [self] in
      defer { admission.signal() }
      guard !stopped else { return }
      action(self)
    }
  }

  private func tick() {
    guard !stopped else { return }
    if callbackLock.withLock({ let value = overflow; overflow = false; return value }) {
      fact(["fact": "link", "generation": callbackLock.withLock { callbackGeneration }, "state": "failed"])
    }
    let cleanup = callbackLock.withLock { () -> (Bool?, Set<UInt64>) in
      let result = (foreground, completedPresentations)
      foreground = nil
      completedPresentations.removeAll()
      return result
    }
    if let foreground = cleanup.0 {
      fact(["fact": "foreground", "foreground": foreground])
    }
    for id in cleanup.1 {
      fact(["fact": "presentation_finished", "id": id])
    }
    let now = UInt64(ProcessInfo.processInfo.systemUptime * 1000)
    let due = timers.filter { $0.value.0 <= now }.keys.sorted().prefix(64)
    for id in due {
      timers.removeValue(forKey: id)
      fact(["fact": "timer", "id": id])
    }
    fact(["fact": "clock"])
    if Date().timeIntervalSince(lastCapabilities) >= 1 {
      capabilities()
    }
    try? exchange(["command": "drain"])
  }

  private func capabilities() {
    lastCapabilities = Date()
    bluetoothFact(bluetooth.currentState())
    let state = location.state()
    fact(["fact": "location_capability", "authorization": state.authorization.rawValue,
          "background": state.backgroundCapture == .available])
    let available = (try? root.resourceValues(forKeys: [.volumeAvailableCapacityForImportantUsageKey])
      .volumeAvailableCapacityForImportantUsage) ?? 0
    fact(["fact": "storage", "available_bytes": max(0, available)])
  }

  private func bluetoothFact(_ state: BluetoothState) {
    func wire(_ raw: String) -> String {
      switch raw {
      case "powered_on": "poweredOn"
      case "powered_off": "poweredOff"
      case "not_required": "notRequired"
      case "not_determined": "notDetermined"
      default: raw
      }
    }
    fact(["fact": "bluetooth", "radio": wire(state.radio.rawValue),
          "scan": wire(state.scanPermission.rawValue), "connect": wire(state.connectPermission.rawValue)])
  }

  private func fact(_ value: [String: Any]) {
    sequence += 1
    try? exchange(["command": "fact", "envelope": ["lifetime": lifetime, "sequence": sequence,
                                                   "monotonic_ms": UInt64(ProcessInfo.processInfo.systemUptime * 1000),
                                                   "epoch_ms": Int64(Date().timeIntervalSince1970 * 1000),
                                                   "fact": value]])
  }

  private func exchange(_ command: [String: Any]) throws {
    let input = try JSONSerialization.data(withJSONObject: command)
    guard input.count <= 1_048_576 else { throw CocoaError(.validationMissingMandatoryProperty) }
    var required = 0
    let status = input.withUnsafeBytes {
      float_view_application_runtime_service_apply(
        handle,
        $0.bindMemory(to: UInt8.self).baseAddress,
        $0.count,
        &required
      )
    }
    guard status == 0, required <= 1_048_576 else { throw CocoaError(.validationMissingMandatoryProperty) }
    var output = [UInt8](repeating: 0, count: required)
    let copied = output.withUnsafeMutableBufferPointer {
      float_view_application_runtime_service_copy_result(handle, $0.baseAddress, $0.count, &required)
    }
    guard copied == 0, let result = try JSONSerialization.jsonObject(with: Data(output)) as? [String: Any],
          let lifetime = result["lifetime"] as? NSNumber, let effects = result["effects"] as? [[String: Any]]
    else {
      throw CocoaError(.validationMissingMandatoryProperty)
    }
    self.lifetime = lifetime.uint64Value
    for effect in effects {
      execute(effect)
    }
  }

  private func execute(_ value: [String: Any]) {
    func number(_ key: String) -> UInt64 {
      (value[key] as? NSNumber)?.uint64Value ?? 0
    }
    let id = number("id")
    switch value["effect"] as? String {
    case "scan":
      callbackLock.withLock { callbackScan = number("scan_id") }
      bluetooth.startDiscovery()
    case "stop_scan": bluetooth.stopDiscovery()
    case "connect":
      guard let device = value["device_id"] as? String else { return }
      callbackLock.withLock { callbackGeneration = number("generation") }
      activeDevice = DeviceID(device)
      bluetooth.connect(DeviceID(device))
    case "disconnect":
      if let activeDevice {
        bluetooth.disconnect(activeDevice)
      }
      activeDevice = nil
    case "write":
      guard number("generation") == callbackLock.withLock({ callbackGeneration }),
            let activeDevice, let bytes = value["bytes"] as? [UInt8] else { return }
      bluetooth.write(Data(bytes), to: activeDevice)
    case "arm_timer":
      guard timers[id] != nil || timers.count < 2048 else { return }
      timers[id] = (
        UInt64(ProcessInfo.processInfo.systemUptime * 1000) + number("delay_ms"),
        value["connection"] as? Bool ?? false
      )
    case "cancel_timer": timers.removeValue(forKey: id)
    case "cancel_connection_timers":
      timers = timers.filter { !$0.value.1 }
    case "start_location":
      let capture = number("capture_id")
      location.startUpdates { [weak self] observation in
        self?.enqueue { service in
          service.fact(["fact": "location", "capture_id": capture, "latitude": observation.latitude,
                        "longitude": observation.longitude, "accuracy_m": observation.accuracyM as Any? ?? NSNull(),
                        "observed_at_ms": Int64(observation.observedAt.timeIntervalSince1970 * 1000)])
        }
      }
    case "stop_location": location.stopUpdates()
    case "request_bluetooth_permission":
      bluetooth.requestPermission(); completeUI(id)
    case "request_location_permission": location.requestPermission(); completeUI(id)
    case "open_location_settings": location.openSettings(); completeUI(id)
    case "present_file":
      guard let path = value["path"] as? String, let name = value["file_name"] as? String else { return }
      let once = PresentationCompletion()
      let finish: @Sendable () -> Void = { [weak self] in
        guard once.claim(), let self else { return }
        _ = self.callbackLock.withLock { self.completedPresentations.insert(id) }
      }
      let accepted = presenter?(NativeFilePresentation(
        fileURL: URL(fileURLWithPath: path),
        fileName: name,
        finish: finish
      )) == true
      fact(["fact": "presentation_started", "id": id, "accepted": accepted])
      if !accepted {
        finish()
      }
    case "host_state":
      if let ids = value["saved_device_ids"] as? [String] {
        bluetooth.configureSavedDeviceIDs(Set(ids.map(DeviceID.init)))
      }
      if let theme = value["theme_preference"] as? String,
         let preference = ThemePreference(rawValue: theme)
      {
        themeHandler?(preference)
      }
    default: break
    }
  }

  private func completeUI(_ id: UInt64) {
    capabilities()
    fact(["fact": "ui_completed", "id": id, "success": true])
  }

  public func shutdown() {
    let stop = { [self] in
      guard !stopped else { return }
      poll?.cancel(); poll = nil
      timers.removeAll()
      location.stopUpdates()
      bluetooth.stopDiscovery()
      if let activeDevice {
        bluetooth.disconnect(activeDevice)
      }
      stopped = float_view_application_runtime_service_destroy(handle) == 0
    }
    if DispatchQueue.getSpecific(key: queueKey) == true {
      stop()
    } else {
      queue.sync(execute: stop)
    }
  }

  deinit { shutdown() }
}

private final class PresentationCompletion: @unchecked Sendable {
  private let lock = NSLock()
  private var completed = false
  func claim() -> Bool {
    lock.withLock {
      guard !completed else { return false }
      completed = true
      return true
    }
  }
}
