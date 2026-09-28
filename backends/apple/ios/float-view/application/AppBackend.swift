import Foundation
import SwiftUI

@MainActor
final class AppBackend: ObservableObject {
  let bluetooth: any BluetoothPort
  let service: SharedService
  @Published var frontend: FrontendEndpoint?
  @Published private(set) var themePreference: ThemePreference
  var frontendServer: LoopbackServer?
  var frontendGeneration = 0

  init() {
    #if DEBUG
      _ = Self.conformanceStorageReset
    #endif
    let relay = BluetoothEventRelay()
    #if FLOAT_VIEW_CONFORMANCE && DEBUG && targetEnvironment(simulator)
      let bluetooth: any BluetoothPort = FixtureBluetoothAdapter(eventHandler: { @Sendable event in relay.emit(event) })
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
    let service = try! SharedService(root: logRoot, bluetooth: bluetooth, location: CoreLocationAdapter())
    relay.setHandler { [weak service] event in service?.receive(event) }
    let html = try! Data(contentsOf: Bundle.main.url(forResource: "index", withExtension: "html")!)
    self.bluetooth = bluetooth
    self.service = service
    themePreference = .system
    frontendServer = LoopbackServer(html: html, port: PlatformLoopbackPort.iOS, service: service, platform: "ios")
    try! service.start(presenter: { @Sendable value in IOSExportPresenter.schedule(value) }) { [weak self] preference in
      Task { @MainActor [weak self] in self?.themePreference = preference }
    }
    activateFrontend()
  }

  func activateFrontend() {
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
    private static let conformanceStorageReset: Void = {
      guard ProcessInfo.processInfo.environment["FLOAT_VIEW_CONFORMANCE_RESET"] == "1" else {
        return
      }
      let applicationSupport = try! FileManager.default.url(
        for: .applicationSupportDirectory,
        in: .userDomainMask,
        appropriateFor: nil,
        create: true
      )
      let root = applicationSupport.appendingPathComponent("FloatView", isDirectory: true)
      if FileManager.default.fileExists(atPath: root.path) {
        try! FileManager.default.removeItem(at: root)
      }
    }()
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
