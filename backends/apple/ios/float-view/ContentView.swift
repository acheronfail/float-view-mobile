import SwiftUI
import UIKit

@MainActor
final class AppBackend: ObservableObject {
  let bluetooth: any BluetoothPort
  let applicationService: ApplicationService
  @Published private(set) var frontend: FrontendEndpoint?
  private var frontendServer: LoopbackServer?
  private var frontendGeneration = 0
  private let eventSubscription: EventSubscription

  init() {
    let relay = BluetoothEventRelay()
    #if targetEnvironment(simulator)
    let bluetooth: any BluetoothPort = SoftwareBluetoothAdapter(
      eventHandler: { @Sendable event in relay.emit(event) }
    )
    #else
    let bluetooth: any BluetoothPort = CoreBluetoothAdapter(
      eventHandler: { @Sendable event in relay.emit(event) },
      diagnosticHandler: { diagnostic in
        #if DEBUG
        print("[FloatViewState] adapter \(diagnostic)")
        #endif
      },
      restorationIdentifier: "fail.acheron.float-view.bluetooth-central"
    )
    #endif
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
    let settings = try! logStore.settings()
    let events = EventHub()
    let debugEvents = DebugEventSink()
    let applicationService = ApplicationService(
      bluetooth: bluetooth,
      clock: SystemClock(),
      logStore: logStore,
      productStore: logStore,
      sessionStore: logStore,
      protocolCodec: VescProtocolService(),
      scheduler: SystemScheduler(),
      events: events,
      samplingSettings: SamplingSettings(
        idleProbeIntervalMs: settings.idleProbeIntervalMs,
        idlePersistIntervalMs: settings.idlePersistIntervalMs,
        ridingPersistIntervalMs: settings.ridingPersistIntervalMs,
        postRideTailMs: settings.postRideTailMs,
        reconnectDurationMs: settings.reconnectDurationMs
      )
    )
    relay.setHandler { [weak applicationService] event in
      try? applicationService?.receive(event)
    }

    let html = try! Data(contentsOf: Bundle.main.url(forResource: "index", withExtension: "html")!)
    self.bluetooth = bluetooth
    self.applicationService = applicationService
    eventSubscription = events.subscribe { debugEvents.publish($0) }
    frontendServer = LoopbackServer(
      html: html,
      port: ProcessInfo.processInfo.environment["FLOAT_VIEW_CONFORMANCE_PORT"]
        .flatMap(UInt16.init) ?? 0,
      applicationService: applicationService,
      exportPresenter: { @Sendable value in IOSExportPresenter.schedule(value) },
      events: events
    )
    activateFrontend()
  }

  func enterBackground() {
    guard frontend != nil else { return }
    #if DEBUG
    print("[FloatViewState] frontend state=stopped reason=background")
    #endif
    frontend = nil
    frontendServer?.stop()
  }

  func enterForeground() {
    guard frontend == nil else { return }
    activateFrontend()
  }

  func recoverFrontend() {
    frontend = nil
    frontendServer?.stop()
    activateFrontend()
  }

  private func activateFrontend() {
    do {
      guard let frontendServer else { return }
      try frontendServer.start()
      frontendGeneration += 1
      frontend = FrontendEndpoint(
        origin: URL(string: frontendServer.origin)!,
        generation: frontendGeneration
      )
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
    frontendServer?.stop()
  }
}

struct FrontendEndpoint {
  let origin: URL
  let generation: Int
}

private enum IOSExportPresenter {
  static func schedule(_ value: SessionExportRecord) -> Bool {
    DispatchQueue.main.async {
      present(value.fileURL)
    }
    return true
  }

  @MainActor
  private static func present(_ fileURL: URL) {
    guard let scene = UIApplication.shared.connectedScenes
      .compactMap({ $0 as? UIWindowScene })
      .first(where: { $0.activationState == .foregroundActive }),
      let root = scene.windows.first(where: \.isKeyWindow)?.rootViewController
    else { return }
    var presenter = root
    while let presented = presenter.presentedViewController {
      presenter = presented
    }
    let controller = UIActivityViewController(activityItems: [fileURL], applicationActivities: nil)
    if let popover = controller.popoverPresentationController {
      popover.sourceView = presenter.view
      popover.sourceRect = CGRect(
        x: presenter.view.bounds.midX,
        y: presenter.view.bounds.midY,
        width: 1,
        height: 1
      )
    }
    presenter.present(controller, animated: true)
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
