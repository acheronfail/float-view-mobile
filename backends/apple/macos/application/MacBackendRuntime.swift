import FloatViewBackend
import Foundation

func trace(_ message: String) {
  #if DEBUG
    FileHandle.standardError.write(Data("\(message)\n".utf8))
  #endif
}

final class MacBackendRuntime: @unchecked Sendable {
  private let bluetooth: any BluetoothPort
  private let applicationService: ApplicationService
  private let html: Data
  private let events: EventHub
  private let eventSubscription: EventSubscription
  let testSavedDeviceID: String?

  init(html: Data) {
    let relay = BluetoothEventRelay()
    let bluetooth: any BluetoothPort
    if ProcessInfo.processInfo.environment["FLOAT_VIEW_SOFTWARE_BACKEND"] == "1" {
      let peripheral = SoftwareVescPeripheral()
      bluetooth = SoftwareBluetoothAdapter(
        eventHandler: { @Sendable event in relay.emit(event) },
        responseHandler: { @Sendable bytes in peripheral.response(to: bytes) }
      )
    } else {
      bluetooth = CoreBluetoothAdapter(
        eventHandler: { @Sendable event in relay.emit(event) },
        diagnosticHandler: { diagnostic in
          trace("[FloatViewState] adapter \(diagnostic)")
        }
      )
    }
    let logRoot: URL
    let testLogRoot = ProcessInfo.processInfo.environment["FLOAT_VIEW_LOG_ROOT"]
    if let override = testLogRoot {
      logRoot = URL(fileURLWithPath: override, isDirectory: true)
    } else {
      let applicationSupport = try! FileManager.default.url(
        for: .applicationSupportDirectory,
        in: .userDomainMask,
        appropriateFor: nil,
        create: true
      )
      logRoot = applicationSupport
        .appending(path: "FloatView")
        .appending(path: "Sessions")
    }
    let bundledSchemaDirectory = Bundle.module.resourceURL!.appending(path: "sql")
    let schemaDirectory = ProcessInfo.processInfo.environment["FLOAT_VIEW_SQL_ROOT"]
      .map(URL.init(fileURLWithPath:))
      ?? bundledSchemaDirectory
    let logStore = try! SQLiteLogStore(
      root: logRoot,
      schemaDirectory: schemaDirectory,
      availableBytes: {
        if testLogRoot != nil { return Int64.max }
        return (try? logRoot.resourceValues(
          forKeys: [.volumeAvailableCapacityForImportantUsageKey]
        ).volumeAvailableCapacityForImportantUsage).map { Int64($0) } ?? Int64.max
      }
    )
    #if DEBUG
      let recoveredSessions = try! logStore.recoverInterrupted()
      trace("[FloatViewState] recovery interruptedSessions=\(recoveredSessions)")
    #else
      try! logStore.recoverInterrupted()
    #endif
    if ProcessInfo.processInfo.environment["FLOAT_VIEW_TEST_SEED_PROFILE"] == "1" {
      let seed = try! logStore.create(
        deviceID: DeviceID("conformance-profile-fixture"),
        at: Date(timeIntervalSince1970: 0)
      )
      var seedSamples: [TelemetrySample] = []
      for index in 0 ..< 60 {
        seedSamples.append(
          TelemetrySample(
            sequence: Int64(index + 1),
            observedAt: Date(timeIntervalSince1970: Double(index)),
            elapsedMs: Int64(index * 1_000),
            rideState: index < 5 ? .idle : .riding,
            footpadState: index < 5 ? FootpadState.none : .both,
            packVoltageV: 60 - Double(index) / 20,
            dutyCycle: Double(index) / 100,
            controllerTempC: 30 + Double(index) / 10,
            motorTempC: 28 + Double(index) / 10,
            motorCurrentA: Double(index),
            batteryCurrentA: Double(index) / 2,
            erpm: Double(index * 100),
            speedKph: Double(index) / 3
          )
        )
      }
      try! logStore.append(jobID: seed.jobID, samples: seedSamples)
      _ = try! logStore.close(
        jobID: seed.jobID,
        at: Date(timeIntervalSince1970: 60),
        reason: "test_seed"
      )
      testSavedDeviceID = try! logStore.savedDevices().first!.savedDeviceID
    } else {
      testSavedDeviceID = nil
    }
    let settings = try! logStore.settings()
    let events = EventHub()
    let debugEvents = DebugEventSink()
    let applicationService = ApplicationService(
      bluetooth: bluetooth,
      clock: SystemClock(),
      logStore: logStore,
      productStore: logStore,
      sessionStore: logStore,
      scheduler: SystemScheduler(),
      events: events,
      samplingSettings: SamplingSettings(
        idleProbeIntervalMs: settings.idleProbeIntervalMs,
        idlePersistIntervalMs: settings.idlePersistIntervalMs,
        ridingPersistIntervalMs: settings.ridingPersistIntervalMs,
        postRideTailMs: settings.postRideTailMs,
        reconnectDurationMs: settings.reconnectDurationMs
      ),
      locationProvider: CoreLocationAdapter()
    )
    relay.setHandler { [weak applicationService] event in
      try? applicationService?.receive(event)
    }
    self.bluetooth = bluetooth
    self.applicationService = applicationService
    self.html = html
    self.events = events
    eventSubscription = events.subscribe { debugEvents.publish($0) }
  }

  func makeServer(
    exportPresenter: (@Sendable (SessionExportRecord) -> Bool)? = nil
  ) -> LoopbackServer {
    let port = ProcessInfo.processInfo.environment["FLOAT_VIEW_CONFORMANCE_PORT"]
      .flatMap(UInt16.init)
      ?? PlatformLoopbackPort.macOS
    return LoopbackServer(
      html: html,
      port: port,
      applicationService: applicationService,
      exportPresenter: exportPresenter,
      events: events
    )
  }

  func shutdown() {
    try? applicationService.disconnect()
  }
}

private final class BluetoothEventRelay: @unchecked Sendable {
  private let lock = NSLock()
  private var handler: (@Sendable (BluetoothEvent) -> Void)?

  func setHandler(_ value: @escaping @Sendable (BluetoothEvent) -> Void) {
    lock.lock()
    handler = value
    lock.unlock()
  }

  func emit(_ event: BluetoothEvent) {
    lock.lock()
    let current = handler
    lock.unlock()
    current?(event)
  }
}

private struct SystemClock: ClockPort {
  func monotonicMilliseconds() -> Int64 {
    Int64(ProcessInfo.processInfo.systemUptime * 1_000)
  }

  func now() -> Date {
    Date()
  }
}

private struct DebugEventSink: EventSink {
  func publish(_ event: any Sendable) {
    #if DEBUG
      switch event {
      case let state as BluetoothState:
        trace(
          "[FloatViewState] bluetooth radio=\(state.radio.rawValue) " +
            "scanPermission=\(state.scanPermission.rawValue) " +
            "connectPermission=\(state.connectPermission.rawValue)"
        )
      case let state as ConnectionState:
        trace("[FloatViewState] connection state=\(state.rawValue)")
      case let session as SessionRecord:
        trace("[FloatViewState] session active=\(session.active)")
      case let message as String:
        trace("[FloatViewBackend] \(message)")
      default:
        break
      }
    #endif
  }
}
