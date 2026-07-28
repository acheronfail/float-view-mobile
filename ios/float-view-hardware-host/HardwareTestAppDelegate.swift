import UIKit

@main
final class HardwareTestAppDelegate: UIResponder, UIApplicationDelegate {
  private var conformanceServer: LoopbackServer?
  private var conformanceRoot: URL?

  func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
  ) -> Bool {
    if CommandLine.arguments.contains("--conformance-host") {
      do {
        try startConformanceHost()
      } catch {
        FileHandle.standardError.write(Data("Conformance host failed: \(error)\n".utf8))
        return false
      }
    }
    return true
  }

  private func startConformanceHost() throws {
    let root = FileManager.default.temporaryDirectory
      .appending(path: "float-view-r5-ios-\(UUID().uuidString)", directoryHint: .isDirectory)
    let schemaDirectory = Bundle.main.resourceURL!.appending(path: "sql", directoryHint: .isDirectory)
    let store = try SQLiteLogStore(
      root: root,
      schemaDirectory: schemaDirectory,
      availableBytes: { Int64.max }
    )
    let seed = try store.create(
      deviceID: DeviceID("conformance-profile-fixture"),
      at: Date(timeIntervalSince1970: 0)
    )
    var samples: [TelemetrySample] = []
    for index in 0..<60 {
      let value = Double(index)
      samples.append(TelemetrySample(
        sequence: Int64(index + 1),
        observedAt: Date(timeIntervalSince1970: value),
        elapsedMs: Int64(index * 1_000),
        rideState: index < 5 ? .idle : .riding,
        packVoltageV: 60 - value / 20,
        dutyCycle: value / 100,
        controllerTempC: 30 + value / 10,
        motorTempC: 28 + value / 10,
        footpadState: index < 5 ? 0 : 3,
        extensionFields: [
          "motorCurrentA": value,
          "batteryCurrentA": value / 2,
          "erpm": Double(index * 100),
          "speedKph": value / 3,
        ]
      ))
    }
    try store.append(jobID: seed.jobID, samples: samples)
    _ = try store.close(
      jobID: seed.jobID,
      at: Date(timeIntervalSince1970: 60),
      reason: "test_seed"
    )
    let service = ApplicationService(
      bluetooth: ConformanceBluetooth(),
      clock: ConformanceClock(),
      logStore: store,
      productStore: store,
      sessionStore: store,
      events: ConformanceEventSink()
    )
    let server = LoopbackServer(
      webRoot: root,
      platform: "ios",
      applicationService: service
    )
    try server.start()
    conformanceRoot = root
    conformanceServer = server
    let readiness: [String: Any] = [
      "webSocketUrl": "ws://127.0.0.1:\(server.port)/api/v1/ws",
      "sessionToken": server.token,
      "origin": server.origin,
      "savedDeviceId": try store.savedDevices().first!.savedDeviceID,
    ]
    let data = try JSONSerialization.data(withJSONObject: readiness)
    FileHandle.standardOutput.write(data + Data("\n".utf8))
  }
}

private final class ConformanceBluetooth: BluetoothPort, @unchecked Sendable {
  func currentState() -> BluetoothState {
    BluetoothState(radio: .poweredOn, scanPermission: .granted, connectPermission: .granted)
  }

  func requestPermission() {}
  func startDiscovery() {}
  func stopDiscovery() {}
  func connect(_ deviceID: DeviceID) {}
  func write(_ bytes: Data, to deviceID: DeviceID) {}
  func disconnect(_ deviceID: DeviceID) {}
}

private struct ConformanceClock: ClockPort {
  func monotonicMilliseconds() -> Int64 { 0 }
  func now() -> Date { Date(timeIntervalSince1970: 0) }
}

private struct ConformanceEventSink: EventSink {
  func publish(_ event: any Sendable) {}
}
