import Foundation
import SwiftUI

@MainActor
final class AppBackend: ObservableObject {
  private(set) var service: SharedService?
  let rideActivity = RideActivityController()
  @Published var frontend: FrontendEndpoint?
  @Published private(set) var startupFailure: StartupFailure?
  @Published private(set) var themePreference: ThemePreference {
    didSet { rideActivity.setThemePreference(themePreference) }
  }

  var frontendServer: LoopbackServer?
  var frontendGeneration = 0

  init() {
    themePreference = .system
    do {
      #if DEBUG
        try Self.resetConformanceStorage()
      #endif
      startOwner()
    } catch {
      startupFailure = .data
    }
  }

  func retryStartup() {
    if service == nil {
      startOwner()
    } else {
      recoverFrontend()
    }
  }

  private func startOwner() {
    guard service == nil else { return }
    startupFailure = nil
    var preparedService: SharedService?
    do {
      let relay = BluetoothEventRelay()
      #if FLOAT_VIEW_CONFORMANCE && DEBUG && targetEnvironment(simulator)
        let bluetooth: any BluetoothPort = FixtureBluetoothAdapter(eventHandler: { @Sendable event in
          relay.emit(event)
        })
      #else
        let bluetooth: any BluetoothPort = CoreBluetoothAdapter(
          eventHandler: { @Sendable event in relay.emit(event) },
          diagnosticHandler: { diagnostic in
            #if DEBUG
              print("[FloatViewState] adapter \(diagnostic)")
            #endif
          },
          restorationIdentifier: "fail.acheron.floatview.bluetooth-central"
        )
      #endif
      let applicationSupport = try FileManager.default.url(
        for: .applicationSupportDirectory,
        in: .userDomainMask,
        appropriateFor: nil,
        create: true
      )
      let logRoot = applicationSupport
        .appendingPathComponent("FloatView", isDirectory: true)
        .appendingPathComponent("Sessions", isDirectory: true)
      let service = try SharedService(root: logRoot, bluetooth: bluetooth, location: CoreLocationAdapter())
      relay.setHandler { [weak service] event in service?.receive(event) }
      preparedService = service
      try service.start(presenter: { @Sendable value in IOSExportPresenter.schedule(value) },
                        liveActivity: { [rideActivity] state in
                          rideActivity.receive(state)
                        }) { [weak self] preference in
        Task { @MainActor [weak self] in self?.themePreference = preference }
      }
      self.service = service
      activateFrontend()
    } catch {
      preparedService?.shutdown()
      frontendServer = nil
      frontend = nil
      startupFailure = .data
    }
  }

  func activateFrontend() {
    guard let service else { return }
    startupFailure = nil
    var stage = StartupFailure.interface
    do {
      if frontendServer == nil {
        guard let htmlURL = Bundle.main.url(forResource: "index", withExtension: "html") else {
          throw CocoaError(.fileNoSuchFile)
        }
        let html = try Data(contentsOf: htmlURL)
        frontendServer = LoopbackServer(html: html, port: PlatformLoopbackPort.iOS, service: service, platform: "ios")
      }
      guard let frontendServer else { return }
      stage = .connection
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
      startupFailure = stage
      frontendServer?.stop()
      frontendServer = nil
      frontend = nil
      #if DEBUG
        let message = loopbackStartupFailure(
          port: PlatformLoopbackPort.iOS,
          error: error
        )
        print("[FloatViewState] frontend state=failed \(message)")
      #endif
    }
  }

  deinit {
    frontendServer?.stop()
  }

  #if DEBUG
    private static func resetConformanceStorage() throws {
      guard ProcessInfo.processInfo.environment["FLOAT_VIEW_CONFORMANCE_RESET"] == "1" else {
        return
      }
      let applicationSupport = try FileManager.default.url(
        for: .applicationSupportDirectory,
        in: .userDomainMask,
        appropriateFor: nil,
        create: true
      )
      let root = applicationSupport.appendingPathComponent("FloatView", isDirectory: true)
      if FileManager.default.fileExists(atPath: root.path) {
        try FileManager.default.removeItem(at: root)
      }
    }
  #endif
}

struct FrontendEndpoint {
  let origin: URL
  let generation: Int
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
