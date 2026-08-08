import SwiftUI

struct FrontendEndpoint {
  let origin: URL
  let generation: Int
}

@MainActor
final class RustAppBackend: ObservableObject {
  @Published private(set) var frontend: FrontendEndpoint?
  private var owner: IOSRustRuntimeOwner?
  private var generation = 0

  init() {
    owner = try? IOSRustRuntimeOwner { [weak self] endpoint, token in
      Task { @MainActor [weak self] in
        guard let self, let endpoint = authenticatedLaunchURL(endpoint: endpoint, token: token) else {
          return
        }
        generation += 1
        frontend = FrontendEndpoint(origin: endpoint, generation: generation)
      }
    }
    owner?.updateLifecycle(foreground: true)
  }

  func update(_ phase: ScenePhase) {
    owner?.updateLifecycle(foreground: phase != .background)
    if phase == .background { frontend = nil }
  }

  func recoverFrontend() {
    owner?.updateLifecycle(foreground: false)
    owner?.updateLifecycle(foreground: true)
  }

  deinit { owner?.shutdown() }
}

struct RustContentView: View {
  @StateObject private var backend = RustAppBackend()
  @Environment(\.scenePhase) private var scenePhase

  var body: some View {
    Group {
      if let frontend = backend.frontend {
        WebView(url: frontend.origin, onContentProcessTerminated: backend.recoverFrontend)
          .id(frontend.generation)
          .ignoresSafeArea()
      } else {
        ContentUnavailableView("Just a sec mate...", systemImage: "gauge.with.dots.needle.33percent")
      }
    }
    .onChange(of: scenePhase, initial: true) { _, phase in backend.update(phase) }
  }
}

private final class IOSRustRuntimeOwner: @unchecked Sendable {
  private let runtime: FloatViewCoreRuntime
  private let adapter: RustCorePlatformAdapter
  private let relay: RustBluetoothRelay
  private let presentation: AppleSystemPresentation

  init(readiness: @escaping RustCorePlatformAdapter.RuntimeReadyHandler) throws {
    let runtime = try FloatViewCoreRuntime()
    let relay = RustBluetoothRelay()
    #if targetEnvironment(simulator)
      let softwareTransport = true
      let bluetooth: any BluetoothPort = InactiveBluetoothPort()
    #else
      let softwareTransport = false
      let bluetooth: any BluetoothPort = CoreBluetoothAdapter(
        eventHandler: { event in relay.emit(event) },
        diagnosticHandler: { diagnostic in
          #if DEBUG
            print("[FloatViewCore] adapter \(diagnostic)")
          #endif
        },
        restorationIdentifier: "fail.acheron.float-view.rust-bluetooth-central"
      )
    #endif
    let presentation = AppleSystemPresentation()
    let host = try ApplePlatformHostServices.standard(presentation: presentation)
    let adapter = RustCorePlatformAdapter(
      runtime: runtime,
      bluetooth: bluetooth,
      location: CoreLocationAdapter(),
      platformHost: host,
      runtimeReadyHandler: readiness
    )
    self.runtime = runtime
    self.adapter = adapter
    self.relay = relay
    self.presentation = presentation
    let requestedPort = ProcessInfo.processInfo.environment["FLOAT_VIEW_CONFORMANCE_PORT"]
      .flatMap(UInt16.init) ?? 0
    let index = Bundle.main.url(forResource: "index", withExtension: "html")!
    try runtime.startConfigured(
      requestedPort: requestedPort,
      softwareTransport: softwareTransport,
      storageRoot: host.durableStorageRoot(),
      frontendIndex: Data(contentsOf: index)
    ) { command in adapter.dispatch(command) }
    relay.setHandler { event in adapter.receiveBluetooth(event) }
  }

  func updateLifecycle(foreground: Bool) {
    adapter.updateLifecycle(foreground: foreground, backgroundEligible: true)
  }

  func shutdown() {
    adapter.shutdown()
    try? runtime.stop()
  }
}

private func authenticatedLaunchURL(endpoint: URL, token: String) -> URL? {
  guard var components = URLComponents(url: endpoint, resolvingAgainstBaseURL: false) else {
    return nil
  }
  var items = components.queryItems ?? []
  items.removeAll { $0.name == "token" }
  items.append(URLQueryItem(name: "token", value: token))
  components.queryItems = items
  return components.url
}

private final class RustBluetoothRelay: @unchecked Sendable {
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
