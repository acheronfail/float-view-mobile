import SwiftUI

@MainActor
final class AppBackend: ObservableObject {
  let bluetooth: CoreBluetoothAdapter
  let applicationService: ApplicationService
  @Published private(set) var frontend: FrontendEndpoint?
  private let frontendLifecycle: FrontendLifecycleCoordinator

  init() {
    let relay = BluetoothEventRelay()
    let bluetooth = CoreBluetoothAdapter(
      eventHandler: { @Sendable event in relay.emit(event) },
      diagnosticHandler: { diagnostic in
        #if DEBUG
        print("[FloatViewState] adapter \(diagnostic)")
        #endif
      },
      restorationIdentifier: "fail.acheron.float-view.bluetooth-central"
    )
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
    #if DEBUG
    let recoveredSessions = try! logStore.recoverInterrupted()
    print("[FloatViewState] recovery interruptedSessions=\(recoveredSessions)")
    #else
    try! logStore.recoverInterrupted()
    #endif
    let applicationService = ApplicationService(
      bluetooth: bluetooth,
      clock: SystemClock(),
      logStore: logStore,
      protocolCodec: VescProtocolService(),
      scheduler: SystemScheduler(),
      events: DebugEventSink()
    )
    relay.setHandler { [weak applicationService] event in
      try? applicationService?.receive(event)
    }

    let webRoot = Bundle.main.resourceURL!.appendingPathComponent("build", isDirectory: true)
    self.bluetooth = bluetooth
    self.applicationService = applicationService
    let frontendLifecycle = FrontendLifecycleCoordinator {
      LoopbackServer(
        webRoot: webRoot,
        platform: "ios",
        applicationService: applicationService
      )
    }
    self.frontendLifecycle = frontendLifecycle
    activateFrontend()
  }

  func enterBackground() {
    guard frontend != nil else { return }
    #if DEBUG
    print("[FloatViewState] frontend state=stopped reason=background")
    #endif
    frontend = nil
    frontendLifecycle.deactivate()
  }

  func enterForeground() {
    guard frontend == nil else { return }
    activateFrontend()
  }

  func recoverFrontend() {
    frontend = nil
    frontendLifecycle.deactivate()
    activateFrontend()
  }

  private func activateFrontend() {
    do {
      frontend = try frontendLifecycle.activate()
      #if DEBUG
      print("[FloatViewState] frontend state=ready generation=\(frontend!.generation)")
      #endif
    } catch {
      frontend = nil
      #if DEBUG
      print("[FloatViewState] frontend state=failed")
      #endif
    }
  }

  deinit {
    frontendLifecycle.deactivate()
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

private struct DebugEventSink: EventSink {
  func publish(_ event: any Sendable) {
    #if DEBUG
    switch event {
    case let state as BluetoothState:
      print(
        "[FloatViewState] bluetooth radio=\(state.radio.rawValue) " +
          "scanPermission=\(state.scanPermission.rawValue) " +
          "connectPermission=\(state.connectPermission.rawValue)"
      )
    case let state as ConnectionState:
      print("[FloatViewState] connection state=\(state.rawValue)")
    case let session as SessionRecord:
      print("[FloatViewState] session active=\(session.active)")
    default:
      break
    }
    #endif
  }
}

struct ContentView: View {
  @StateObject private var backend = AppBackend()
  @Environment(\.scenePhase) private var scenePhase

  var body: some View {
    Group {
      if let frontend = backend.frontend {
        WebView(url: frontend.origin, onContentProcessTerminated: backend.recoverFrontend)
          .id(frontend.generation)
          .ignoresSafeArea()
      } else if scenePhase == .background {
        Color.black.ignoresSafeArea()
      } else {
        ContentUnavailableView("Backend unavailable", systemImage: "exclamationmark.triangle")
      }
    }
    .onChange(of: scenePhase, initial: true) { _, phase in
      switch phase {
      case .active:
        backend.enterForeground()
      case .background:
        backend.enterBackground()
      case .inactive:
        break
      @unknown default:
        break
      }
    }
  }
}
