import SwiftUI

final class AppBackend: ObservableObject {
  let server: LoopbackServer
  let bluetooth: CoreBluetoothAdapter
  let applicationService: ApplicationService
  @Published var url: URL?

  init() {
    let relay = BluetoothEventRelay()
    let bluetooth = CoreBluetoothAdapter(eventHandler: { @Sendable event in relay.emit(event) })
    let applicationSupport = try! FileManager.default.url(
      for: .applicationSupportDirectory,
      in: .userDomainMask,
      appropriateFor: nil,
      create: true
    )
    let logRoot = applicationSupport
      .appendingPathComponent("FloatView", isDirectory: true)
      .appendingPathComponent("Sessions", isDirectory: true)
    let schemaDirectory = Bundle.main.resourceURL!.appendingPathComponent("sql", isDirectory: true)
    let logStore = try! SQLiteLogStore(
      root: logRoot,
      schemaDirectory: schemaDirectory,
      availableBytes: {
        (try? logRoot.resourceValues(
          forKeys: [.volumeAvailableCapacityForImportantUsageKey]
        ).volumeAvailableCapacityForImportantUsage).map { Int64($0) } ?? Int64.max
      }
    )
    let applicationService = ApplicationService(
      bluetooth: bluetooth,
      clock: SystemClock(),
      logStore: logStore,
      protocolCodec: VescProtocolService(),
      scheduler: SystemScheduler(),
      events: IgnoreEvents()
    )
    relay.setHandler { [weak applicationService] event in
      try? applicationService?.receive(event)
    }

    let webRoot = Bundle.main.resourceURL!.appendingPathComponent("build", isDirectory: true)
    self.bluetooth = bluetooth
    self.applicationService = applicationService
    server = LoopbackServer(
      webRoot: webRoot,
      platform: "ios",
      applicationService: applicationService
    )
    do {
      try server.start()
      url = URL(string: server.origin)
    } catch {
      url = nil
    }
  }

  deinit {
    server.stop()
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

  func now() -> Date { Date() }
}

private struct IgnoreEvents: EventSink {
  func publish(_: any Sendable) {}
}

struct ContentView: View {
  @StateObject private var backend = AppBackend()

  var body: some View {
    if let url = backend.url {
      WebView(url: url).ignoresSafeArea()
    } else {
      ContentUnavailableView("Backend unavailable", systemImage: "exclamationmark.triangle")
    }
  }
}
